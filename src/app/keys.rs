//! Rune key events → Rex `client.key` payloads (Ghostty key names, by physical position).
use rune::{Key, KeyEvent, Mods, PhysicalKeyCode};
use serde_json::{Value, json};

/// Ghostty's key name for a W3C code: `KeyA` → `key_a`, `Digit1` → `digit_1`,
/// `ArrowUp` → `arrow_up`, `Numpad0` → `numpad_0`, `F12` → `f12`.
pub fn code_name(code: PhysicalKeyCode) -> String {
    wire_code_name(&format!("{code:?}"))
}

/// Also used for validated W3C codes in synthesized pane.send_key events.
pub fn wire_code_name(debug: &str) -> String {
    if let Some(n) = debug.strip_prefix('F')
        && !n.is_empty()
        && n.bytes().all(|b| b.is_ascii_digit())
    {
        return format!("f{n}");
    }
    let mut out = String::with_capacity(debug.len() + 4);
    let mut previous: Option<char> = None;
    for ch in debug.chars() {
        let boundary = match previous {
            Some(p) => {
                (ch.is_ascii_uppercase() && (p.is_ascii_lowercase() || p.is_ascii_digit()))
                    || (ch.is_ascii_digit() && p.is_ascii_alphabetic())
            }
            None => false,
        };
        if boundary {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
        previous = Some(ch);
    }
    out
}

/// A logical key without a physical origin (tests, scripts) still needs a Rex name.
fn logical_name(key: &Key) -> Option<String> {
    Some(
        match key {
            Key::Enter => "enter",
            Key::Backspace => "backspace",
            Key::Delete => "delete",
            Key::Left => "arrow_left",
            Key::Right => "arrow_right",
            Key::Up => "arrow_up",
            Key::Down => "arrow_down",
            Key::Home => "home",
            Key::End => "end",
            Key::PageUp => "page_up",
            Key::PageDown => "page_down",
            Key::Escape => "escape",
            Key::Tab => "tab",
            Key::Function(n) => return Some(format!("f{n}")),
            Key::Text(t) if t == " " => "space",
            Key::Command(c) => return char_name(*c),
            Key::Text(t) => {
                return t
                    .chars()
                    .next()
                    .filter(|_| t.chars().count() == 1)
                    .and_then(char_name);
            }
            _ => return None,
        }
        .to_owned(),
    )
}

fn char_name(c: char) -> Option<String> {
    let c = c.to_ascii_lowercase();
    Some(match c {
        'a'..='z' => format!("key_{c}"),
        '0'..='9' => format!("digit_{c}"),
        '-' => "minus".into(),
        '=' => "equal".into(),
        '[' => "bracket_left".into(),
        ']' => "bracket_right".into(),
        '\\' => "backslash".into(),
        ';' => "semicolon".into(),
        '\'' => "quote".into(),
        ',' => "comma".into(),
        '.' => "period".into(),
        '/' => "slash".into(),
        '`' => "backquote".into(),
        ' ' => "space".into(),
        _ => return None,
    })
}

pub fn mods(m: Mods) -> String {
    let ctrl = m.ctrl || (m.cmd && !cfg!(target_os = "macos"));
    [
        (m.shift, "shift"),
        (ctrl, "ctrl"),
        (m.alt, "alt"),
        (m.logo, "super"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect::<Vec<_>>()
    .join("+")
}

/// Text a key types into the terminal: what the server expects (Enter `\n`, Tab `\t`, no text
/// with Ctrl/Alt/Super).
fn text(ev: &KeyEvent) -> String {
    let m = ev.mods;
    let plain = !(m.ctrl || m.cmd || m.logo);
    match &ev.key {
        Key::Text(t) if plain || m.alt => t.clone(),
        Key::Enter if plain && !m.alt => "\n".into(),
        Key::Tab if plain && !m.alt => "\t".into(),
        _ => String::new(),
    }
}

/// Whether the layout used Shift to type `ev`'s text, so the encoder should not report it.
/// A heuristic from the text and the key's unmodified character (`base`), not the layout itself:
/// - On a cased key, Shift counts even when Caps Lock cancels it, as xkb reports.
/// - Otherwise Shift counts only when the text differs from what the key types alone: `!` from
///   `1` counts, Shift+Space does not. Text a dead key composed also differs, so it counts.
/// - Without a base (logical-only events), any text but a space counts.
///
/// A release decides from its own event, so after a dead key it can disagree with its press;
/// Ghostty ignores consumed modifiers on a release.
fn shift_consumed(ev: &KeyEvent, typed: &str, base: Option<char>) -> bool {
    let Key::Text(t) = &ev.key else {
        return false;
    };
    if !ev.mods.shift || typed.is_empty() {
        return false;
    }
    match base {
        Some(b) => b.is_lowercase() || b.is_uppercase() || t.chars().ne(std::iter::once(b)),
        None => t != " ",
    }
}

pub enum Action {
    Press,
    Repeat,
    Release,
}

/// Returns `None` for keys Rex has no name for.
pub fn payload(
    ev: &KeyEvent,
    physical: Option<PhysicalKeyCode>,
    base: Option<char>,
    action: Action,
) -> Option<Value> {
    let name = physical.map(code_name).or_else(|| logical_name(&ev.key))?;
    let action = match action {
        Action::Press => "press",
        Action::Repeat => "repeat",
        Action::Release => "release",
    };
    let typed = text(ev);
    let consumed = shift_consumed(ev, &typed, base);
    let text = if action == "release" {
        String::new()
    } else {
        typed
    };
    let unshifted = base
        .or_else(|| match &ev.key {
            Key::Command(c) => Some(c.to_ascii_lowercase()),
            Key::Text(t) if t.chars().count() == 1 && !ev.mods.shift => t.chars().next(),
            _ => None,
        })
        .map_or(0, |c| c as u32);
    Some(json!({
        "key": name,
        "mods": mods(ev.mods),
        "consumed_mods": if consumed { "shift" } else { "" },
        "text": text,
        "unshifted_codepoint": unshifted,
        "composing": matches!(ev.key, Key::Dead(_)),
        "action": action,
    }))
}

/// Explicit, layout-free pane.send_key: use the regular payload builder for both edges,
/// never a US shifted-character table. Physical spellings carry no invented text.
pub fn synthesize(spelling: &str) -> Option<[Value; 2]> {
    use rex_rune::keymap::{Chord, Key as K};
    let chord = Chord::parse(spelling).ok()?;
    let mods = Mods {
        ctrl: chord.modifiers.ctrl,
        alt: chord.modifiers.alt,
        shift: chord.modifiers.shift,
        logo: chord.modifiers.logo,
        ..Mods::default()
    };
    let (key, base, wire) = match chord.key {
        K::Character(c) => (Key::Text(c.to_string()), Some(c), None),
        K::Physical(code) => (Key::Command('a'), None, Some(wire_code_name(code))),
        K::Named(name) => {
            let key = match name {
                "enter" => Key::Enter,
                "escape" => Key::Escape,
                "tab" => Key::Tab,
                "space" => Key::Text(" ".into()),
                "backspace" => Key::Backspace,
                "delete" => Key::Delete,
                "left" => Key::Left,
                "right" => Key::Right,
                "up" => Key::Up,
                "down" => Key::Down,
                "home" => Key::Home,
                "end" => Key::End,
                "pageup" => Key::PageUp,
                "pagedown" => Key::PageDown,
                "insert" => Key::Command('a'),
                f => Key::Function(f.strip_prefix('f')?.parse().ok()?),
            };
            (key, None, (name == "insert").then(|| "insert".into()))
        }
    };
    let event = KeyEvent { key, mods };
    let mut press = payload(&event, None, base, Action::Press)?;
    let mut release = payload(&event, None, base, Action::Release)?;
    if let Some(wire) = wire {
        for payload in [&mut press, &mut release] {
            payload["key"] = json!(wire);
            payload["text"] = json!("");
            payload["unshifted_codepoint"] = json!(0);
        }
    }
    Some([press, release])
}

#[cfg(test)]
mod tests;
