//! File downloads from a pane's machine (Rex API revision 13): the dialog that asks for one,
//! the notices when transfers end, and what a finished one offers. The transfers themselves
//! live in the library (`rex_linux::download`) and in each host's model; this module only reads
//! them, answers their questions and tells the person.
//!
//! Nothing here starts a download except the dialog's own button, and a finished file is shown
//! in its folder only by a press on that transfer's row. Rex Linux never opens or runs it.
use super::{
    panel::{Intent, PaneTarget, PanelKind},
    workspace::{HostId, Workspace},
};
use rex_linux::{
    connection::{HostCommand, downloads::Request},
    download::{
        self, Confirmation, Failure, Reason, State, Transfer, number_text, power_of, size_text,
        store,
    },
};
use rune::{TextEdit, Tx};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

/// The state of the Download dialog beyond its fields (`Panel::inputs`: the file on the host,
/// then the folder while it is being changed).
#[derive(Clone, Debug, Default)]
pub struct Dialog {
    /// The transfer this dialog started, until it fails or its bytes begin to arrive.
    pub id: Option<u64>,
    /// The folder is a field now, not just a line.
    pub changing: bool,
}

pub struct Downloads {
    /// Finished transfers already told to the person, so each is told once.
    announced: HashSet<(HostId, u64)>,
    /// Confirmations nobody can answer any more, already cancelled.
    orphans: HashSet<(HostId, u64)>,
    /// Where the toolbar chip was laid out, for the popover that opens under it.
    pub chip: [f32; 4],
    /// Tests: the files **Show in folder** was pressed for, instead of reaching a real file
    /// manager.
    #[cfg(test)]
    pub shown: Vec<PathBuf>,
}

impl Default for Downloads {
    fn default() -> Self {
        Self {
            announced: HashSet::new(),
            orphans: HashSet::new(),
            chip: [0.; 4],
            #[cfg(test)]
            shown: Vec::new(),
        }
    }
}

/// A transfer of one host, for the popover and the chip.
pub struct Row<'a> {
    pub host: HostId,
    pub label: &'a str,
    pub transfer: &'a Transfer,
}

/// `12.4 of 80 MB` while the size is known, else `12.4 MB`.
pub fn progress_text(received: u64, hint: u64) -> String {
    let power = power_of(hint);
    if hint == 0 || power == 0 {
        return if hint == 0 {
            size_text(received)
        } else {
            format!("{received} of {}", size_text(hint))
        };
    }
    format!(
        "{} of {}",
        number_text(received as f64 / 1000f64.powi(power as i32)),
        size_text(hint)
    )
}

/// The size Rex reported when a file came up short: plain bytes while small, as `4096`.
fn reported_text(hint: u64) -> String {
    if hint < 10_000 {
        hint.to_string()
    } else {
        size_text(hint)
    }
}

/// A path with the home folder written `~`.
pub fn tilde(path: &Path) -> String {
    tilde_in(path, std::env::var_os("HOME").map(PathBuf::from).as_deref())
}

pub(super) fn tilde_in(path: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && home.is_absolute()
        && home.components().count() > 1
        && let Ok(rest) = path.strip_prefix(home)
    {
        return if rest.as_os_str().is_empty() {
            "~".into()
        } else {
            format!("~/{}", rest.display())
        };
    }
    path.display().to_string()
}

/// What a download of `host` that is too old to send files says.
pub fn too_old(host: &str) -> String {
    format!("Rex on {host} is too old to send files; update Rex there.")
}

pub fn failure_text(host: &str, failure: &Failure) -> String {
    match failure {
        Failure::ServerTooOld => too_old(host),
        // The library's text here is the WebSocket stack's ("Connection reset without closing
        // handshake"), which says nothing a person can use.
        Failure::Connection(_) => format!("the connection to {host} was lost"),
        other => other.to_string(),
    }
}

/// A failure as the dialog shows it: a sentence. Rex's own words and text that ends in a path
/// or an error from the system are left as they came; the client's own sentences start with a
/// capital and end with a full stop, like the rest of the dialog.
pub fn dialog_failure_text(host: &str, failure: &Failure) -> String {
    let text = failure_text(host, failure);
    if matches!(
        failure,
        Failure::Server(_)
            | Failure::Destination(_)
            | Failure::Disk(_)
            | Failure::UnexpectedReply(_)
            | Failure::Closed { .. }
            | Failure::Protocol(_)
    ) {
        return text;
    }
    let mut chars = text.chars();
    let mut sentence: String = chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    if !sentence.ends_with(['.', '!', '?']) {
        sentence.push('.');
    }
    sentence
}

/// Failures that can only follow received bytes: they belong in a notice, never the dialog.
fn after_bytes(failure: &Failure) -> bool {
    matches!(
        failure,
        Failure::Stalled
            | Failure::TooMuchData { .. }
            | Failure::Disk(_)
            | Failure::Protocol(_)
            | Failure::Closed { .. }
    )
}

/// The question a confirmation asks, in the words the dialog shows. A yes to the Large question
/// only commits the time and the space; a yes to the LowSpace one says how little is left.
pub fn question(name: &str, folder: &str, confirmation: &Confirmation) -> String {
    let size = size_text(confirmation.hint);
    match (confirmation.reason, confirmation.free) {
        (Reason::LowSpace, Some(free)) if free > confirmation.hint => format!(
            "{name} is {size}. {folder} has {} free, so saving it leaves about {}.",
            size_text(free),
            size_text(free - confirmation.hint)
        ),
        (_, Some(free)) => format!("{name} is {size}. {folder} has {} free.", size_text(free)),
        (_, None) => format!("{name} is {size}."),
    }
}

/// The widest a folder is written in a line of the popover, the dialog or a notice.
pub const FOLDER_CHARS: usize = 36;

/// `s` cut in the middle with an ellipsis so it is at most `max` characters.
fn cut_middle(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_owned();
    }
    let keep = max.saturating_sub(1);
    let head = keep.div_ceil(2);
    let tail = keep - head;
    chars[..head]
        .iter()
        .chain(std::iter::once(&'…'))
        .chain(&chars[chars.len() - tail..])
        .collect()
}

/// A folder as a line shows it: the home folder as `~`, and a long one cut in the middle by
/// whole folders (`~/…/reports/2026`), so the end, the part that tells folders apart, stays.
pub fn folder_text(path: &Path, max: usize) -> String {
    folder_text_in(
        path,
        std::env::var_os("HOME").map(PathBuf::from).as_deref(),
        max,
    )
}

pub(super) fn folder_text_in(path: &Path, home: Option<&Path>, max: usize) -> String {
    let full = tilde_in(path, home);
    if full.chars().count() <= max {
        return full;
    }
    let (root, rest) = match full.strip_prefix("~/") {
        Some(rest) => ("~", rest),
        None => ("", full.trim_start_matches('/')),
    };
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
    // `~` or `/first`, then `…`, then as many of the last folders as fit.
    let (head, tail) = match (root, parts.split_first()) {
        ("", Some((first, tail))) => (format!("/{first}"), tail),
        _ => (root.to_owned(), parts.as_slice()),
    };
    let mut kept: Vec<&str> = Vec::new();
    let mut width = head.chars().count() + 2; // "/…"
    for part in tail.iter().rev() {
        let next = width + 1 + part.chars().count();
        if next > max {
            break;
        }
        width = next;
        kept.push(part);
    }
    if kept.is_empty() {
        return cut_middle(&full, max);
    }
    kept.reverse();
    format!("{head}/…/{}", kept.join("/"))
}

/// The line under a transfer's name in the popover.
pub fn state_line(host: &str, transfer: &Transfer) -> String {
    match &transfer.state {
        State::Requested => format!("Waiting for {host}…"),
        State::AwaitingConfirmation(_) => "Waiting for your answer".into(),
        State::Receiving { received, hint } => progress_text(*received, *hint),
        State::Saved {
            path,
            bytes,
            short_of,
        } => match short_of {
            Some(hint) => format!(
                "Saved {}; Rex reported {}",
                size_text(*bytes),
                reported_text(*hint)
            ),
            None => format!(
                "Saved to {}",
                folder_text(path.parent().unwrap_or(&transfer.directory), FOLDER_CHARS)
            ),
        },
        State::Cancelled => "Cancelled".into(),
        State::Failed(failure) => format!("Failed: {}", failure_text(host, failure)),
    }
}

impl Workspace {
    /// The chip's press: the Downloads popover, under the chip.
    pub fn open_downloads_from_chip(&mut self, tx: &mut Tx) {
        self.perform(Intent::Downloads, tx);
        let chip = self.downloads.chip;
        if let Some(panel) = &mut self.panel {
            panel.anchor = Some((chip[0], chip[1] + chip[3]));
        }
    }

    /// Which chips share the bar with the tabs (bit 0 downloads, bit 1 a pending key sequence),
    /// so the active tab is scrolled back into view when one comes or goes.
    pub fn bar_chips(&self) -> u8 {
        u8::from(super::downloads_view::chip_label(self).is_some())
            | u8::from(self.bindings.sequence.depth > 0) << 1
    }

    /// Every host's transfers, running ones first, newest first within each group.
    pub fn download_rows(&self) -> Vec<Row<'_>> {
        let mut rows: Vec<Row> = self
            .hosts
            .iter()
            .enumerate()
            .flat_map(|(index, host)| {
                let label = self.settings.hosts[index].label.as_str();
                host.model.downloads.iter().map(move |transfer| Row {
                    host: host.id,
                    label,
                    transfer,
                })
            })
            .collect();
        rows.sort_by_key(|r| {
            (
                r.transfer.state.is_final(),
                std::cmp::Reverse(r.transfer.id),
            )
        });
        rows
    }

    /// The folder downloads are saved in: `downloads.directory`, else the XDG download folder.
    pub fn download_folder(&self) -> PathBuf {
        match self
            .settings
            .downloads
            .directory
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
        {
            Some(directory) => store::expand_home(directory),
            None => store::default_directory(),
        }
    }

    /// Whether the host's Rex is too old to send files. Decided from the negotiated revision,
    /// before anything is sent; the core refuses too.
    pub fn download_unsupported(&self, host: usize) -> bool {
        let model = &self.hosts[host].model;
        model.connected && model.protocol.api_revision < download::MIN_API_REVISION
    }

    /// The pane's selection when it is one line: what the field starts with.
    fn download_prefill(&self, target: &PaneTarget) -> String {
        let key = (self.hosts[target.host].id, target.block.clone());
        let Some(text) = self.terminals.get(&key).and_then(|t| t.copy_text()) else {
            return String::new();
        };
        let text = text.trim();
        if text.is_empty() || text.len() > 4096 || text.chars().any(char::is_control) {
            return String::new();
        }
        text.to_owned()
    }

    /// Open Download from <host> for the captured pane.
    pub fn download_dialog(&mut self, target: PaneTarget) {
        let host = self.settings.hosts[target.host].label.clone();
        let prefill = self.download_prefill(&target);
        let folder = tilde(&self.download_folder());
        self.open_panel(
            PanelKind::Download(Dialog::default()),
            &format!("Download from {host}"),
            &[
                (
                    &format!("File on {host}"),
                    "/path/to/file, ~/file, or relative to the pane's folder",
                    &prefill,
                ),
                ("Save in", "~/Downloads", &folder),
            ],
        );
        self.panel.as_mut().unwrap().target = Some(target);
    }

    /// The dialog's transfer, once it has one.
    pub fn dialog_transfer(&self) -> Option<&Transfer> {
        let panel = self.panel.as_ref()?;
        let PanelKind::Download(dialog) = &panel.kind else {
            return None;
        };
        let (id, target) = (dialog.id?, panel.target.as_ref()?);
        self.hosts
            .get(target.host)?
            .model
            .downloads
            .iter()
            .find(|t| t.id == id)
    }

    fn dialog_mut(&mut self) -> Option<&mut Dialog> {
        match &mut self.panel.as_mut()?.kind {
            PanelKind::Download(dialog) => Some(dialog),
            _ => None,
        }
    }

    /// Enter in the dialog: ask the pane's host for the file. Nothing is sent for a host whose
    /// Rex is too old, or for an empty path.
    pub fn start_download(&mut self) {
        let Some(panel) = &self.panel else { return };
        let PanelKind::Download(dialog) = &panel.kind else {
            return;
        };
        let Some(target) = panel.target.clone() else {
            return;
        };
        if dialog.id.is_some() || dialog.changing || self.download_unsupported(target.host) {
            return;
        }
        let path = panel.inputs[0].text.trim().to_owned();
        let host = self.settings.hosts[target.host].label.clone();
        if path.is_empty() {
            self.panel.as_mut().unwrap().error =
                Some(format!("Enter the path of a file on {host}."));
            return;
        }
        if !self.target_exists(&target) {
            self.panel = None;
            self.inform("That pane is no longer available.");
            return;
        }
        let id = download::next_id();
        let request = Request::new(
            id,
            target.session.clone(),
            target.block.clone(),
            path,
            self.download_folder(),
        );
        self.send(target.host, HostCommand::Download(Box::new(request)));
        if let Some(dialog) = self.dialog_mut() {
            dialog.id = Some(id);
        }
        self.panel.as_mut().unwrap().error = None;
    }

    /// "Download anyway" on the confirmation.
    pub fn confirm_download(&mut self) {
        let Some(panel) = &self.panel else { return };
        let Some(target) = &panel.target else { return };
        let host = target.host;
        if let Some(t) = self.dialog_transfer()
            && matches!(t.state, State::AwaitingConfirmation(_))
        {
            let id = t.id;
            self.send(host, HostCommand::DownloadConfirm(id));
        }
    }

    /// Escape (or Cancel) while the folder is a field only leaves the field.
    pub fn leave_folder_field(&mut self) -> bool {
        let changing = matches!(
            self.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Download(d)) if d.changing
        );
        if changing {
            self.stop_changing_folder();
        }
        changing
    }

    /// Whatever the dialog still has pending is cancelled when it goes away by any road but
    /// its bytes beginning: the person said no, or never saw the question. A transfer Rex
    /// has not answered yet (and so is not listed) is cancelled too; the host's worker
    /// handles commands in order, so the cancel reaches it after the request.
    pub fn leave_download_dialog(&mut self) {
        let Some(panel) = &self.panel else { return };
        let PanelKind::Download(dialog) = &panel.kind else {
            return;
        };
        let (Some(id), Some(target)) = (dialog.id, &panel.target) else {
            return;
        };
        let host = target.host;
        let pending = self.hosts.get(host).is_some_and(|h| {
            h.model
                .downloads
                .iter()
                .find(|t| t.id == id)
                .is_none_or(|t| {
                    matches!(t.state, State::Requested | State::AwaitingConfirmation(_))
                })
        });
        // Once is enough: leaving can run twice (Cancel, then the panel closing).
        if let Some(dialog) = self.dialog_mut() {
            dialog.id = None;
        }
        if pending {
            // Told once: the sweep for unowned questions must not ask again before the core
            // has finished the cancel.
            let host_id = self.hosts[host].id;
            self.downloads.orphans.insert((host_id, id));
            self.send(host, HostCommand::DownloadCancel(id));
        }
    }

    /// Cancel on a running row.
    pub fn cancel_download(&mut self, host: HostId, id: u64) {
        if let Some(index) = self.host_index(host) {
            self.send(index, HostCommand::DownloadCancel(id));
        }
    }

    /// **Change** beside Save in: the folder becomes a field.
    pub fn change_download_folder(&mut self) {
        let text = match &self.settings.downloads.directory {
            Some(directory) if !directory.trim().is_empty() => directory.trim().to_owned(),
            _ => tilde(&self.download_folder()),
        };
        let Some(panel) = &mut self.panel else { return };
        let PanelKind::Download(dialog) = &mut panel.kind else {
            return;
        };
        dialog.changing = true;
        panel.error = None;
        let mut edit = TextEdit::new(&text);
        edit.anchor = 0;
        edit.focus = text.len();
        edit.focused = true;
        panel.inputs[1] = edit;
        panel.inputs[0].focused = false;
    }

    /// Enter in the folder field: keep it, saved as `downloads.directory`. An empty field goes
    /// back to the system's Downloads folder.
    pub fn apply_download_folder(&mut self) {
        let Some(panel) = &self.panel else { return };
        let text = panel.inputs[1].text.trim().to_owned();
        if !text.is_empty()
            && (text.chars().any(char::is_control)
                || !(text.starts_with('/') || text == "~" || text.starts_with("~/")))
        {
            self.panel.as_mut().unwrap().error =
                Some("Enter an absolute folder path, or one starting with ~/".into());
            return;
        }
        self.settings.downloads.directory = (!text.is_empty()).then_some(text);
        self.save();
        self.stop_changing_folder();
    }

    /// Escape in the folder field: back to the line, nothing changed.
    pub fn stop_changing_folder(&mut self) {
        let folder = tilde(&self.download_folder());
        let Some(panel) = &mut self.panel else { return };
        let PanelKind::Download(dialog) = &mut panel.kind else {
            return;
        };
        dialog.changing = false;
        panel.error = None;
        panel.inputs[1] = TextEdit::new(&folder);
        panel.inputs[0].focused = true;
    }

    /// A host's model changed: move the dialog on, tell the person what ended, and cancel a
    /// question nobody can answer.
    pub fn process_downloads(&mut self, index: usize, tx: &mut Tx) {
        self.advance_download_dialog(index, tx);
        let id = self.hosts[index].id;
        let label = self.settings.hosts[index].label.clone();
        let transfers = self.hosts[index].model.downloads.clone();
        let owned = self.dialog_transfer().map(|t| t.id);
        for transfer in transfers {
            let key = (id, transfer.id);
            if transfer.state.is_final() {
                if self.downloads.announced.insert(key) {
                    self.announce_download(&label, &transfer);
                }
            } else if matches!(transfer.state, State::AwaitingConfirmation(_))
                && owned != Some(transfer.id)
                && self.downloads.orphans.insert(key)
            {
                self.send(index, HostCommand::DownloadCancel(transfer.id));
            }
        }
    }

    /// The dialog's own transfer: a question stays in it, its bytes beginning hand it to the
    /// chip, and a failure before any stays in it.
    fn advance_download_dialog(&mut self, index: usize, tx: &mut Tx) {
        let Some(panel) = &self.panel else { return };
        let PanelKind::Download(dialog) = &panel.kind else {
            return;
        };
        if dialog.id.is_none() || panel.target.as_ref().is_none_or(|t| t.host != index) {
            return;
        }
        // Not listed yet: the host's worker has not got to the request.
        let Some(transfer) = self.dialog_transfer().cloned() else {
            return;
        };
        match &transfer.state {
            State::Requested | State::AwaitingConfirmation(_) => {}
            State::Receiving { .. } | State::Saved { .. } | State::Cancelled => {
                self.close_panel(tx);
            }
            State::Failed(failure) => {
                let key = (self.hosts[index].id, transfer.id);
                if after_bytes(failure) {
                    self.close_panel(tx);
                    return;
                }
                let host = self.settings.hosts[index].label.clone();
                // The dialog says it; a fourth transfer is also told in a notice.
                self.downloads.announced.insert(key);
                if matches!(failure, Failure::TooManyActive { .. }) {
                    self.inform(format!(
                        "Download from {host} refused: {}.",
                        failure_text(&host, failure)
                    ));
                }
                let panel = self.panel.as_mut().unwrap();
                panel.error = Some(dialog_failure_text(&host, failure));
                if let PanelKind::Download(dialog) = &mut panel.kind {
                    dialog.id = None;
                }
                if let Some(first) = panel.inputs.first_mut() {
                    first.focused = true;
                }
            }
        }
    }

    fn announce_download(&mut self, label: &str, transfer: &Transfer) {
        let name = transfer.display_name();
        match &transfer.state {
            State::Saved {
                path,
                bytes,
                short_of,
            } => {
                let saved = path
                    .file_name()
                    .map_or(name, |n| n.to_string_lossy().into_owned());
                let folder =
                    folder_text(path.parent().unwrap_or(&transfer.directory), FOLDER_CHARS);
                let mut text = format!("Saved {saved} from {label} to {folder}");
                if let Some(hint) = short_of {
                    text.push_str(&format!(
                        " ({}; Rex reported {})",
                        size_text(*bytes),
                        reported_text(*hint)
                    ));
                }
                self.inform(text);
            }
            State::Failed(failure) => self.notify(format!(
                "Download of {name} from {label} failed: {}",
                failure_text(label, failure)
            )),
            // The person cancelled it; they know.
            _ => {}
        }
    }

    /// **Show in folder** on a finished row.
    pub fn show_download(&mut self, host: HostId, id: u64) {
        if let Some(path) = self.saved_path(host, id) {
            self.show_in_folder(path);
        }
    }

    fn saved_path(&self, host: HostId, id: u64) -> Option<PathBuf> {
        let index = self.host_index(host)?;
        self.hosts[index]
            .model
            .downloads
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| match &t.state {
                State::Saved { path, .. } => Some(path.clone()),
                _ => None,
            })
    }

    #[cfg(test)]
    fn show_in_folder(&mut self, path: PathBuf) {
        self.downloads.shown.push(path);
    }

    #[cfg(not(test))]
    fn show_in_folder(&mut self, path: PathBuf) {
        self.on_desktop("show", move || {
            // The file manager's own call, which selects the file; else its folder.
            if show_items(&path).is_ok() {
                return Ok(());
            }
            xdg_open(path.parent().unwrap_or(Path::new("/")))
        });
    }

    /// Run a desktop call off the UI thread; a failure becomes a notice.
    #[cfg(not(test))]
    fn on_desktop(
        &mut self,
        what: &'static str,
        f: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) {
        let handle = self.handle.clone();
        let _ = std::thread::Builder::new()
            .name(format!("rex-{what}-download"))
            .spawn(move || {
                if let Err(error) = f() {
                    handle
                        .update(move |s, _| s.notify(format!("Couldn't {what} the file: {error}")));
                }
            });
    }

    /// Once at launch (`app::run` is the only caller), on a worker: delete the partial files an
    /// earlier run left behind and recorded. The first frame never waits for it.
    pub fn clean_up_download_leftovers() {
        let _ = Self::clean_up_leftovers_in(store::Records::default());
    }

    /// The launch cleanup against `records`, on its own thread. Split out so a test can point it
    /// at a private records file and wait for it.
    pub(super) fn clean_up_leftovers_in(
        records: store::Records,
    ) -> Option<std::thread::JoinHandle<()>> {
        std::thread::Builder::new()
            .name("rex-download-cleanup".into())
            .spawn(move || {
                records.clean_up_leftovers();
            })
            .ok()
    }
}

/// The `xdg-open` call for one absolute path, which is its only argument. `None` for a relative
/// path, which `xdg-open` could read as an option or an URL.
pub(super) fn xdg_open_command(target: &Path) -> Option<std::process::Command> {
    use std::process::{Command, Stdio};
    if !target.is_absolute() {
        return None;
    }
    let mut command = Command::new("xdg-open");
    command
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Some(command)
}

/// Hand an absolute folder to the desktop's opener and wait for it. A failure is its text.
#[cfg(not(test))]
fn xdg_open(target: &Path) -> Result<(), String> {
    let mut command = xdg_open_command(target).ok_or("not an absolute path")?;
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(status.to_string()),
        Err(error) => Err(error.to_string()),
    }
}

/// `org.freedesktop.FileManager1.ShowItems`: the file manager opens the folder with the file
/// selected.
#[cfg(not(test))]
fn show_items(path: &Path) -> Result<(), String> {
    let uri = url::Url::from_file_path(path).map_err(|()| "not an absolute path".to_owned())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        const NAME: &str = "org.freedesktop.FileManager1";
        let call = async {
            let conn = zbus::Connection::session().await?;
            let proxy =
                zbus::Proxy::new(&conn, NAME, "/org/freedesktop/FileManager1", NAME).await?;
            proxy
                .call::<_, _, ()>("ShowItems", &(vec![uri.to_string()], ""))
                .await
        };
        match tokio::time::timeout(std::time::Duration::from_secs(5), call).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.to_string()),
            Err(_) => Err("the file manager did not answer".into()),
        }
    })
}
