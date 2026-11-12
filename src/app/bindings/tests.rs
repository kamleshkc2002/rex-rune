use super::*;
use rex_linux::keymap::{Overrides, resolve};
use rune::Mods;
use serde_json::json;

#[test]
fn layout_matrix() {
    for (binding, text, code, base, shift, logo, expected) in [
        (
            "ctrl+shift+t",
            "Е",
            Code::KeyT,
            Some('е'),
            true,
            false,
            true,
        ),
        ("ctrl+,", "б", Code::Comma, Some('б'), false, false, true),
        (
            "ctrl++",
            "+",
            Code::BracketRight,
            Some('+'),
            false,
            false,
            true,
        ),
        ("ctrl+0", "à", Code::Digit0, Some('à'), false, false, true),
        (
            "ctrl+shift+t",
            "T",
            Code::KeyK,
            Some('t'),
            true,
            false,
            true,
        ),
        (
            "ctrl+shift+k",
            "T",
            Code::KeyK,
            Some('t'),
            true,
            false,
            false,
        ),
        (
            "ctrl+shift+5",
            "%",
            Code::Digit5,
            Some('5'),
            true,
            false,
            true,
        ),
        (
            "ctrl+[KeyQ]",
            "a",
            Code::KeyQ,
            Some('a'),
            false,
            false,
            true,
        ),
        ("ctrl+shift+t", "T", Code::KeyK, None, true, false, true),
        ("ctrl+shift+t", "Е", Code::KeyT, None, true, false, true),
        ("ctrl+,", "б", Code::Comma, None, false, false, true),
        ("ctrl+0", "à", Code::Digit0, None, false, false, true),
        ("ctrl+shift+5", "%", Code::Digit5, None, true, false, true),
        ("cmd+k", "k", Code::KeyK, Some('k'), false, false, false),
    ] {
        let ev = KeyEvent {
            key: Key::Text(text.into()),
            mods: Mods {
                ctrl: true,
                shift,
                logo,
                ..Mods::default()
            },
        };
        assert_eq!(
            matches(
                &Chord::parse(binding).unwrap(),
                Input {
                    event: &ev,
                    code: Some(code),
                    base,
                    repeat: false
                }
            ),
            expected,
            "{binding}, {text}, {base:?}"
        );
    }
    let ev = KeyEvent {
        key: Key::Command('k'),
        mods: Mods {
            logo: true,
            ..Mods::default()
        },
    };
    assert!(matches(
        &Chord::parse("cmd+k").unwrap(),
        Input {
            event: &ev,
            code: None,
            base: None,
            repeat: false
        }
    ));
    assert!(!matches(
        &Chord::parse("ctrl+k").unwrap(),
        Input {
            event: &ev,
            code: None,
            base: None,
            repeat: false
        }
    ));
}

#[test]
fn sequences_complete_cancel_and_do_not_retry_failed_tail() {
    let map = resolve(
        &[],
        &Overrides::parse(&json!([
            {"keys":"ctrl+b>c>s", "action":"session.switch"},
            {"keys":"ctrl+b>z", "action":"pane.zoom"},
            {"keys":"x", "action":"client.settings"}
        ])),
        &[],
    );
    let mut seq = Sequence::default();
    let mut step = |key: Key, ctrl| {
        let event = KeyEvent {
            key,
            mods: Mods {
                ctrl,
                ..Mods::default()
            },
        };
        seq.advance(
            &map,
            Input {
                event: &event,
                code: None,
                base: None,
                repeat: false,
            },
            |_| true,
        )
    };
    assert_eq!(step(Key::Command('b'), true), Step::Pending);
    assert_eq!(step(Key::Text("c".into()), false), Step::Pending);
    assert!(matches!(
        step(Key::Text("s".into()), false),
        Step::Run { sequence: true, .. }
    ));
    assert_eq!(step(Key::Command('b'), true), Step::Pending);
    assert_eq!(step(Key::Escape, false), Step::Pending);
    assert_eq!(step(Key::Text("z".into()), false), Step::None);
    assert_eq!(step(Key::Command('b'), true), Step::Pending);
    assert_eq!(step(Key::Text("x".into()), false), Step::None);
    assert!(matches!(
        step(Key::Text("x".into()), false),
        Step::Run {
            sequence: false,
            ..
        }
    ));
}

#[test]
fn newest_available_binding_decides_sequence_or_single() {
    for (entries, expected) in [
        (
            json!([{"keys":"ctrl+b", "action":"pane.zoom"},{"keys":"ctrl+b>c", "action":"client.tab.new"}]),
            true,
        ),
        (
            json!([{"keys":"ctrl+b>c", "action":"client.tab.new"},{"keys":"ctrl+b", "action":"pane.zoom"}]),
            false,
        ),
    ] {
        let map = resolve(&[], &Overrides::parse(&entries), &[]);
        let event = KeyEvent {
            key: Key::Command('b'),
            mods: Mods {
                ctrl: true,
                ..Mods::default()
            },
        };
        let input = Input {
            event: &event,
            code: None,
            base: None,
            repeat: false,
        };
        let step = Sequence::default().advance(&map, input, |_| true);
        assert_eq!(step == Step::Pending, expected);
        let step = Sequence::default().advance(&map, input, |i| i != 0);
        assert_eq!(step == Step::Pending, !expected);
    }
}

#[test]
fn modifier_only_passes_at_depth_zero_but_does_not_cancel_a_prefix() {
    let map = resolve(
        &[],
        &Overrides::parse(&json!([{"keys":"ctrl+b>c","action":"pane.zoom"}])),
        &[],
    );
    let mut seq = Sequence::default();
    let modifier = KeyEvent {
        key: Key::Modifier {
            modifier: rune::Modifier::ShiftLeft,
            pressed: true,
        },
        mods: Mods::default(),
    };
    let input = Input {
        event: &modifier,
        code: None,
        base: None,
        repeat: false,
    };
    assert_eq!(seq.advance(&map, input, |_| true), Step::None);
    let prefix = KeyEvent {
        key: Key::Command('b'),
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    };
    assert_eq!(
        seq.advance(
            &map,
            Input {
                event: &prefix,
                ..input
            },
            |_| true
        ),
        Step::Pending
    );
    assert_eq!(seq.advance(&map, input, |_| true), Step::Pending);
    assert_eq!(seq.depth, 1);
}

#[test]
fn send_key_uses_payload_builder_for_both_edges_and_rejects_f13() {
    let [press, release] = crate::app::keys::synthesize("ctrl+b").unwrap();
    assert_eq!(press["key"], "key_b");
    assert_eq!(press["mods"], "ctrl");
    assert_eq!(press["action"], "press");
    assert_eq!(release["action"], "release");
    assert_eq!(release["text"], "");
    assert!(crate::app::keys::synthesize("f13").is_none());
    assert!(crate::app::keys::synthesize("ctrl+b>c").is_none());
    let [press, _] = crate::app::keys::synthesize("ctrl+[KeyT]").unwrap();
    assert_eq!(press["key"], "key_t");
    assert_eq!(press["unshifted_codepoint"], 0);
}

#[test]
fn a_press_spells_the_chord_that_finds_it_on_any_layout() {
    let ctrl = Mods {
        ctrl: true,
        cmd: true,
        ..Mods::default()
    };
    let shift = Mods {
        shift: true,
        ..Mods::default()
    };
    let ctrl_shift = Mods {
        shift: true,
        ..ctrl
    };
    for (key, mods, code, base, spelled) in [
        // A native Ctrl chord: the lowercased character, with or without its code.
        (Key::Command('j'), ctrl_shift, None, None, "ctrl+shift+j"),
        (
            Key::Command('j'),
            ctrl_shift,
            Some(Code::KeyJ),
            Some('j'),
            "ctrl+shift+j",
        ),
        (
            Key::Text("J".into()),
            ctrl_shift,
            None,
            None,
            "ctrl+shift+j",
        ),
        // Russian: the key's US character, which the defaults are spelled with.
        (
            Key::Text("Е".into()),
            ctrl_shift,
            Some(Code::KeyT),
            Some('е'),
            "ctrl+shift+t",
        ),
        // Dvorak: the letter the layout types, not the key's position.
        (
            Key::Text("T".into()),
            ctrl_shift,
            Some(Code::KeyK),
            Some('t'),
            "ctrl+shift+t",
        ),
        // AZERTY's digit row and a shifted symbol: the unshifted key.
        (
            Key::Text("à".into()),
            ctrl,
            Some(Code::Digit0),
            Some('à'),
            "ctrl+0",
        ),
        (
            Key::Text("%".into()),
            ctrl_shift,
            Some(Code::Digit5),
            Some('5'),
            "ctrl+shift+5",
        ),
        // German: plus is a key of its own.
        (
            Key::Text("+".into()),
            ctrl,
            Some(Code::BracketRight),
            Some('+'),
            "ctrl++",
        ),
        (Key::Command(','), ctrl, None, None, "ctrl+,"),
        (
            Key::Text(" ".into()),
            ctrl_shift,
            Some(Code::Space),
            None,
            "ctrl+shift+space",
        ),
        (Key::Enter, ctrl_shift, None, None, "ctrl+shift+enter"),
        (Key::PageUp, shift, None, None, "shift+pageup"),
        (Key::Function(3), Mods::default(), None, None, "f3"),
        (Key::Tab, ctrl, None, None, "ctrl+tab"),
        (
            Key::Left,
            Mods { alt: true, ..ctrl },
            None,
            None,
            "ctrl+alt+left",
        ),
        // The numpad's plus is the character it types.
        (
            Key::Text("+".into()),
            ctrl,
            Some(Code::NumpadAdd),
            None,
            "ctrl++",
        ),
        // A key with no character and no name: its physical code.
        (
            Key::Insert,
            Mods::default(),
            Some(Code::Insert),
            None,
            "[Insert]",
        ),
        (
            Key::Command('k'),
            Mods {
                logo: true,
                ..Mods::default()
            },
            None,
            None,
            "cmd+k",
        ),
    ] {
        let event = KeyEvent { key, mods };
        let input = Input {
            event: &event,
            code,
            base,
            repeat: false,
        };
        let chord = chord_of(input).unwrap_or_else(|| panic!("{spelled}: no chord"));
        assert_eq!(chord.spelling(), spelled);
        assert!(matches(&chord, input), "{spelled} finds its own press");
        // And it reads back as the same chord from a file.
        assert_eq!(Chord::parse(spelled).unwrap(), chord);
    }
    // A modifier going down, a dead key and a key with nothing to call it are no chord.
    for (key, code) in [
        (
            Key::Modifier {
                modifier: rune::Modifier::ControlLeft,
                pressed: true,
            },
            Some(Code::ControlLeft),
        ),
        (Key::Dead(Some('´')), Some(Code::Equal)),
        (Key::Insert, None),
    ] {
        let event = KeyEvent {
            key,
            mods: Mods::default(),
        };
        assert!(
            chord_of(Input {
                event: &event,
                code,
                base: None,
                repeat: false
            })
            .is_none()
        );
    }
}
