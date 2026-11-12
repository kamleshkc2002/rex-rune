//! Recording a shortcut in Settings → Shortcuts, and the layer the recordings are kept in.
//!
//! - A row's keys are buttons ([`key_button`], [`record_button`]). Pressing one turns the
//!   row's trailing slot into the recorder ([`control`]): the keys typed next are shown,
//!   Enter saves them, Escape cancels, and Backspace takes back the last key or, with none
//!   typed, removes the binding being replaced. While it is open it has the keyboard and a
//!   press anywhere else cancels it.
//! - What is recorded goes to `Settings::recorded_shortcuts`, a keymap layer above the
//!   user's `shortcuts.json` (`rex_rune::settings::recorded`), saved in the app's own
//!   settings file. `shortcuts.json` is never written. The layer is declared to every host
//!   with `client.keymap` and resolved locally at once, so a new key works immediately.
//! - The Configuration menu ([`menu_button`]) says how much is recorded, copies the path of
//!   `shortcuts.json`, copies the recordings as a workspace.json fragment, and resets them
//!   all after asking.
//! - In Commands, Ctrl Enter opens the highlighted command's row here, recording.
use super::{
    actions,
    bindings::{Input, chord_of},
    commands::Run,
    keycaps::{Chords, keycaps, spell},
    panel::{Entry, Intent, PanelKind, SettingsSection},
    shortcuts_page::{Row, holder, identity},
    theme::*,
    widgets::{Pill, glyph, pill},
    workspace::Workspace,
};
use rex_rune::{
    connection::HostCommand,
    keymap::{self, Chord, Key as BoundKey, RECORDED, Resolved},
    settings::{Settings, shortcuts::Shortcuts},
};
use rexrs::api::client::{Binding, KeymapDefault};
use rune::{Cursor, El, Icon, Ime, Key, KeyEvent, TextEdit, Tx, col, row, text};
use serde_json::Value;

/// Opens Settings → Shortcuts.
pub const EDIT: &str = "client.shortcuts";
/// The Configuration menu's button, which its list is placed against.
pub const MENU: &str = "shortcut-menu";
/// The recorder's field.
pub const FIELD: &str = "shortcut-recorder";
/// The most chords a recorded sequence has.
const LONGEST: usize = 4;
const FIELD_WIDTH: f32 = 176.;
/// The field in a window too narrow for that and the row's name.
const FIELD_LEAST: f32 = 96.;
const CONTROL: f32 = 26.;

static MORE: Icon = Icon {
    name: "rex-more",
    view: 20.0,
    body: r#"<circle cx="5" cy="10" r="1.3" fill="currentColor"/><circle cx="10" cy="10" r="1.3" fill="currentColor"/><circle cx="15" cy="10" r="1.3" fill="currentColor"/>"#,
};
static TRASH: Icon = Icon {
    name: "rex-trash",
    view: 20.0,
    body: r#"<path d="M4.5 6h11M8 6V4.5h4V6M6 6l.7 9.5h6.6L14 6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

/// A row of Settings → Shortcuts taking a new key.
#[derive(Clone, Debug)]
pub struct Recorder {
    /// The row: its `Row::id`.
    pub row: String,
    title: String,
    action: String,
    /// The row's arguments.
    args: Option<Value>,
    /// A replacement stays in the host mode that owned the old binding.
    mode: Option<String>,
    /// The binding the new keys replace; none on a row that is getting its first.
    replacing: Option<Binding>,
    /// The chords typed so far.
    pub chords: Vec<Chord>,
    /// Why the last key was not taken.
    refused: Option<&'static str>,
    /// An input method is composing: the keys it takes are not chords.
    composing: bool,
}

/// What saving the typed keys would do to the rest of the keymap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing else has the keys.
    Free,
    /// The row has them already.
    Same,
    /// Another action has them (`exact`), or a sequence that overlaps them: it loses `keys`.
    Taken {
        title: String,
        keys: String,
        exact: bool,
    },
}

/// A row of the Configuration menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    /// How much is recorded: a line of the menu, never a command.
    Status,
    CopyPath,
    CopyJson,
    /// Ask, then forget every recording.
    ResetAll,
}

fn caps(chords: &[Chord]) -> Chords {
    chords.iter().map(Chord::keycaps).collect()
}

/// Whether a shortcut may start with `chord`: one that has Ctrl, Alt or Super, a function
/// key, a key with no character of its own (Insert, Pause), or Shift with a named key. A
/// plain character or editing key is the terminal program's.
fn starts(chord: &Chord) -> bool {
    let m = chord.modifiers;
    if m.ctrl || m.alt || m.logo {
        return true;
    }
    match chord.key {
        BoundKey::Character(_) => false,
        BoundKey::Named(name) => {
            name.strip_prefix('f')
                .is_some_and(|n| n.parse::<u8>().is_ok())
                || (m.shift && name != "space")
        }
        BoundKey::Physical(_) => true,
    }
}

/// The arguments a first key for a row is recorded with: the row's, and the `when` of the
/// Linux default for it, so a key recorded for Find next still waits for the find bar.
fn new_args(row: &Row) -> Option<Value> {
    keymap::defaults()
        .bindings
        .into_iter()
        .filter(|default| row.runs(&default.action, default.args.as_ref()))
        .filter_map(|default| default.args)
        .find(|args| args.get("when").is_some())
        .or_else(|| row.args.clone())
}

/// The override layers declared to a host, lowest first: the user's file, then what was
/// recorded here. Nothing recorded declares what earlier versions did.
pub fn layers(settings: &Settings) -> Vec<KeymapDefault> {
    let mut layers = vec![settings.shortcut_file.overrides.layer.clone()];
    if !settings.recorded_shortcuts.is_empty() {
        layers.push(settings.recorded_shortcuts.layer());
    }
    layers
}

/// This client's keymap without a host: the Linux defaults, the user's file and what was
/// recorded here.
pub fn local_keymap(settings: &Settings) -> Resolved {
    actions::resolve_all(&[
        &settings.shortcut_file.overrides,
        &settings.recorded_shortcuts.overrides(),
    ])
}

/// Whether the user's file or the Linux defaults bind `keys` to `action` with `args`.
fn bound_below(
    settings: &Settings,
    keys: &[String],
    action: &str,
    args: Option<&Value>,
    mode: Option<&str>,
) -> bool {
    let wanted = identity(action, args);
    keymap::defaults()
        .bindings
        .iter()
        .chain(&settings.shortcut_file.overrides.layer.bindings)
        .any(|binding| {
            binding.keys == keys
                && binding.mode.as_deref() == mode
                && identity(&binding.action, binding.args.as_ref()) == wanted
                && binding.args.as_ref().and_then(|a| a.get("when"))
                    == args.and_then(|a| a.get("when"))
        })
}

impl Recorder {
    /// What saving the keys typed so far would do, given the page's `rows`.
    pub fn verdict(&self, rows: &[Row]) -> Verdict {
        if self.chords.is_empty() {
            return Verdict::Free;
        }
        if self.unchanged() {
            return Verdict::Same;
        }
        let mut candidate = Binding::new(self.chords.iter().map(Chord::spelling), &self.action);
        candidate.args = self.args.clone();
        candidate.mode = self.mode.clone();
        match holder(rows, &self.chords, &candidate, self.replacing.as_ref()) {
            None => Verdict::Free,
            Some((row, held))
                if row.id() == self.row
                    && held.chords == self.chords
                    && actions::covers_context(&held.binding, &candidate) =>
            {
                Verdict::Same
            }
            Some((row, held)) => Verdict::Taken {
                title: row.title.clone(),
                keys: spell(&caps(&held.chords)),
                exact: held.chords == self.chords,
            },
        }
    }

    /// The keys typed are the ones being replaced.
    fn unchanged(&self) -> bool {
        self.replacing.as_ref().is_some_and(|old| {
            keymap::parse_keys(&old.keys).is_ok_and(|chords| chords == self.chords)
        })
    }

    /// The keys being replaced, as a hint spells them.
    fn old_keys(&self) -> Option<String> {
        let old = self.replacing.as_ref()?;
        Some(keymap::display(&old.keys))
    }

    /// The row's helper line while it records, and whether it is a warning.
    pub fn status(&self, verdict: &Verdict) -> (String, bool) {
        if let Some(why) = self.refused {
            return (why.to_owned(), true);
        }
        if self.chords.is_empty() {
            return (
                match self.old_keys() {
                    Some(old) => {
                        format!("Type the new keys. Backspace removes {old}; Esc cancels.")
                    }
                    None => "Type the keys. Esc cancels.".to_owned(),
                },
                false,
            );
        }
        match verdict {
            Verdict::Free => (
                "Enter saves; Esc cancels. Another key makes it a sequence.".to_owned(),
                false,
            ),
            Verdict::Same => ("Already its shortcut.".to_owned(), false),
            Verdict::Taken {
                title, exact: true, ..
            } => (
                format!("Also bound to {title}. Saving takes priority where both apply."),
                true,
            ),
            Verdict::Taken { title, keys, .. } => (
                format!("Overlaps {keys} ({title}). Saving takes priority where both apply."),
                true,
            ),
        }
    }
}

impl Workspace {
    /// The recorder, while Settings shows the page it is on.
    pub fn recording(&self) -> Option<&Recorder> {
        let settings = self
            .panel
            .as_ref()
            .is_some_and(|panel| matches!(panel.kind, PanelKind::Settings));
        self.recorder
            .as_ref()
            .filter(|_| settings && self.settings_section == SettingsSection::Shortcuts)
    }

    /// Start recording on the Shortcuts row `row`: over its `index`th shown key, or a first
    /// key for a row that has none.
    pub fn record_shortcut(&mut self, row: &str, index: Option<usize>, tx: &mut Tx) {
        let rows = self.shortcut_rows();
        let Some(found) = rows.iter().find(|r| r.id() == row && r.recordable) else {
            return;
        };
        let replacing = index
            .and_then(|index| found.shown().nth(index))
            .map(|held| held.binding.clone());
        self.bindings.sequence.clear();
        // The row grows for the recorder: all of it stays on the page.
        tx.scroll_into_view(format!("shortcut:{row}"));
        self.recorder = Some(Recorder {
            row: row.to_owned(),
            title: found.title.clone(),
            action: found.action.clone(),
            args: match &replacing {
                Some(old) => old.args.clone(),
                None => new_args(found),
            },
            mode: replacing.as_ref().and_then(|b| b.mode.clone()),
            replacing,
            chords: Vec::new(),
            refused: None,
            composing: false,
        });
    }

    /// Leave the recorder with nothing changed.
    pub fn cancel_recording(&mut self, tx: &mut Tx) {
        if let Some(recorder) = self.recorder.take() {
            focus_row(&recorder.row, tx);
        }
    }

    /// Give the typed keys to the row's action, in place of the binding being replaced.
    pub fn save_recording(&mut self, tx: &mut Tx) {
        let Some(recorder) = self.recorder.take() else {
            return;
        };
        if recorder.chords.is_empty() {
            self.recorder = Some(recorder);
            return;
        }
        focus_row(&recorder.row, tx);
        if recorder.unchanged() {
            return;
        }
        let verdict = recorder.verdict(&self.shortcut_rows());
        if let Some(old) = &recorder.replacing {
            self.take_off(old);
        }
        // The row has the keys from another of its bindings: only the old ones go.
        if verdict != Verdict::Same {
            let keys: Vec<String> = recorder.chords.iter().map(Chord::spelling).collect();
            // Only restore a matching lower-layer binding. Another row of the same
            // action may have removed these keys; its removal must survive this binding.
            let restored = bound_below(
                &self.settings,
                &keys,
                &recorder.action,
                recorder.args.as_ref(),
                recorder.mode.as_deref(),
            ) && self.settings.recorded_shortcuts.restore(
                &keys,
                &recorder.action,
                recorder.mode.as_deref(),
            );
            if !restored {
                self.settings.recorded_shortcuts.bind(
                    keys,
                    &recorder.action,
                    recorder.args,
                    recorder.mode,
                );
            }
        }
        self.recorded_changed();
        self.reveal_recorded_row(&recorder.row, &recorder.title, tx);
    }

    /// Keep keyboard focus useful when a key-filtered row loses its matching binding.
    fn reveal_recorded_row(&mut self, id: &str, title: &str, tx: &mut Tx) {
        let all = self.shortcut_rows();
        if !all.iter().any(|row| row.id() == id) {
            tx.focus("shortcut-filter", true);
            return;
        }
        let shown = super::shortcuts_page::listing(all, &self.shortcut_filter.text);
        if !shown
            .rows
            .iter()
            .chain(shown.key.iter().flat_map(|(_, rows)| rows))
            .any(|row| row.id() == id)
        {
            self.shortcut_filter = TextEdit::new(title);
        }
        tx.scroll_into_view(format!("shortcut:{id}"));
        focus_row(id, tx);
    }

    /// Take the binding being replaced off the row's action, and leave the recorder.
    pub fn remove_recording(&mut self, tx: &mut Tx) {
        let Some(recorder) = self.recorder.take() else {
            return;
        };
        focus_row(&recorder.row, tx);
        if let Some(old) = &recorder.replacing {
            self.take_off(old);
            self.recorded_changed();
        }
        self.reveal_recorded_row(&recorder.row, &recorder.title, tx);
    }

    /// Stop `old` running its action: forget it when it was recorded here, else record that
    /// its keys are off the action in the layers below.
    fn take_off(&mut self, old: &Binding) {
        let recorded = &mut self.settings.recorded_shortcuts;
        if old.source.as_deref() == Some(RECORDED)
            && recorded.unbind(
                &old.keys,
                &old.action,
                old.args.as_ref(),
                old.mode.as_deref(),
            )
        {
            return;
        }
        recorded.remove(
            old.keys.clone(),
            &old.action,
            old.args.clone(),
            old.mode.clone(),
        );
    }

    /// Save the recorded layer and put it in effect: here at once, and on every host as its
    /// next keymap.
    fn recorded_changed(&mut self) {
        self.save();
        self.keymap_changed();
    }

    /// Forget everything recorded here.
    pub fn reset_recorded(&mut self) {
        self.recorder = None;
        if self.settings.recorded_shortcuts.is_empty() {
            return;
        }
        self.settings.recorded_shortcuts.clear();
        self.recorded_changed();
        self.inform("Recorded shortcuts were reset.");
    }

    /// Settings → Shortcuts.
    pub fn open_shortcuts(&mut self, tx: &mut Tx) {
        self.perform(Intent::Settings, tx);
        self.open_section(SettingsSection::Shortcuts);
    }

    /// Ctrl Enter in Commands: open the highlighted command's row in Settings → Shortcuts,
    /// recording. A row that is not a command with an action stays where it is.
    pub fn edit_selected_shortcut(&mut self, tx: &mut Tx) {
        let Some(selected) = self.panel.as_ref().map(|panel| panel.selected) else {
            return;
        };
        let Some(entry) = self.entries().into_iter().nth(selected) else {
            return;
        };
        let identity = match &entry.action {
            Intent::Command(Run::ServerAction(name)) => Some((name.clone(), Value::Null)),
            Intent::CreateOn(host, None) if *host == self.active => {
                Some(("session.new".to_owned(), Value::Null))
            }
            action => action
                .shortcut()
                .map(|(name, args)| (name.to_owned(), args)),
        };
        let label = entry.label.trim_end_matches('…').to_owned();
        let Some((action, args)) = identity else {
            self.inform(format!("{label} has no action a shortcut could run."));
            return;
        };
        self.open_shortcuts(tx);
        let rows = self.shortcut_rows();
        let row = rows
            .iter()
            .find(|row| row.recordable && row.runs(&action, Some(&args)));
        match row {
            Some(row) => {
                let id = row.id();
                self.record_shortcut(&id, row.shown().next().map(|_| 0), tx);
            }
            // An action that needs an argument: its rows, whatever they are bound with.
            None => self.shortcut_filter = TextEdit::new(label),
        }
    }

    /// The Configuration menu, against its button.
    pub fn open_shortcut_menu(&mut self) {
        self.recorder = None;
        self.open_panel(PanelKind::ShortcutMenu, "Shortcuts", &[]);
        let panel = self.panel.as_mut().expect("the menu just opened");
        panel.anchor_to = Some(MENU.to_owned());
    }

    /// The Configuration menu's rows: how much is recorded, the two copies, and Reset all.
    pub(super) fn shortcut_menu_entries(&self) -> Vec<Entry> {
        let count = self.settings.recorded_shortcuts.len();
        let status = match count {
            0 => "No recorded shortcuts".to_owned(),
            1 => "1 recorded change".to_owned(),
            n => format!("{n} recorded changes"),
        };
        let mut status = Entry::new(status, "", "", Intent::Shortcuts(Menu::Status));
        status.enabled = false;
        let path = Entry::new(
            "Copy shortcuts.json path",
            "",
            "",
            Intent::Shortcuts(Menu::CopyPath),
        )
        .in_section(1);
        let mut json = Entry::new(
            "Copy recorded shortcuts as JSON",
            "",
            "",
            Intent::Shortcuts(Menu::CopyJson),
        )
        .in_section(1);
        json.enabled = count > 0;
        let mut reset = Entry::new("Reset all…", "", "", Intent::Shortcuts(Menu::ResetAll))
            .in_section(2)
            .destructive();
        reset.enabled = count > 0;
        vec![status, path, json, reset]
    }

    /// Run a row of the Configuration menu, back over Settings.
    pub(super) fn shortcut_menu(&mut self, item: Menu, tx: &mut Tx) {
        match self.settings_back() {
            Some(back) => self.panel = Some(back),
            None => self.close_panel(tx),
        }
        match item {
            Menu::Status => {}
            Menu::CopyPath => {
                tx.copy(Shortcuts::path(&self.settings_path).display().to_string());
                self.inform("Copied the path of shortcuts.json.");
            }
            Menu::CopyJson => {
                let file = self.settings.recorded_shortcuts.as_file();
                tx.copy(serde_json::to_string_pretty(&file).expect("a JSON value"));
                self.inform("Copied recorded_shortcuts as JSON for workspace.json.");
            }
            Menu::ResetAll => {
                if !self.settings.recorded_shortcuts.is_empty() {
                    self.open_panel(
                        PanelKind::ResetShortcuts,
                        "Reset all recorded shortcuts?",
                        &[],
                    );
                }
            }
        }
    }

    /// What the Reset all confirmation says will happen.
    pub(super) fn reset_shortcuts_words(&self) -> String {
        let count = self.settings.recorded_shortcuts.len();
        format!(
            "The {} recorded in Settings {} forgotten. Every action goes back to its keys from shortcuts.json, the host's Rex configuration or the Linux defaults; shortcuts.json itself is not changed.",
            if count == 1 {
                "one change".to_owned()
            } else {
                format!("{count} changes")
            },
            if count == 1 { "is" } else { "are" },
        )
    }

    /// The local keymap or its layers changed: resolve again and declare it to every host.
    pub(super) fn keymap_changed(&mut self) {
        self.local_keymap = local_keymap(&self.settings);
        self.keymap_generation += 1;
        self.bindings.sequence.clear();
        // Disconnected workers retain the declaration for their next successful connect too.
        let declaration = self.keymap_declaration();
        for index in 0..self.hosts.len() {
            self.send(index, HostCommand::Keymap(Box::new(declaration.clone())));
        }
    }
}

/// Focus back on the row's first key button once the recorder is gone.
fn focus_row(row: &str, tx: &mut Tx) {
    tx.focus(format!("shortcut-key:{row}:0"), tx.focus_visible());
}

/// Keys while the recorder is open. Every key is its own: none reaches a shortcut, the page
/// or the terminal.
pub fn key(s: &mut Workspace, event: &KeyEvent, tx: &mut Tx) -> bool {
    if s.recording().is_none() {
        return false;
    }
    // Keep repeats/releases local even if Save or Escape closes the recorder this press.
    let identity = Input {
        event,
        code: tx.physical_key(),
        base: tx.key_base(),
        repeat: tx.key_repeat(),
    }
    .identity();
    s.bindings.consumed.insert(identity, true);
    let Some(recorder) = s.recorder.as_mut() else {
        return false;
    };
    // Escape always leaves the recorder, even during composition.
    if event.key == Key::Escape {
        s.cancel_recording(tx);
        return true;
    }
    // A held key is one press, and a key an input method is composing with is its own.
    if tx.key_repeat() || recorder.composing {
        return true;
    }
    let m = event.mods;
    let bare = !(m.ctrl || m.cmd || m.alt || m.shift || m.logo);
    match event.key {
        Key::Enter if bare && !recorder.chords.is_empty() => {
            s.save_recording(tx);
            return true;
        }
        Key::Backspace | Key::Delete if bare => {
            recorder.refused = None;
            if recorder.chords.pop().is_none() && recorder.replacing.is_some() {
                s.remove_recording(tx);
            }
            return true;
        }
        Key::Modifier { .. } | Key::Dead(_) => return true,
        _ => {}
    }
    let input = Input {
        event,
        code: tx.physical_key(),
        base: tx.key_base(),
        repeat: false,
    };
    recorder.refused = match chord_of(input) {
        None => Some("That key has no name a shortcut can use."),
        Some(chord) if recorder.chords.is_empty() && !starts(&chord) => {
            Some("Start with Ctrl, Alt or Super, or use a function key.")
        }
        Some(_) if recorder.chords.len() == LONGEST => {
            Some("Four keys at most. Backspace takes one back.")
        }
        Some(chord) => {
            recorder.chords.push(chord);
            None
        }
    };
    true
}

/// Input-method events while the recorder is open: composing text is not a shortcut, and
/// what it commits is not typed anywhere. False when nothing is recording.
pub fn ime(s: &mut Workspace, ime: &Ime) -> bool {
    if s.recording().is_none() {
        return false;
    }
    let Some(recorder) = s.recorder.as_mut() else {
        return false;
    };
    recorder.composing = matches!(ime, Ime::Preedit(text, _) if !text.is_empty());
    true
}

/// The frame a row's keys and its Record shortcut button share: a quiet outline that says
/// it can be pressed, filled under the pointer.
fn slot(key: String, label: String, help: &str) -> El {
    row()
        .key(key)
        .role("button")
        .label(label)
        .help(help.to_owned())
        .h(CONTROL)
        .px(5.)
        .center()
        .radius(8.)
        .continuous_corners()
        .border(1., white(0.07))
        .cursor(Cursor::Pointer)
        .hover(white(0.07))
        .press(white(0.11))
        .focusable(true)
        .focus_ring(2., 2., rgb(accent()))
}

/// Return presses a focused control as Space does: Settings has no default button.
fn pressable(e: El, action: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static) -> El {
    let on_return = action.clone();
    e.on_press_tx(action)
        .on_key(move |s: &mut Workspace, event: &KeyEvent, tx: &mut Tx| {
            (event.key == Key::Enter)
                .then(|| on_return(s, tx))
                .is_some()
        })
}

/// One binding of a row, as keycaps on a button that records others in its place.
pub fn key_button(id: &str, index: usize, title: &str, chords: &Chords) -> El {
    let row_id = id.to_owned();
    pressable(
        slot(
            format!("shortcut-key:{id}:{index}"),
            format!("Change the shortcut for {title}, {}", spell(chords)),
            &format!("Change shortcut · {}", spell(chords)),
        )
        .h(CONTROL + chords.len().saturating_sub(1) as f32 * 21.)
        .child(
            col()
                .gap(4.)
                .items_end()
                .children(chords.iter().enumerate().map(|(i, chord)| {
                    let mut line = row().gap(4.).center_y();
                    if i > 0 {
                        line = line.child(text("then").font(ui(10.5)).color(rgb(muted())));
                    }
                    line.child(keycaps(std::slice::from_ref(chord), false))
                })),
        ),
        move |s: &mut Workspace, tx: &mut Tx| s.record_shortcut(&row_id, Some(index), tx),
    )
}

/// The button of a row with no key.
pub fn record_button(id: &str, title: &str) -> El {
    let row_id = id.to_owned();
    pressable(
        slot(
            format!("shortcut-key:{id}:0"),
            format!("Record a shortcut for {title}"),
            "Record shortcut",
        )
        .px(9.)
        .hover_group()
        .child(
            text("Record shortcut")
                .font(ui(12.))
                .color(rgb(muted()))
                .group_color(rgb(ink())),
        ),
        move |s: &mut Workspace, tx: &mut Tx| s.record_shortcut(&row_id, None, tx),
    )
}

/// A round icon button of the recorder. It never takes focus: the keys stay the field's.
fn icon_button(
    key: &str,
    icon: &'static Icon,
    label: String,
    destructive: bool,
    action: impl Fn(&mut Workspace, &mut Tx) + 'static,
) -> El {
    let tint = if destructive { danger() } else { muted() };
    row()
        .key(key.to_owned())
        .role("button")
        .label(label.clone())
        .help(label)
        .size(CONTROL, CONTROL)
        .center()
        .radius(CONTROL / 2.)
        .cursor(Cursor::Pointer)
        .hover(if destructive {
            rgb(danger()).alpha(0.16)
        } else {
            white(0.08)
        })
        .press(white(0.12))
        .focusable(false)
        .preserve_focus_on_press(true)
        .on_press_tx(action)
        .child(glyph(icon, 15., rgb(tint)))
}

/// The recorder, in the row's trailing slot: the field that shows what is typed, Save once
/// something is, Cancel, and Remove for a binding being replaced.
pub fn control(recorder: &Recorder) -> El {
    let typed = caps(&recorder.chords);
    let said = if typed.is_empty() {
        "type the keys".to_owned()
    } else {
        spell(&typed)
    };
    let field = row()
        .key(FIELD)
        .label(format!("Shortcut for {}: {said}", recorder.title))
        .help(said.clone())
        .w(FIELD_WIDTH)
        .shrink()
        .min_w(FIELD_LEAST)
        .h(CONTROL + 2.)
        .px(8.)
        .center_y()
        .radius(8.)
        .continuous_corners()
        .fill(white(0.05))
        .border(1.5, rgb(accent()))
        .focusable(true)
        .autofocus()
        .no_focus_ring()
        .child(if typed.is_empty() {
            text("Type shortcut…").font(ui(12.)).color(rgb(muted()))
        } else {
            // Four chords can exceed even the full-size field. Keep the readout inside
            // it; the tooltip and accessible name retain the complete sequence.
            text(said)
                .key("shortcut-readout")
                .font(ui(12.))
                .color(rgb(ink()))
                .truncate()
                .min_w(0.)
                .grow(1.)
        });
    // The field takes the place the row's keys had, under the pointer that pressed them;
    // what can be pressed next is beside it, never under that pointer.
    let mut control = row()
        .gap(4.)
        .center_y()
        .shrink()
        .min_w(0.)
        .modal_tx(true, |s: &mut Workspace, tx: &mut Tx| {
            s.cancel_recording(tx)
        })
        .on_key(key);
    if let Some(old) = recorder.old_keys() {
        control = control.child(icon_button(
            "shortcut-remove",
            &TRASH,
            format!("Remove {old} from {}", recorder.title),
            true,
            |s, tx| s.remove_recording(tx),
        ));
    }
    control = control.child(icon_button(
        "shortcut-cancel",
        &CLOSE,
        "Cancel recording".to_owned(),
        false,
        |s, tx| s.cancel_recording(tx),
    ));
    if !typed.is_empty() {
        control = control.child(
            pill("shortcut-save", "Save", Pill::Primary, CONTROL, |s, tx| {
                s.save_recording(tx)
            })
            .focusable(false)
            .preserve_focus_on_press(true)
            .ml(2.),
        );
    }
    control.child(field.ml(4.))
}

/// The Configuration menu's button, beside the filter.
pub fn menu_button() -> El {
    pressable(
        row()
            .key(MENU)
            .role("button")
            .label("Shortcut configuration")
            .help("Shortcut configuration")
            .size(32., 32.)
            .center()
            .radius(16.)
            .fill(white(0.05))
            .hover(white(0.09))
            .press(white(0.12))
            .cursor(Cursor::Pointer)
            .focusable(true)
            .focus_ring(2., 2., rgb(accent()))
            .child(glyph(&MORE, 16., rgb(muted()))),
        |s: &mut Workspace, _: &mut Tx| s.open_shortcut_menu(),
    )
}
