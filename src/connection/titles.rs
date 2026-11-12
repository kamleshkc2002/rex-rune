//! Generated titles: what a terminal is called from the program in its foreground and the
//! directory that program is in (`vim - ~/rex`), in place of a title its shell set. A title a
//! program set for itself is not replaced, and neither is a name the user gave: that is a
//! label, which the caller checks before it asks here.
//!
//! Display only. The server's title, labels and processes are read and never changed.
use super::{HostModel, activity::program_name};
use serde_json::Value;
use std::borrow::Cow;

/// Where a terminal's foreground program is, and which process is the terminal's own shell,
/// as a `process_changed` payload or a `process` reply has them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Place {
    /// The foreground program's working directory, or that of the nearest process it runs
    /// under when its own can't be read (another user's, as under sudo).
    pub cwd: Option<String>,
    /// The home directory of the terminal's program. Servers older than the field leave it
    /// out of `process_changed`; a `process` reply has it.
    pub home: Option<String>,
    /// The process the terminal started, by pid, when that is a shell.
    pub shell: Option<u64>,
}

impl Place {
    pub fn parse(process: &Value) -> Self {
        // The foreground, then what it runs under, nearest first, ending with the process the
        // terminal started.
        let chain: Vec<&Value> = std::iter::once(&process["foreground"])
            .chain(process["ancestors"].as_array().into_iter().flatten())
            .filter(|p| p.is_object())
            .collect();
        let path = |value: &Value| {
            value
                .as_str()
                .filter(|dir| dir.starts_with('/'))
                .map(str::to_owned)
        };
        // A reply names the terminal's process outright. An event leaves the chain to say,
        // and on macOS a login shell runs under login: the shell is the process below it.
        let child = &process["child"];
        let own = if child.is_object() && child["name"] != "login" {
            Some(child)
        } else {
            match chain.as_slice() {
                [.., shell, login] if login["name"] == "login" => Some(*shell),
                [.., own] if own["name"] != "login" => Some(*own),
                _ => None,
            }
        };
        Self {
            cwd: chain.iter().find_map(|p| path(&p["cwd"])),
            home: path(&process["home"]),
            shell: own
                .filter(|p| program_name(p).is_some_and(|name| is_shell(&name)))
                .and_then(|p| p["pid"].as_u64()),
        }
    }
}

/// Whether a program of this name is a shell, whose titles a generated one replaces.
fn is_shell(name: &str) -> bool {
    matches!(
        name,
        "sh" | "bash"
            | "zsh"
            | "fish"
            | "dash"
            | "ash"
            | "ksh"
            | "mksh"
            | "csh"
            | "tcsh"
            | "nu"
            | "xonsh"
            | "elvish"
            | "pwsh"
    )
}

/// The most characters a title gives its directory. A tab is 240 pt at its widest, and a
/// label cut at its end loses the folder the program is in, the part worth reading.
const DIRECTORY_ROOM: usize = 24;

/// A directory as a title shows it: `home` as `~`, and of a long path only as many of its
/// last folders as fit, after `…/`.
pub fn directory(cwd: &str, home: Option<&str>) -> String {
    let home = home
        .map(|h| h.trim_end_matches('/'))
        .filter(|h| !h.is_empty());
    let dir = match home.and_then(|h| cwd.strip_prefix(h)) {
        Some("") => "~".to_owned(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => cwd.to_owned(),
    };
    if dir.chars().count() <= DIRECTORY_ROOM {
        return dir;
    }
    // The last folder always shows, however long: the tab truncates it if it must.
    let mut kept = Vec::new();
    let mut used = 1;
    for folder in dir.rsplit('/').filter(|f| !f.is_empty()) {
        used += 1 + folder.chars().count();
        if !kept.is_empty() && used > DIRECTORY_ROOM {
            break;
        }
        kept.push(folder);
    }
    kept.reverse();
    format!("…/{}", kept.join("/"))
}

impl HostModel {
    /// Record where `block`'s foreground program is. A home directory the server gave once
    /// stays known: it is the terminal's, and events from older servers leave it out.
    pub(super) fn set_place(&mut self, block: &str, place: Place) {
        let known = self.places.entry(block.to_owned()).or_default();
        let home = place.home.or(known.home.take());
        *known = Place { home, ..place };
    }

    /// Whether the block's title was set by the terminal's own shell, not by a program
    /// running in it. An owner the server couldn't name counts as a program, and so does a
    /// shell started inside the terminal's: a script and a nested shell look alike here, and
    /// a script's title is its own.
    fn shell_titled(&self, block: &str) -> bool {
        let owner = self.titles.get(block).and_then(|(_, owner)| *owner);
        let shell = self.places.get(block).and_then(|place| place.shell);
        owner.is_some() && owner == shell
    }

    /// The block's foreground program and its directory, such as `vim - ~/rex`; the program
    /// alone while the server can't read a directory. None while it names no program.
    pub fn generated_title(&self, block: &str) -> Option<String> {
        let program = self.programs.get(block)?;
        let place = self.places.get(block);
        Some(match place.and_then(|p| p.cwd.as_deref()) {
            Some(cwd) => format!(
                "{program} - {}",
                directory(cwd, place.and_then(|p| p.home.as_deref()))
            ),
            None => program.clone(),
        })
    }

    /// What `block` is called while it has no name of its own. With `generated` off that is
    /// its current terminal title ([`HostModel::title`]). With it on, a title the terminal's
    /// shell set, or no title at all, gives way to the program and its directory; a title a
    /// program set for itself still shows. Until the server names a program the title stays.
    pub fn shown_title(&self, block: &str, generated: bool) -> Option<Cow<'_, str>> {
        let title = self.title(block).map(Cow::Borrowed);
        if !generated || (title.is_some() && !self.shell_titled(block)) {
            return title;
        }
        self.generated_title(block).map(Cow::Owned).or(title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::activity::{self, parse_process, parse_title};
    use serde_json::json;

    const SESSION: &str = "s";

    /// A terminal whose shell, pid 1, is `shell`, with `program` (pid 7) in its foreground
    /// in `cwd`; none leaves the shell at its prompt.
    fn terminal(shell: &str, program: Option<&str>, cwd: &str) -> HostModel {
        let mut model = HostModel::default();
        let root = json!({"pid": 1, "name": shell, "argv0": format!("-{shell}"), "cwd": cwd});
        let process = match program {
            Some(name) => json!({"home": "/home/u", "ancestors": [root],
                "foreground": {"pid": 7, "name": name, "cwd": cwd}}),
            None => json!({"home": "/home/u", "foreground": root}),
        };
        model.apply_block_event(SESSION, "b".into(), parse_process(&process));
        model
    }

    fn title(model: &mut HostModel, text: &str, owner: Option<u64>) {
        let owner = owner.map(|pid| json!({"pid": pid, "name": "x"}));
        let event = parse_title(&json!({"title": text, "owner": owner}));
        model.apply_block_event(SESSION, "b".into(), event);
    }

    fn shown(model: &HostModel, generated: bool) -> Option<String> {
        model.shown_title("b", generated).map(Cow::into_owned)
    }

    #[test]
    fn a_program_and_its_directory_replace_a_title_the_shell_set() {
        let mut model = terminal("zsh", Some("vim"), "/home/u/rex");
        assert_eq!(shown(&model, true).as_deref(), Some("vim - ~/rex"));
        assert_eq!(shown(&model, false), None, "off: no title, as before");

        // What a shell's prompt and preexec hooks write is the shell's.
        title(&mut model, "u@host:~/rex", Some(1));
        assert_eq!(shown(&model, true).as_deref(), Some("vim - ~/rex"));
        assert_eq!(shown(&model, false).as_deref(), Some("u@host:~/rex"));

        // Back at the prompt the shell is the program.
        let mut model = terminal("zsh", None, "/home/u");
        title(&mut model, "u@host:~", Some(1));
        assert_eq!(shown(&model, true).as_deref(), Some("zsh - ~"));
    }

    #[test]
    fn a_title_a_program_set_for_itself_still_shows() {
        let mut model = terminal("zsh", Some("vim"), "/home/u/rex");
        title(&mut model, "notes.md - VIM", Some(7));
        assert_eq!(shown(&model, true).as_deref(), Some("notes.md - VIM"));
        assert_eq!(shown(&model, false).as_deref(), Some("notes.md - VIM"));

        // It goes stale with its program, and the shell's turn is generated.
        let prompt = json!({"foreground": {"pid": 1, "name": "zsh", "cwd": "/home/u/rex"}});
        model.apply_block_event(SESSION, "b".into(), parse_process(&prompt));
        assert_eq!(model.title("b"), None);
        assert_eq!(shown(&model, true).as_deref(), Some("zsh - ~/rex"));

        // A title nobody is known to own can't be called the shell's.
        title(&mut model, "deploy", None);
        assert_eq!(shown(&model, true).as_deref(), Some("deploy"));
    }

    #[test]
    fn a_terminal_that_started_a_program_keeps_that_programs_title() {
        // No shell: the terminal runs claude itself, and claude titles it.
        let mut model = HostModel::default();
        let process = json!({"home": "/home/u",
            "foreground": {"pid": 1, "name": "claude", "cwd": "/home/u/rex"}});
        model.apply_block_event(SESSION, "b".into(), parse_process(&process));
        assert_eq!(model.places["b"].shell, None);
        title(&mut model, "Claude Code", Some(1));
        assert_eq!(shown(&model, true).as_deref(), Some("Claude Code"));
        title(&mut model, "", None);
        assert_eq!(shown(&model, true).as_deref(), Some("claude - ~/rex"));
    }

    #[test]
    fn the_recorded_events_tell_a_scripts_title_from_the_login_shells() {
        let mut model = HostModel::default();
        let block = "block:01a19e0b-91d8-7213-8fda-04f8b42016e6";
        let fixture: Vec<_> = include_str!("fixtures/title_events.jsonl")
            .lines()
            .collect();
        let replay = |model: &mut HostModel, lines: &[&str]| {
            for line in lines {
                let payload: Value = serde_json::from_str(line).unwrap();
                let (block, event) = activity::parse_block_event(&payload).unwrap();
                model.apply_block_event(SESSION, block, event);
            }
        };
        // A bash script under the login bash titled the terminal: a program's own title.
        replay(&mut model, &fixture[..2]);
        assert_eq!(model.places[block].shell, Some(2903380));
        assert_eq!(
            model.shown_title(block, true).as_deref(),
            Some("OC | USB device exploration")
        );
        // The login shell titles it at its prompt: generated. This server's events name no
        // home directory, so the path is whole.
        replay(&mut model, &fixture[2..]);
        assert_eq!(
            model.shown_title(block, false).as_deref(),
            Some("shuv@shuvdev:~")
        );
        assert_eq!(
            model.shown_title(block, true).as_deref(),
            Some("bash - /home/shuv")
        );
        // A `process` reply names it, and later events without one keep it.
        let reply = json!({"home": "/home/shuv",
            "child": {"pid": 2903380, "name": "bash", "argv0": "-bash", "cwd": "/home/shuv"},
            "foreground": {"pid": 2903380, "name": "bash", "argv0": "-bash", "cwd": "/home/shuv"}});
        model.apply_block_event(SESSION, block.into(), parse_process(&reply));
        assert_eq!(model.shown_title(block, true).as_deref(), Some("bash - ~"));
        replay(&mut model, &fixture[3..]);
        assert_eq!(model.shown_title(block, true).as_deref(), Some("bash - ~"));
    }

    #[test]
    fn the_shell_is_the_process_the_terminal_started() {
        let shell = |process: Value| Place::parse(&process).shell;
        let zsh = json!({"pid": 5, "name": "zsh", "argv0": "-zsh"});
        let login = json!({"pid": 2, "name": "login"});
        let vim = json!({"pid": 9, "name": "vim"});
        assert_eq!(shell(json!({"foreground": zsh})), Some(5));
        assert_eq!(
            shell(json!({"foreground": vim, "ancestors": [zsh]})),
            Some(5)
        );
        // macOS: a login shell runs under login.
        assert_eq!(
            shell(json!({"foreground": vim, "ancestors": [zsh, login], "child": login})),
            Some(5)
        );
        assert_eq!(
            shell(json!({"foreground": zsh, "ancestors": [login]})),
            Some(5)
        );
        assert_eq!(shell(json!({"foreground": login})), None);
        // A reply's child says, even when the chain between was left out.
        assert_eq!(shell(json!({"foreground": vim, "child": zsh})), Some(5));
        // A shell inside the terminal's shell is a program there.
        let bash = json!({"pid": 11, "name": "bash"});
        assert_eq!(
            shell(json!({"foreground": bash, "ancestors": [zsh]})),
            Some(5)
        );
        // An interpreter is named by its script.
        let xonsh = json!({"pid": 3, "name": "python3", "script": "/usr/bin/xonsh"});
        assert_eq!(shell(json!({"foreground": xonsh})), Some(3));
        let script = json!({"pid": 3, "name": "python3", "script": "/home/u/serve.py"});
        assert_eq!(shell(json!({"foreground": script})), None);
        assert_eq!(shell(json!({"foreground": null})), None);
        assert_eq!(shell(Value::Null), None);
    }

    #[test]
    fn the_directory_is_the_foregrounds_or_the_nearest_readable_one() {
        let cwd = |process: Value| Place::parse(&process).cwd;
        let shell = json!({"pid": 1, "name": "zsh", "cwd": "/home/u/rex"});
        assert_eq!(
            cwd(
                json!({"foreground": {"pid": 7, "name": "vim", "cwd": "/srv/app"},
                "ancestors": [shell]})
            )
            .as_deref(),
            Some("/srv/app")
        );
        // Another user's process, as under sudo: the shell's directory stands in.
        assert_eq!(
            cwd(json!({"foreground": {"pid": 7, "name": "vim"}, "ancestors": [shell]})).as_deref(),
            Some("/home/u/rex")
        );
        assert_eq!(cwd(json!({"foreground": {"pid": 7, "name": "vim"}})), None);
        assert_eq!(
            cwd(json!({"foreground": {"pid": 7, "cwd": "relative"}})),
            None,
            "only absolute paths"
        );
        // Without a directory the program alone is the title.
        let mut model = HostModel::default();
        let process = json!({"foreground": {"pid": 7, "name": "vim"}});
        model.apply_block_event(SESSION, "b".into(), parse_process(&process));
        assert_eq!(model.generated_title("b").as_deref(), Some("vim"));
        // And without a program there is none to generate: the title stays.
        let mut model = HostModel::default();
        title(&mut model, "u@host:~", Some(1));
        assert_eq!(model.generated_title("b"), None);
        assert_eq!(shown(&model, true).as_deref(), Some("u@host:~"));
    }

    #[test]
    fn directories_show_home_as_a_tilde_and_keep_their_last_folders() {
        let home = Some("/home/u");
        assert_eq!(directory("/home/u", home), "~");
        assert_eq!(directory("/home/u/rex", home), "~/rex");
        assert_eq!(directory("/home/u/rex", Some("/home/u/")), "~/rex");
        assert_eq!(directory("/home/user2/rex", home), "/home/user2/rex");
        assert_eq!(directory("/srv/app", home), "/srv/app");
        assert_eq!(directory("/home/u/rex", None), "/home/u/rex");
        assert_eq!(
            directory("/", Some("/")),
            "/",
            "a home of / abbreviates nothing"
        );
        assert_eq!(directory("/etc", Some("")), "/etc");
        // 24 characters fit; one more keeps the folders that do.
        assert_eq!(
            directory("/home/u/repos/rex-rune/src/app", home),
            "~/repos/rex-rune/src/app"
        );
        assert_eq!(
            directory("/home/u/.cache/agent-ws/rex-parity/c", home),
            "…/agent-ws/rex-parity/c"
        );
        assert_eq!(
            directory("/usr/share/fonts/truetype/jetbrains-mono", None),
            "…/jetbrains-mono"
        );
        assert_eq!(
            directory("/srv/a-folder-with-a-very-long-name-indeed", None),
            "…/a-folder-with-a-very-long-name-indeed",
            "the last folder is never dropped"
        );
        assert_eq!(
            directory("/home/u/日本語/プロジェクト", home),
            "~/日本語/プロジェクト"
        );
    }

    #[test]
    fn homes_stay_per_terminal_and_missing_metadata_drops_the_old_directory() {
        let mut model = terminal("zsh", Some("vim"), "/home/u/rex");
        let other = json!({"home": "/srv/other", "foreground": {
            "pid": 9, "name": "bash", "cwd": "/srv/other/work"}});
        model.apply_block_event(SESSION, "other".into(), parse_process(&other));
        assert_eq!(
            model.generated_title("other").as_deref(),
            Some("bash - ~/work")
        );
        assert_eq!(model.generated_title("b").as_deref(), Some("vim - ~/rex"));
        // Events omit home on older servers: retain only that terminal's home.
        let moved = json!({"foreground": {"pid": 7, "name": "vim", "cwd": "/home/u/next"}});
        model.apply_block_event(SESSION, "b".into(), parse_process(&moved));
        assert_eq!(model.generated_title("b").as_deref(), Some("vim - ~/next"));
        let unknown = json!({"foreground": {"pid": 7, "name": "vim"}});
        model.apply_block_event(SESSION, "b".into(), parse_process(&unknown));
        assert_eq!(model.generated_title("b").as_deref(), Some("vim"));
        model.apply_block_event(SESSION, "b".into(), parse_process(&Value::Null));
        assert_eq!(model.generated_title("b"), None);
    }

    #[test]
    fn a_closed_block_forgets_where_its_program_was() {
        let mut model = terminal("zsh", Some("vim"), "/home/u/rex");
        assert!(model.places.contains_key("b"));
        model.forget_block("b");
        assert!(model.places.is_empty());
        assert_eq!(shown(&model, true), None);
    }
}
