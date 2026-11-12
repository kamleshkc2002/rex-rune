//! Layout-aware matching and sequence/repeat state, adapted from Andrea Zito
//! (nivox), 8037d78. Input is plain data: no Tx or workspace needed by the matcher.
use rex_linux::keymap::{Chord, Key as BindingKey, Modifiers, Resolved};
use rune::{Key, KeyEvent, PhysicalKeyCode as Code};

#[derive(Clone, Copy)]
pub struct Input<'a> {
    pub event: &'a KeyEvent,
    pub code: Option<Code>,
    pub base: Option<char>,
    pub repeat: bool,
}

/// Identity survives modifier release; logical-only drivers have no physical code.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Identity {
    Physical(Code),
    Logical(String),
}
impl Input<'_> {
    pub fn identity(&self) -> Identity {
        self.code.map(Identity::Physical).unwrap_or_else(|| {
            Identity::Logical(match &self.event.key {
                Key::Text(t) => t.to_lowercase(),
                Key::Command(c) => c.to_lowercase().to_string(),
                other => format!("{other:?}"),
            })
        })
    }
}

fn us_character(code: Code) -> Option<char> {
    let name = format!("{code:?}");
    if let Some(letter) = name.strip_prefix("Key").filter(|s| s.len() == 1) {
        return letter.chars().next().map(|c| c.to_ascii_lowercase());
    }
    if let Some(digit) = name.strip_prefix("Digit").filter(|s| s.len() == 1) {
        return digit.chars().next();
    }
    Some(match code {
        Code::Comma => ',',
        Code::Period => '.',
        Code::Slash => '/',
        Code::Backslash => '\\',
        Code::Semicolon => ';',
        Code::Quote => '\'',
        Code::BracketLeft => '[',
        Code::BracketRight => ']',
        Code::Backquote => '`',
        Code::Minus => '-',
        Code::Equal => '=',
        Code::Space => ' ',
        _ => return None,
    })
}

fn character(key: &Key) -> Option<char> {
    match key {
        Key::Command(c) => Some(*c),
        Key::Text(t) if t.chars().count() == 1 => t.chars().next(),
        _ => None,
    }
}

pub fn matches(chord: &Chord, input: Input<'_>) -> bool {
    let m = input.event.mods;
    let mods = Modifiers {
        ctrl: m.ctrl || m.cmd,
        alt: m.alt,
        shift: m.shift,
        logo: m.logo,
    };
    if chord.modifiers != mods {
        return false;
    }
    match &chord.key {
        BindingKey::Physical(name) => input.code.is_some_and(|code| {
            let code = match code {
                Code::SuperLeft => "MetaLeft".into(),
                Code::SuperRight => "MetaRight".into(),
                code => format!("{code:?}"),
            };
            code == *name
        }),
        BindingKey::Character(wanted) => {
            let typed = character(&input.event.key);
            let logical = input.base.or(typed);
            if logical.is_some_and(|c| c.eq_ignore_ascii_case(wanted)) {
                return true;
            }
            match logical {
                Some(c) if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ' => {
                    c.eq_ignore_ascii_case(wanted)
                        // Without a base, a shifted symbol still has its physical unshifted
                        // spelling. Do not use this for letters: Dvorak stays logical.
                        || (input.base.is_none() && m.shift && c.is_ascii_punctuation()
                            && input.code.and_then(us_character) == Some(*wanted))
                }
                _ => input.code.and_then(us_character) == Some(*wanted),
            }
        }
        BindingKey::Named(wanted) => {
            let name = match &input.event.key {
                Key::Enter => "enter",
                Key::Escape => "escape",
                Key::Tab => "tab",
                Key::Backspace => "backspace",
                Key::Delete => "delete",
                Key::Left => "left",
                Key::Right => "right",
                Key::Up => "up",
                Key::Down => "down",
                Key::Home => "home",
                Key::End => "end",
                Key::PageUp => "pageup",
                Key::PageDown => "pagedown",
                Key::Function(n) => return *wanted == format!("f{n}"),
                key if character(key) == Some(' ') => "space",
                _ => return false,
            };
            name == *wanted
        }
    }
}

/// The chord a key press spells, for recording a shortcut: one [`matches`] finds this press
/// by. A named key by its name, a character as the layout's own when that is ASCII, else as
/// the key's US character, so a shortcut recorded on any layout is spelled as the defaults
/// are; a key with neither by its physical code. None for a modifier on its own, a dead key
/// and a key that has no spelling.
pub fn chord_of(input: Input<'_>) -> Option<Chord> {
    if matches!(input.event.key, Key::Modifier { .. } | Key::Dead(_)) {
        return None;
    }
    let m = input.event.mods;
    let modifiers = Modifiers {
        ctrl: m.ctrl || m.cmd,
        alt: m.alt,
        shift: m.shift,
        logo: m.logo,
    };
    let typed = character(&input.event.key);
    let mut spellings: Vec<String> = Vec::new();
    match &input.event.key {
        Key::Enter => spellings.push("enter".into()),
        Key::Escape => spellings.push("escape".into()),
        Key::Tab => spellings.push("tab".into()),
        Key::Backspace => spellings.push("backspace".into()),
        Key::Delete => spellings.push("delete".into()),
        Key::Left => spellings.push("left".into()),
        Key::Right => spellings.push("right".into()),
        Key::Up => spellings.push("up".into()),
        Key::Down => spellings.push("down".into()),
        Key::Home => spellings.push("home".into()),
        Key::End => spellings.push("end".into()),
        Key::PageUp => spellings.push("pageup".into()),
        Key::PageDown => spellings.push("pagedown".into()),
        Key::Function(n) => spellings.push(format!("f{n}")),
        _ if typed == Some(' ') => spellings.push("space".into()),
        _ => {}
    }
    let characters = [input.base, typed, input.code.and_then(us_character)];
    for ascii in [true, false] {
        for c in characters.into_iter().flatten() {
            if c.is_ascii() == ascii {
                spellings.extend(c.to_lowercase().next().map(String::from));
            }
        }
    }
    if let Some(code) = input.code {
        spellings.push(match code {
            Code::SuperLeft => "[MetaLeft]".into(),
            Code::SuperRight => "[MetaRight]".into(),
            code => format!("[{code:?}]"),
        });
    }
    spellings
        .iter()
        .filter_map(|key| {
            let mut chord = Chord::parse(key).ok()?;
            chord.modifiers = modifiers;
            Some(chord)
        })
        .find(|chord| matches(chord, input))
}

#[derive(Default)]
pub struct Sequence {
    candidates: Vec<usize>,
    pub depth: usize,
}
#[derive(Debug, PartialEq)]
pub enum Step {
    None,
    Pending,
    Run { index: usize, sequence: bool },
}
impl Sequence {
    pub fn label(&self, map: &Resolved) -> Option<String> {
        if self.depth == 0 {
            return None;
        }
        let chords = map
            .bindings
            .get(*self.candidates.first()?)?
            .chords
            .as_ref()?;
        Some(
            chords
                .iter()
                .take(self.depth)
                .map(|c| c.keycaps().join(" "))
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.depth = 0;
    }

    /// Map is already in try order. Availability must include action kind, args and
    /// panel/global restrictions. Failed tails are never retried as fresh shortcuts.
    pub fn advance(
        &mut self,
        map: &Resolved,
        input: Input<'_>,
        available: impl Fn(usize) -> bool,
    ) -> Step {
        let depth = self.depth;
        if depth > 0 && matches!(input.event.key, Key::Modifier { .. }) {
            return Step::Pending;
        }
        if depth > 0 && input.event.key == Key::Escape {
            self.clear();
            return Step::Pending;
        }
        let indices: Vec<_> = if depth == 0 {
            (0..map.bindings.len()).collect()
        } else {
            self.candidates.clone()
        };
        let matching: Vec<_> = indices
            .into_iter()
            .filter(|&i| {
                available(i)
                    && map.bindings[i]
                        .chords
                        .as_ref()
                        .and_then(|c| c.get(depth))
                        .is_some_and(|c| matches(c, input))
            })
            .collect();
        let Some(&first) = matching.first() else {
            self.clear();
            return Step::None;
        };
        if map.bindings[first].binding.keys.len() == depth + 1 {
            self.clear();
            return Step::Run {
                index: first,
                sequence: depth > 0,
            };
        }
        self.depth += 1;
        self.candidates = matching
            .into_iter()
            .filter(|&i| map.bindings[i].binding.keys.len() > self.depth)
            .collect();
        Step::Pending
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub host: super::workspace::HostId,
    pub session: Option<String>,
    pub pane: Option<String>,
    pub panel: bool,
    pub focus: Option<String>,
    pub generation: u64,
    pub server_revision: u64,
    pub connected: bool,
}

#[derive(Default)]
pub struct State {
    pub modes: Vec<(String, bool)>,
    /// Map used for the current sequence, including mode blocks and priority.
    pub mode_map: Option<Resolved>,
    pub sequence: Sequence,
    pub context: Option<Context>,
    /// Consumed presses, even repeating actions, keep releases local.
    pub consumed: std::collections::HashMap<Identity, bool>,
}

impl State {
    pub fn pending_label(&self, map: &Resolved) -> Option<String> {
        self.sequence.label(self.mode_map.as_ref().unwrap_or(map))
    }

    pub fn enter_mode(&mut self, name: &str, once: bool) {
        self.sequence.clear();
        self.modes.push((name.into(), once));
    }
    pub fn exit_mode(&mut self) {
        self.sequence.clear();
        self.modes.pop();
    }
    pub fn clear_modes(&mut self) {
        self.sequence.clear();
        self.modes.clear();
        self.mode_map = None;
    }
}

/// Highest active mode first; its explicit bindings beat blocks. Unbound keys fall through
/// to the next mode and then the ordinary map, unless a mode is exclusive.
pub fn mode_map(
    map: &Resolved,
    active: &[(String, bool)],
    modes: &[rexrs::api::client::Mode],
) -> Resolved {
    let mut result = Resolved::default();
    for (name, _) in active.iter().rev() {
        result.bindings.extend(
            map.bindings
                .iter()
                .filter(|b| b.binding.mode.as_deref() == Some(name))
                .cloned(),
        );
        if let Some(mode) = modes.iter().find(|m| m.name == *name) {
            for keys in mode.blocked.iter().flatten() {
                result.bindings.push(
                    rexrs::api::client::Binding::new(keys.clone(), "linux.mode.block")
                        .mode(name.clone())
                        .into(),
                );
            }
        }
        // Escape leaves the top mode unless it explicitly binds Escape.
        result.bindings.push(
            rexrs::api::client::Binding::new(["escape"], "client.mode.exit")
                .mode(name.clone())
                .into(),
        );
        if modes.iter().any(|m| m.name == *name && m.exclusive) {
            return result;
        }
    }
    result.bindings.extend(
        map.bindings
            .iter()
            .filter(|b| b.binding.mode.is_none())
            .cloned(),
    );
    result
}

#[cfg(test)]
mod tests;
