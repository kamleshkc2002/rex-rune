//! Insert path: a captured pane, one coalesced read at a time, and one ordinary paste.
use super::{
    panel::{Entry, Intent, PaneTarget, PanelKind},
    workspace::Workspace,
};
use rex_linux::connection::{
    HostCommand,
    directory::{Failure, Request},
};
use rexrs::api::terminal::{ListDir, ListDirResponse};
use rune::{TextEdit, Tx};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub const ACTION: &str = "pane.insert_path";
/// The same browser, listing folders only: it types `cd <folder>` and leaves Enter to the
/// person, because the client cannot see what the prompt already holds.
pub const GO: &str = "pane.go_to_directory";

#[derive(Clone, Debug, Default)]
pub struct Browser {
    open: Arc<()>,
    pending: Option<(u64, String)>,
    epoch: Option<u64>,
    pub rows: Vec<PathRow>,
    pub truncated: bool,
    /// Go to directory rather than Insert path.
    pub go: bool,
    /// Once edited, this opening stays a plain directory listing, even if text is restored.
    edited: bool,
    recent_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathRow {
    pub label: String,
    pub path: String,
    pub directory: bool,
}

/// The final component filters its parent; a trailing slash lists the directory itself.
pub fn split_path(text: &str) -> (&str, &str) {
    if text == "~" {
        return ("~", "");
    }
    text.rfind('/')
        .map_or(("", text), |at| (&text[..=at], &text[at + 1..]))
}

// Shared with settings load so edited history uses exactly the browser's refusal rules.
pub use rex_linux::settings::recent_paths::refused;
use rex_linux::settings::recent_paths::{LIMIT, PATH_MAX, valid};
const REFUSED: &str = "Paths with control or invisible formatting characters can't be inserted.";

/// POSIX single quoting, never raw controls. Tilde is resolved by Rex before this call.
pub fn quote(path: &str) -> Result<String, &'static str> {
    if path.chars().any(refused) {
        return Err(REFUSED);
    }
    // A bare relative option-like name must remain a path when pasted after a command.
    let path = if path.starts_with('-') {
        format!("./{path}")
    } else {
        path.to_owned()
    };
    if !path.is_empty()
        && path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/_-.".contains(&c))
    {
        Ok(path)
    } else {
        Ok(format!("'{}'", path.replace('\'', "'\\''")))
    }
}

fn display(path: &str, home: &str) -> String {
    if !home.is_empty() && path == home {
        "~".into()
    } else if !home.is_empty()
        && let Some(rest) = path.strip_prefix(&format!("{}/", home.trim_end_matches('/')))
    {
        format!("~/{rest}")
    } else {
        path.to_owned()
    }
}
fn join(directory: &str, name: &str) -> String {
    if name.is_empty() {
        directory.to_owned()
    } else {
        format!("{}/{name}", directory.trim_end_matches('/'))
    }
}

pub fn rows(reply: &ListDirResponse, prefix: &str) -> Vec<PathRow> {
    let path = join(&reply.path, prefix);
    let mut rows = vec![PathRow {
        label: display(&path, &reply.home),
        path,
        directory: prefix.is_empty(),
    }];
    let lower = prefix.to_lowercase();
    let mut entries: Vec<_> = reply
        .entries
        .iter()
        .filter(|e| {
            !e.name.is_empty()
                && !matches!(e.name.as_str(), "." | "..")
                && !e.name.contains('/')
                && !e.name.chars().any(refused)
                && (!e.name.starts_with('.') || prefix.starts_with('.'))
                && e.name.to_lowercase().starts_with(&lower)
        })
        .collect();
    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name))
    });
    rows.extend(entries.into_iter().map(|e| PathRow {
        label: e.name.clone(),
        path: join(&reply.path, &e.name),
        directory: e.file != Some(true),
    }));
    rows
}

impl Workspace {
    pub fn insert_path_dialog(&mut self, target: PaneTarget) {
        self.directory_dialog(target, false);
    }

    pub fn go_to_directory_dialog(&mut self, target: PaneTarget) {
        self.directory_dialog(target, true);
    }

    fn directory_dialog(&mut self, target: PaneTarget, go: bool) {
        let host = self.settings.hosts[target.host].label.clone();
        let (title, placeholder) = if go {
            ("Go to directory", "Folder on the terminal's machine…")
        } else {
            ("Insert path", "Path on the terminal's machine…")
        };
        self.open_panel(
            PanelKind::Directory(Browser {
                go,
                ..Browser::default()
            }),
            &format!("{title} · {host}"),
            &[("", placeholder, "")],
        );
        self.panel.as_mut().unwrap().target = Some(target);
        self.list_directory();
    }

    pub fn hide_recent_paths(&mut self) {
        if let Some(panel) = &mut self.panel
            && let PanelKind::Directory(browser) = &mut panel.kind
        {
            if browser.recent_count > 0 && !browser.rows.is_empty() {
                browser.rows.drain(1..=browser.recent_count);
            }
            browser.recent_count = 0;
            panel.selected = 0;
            panel.reveal = true;
        }
    }

    pub fn directory_field_changed(&mut self) {
        if let Some(panel) = &mut self.panel
            && let PanelKind::Directory(browser) = &mut panel.kind
        {
            browser.edited = true;
        }
        self.list_directory();
    }

    /// Every edit replaces the wanted path, never queues behind an outstanding read.
    pub fn list_directory(&mut self) {
        let Some(panel) = &mut self.panel else { return };
        let PanelKind::Directory(browser) = &mut panel.kind else {
            return;
        };
        browser.rows.clear();
        browser.recent_count = 0;
        browser.truncated = false;
        panel.error = None;
        panel.selected = 0;
        panel.reveal = true;
        if browser.pending.is_some() {
            return;
        }
        let Some(target) = panel.target.clone() else {
            return;
        };
        let text = panel.inputs[0].text.clone();
        if text.chars().any(refused) {
            panel.error = Some(REFUSED.into());
            return;
        }
        // No path this long can be listed, and a pasted megabyte must not become a request.
        if text.len() > PATH_MAX {
            panel.error = Some("That path is too long.".into());
            return;
        }
        let model = &self.hosts[target.host].model;
        if !model.connected {
            panel.error = Some(Failure::Disconnected.message().into());
            return;
        }
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let epoch = model.connection_epoch;
        let (path, prefix) = split_path(&text);
        let request = Request {
            id,
            epoch,
            session: target.session,
            block: target.block,
            // Only folders can be gone to.
            args: ListDir::new().path(path).prefix(prefix).files(!browser.go),
            deadline: Instant::now() + Duration::from_secs(8),
            open: Arc::downgrade(&browser.open),
        };
        browser.epoch = Some(epoch);
        browser.pending = Some((id, text));
        if self.hosts[target.host]
            .connection
            .commands
            .send(HostCommand::ListDir(Box::new(request)))
            .is_err()
        {
            let panel = self.panel.as_mut().unwrap();
            if let PanelKind::Directory(browser) = &mut panel.kind {
                browser.pending = None;
            }
            panel.error = Some(Failure::Disconnected.message().into());
        }
    }

    pub fn process_directory(&mut self, host: usize) {
        let Some(panel) = &mut self.panel else { return };
        let PanelKind::Directory(browser) = &mut panel.kind else {
            return;
        };
        if panel.target.as_ref().is_none_or(|t| t.host != host) {
            return;
        }
        let model = &self.hosts[host].model;
        if !model.connected
            || browser
                .epoch
                .is_some_and(|epoch| model.connection_epoch != epoch)
        {
            browser.pending = None;
            browser.rows.clear();
            browser.recent_count = 0;
            browser.truncated = false;
            panel.error = Some(Failure::Disconnected.message().into());
            return;
        }
        let Some((id, text)) = browser.pending.clone() else {
            return;
        };
        let Some(reply) = &model.directory else {
            return;
        };
        if reply.id != id || Some(reply.epoch) != browser.epoch {
            return;
        }
        browser.pending = None;
        if text != panel.inputs[0].text {
            self.list_directory();
            return;
        }
        match &reply.result {
            Err(error) => panel.error = Some(error.message().into()),
            Ok(reply) => {
                panel.error = match reply.status.as_str() {
                    "ok" => None,
                    "missing" => Some("That folder doesn't exist.".into()),
                    "not_dir" => Some("That path isn't a folder.".into()),
                    "denied" => Some("Rex isn't allowed to read that folder.".into()),
                    _ => Some(Failure::Request.message().into()),
                };
                if panel.error.is_some() {
                    return;
                }
                if !reply.path.starts_with('/') || reply.path.chars().any(refused) {
                    panel.error = Some("Rex returned a path that can't be inserted.".into());
                    return;
                }
                browser.rows = rows(reply, split_path(&text).1);
                if browser.go {
                    // The server was asked for folders only; a file it lists anyway is
                    // not somewhere to go. The typed path stays first.
                    let mut typed = true;
                    browser
                        .rows
                        .retain(|row| std::mem::take(&mut typed) || row.directory);
                }
                if !browser.edited && self.settings.terminal.show_recent_paths {
                    let endpoint = &self.settings.hosts[host].endpoint;
                    let mut seen = std::collections::HashSet::new();
                    let recent: Vec<_> = self
                        .settings
                        .recent_paths
                        .get(endpoint)
                        .into_iter()
                        .flatten()
                        .filter(|r| {
                            valid(&r.path)
                                && (!browser.go || r.directory)
                                && seen.insert(r.path.clone())
                        })
                        .take(LIMIT)
                        .map(|r| PathRow {
                            label: display(&r.path, &reply.home),
                            path: r.path.clone(),
                            directory: r.directory,
                        })
                        .collect();
                    browser.recent_count = recent.len();
                    browser.rows.splice(1..1, recent);
                }
                panel.reveal = true;
                browser.truncated = reply.truncated;
                if text.is_empty() {
                    let mut edit = TextEdit::new(format!(
                        "{}/",
                        display(&reply.path, &reply.home).trim_end_matches('/')
                    ));
                    edit.focused = true;
                    panel.inputs[0] = edit;
                }
            }
        }
    }

    pub fn directory_entries(&self, browser: &Browser) -> Vec<Entry> {
        browser
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let mut entry = Entry::new(
                    &row.label,
                    if index == 0 && browser.go {
                        "Go to typed folder"
                    } else if index == 0 {
                        "Insert typed path"
                    } else if row.directory {
                        "Folder"
                    } else {
                        "File"
                    },
                    "",
                    Intent::InsertPath(index),
                );
                // The intent keeps the panel until paste captures its row; clicking does not
                // open a follow-up, so it must not draw a menu chevron.
                entry.followup = false;
                if browser.recent_count > 0 && index > 0 {
                    entry.group = if index <= browser.recent_count {
                        "Recent"
                    } else {
                        "In this folder"
                    }
                    .into();
                }
                entry
            })
            .collect()
    }

    pub fn descend_directory(&mut self) {
        let Some(panel) = &mut self.panel else { return };
        let PanelKind::Directory(browser) = &panel.kind else {
            return;
        };
        let Some(row) = browser.rows.get(panel.selected).filter(|r| r.directory) else {
            return;
        };
        let mut edit = TextEdit::new(format!("{}/", row.path.trim_end_matches('/')));
        edit.focused = true;
        panel.inputs[0] = edit;
        self.directory_field_changed();
    }

    pub fn insert_path(&mut self, index: usize, tx: &mut Tx) {
        let Some(panel) = &self.panel else { return };
        let PanelKind::Directory(browser) = &panel.kind else {
            return;
        };
        let (Some(target), Some(row)) = (panel.target.clone(), browser.rows.get(index)) else {
            return;
        };
        let text = match quote(&row.path) {
            // Typed, never run: Enter stays with the person. The path is absolute, so it
            // cannot be read as an option.
            Ok(text) if browser.go => format!("cd {text}"),
            Ok(text) => text,
            Err(error) => {
                self.panel.as_mut().unwrap().error = Some(error.into());
                return;
            }
        };
        let row = row.clone();
        // A folder is one the server listed, or the listed folder itself.
        let directory = row.directory
            || browser
                .rows
                .iter()
                .skip(1 + browser.recent_count)
                .any(|listed| listed.path == row.path && listed.directory);
        // Go's typed row for a name that was never listed may not be a folder, or exist:
        // it is typed as asked and not remembered as somewhere to go.
        let remember = directory || !browser.go;
        self.close_panel(tx);
        if self.paste_text(&target, text) && remember && self.settings.terminal.show_recent_paths {
            let endpoint = self.settings.hosts[target.host].endpoint.clone();
            self.settings.remember_path(&endpoint, row.path, directory);
            self.save();
        }
    }
}
