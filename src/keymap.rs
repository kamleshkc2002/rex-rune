//! Rex key spelling and layered workspace shortcuts.
//!
//! Adapted from Andrea Zito (nivox)'s `8037d78` modifier aliases, sequence syntax
//! and tests. Rex's recorded `client.keymap` reply is the spelling authority.
use anyhow::{Result, bail, ensure};
use rexrs::api::client::{Binding, Diagnostic, InspectResponseAction, KeymapDefault, Removal};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// `cmd` in Rex means the logo/Super key, not Rune's platform shortcut modifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Character(char),
    Named(&'static str),
    Physical(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub modifiers: Modifiers,
    pub key: Key,
}

const NAMED: &[&str] = &[
    "enter",
    "escape",
    "tab",
    "space",
    "backspace",
    "delete",
    "insert",
    "left",
    "right",
    "up",
    "down",
    "home",
    "end",
    "pageup",
    "pagedown",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
];

// W3C KeyboardEvent.code spellings, independent of Rune so protocol-only builds
// can parse overrides too. Unknown codes must not silently become dead shortcuts.
const PHYSICAL: &[&str] = &[
    "Backquote",
    "Backslash",
    "BracketLeft",
    "BracketRight",
    "Comma",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Equal",
    "IntlBackslash",
    "IntlRo",
    "IntlYen",
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Minus",
    "Period",
    "Quote",
    "Semicolon",
    "Slash",
    "AltLeft",
    "AltRight",
    "Backspace",
    "CapsLock",
    "ContextMenu",
    "ControlLeft",
    "ControlRight",
    "Enter",
    "MetaLeft",
    "MetaRight",
    "ShiftLeft",
    "ShiftRight",
    "Space",
    "Tab",
    "Convert",
    "KanaMode",
    "Lang1",
    "Lang2",
    "Lang3",
    "Lang4",
    "Lang5",
    "NonConvert",
    "Delete",
    "End",
    "Help",
    "Home",
    "Insert",
    "PageDown",
    "PageUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "NumLock",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadAdd",
    "NumpadBackspace",
    "NumpadClear",
    "NumpadClearEntry",
    "NumpadComma",
    "NumpadDecimal",
    "NumpadDivide",
    "NumpadEnter",
    "NumpadEqual",
    "NumpadHash",
    "NumpadMemoryAdd",
    "NumpadMemoryClear",
    "NumpadMemoryRecall",
    "NumpadMemoryStore",
    "NumpadMemorySubtract",
    "NumpadMultiply",
    "NumpadParenLeft",
    "NumpadParenRight",
    "NumpadStar",
    "NumpadSubtract",
    "Escape",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
    "Fn",
    "FnLock",
    "PrintScreen",
    "ScrollLock",
    "Pause",
    "BrowserBack",
    "BrowserFavorites",
    "BrowserForward",
    "BrowserHome",
    "BrowserRefresh",
    "BrowserSearch",
    "BrowserStop",
    "Eject",
    "LaunchApp1",
    "LaunchApp2",
    "LaunchMail",
    "MediaPlayPause",
    "MediaSelect",
    "MediaStop",
    "MediaTrackNext",
    "MediaTrackPrevious",
    "Power",
    "Sleep",
    "AudioVolumeDown",
    "AudioVolumeMute",
    "AudioVolumeUp",
    "WakeUp",
    "Again",
    "Copy",
    "Cut",
    "Find",
    "Open",
    "Paste",
    "Props",
    "Select",
    "Undo",
];

impl Chord {
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        // split('+') alone loses the literal plus key (German layouts).
        let (mods, key) = if input == "+" {
            ("", "+")
        } else if let Some(mods) = input.strip_suffix("++") {
            (mods, "+")
        } else {
            input.rsplit_once('+').unwrap_or(("", input))
        };
        let mut modifiers = Modifiers::default();
        if !mods.is_empty() {
            for modifier in mods.split('+') {
                let modifier = modifier.trim().to_ascii_lowercase();
                let enabled = match modifier.as_str() {
                    "ctrl" | "control" => &mut modifiers.ctrl,
                    "alt" | "opt" | "option" => &mut modifiers.alt,
                    "shift" => &mut modifiers.shift,
                    "cmd" | "command" | "super" | "meta" | "win" => &mut modifiers.logo,
                    _ => bail!("unknown modifier {modifier:?}"),
                };
                ensure!(!*enabled, "duplicate modifier {modifier:?}");
                *enabled = true;
            }
        }
        // A leading separator is not an empty modifier list.
        ensure!(!input.starts_with('+') || input == "+", "empty modifier");
        let key = key.trim();
        let key = if let Some(code) = key.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            Key::Physical(
                PHYSICAL
                    .iter()
                    .copied()
                    .find(|name| name.eq_ignore_ascii_case(code))
                    .ok_or_else(|| anyhow::anyhow!("unknown physical key {key:?}"))?,
            )
        } else if let Some(name) = NAMED.iter().copied().find(|n| n.eq_ignore_ascii_case(key)) {
            Key::Named(name)
        } else {
            let lower = key.to_lowercase();
            let mut chars = lower.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if !c.is_control() && !c.is_whitespace() && c != '>' => {
                    Key::Character(c)
                }
                _ => bail!("unknown key {key:?}"),
            }
        };
        Ok(Self { modifiers, key })
    }

    pub fn spelling(&self) -> String {
        let mut parts = Vec::new();
        for (enabled, name) in [
            (self.modifiers.ctrl, "ctrl"),
            (self.modifiers.alt, "alt"),
            (self.modifiers.shift, "shift"),
            (self.modifiers.logo, "cmd"),
        ] {
            if enabled {
                parts.push(name.to_owned());
            }
        }
        parts.push(match self.key {
            Key::Character(c) => c.to_string(),
            Key::Named(name) => name.into(),
            Key::Physical(code) => format!("[{code}]"),
        });
        parts.join("+")
    }

    pub fn keycaps(&self) -> Vec<String> {
        let mut caps = Vec::new();
        for (enabled, label) in [
            (self.modifiers.ctrl, "Ctrl"),
            (self.modifiers.alt, "Alt"),
            (self.modifiers.shift, "Shift"),
            (self.modifiers.logo, "Super"),
        ] {
            if enabled {
                caps.push(label.into());
            }
        }
        caps.push(match self.key {
            Key::Character('+') => "Plus".into(),
            Key::Character(c) => c.to_uppercase().to_string(),
            Key::Physical(code) => code.into(),
            Key::Named("pageup") => "PageUp".into(),
            Key::Named("pagedown") => "PageDown".into(),
            Key::Named(name) => {
                let mut label = name.to_owned();
                label[..1].make_ascii_uppercase();
                label
            }
        });
        caps
    }
}

/// User-file syntax only. Wire keys are already split, and must not be joined
/// then reparsed: an unreadable server spelling is preserved verbatim.
pub fn normalize(input: &str) -> Result<Vec<String>> {
    input
        .split('>')
        .map(|part| Chord::parse(part).map(|chord| chord.spelling()))
        .collect()
}

/// Parse a server binding once. Failure disables the entire binding, not just
/// its unreadable chord; keep the original keys separately for display/prefix.
pub fn parse_keys(keys: &[String]) -> Result<Vec<Chord>> {
    ensure!(!keys.is_empty(), "empty key sequence");
    keys.iter().map(|key| Chord::parse(key)).collect()
}

/// Unknown server spellings stay visible, but are never usable as match keys.
pub fn display(keys: &[String]) -> String {
    keys.iter()
        .map(|key| {
            Chord::parse(key)
                .map(|chord| chord.keycaps().join(" "))
                .unwrap_or_else(|_| key.clone())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Built-in defaults are declared to Rex, never copied into the user's file.
pub fn defaults() -> KeymapDefault {
    let mut bindings = Vec::new();
    for (keys, action) in [
        ("ctrl+k", "session.switch"),
        ("ctrl+shift+p", "client.palette.toggle"),
        ("ctrl+,", "client.settings"),
        ("ctrl+shift+b", "client.sidebar.toggle"),
        ("ctrl+shift+n", "session.new"),
        ("ctrl+shift+t", "client.tab.new"),
        ("ctrl+shift+w", "client.close"),
        ("ctrl+tab", "client.tab.next_global"),
        ("ctrl+shift+tab", "client.tab.previous_global"),
        ("ctrl+pageup", "client.tab.previous"),
        ("ctrl+pagedown", "client.tab.next"),
        ("ctrl+shift+enter", "pane.zoom"),
        ("ctrl+shift+space", "pane.actions"),
        ("ctrl+shift+c", "pane.copy"),
        ("ctrl+shift+v", "pane.paste"),
        ("ctrl+shift+f", "client.search"),
        ("ctrl+shift+up", "client.prompt.previous"),
        ("ctrl+shift+down", "client.prompt.next"),
        ("ctrl+shift+r", "pane.reconnect"),
        // Of an action's bindings the last is the one Commands and menus show: the main
        // block's key, not the numpad's.
        ("ctrl+[NumpadAdd]", "client.zoom.in"),
        ("ctrl+=", "client.zoom.in"),
        ("ctrl+shift+=", "client.zoom.in"),
        ("ctrl++", "client.zoom.in"),
        ("ctrl+[NumpadSubtract]", "client.zoom.out"),
        ("ctrl+-", "client.zoom.out"),
        ("ctrl+[Numpad0]", "client.zoom.reset"),
        ("ctrl+0", "client.zoom.reset"),
    ] {
        bindings.push(Binding::new([keys], action));
    }
    for (keys, direction) in [("ctrl+shift+d", "right"), ("ctrl+alt+d", "down")] {
        bindings.push(Binding::new([keys], "pane.split").args(json!({"direction": direction})));
    }
    for direction in ["left", "right", "up", "down"] {
        bindings.push(
            Binding::new([format!("ctrl+alt+{direction}")], "pane.focus")
                .args(json!({"direction": direction})),
        );
    }
    for (keys, direction) in [
        ("ctrl+shift+pageup", "left"),
        ("ctrl+shift+pagedown", "right"),
    ] {
        bindings
            .push(Binding::new([keys], "client.tab.move").args(json!({"direction": direction})));
    }
    for (keys, lines) in [("shift+pageup", -20), ("shift+pagedown", 20)] {
        bindings.push(Binding::new([keys], "pane.scroll").args(json!({"lines": lines})));
    }
    KeymapDefault::new("linux").bindings(bindings)
}

/// One file entry. A null action clears the exact chord/sequence, not its prefix.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub keys: String,
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
}

/// Parse/fold once, on file changes. Send this exact layer to every host and use
/// it unchanged in offline resolution: the wire separates bindings/removals.
#[derive(Clone, Debug, PartialEq)]
pub struct Overrides {
    pub layer: KeymapDefault,
    pub diagnostics: Vec<Diagnostic>,
}

impl Default for Overrides {
    fn default() -> Self {
        Self {
            layer: KeymapDefault::new("settings"),
            diagnostics: Vec::new(),
        }
    }
}

fn problem(
    source: &str,
    message: impl Into<String>,
    action: Option<&str>,
    key: Option<String>,
) -> Diagnostic {
    Diagnostic {
        severity: "warning".into(),
        source: Some(source.into()),
        message: message.into(),
        action: action.map(str::to_owned),
        key,
        mode: None,
    }
}

impl Overrides {
    pub fn parse(value: &Value) -> Self {
        let mut result = Self::default();
        let Some(entries) = value.as_array() else {
            result.diagnostics.push(problem(
                "settings",
                "shortcuts: expected an array",
                None,
                None,
            ));
            return result;
        };
        for (index, value) in entries.iter().enumerate() {
            let parsed = (|| -> Result<(Override, Vec<String>)> {
                ensure!(
                    value.get("action").is_some(),
                    "action is required (use null to unbind)"
                );
                if let Some(args) = value.get("args") {
                    ensure!(args.is_object(), "args must be an object");
                }
                let entry: Override = serde_json::from_value(value.clone())?;
                let keys = normalize(&entry.keys)?;
                if let Some(action) = &entry.action {
                    ensure!(!action.trim().is_empty(), "action must not be empty");
                } else {
                    ensure!(entry.args.is_none(), "an unbinding cannot have args");
                }
                Ok((entry, keys))
            })();
            match parsed {
                Ok((entry, keys)) => {
                    if let Some(action) = entry.action {
                        let mut binding = Binding::new(keys, action);
                        binding.args = entry.args;
                        result.layer.bindings.push(binding);
                    } else {
                        // Rex removals only touch lower layers. Fold earlier own-layer
                        // bindings away first, or binding-then-null would still run.
                        result.layer.bindings.retain(|b| b.keys != keys);
                        if !result.layer.removals.iter().any(|r| r.keys == keys) {
                            result.layer.removals.push(Removal::new(keys));
                        }
                    }
                }
                Err(error) => result.diagnostics.push(problem(
                    "settings",
                    format!("shortcuts[{index}]: {error}"),
                    value.get("action").and_then(Value::as_str),
                    value
                        .get("keys")
                        .and_then(Value::as_str)
                        .and_then(|k| normalize(k).ok())
                        .map(|k| k.join(" ")),
                )),
            }
        }
        result
    }
}

/// Parsed once, with original server keys retained for `client.key.prefix`.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedBinding {
    pub binding: Binding,
    pub chords: Option<Vec<Chord>>,
}
impl From<Binding> for ResolvedBinding {
    fn from(binding: Binding) -> Self {
        let chords = parse_keys(&binding.keys).ok();
        Self { binding, chords }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolved {
    pub bindings: Vec<ResolvedBinding>,
    pub diagnostics: Vec<Diagnostic>,
}
impl Resolved {
    /// Do not normalize server keys here: prefixes must echo them verbatim.
    pub fn from_server(reply: &rexrs::api::client::KeymapResponse, overrides: &Overrides) -> Self {
        Self {
            bindings: reply
                .bindings
                .iter()
                .cloned()
                .map(ResolvedBinding::from)
                .collect(),
            diagnostics: overrides
                .diagnostics
                .iter()
                .chain(&reply.diagnostics)
                .cloned()
                .collect(),
        }
    }
}

/// The name of the layer of shortcuts recorded in Settings, which sits above the user's file.
pub const RECORDED: &str = "recorded";

/// Local fallback until the active host answers (or while it is offline).
/// Action-specific argument checks belong to the UI's action registry.
pub fn resolve(
    defaults: &[KeymapDefault],
    overrides: &Overrides,
    actions: &[InspectResponseAction],
) -> Resolved {
    resolve_all(defaults, &[overrides], actions)
}

/// [`resolve`] with every override layer, lowest first: the user's file, then what was
/// recorded in Settings.
pub fn resolve_all(
    defaults: &[KeymapDefault],
    overrides: &[&Overrides],
    actions: &[InspectResponseAction],
) -> Resolved {
    let mut layers = defaults.to_vec();
    layers.extend(overrides.iter().map(|layer| layer.layer.clone()));
    let mut resolved = resolve_layers(&layers, defaults.len(), actions);
    let diagnostics: Vec<Diagnostic> = overrides
        .iter()
        .flat_map(|layer| layer.diagnostics.iter().cloned())
        .collect();
    resolved.diagnostics.splice(0..0, diagnostics);
    resolved
}

// If any wire chord is unreadable, Rex keeps the whole sequence as written.
fn canonical_keys(keys: &[String]) -> Vec<String> {
    parse_keys(keys)
        .map(|chords| chords.iter().map(Chord::spelling).collect())
        .unwrap_or_else(|_| keys.to_vec())
}

fn resolve_layers(
    layers: &[KeymapDefault],
    default_count: usize,
    actions: &[InspectResponseAction],
) -> Resolved {
    let mut stack: Vec<(Binding, bool)> = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        let is_default = index < default_count;
        // Removals affect only lower layers, not the bindings about to be added.
        for removal in &layer.removals {
            if removal.mode.is_some() {
                continue;
            } // Modes are a later phase.
            let keys = canonical_keys(&removal.keys);
            let before = stack.len();
            stack.retain(|(b, _)| {
                b.mode.is_some()
                    || b.keys != keys
                    || removal
                        .action
                        .as_ref()
                        .is_some_and(|action| action != &b.action)
            });
            if before == stack.len() && !is_default && parse_keys(&keys).is_ok() {
                diagnostics.push(problem(
                    &layer.name,
                    format!("nothing is bound to {:?}", keys.join(" ")),
                    removal.action.as_deref(),
                    Some(keys.join(" ")),
                ));
            }
        }
        for binding in &layer.bindings {
            let mut binding = binding.clone();
            binding.keys = canonical_keys(&binding.keys);
            binding.source = Some(layer.name.clone());
            if !is_default
                && !actions.is_empty()
                && binding.action != "client.key.forward"
                && ["client.", "session.", "pane."]
                    .iter()
                    .any(|prefix| binding.action.starts_with(prefix))
                && !actions.iter().any(|a| a.name == binding.action)
            {
                diagnostics.push(problem(
                    &layer.name,
                    format!("unknown action {:?}", binding.action),
                    Some(&binding.action),
                    Some(binding.keys.join(" ")),
                ));
            }
            stack.push((binding, is_default));
        }
    }
    stack.reverse();
    let mut checked = std::collections::HashSet::new();
    for (winner_index, (winner, winner_default)) in stack.iter().enumerate() {
        let Some(first) = winner.keys.first() else {
            continue;
        };
        if winner.mode.is_some() || !checked.insert(first) {
            continue;
        }
        if let Some((loser, loser_default)) = stack[winner_index + 1..].iter().find(|(b, _)| {
            b.mode.is_none()
                && b.keys.first() == Some(first)
                && (b.keys.len() == 1) != (winner.keys.len() == 1)
        }) {
            if *winner_default && *loser_default {
                continue;
            }
            let message = if winner.keys.len() > 1 {
                format!(
                    "{:?} ({}) cannot fire, because key sequence {:?} is bound above it ({})",
                    loser.keys.join(" "),
                    loser.source.as_deref().unwrap_or_default(),
                    winner.keys.join(" "),
                    winner.source.as_deref().unwrap_or_default()
                )
            } else {
                format!(
                    "key sequence {:?} ({}) cannot start, because {:?} is bound to {:?} above it ({})",
                    loser.keys.join(" "),
                    loser.source.as_deref().unwrap_or_default(),
                    winner.keys.join(" "),
                    winner.action,
                    winner.source.as_deref().unwrap_or_default()
                )
            };
            diagnostics.push(problem(
                loser.source.as_deref().unwrap_or_default(),
                message,
                Some(&loser.action),
                Some(loser.keys.join(" ")),
            ));
        }
    }
    Resolved {
        bindings: stack.into_iter().map(|(b, _)| b.into()).collect(),
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_order_follow_the_recorded_server_spelling() {
        for logo in ["cmd", "command", "super", "meta", "win"] {
            assert_eq!(
                normalize(&format!("{logo}+SHIFT+Option+Control+K")).unwrap(),
                ["ctrl+alt+shift+cmd+k"]
            );
        }
        assert_eq!(
            normalize(" opt + control + K > Shift+5 ").unwrap(),
            ["ctrl+alt+k", "shift+5"]
        );
        assert_eq!(
            normalize("Win+SHIFT+D").unwrap(),
            normalize("shift+cmd+d").unwrap()
        );
        let chord = Chord::parse("cmd+k").unwrap();
        assert!(chord.modifiers.logo && !chord.modifiers.ctrl);
    }

    #[test]
    fn punctuation_named_keys_and_physical_codes_round_trip() {
        for input in [
            "ctrl++",
            "+",
            "ctrl+shift++",
            "ctrl+=",
            "ctrl+,",
            "shift+5",
            "ctrl+é",
            "[",
        ] {
            assert_eq!(normalize(input).unwrap(), [input]);
        }
        assert_eq!(normalize("CTRL+[keyt]").unwrap(), ["ctrl+[KeyT]"]);
        for key in NAMED {
            assert_eq!(Chord::parse(key).unwrap().spelling(), *key);
        }
        for code in PHYSICAL {
            let input = format!("ctrl+[{code}]");
            assert_eq!(Chord::parse(&input).unwrap().spelling(), input);
        }
    }

    #[test]
    fn invalid_chords_are_rejected_not_silently_normalized() {
        for input in [
            "",
            "ctrl+b>",
            "ctrl+",
            "ctrl++t",
            "+k",
            "++",
            "ctrl+++",
            "ctrl+k+t",
            "ctrl+control+k",
            "hyper+k",
            "f0",
            "f13",
            "f24",
            "ctrl+[Foo]",
            "ctrl+[KeyT",
            "ctrl+\n",
            "ctrl+ ",
        ] {
            assert!(normalize(input).is_err(), "{input:?}");
        }
        assert!(parse_keys(&[]).is_err());
        assert!(parse_keys(&["ctrl+b>c".into()]).is_err());
    }

    #[test]
    fn display_preserves_unknown_spelling_without_matching_it() {
        for (keys, expected) in [
            (vec!["ctrl+shift+t".into()], "Ctrl Shift T"),
            (vec!["ctrl+b".into(), "c".into()], "Ctrl B, C"),
            (vec!["ctrl+[KeyT]".into()], "Ctrl KeyT"),
            (vec!["shift+pageup".into()], "Shift PageUp"),
            (vec!["ctrl++".into()], "Ctrl Plus"),
            (vec!["meta+k".into()], "Super K"),
        ] {
            assert_eq!(display(&keys), expected);
        }
        let keys = vec!["ctrl+b".into(), "ctrl+FutureKey".into()];
        assert!(parse_keys(&keys).is_err());
        assert_eq!(display(&keys), "Ctrl B, ctrl+FutureKey");
    }

    fn local(value: Value) -> Resolved {
        resolve(&[defaults()], &Overrides::parse(&value), &[])
    }

    fn on<'a>(map: &'a Resolved, keys: &[&str]) -> Vec<&'a Binding> {
        map.bindings
            .iter()
            .map(|b| &b.binding)
            .filter(|b| b.keys == keys)
            .collect()
    }

    #[test]
    fn defaults_are_canonical_and_include_search_global_navigation_and_zoom_variants() {
        let layer = defaults();
        assert_eq!(layer.name, "linux");
        assert!(layer.removals.is_empty());
        assert_eq!(layer.bindings.len(), 37);
        // The terminal font size actions have no default: Ctrl Alt + / - / 0 belong to the
        // program in the terminal until the user binds them.
        assert!(
            !layer
                .bindings
                .iter()
                .any(|b| b.action.starts_with("client.font."))
        );
        for binding in &layer.bindings {
            assert_eq!(normalize(&binding.keys.join(">")).unwrap(), binding.keys);
            assert!(binding.source.is_none() && binding.mode.is_none());
        }
        for (key, action) in [
            ("ctrl+tab", "client.tab.next_global"),
            ("ctrl+shift+tab", "client.tab.previous_global"),
            ("ctrl+shift+f", "client.search"),
            ("ctrl+shift+up", "client.prompt.previous"),
            ("ctrl+shift+down", "client.prompt.next"),
            ("ctrl++", "client.zoom.in"),
            ("ctrl+[Numpad0]", "client.zoom.reset"),
            ("ctrl+0", "client.zoom.reset"),
            ("ctrl+shift+pageup", "client.tab.move"),
            ("ctrl+shift+pagedown", "client.tab.move"),
        ] {
            assert!(
                layer
                    .bindings
                    .iter()
                    .any(|b| b.keys == [key] && b.action == action)
            );
        }
    }

    #[test]
    fn aliases_replace_defaults_and_null_unbinds_exact_sequences() {
        // Adapted from nivox's test: keep Rex's full try stack rather than erase defaults.
        let map = local(json!([
            {"keys":"Control+K", "action":"client.sidebar.toggle"},
            {"keys":"ctrl+b>c", "action":"client.tab.new"},
            {"keys":"ctrl+b>z", "action":"pane.zoom"},
            {"keys":"ctrl+b>c", "action":null},
            {"keys":"ctrl+b", "action":null}
        ]));
        assert_eq!(on(&map, &["ctrl+k"])[0].action, "client.sidebar.toggle");
        assert_eq!(on(&map, &["ctrl+k"])[1].action, "session.switch");
        assert!(on(&map, &["ctrl+b", "c"]).is_empty());
        assert_eq!(on(&map, &["ctrl+b", "z"]).len(), 1);
    }

    #[test]
    fn binding_then_null_drops_earlier_own_bindings_and_removes_lower_layers() {
        let overrides = Overrides::parse(&json!([
            {"keys":"ctrl+tab", "action":"client.settings"},
            {"keys":"Control+TAB", "action":null}
        ]));
        assert!(overrides.layer.bindings.is_empty());
        assert_eq!(overrides.layer.removals, [Removal::new(["ctrl+tab"])]);
        let map = resolve(&[defaults()], &overrides, &[]);
        assert!(on(&map, &["ctrl+tab"]).is_empty());
        assert!(map.diagnostics.is_empty());
    }

    #[test]
    fn null_then_binding_keeps_own_binding_but_removes_lower_layers() {
        let overrides = Overrides::parse(&json!([
            {"keys":"ctrl+tab", "action":null},
            {"keys":"ctrl+tab", "action":"client.settings"}
        ]));
        assert_eq!(overrides.layer.bindings.len(), 1);
        assert_eq!(overrides.layer.removals.len(), 1);
        let map = resolve(&[defaults()], &overrides, &[]);
        let bindings = on(&map, &["ctrl+tab"]);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].action, "client.settings");
    }

    #[test]
    fn later_bindings_win_without_discarding_try_order() {
        let map = local(json!([
            {"keys":"ctrl+k", "action":"client.settings"},
            {"keys":"ctrl+k", "action":"client.tab.new"}
        ]));
        let actions: Vec<_> = on(&map, &["ctrl+k"])
            .iter()
            .map(|b| b.action.as_str())
            .collect();
        assert_eq!(
            actions,
            ["client.tab.new", "client.settings", "session.switch"]
        );
    }

    #[test]
    fn invalid_entries_warn_with_index_and_leave_valid_entries_usable() {
        let overrides = Overrides::parse(&json!([
            {"keys":"ctrl+b>", "action":"pane.zoom"},
            {"keys":"ctrl+k", "action":null, "args":{}},
            {"keys":"ctrl+k"},
            {"keys":"ctrl+k", "action":"pane.zoom", "args":null},
            {"keys":"ctrl+k", "action":"pane.zoom", "args":[]},
            {"keys":"ctrl+k", "action":""},
            {"keys":"ctrl+[Foo]", "action":"pane.zoom"},
            {"keys":"ctrl+k", "action":"pane.zoom", "unexpected":true},
            {"keys":"ctrl++", "action":"client.zoom.in"}
        ]));
        assert_eq!(overrides.layer.bindings.len(), 1);
        assert_eq!(overrides.diagnostics.len(), 8);
        for (i, diagnostic) in overrides.diagnostics.iter().enumerate() {
            assert!(diagnostic.message.starts_with(&format!("shortcuts[{i}]:")));
            assert_eq!(diagnostic.source.as_deref(), Some("settings"));
        }
        assert_eq!(overrides.diagnostics[0].key, None);
        assert_eq!(overrides.diagnostics[1].key.as_deref(), Some("ctrl+k"));
        let alias =
            Overrides::parse(&json!([{"keys":"CTRL+B>C", "action":"pane.zoom", "args":null}]));
        assert_eq!(alias.diagnostics[0].key.as_deref(), Some("ctrl+b c"));
        for invalid in [Value::Null, json!({}), json!("ctrl+k")] {
            let parsed = Overrides::parse(&invalid);
            assert!(parsed.layer.bindings.is_empty());
            assert_eq!(parsed.diagnostics.len(), 1);
        }
        assert!(Overrides::parse(&json!([])).diagnostics.is_empty());
    }

    #[test]
    fn sdk_layer_round_trip_omits_absent_args_and_preserves_present_args() {
        let overrides = Overrides::parse(&json!([
            {"keys":"ctrl+k", "action":null},
            {"keys":"ctrl+k", "action":"client.settings"},
            {"keys":"ctrl+b>d", "action":"pane.split", "args":{"direction":"down"}}
        ]));
        let wire = serde_json::to_value(&overrides.layer).unwrap();
        assert!(wire["bindings"][0].get("args").is_none());
        assert_eq!(wire["bindings"][1]["args"], json!({"direction":"down"}));
        assert_eq!(
            serde_json::from_value::<KeymapDefault>(wire).unwrap(),
            overrides.layer
        );
    }

    #[test]
    fn conflicts_report_the_loser_but_not_defaults_hiding_defaults() {
        let map = local(json!([{"keys":"ctrl+k>x", "action":"client.tab.new"}]));
        assert!(
            map.diagnostics
                .iter()
                .any(|d| d.source.as_deref() == Some("linux")
                    && d.action.as_deref() == Some("session.switch")
                    && d.key.as_deref() == Some("ctrl+k"))
        );
        let layer = KeymapDefault::new("linux").bindings([
            Binding::new(["ctrl+b"], "client.settings"),
            Binding::new(["ctrl+b", "c"], "client.tab.new"),
        ]);
        assert!(
            resolve(&[layer], &Overrides::default(), &[])
                .diagnostics
                .is_empty()
        );
        let reversed = local(json!([
            {"keys":"ctrl+b>c", "action":"client.tab.new"},
            {"keys":"ctrl+b", "action":"client.settings"}
        ]));
        assert_eq!(reversed.diagnostics.len(), 1);
        assert_eq!(reversed.diagnostics[0].key.as_deref(), Some("ctrl+b c"));
        assert_eq!(reversed.diagnostics[0].source.as_deref(), Some("settings"));
    }

    #[test]
    fn server_reply_preserves_local_parse_problems_and_offline_actions_defer_to_server() {
        let overrides = Overrides::parse(&json!([
            {"keys":"ctrl+b>","action":"client.settings"},
            {"keys":"ctrl+l","action":"clear_screen"},
            {"keys":"ctrl+j","action":"com.example.clear"}
        ]));
        let actions = [InspectResponseAction::new("client.settings")];
        let local = resolve(&[defaults()], &overrides, &actions);
        assert_eq!(local.diagnostics, overrides.diagnostics);
        let reply = rexrs::api::client::KeymapResponse::default();
        assert_eq!(
            Resolved::from_server(&reply, &overrides).diagnostics,
            overrides.diagnostics
        );
    }

    #[test]
    fn stale_actions_are_retained_and_reported_not_fatal() {
        let actions = serde_json::from_value::<Vec<InspectResponseAction>>(json!([
            {"name":"client.settings", "kind":"client"}
        ]))
        .unwrap();
        let overrides = Overrides::parse(&json!([
            {"keys":"ctrl+k", "action":"client.missing"}
        ]));
        let map = resolve(&[defaults()], &overrides, &actions);
        assert_eq!(on(&map, &["ctrl+k"])[0].action, "client.missing");
        assert_eq!(map.diagnostics.len(), 1);
        assert_eq!(map.diagnostics[0].action.as_deref(), Some("client.missing"));
    }

    #[test]
    fn server_binding_keeps_prefix_strings_and_parses_mode_keys() {
        let readable = Binding::new(["SUPER+K", "C"], "client.key.forward");
        let parsed = ResolvedBinding::from(readable.clone());
        assert_eq!(parsed.binding.keys, readable.keys);
        assert_eq!(parsed.chords.unwrap()[0].spelling(), "cmd+k");
        let unknown = ResolvedBinding::from(Binding::new(["ctrl+FutureKey"], "client.settings"));
        assert!(unknown.chords.is_none());
        assert_eq!(display(&unknown.binding.keys), "ctrl+FutureKey");
        let mode = ResolvedBinding::from(Binding::new(["left"], "pane.focus").mode("resize"));
        assert!(mode.chords.is_some());
    }

    #[test]
    fn local_layering_matches_recorded_server_try_order_and_diagnostic_targets() {
        let request: rexrs::api::client::Keymap =
            serde_json::from_str(include_str!("../tests/fixtures/keymap/request.json")).unwrap();
        let reply: rexrs::api::client::KeymapResponse =
            serde_json::from_str(include_str!("../tests/fixtures/keymap/response.json")).unwrap();
        let mut layers = request.defaults.clone();
        // Same file layer as init.lua, broken into lines only to retain its sources.
        for (line, binding) in [
            (2, Binding::new(["ctrl+k"], "client.settings")),
            (3, Binding::new(["ctrl+b", "c"], "client.tab.new")),
            (4, Binding::new(["ctrl+b", "x"], "client.key.forward")),
            (5, Binding::new(["ctrl+l"], "client.key.forward")),
            (6, Binding::new(["ctrl+[KeyT]"], "client.tab.new")),
            (7, Binding::new(["ctrl+m"], "client.mac.only")),
            (9, Binding::new(["ctrl+b", "z"], "client.mac.only")),
            (10, Binding::new(["super+u"], "client.settings")),
            (11, Binding::new(["meta+i"], "client.settings")),
            (12, Binding::new(["opt+o"], "client.settings")),
            (13, Binding::new(["command+h"], "client.settings")),
        ] {
            layers.push(KeymapDefault::new(format!("init.lua:{line}")).bindings([binding]));
        }
        layers.push(KeymapDefault::new("init.lua:8").removals([Removal::new(["ctrl+r"])]));
        layers.extend(request.overrides);
        let map = resolve_layers(&layers, request.defaults.len(), &request.actions);
        assert_eq!(
            map.bindings
                .into_iter()
                .map(|b| b.binding)
                .collect::<Vec<_>>(),
            reply.bindings
        );
        assert_eq!(map.diagnostics, reply.diagnostics);
    }

    #[test]
    fn removal_clears_default_and_middle_layer_but_not_a_sequence() {
        let layers = [
            KeymapDefault::new("linux").bindings([Binding::new(["ctrl+b"], "client.settings")]),
            KeymapDefault::new("file").bindings([
                Binding::new(["ctrl+b"], "client.tab.new"),
                Binding::new(["ctrl+b", "c"], "client.tab.new"),
            ]),
            KeymapDefault::new("settings").removals([Removal::new(["ctrl+b"])]),
        ];
        let map = resolve_layers(&layers, 1, &[]);
        assert!(on(&map, &["ctrl+b"]).is_empty());
        assert_eq!(on(&map, &["ctrl+b", "c"]).len(), 1);
        assert!(map.diagnostics.is_empty());
    }

    #[test]
    fn recorded_readable_keys_use_server_spelling() {
        let reply: rexrs::api::client::KeymapResponse =
            serde_json::from_str(include_str!("../tests/fixtures/keymap/response.json")).unwrap();
        for binding in reply.bindings {
            if binding.keys == ["ctrl+FutureKey"] {
                assert!(parse_keys(&binding.keys).is_err());
            } else {
                let chords = parse_keys(&binding.keys).unwrap();
                assert_eq!(
                    chords.iter().map(Chord::spelling).collect::<Vec<_>>(),
                    binding.keys
                );
            }
        }
    }
}
