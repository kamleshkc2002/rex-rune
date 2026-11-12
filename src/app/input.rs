//! Keyboard: workspace shortcuts first, then every other key to the focused pane's PTY.
//! Terminals never take Rune focus. With nothing focused Rune offers a key to the window's
//! default button, then to `on_key`, then to Tab traversal, Space/Return presses and
//! scrolling. So no default button may exist outside an open modal (the main view has none),
//! and `on_key` takes every key it forwards.
use super::{
    actions,
    bindings::{Context, Input, Step},
    keys::{self, Action},
    panel::{Intent, PanelKind},
    workspace::{HostId, Workspace},
};
use rex_rune::connection::Command;
use rune::{Ime, Key, KeyEvent, PhysicalKeyCode as Code, Tx};

fn ctrl(ev: &KeyEvent) -> bool {
    ev.mods.ctrl || ev.mods.cmd
}

fn space(ev: &KeyEvent, tx: &Tx) -> bool {
    tx.physical_key() == Some(Code::Space)
        || ev.is_command(' ')
        || matches!(&ev.key, Key::Text(t) if t == " ")
}

/// Global actions also reach fields and dialogs. Matching is still data-driven.
pub fn global_key(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if s.recording().is_some() {
        return super::recorder::key(s, ev, tx);
    }
    dispatch(s, ev, tx, true)
}

fn dispatch(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx, global_only: bool) -> bool {
    let input = Input {
        event: ev,
        code: tx.physical_key(),
        base: tx.key_base(),
        repeat: tx.key_repeat(),
    };
    let identity = input.identity();
    // A modal may have taken the previous release; a fresh press starts a new hold.
    if !input.repeat {
        s.bindings.consumed.remove(&identity);
    }
    if input.repeat && s.bindings.consumed.get(&identity) == Some(&true) {
        return true;
    }
    let context = Context {
        host: s.hosts[s.active].id,
        session: s.model().selected.clone(),
        pane: s.focused_block(),
        panel: s.panel.is_some(),
        focus: tx.focused().map(str::to_owned),
        generation: s.keymap_generation,
        server_revision: s.model().keymap_revision,
        connected: s.model().connected,
    };
    if s.bindings.context.as_ref() != Some(&context) {
        if s.bindings.context.as_ref().is_some_and(|old| {
            old.host != context.host
                || old.session != context.session
                || old.generation != context.generation
                || old.server_revision != context.server_revision
                || old.connected != context.connected
        }) {
            s.bindings.clear_modes();
        }
        s.bindings.sequence.clear();
        s.bindings.context = Some(context);
    }
    let model = &s.hosts[s.active].model;
    let snapshot = model
        .keymap
        .as_ref()
        .filter(|k| model.connected && k.generation == s.keymap_generation);
    let server_actions = snapshot.map(|k| k.reply.actions.as_slice()).unwrap_or(&[]);
    let map = model
        .keymap
        .as_ref()
        .filter(|k| model.connected && k.generation == s.keymap_generation)
        .map(|k| &k.resolved)
        .unwrap_or(&s.local_keymap);
    let modes = snapshot.map(|k| k.reply.modes.as_slice()).unwrap_or(&[]);
    let exclusive = s
        .bindings
        .modes
        .iter()
        .any(|(name, _)| modes.iter().any(|m| m.name == *name && m.exclusive));
    let in_mode = !s.bindings.modes.is_empty();
    s.bindings.mode_map = in_mode.then(|| super::bindings::mode_map(map, &s.bindings.modes, modes));
    let map = s.bindings.mode_map.as_ref().unwrap_or(map);
    let searching = s.panel.is_none() && s.active_terminal().is_some_and(|t| t.search.is_some());
    let step = s.bindings.sequence.advance(map, input, |i| {
        let binding = &map.bindings[i].binding;
        if !in_mode && binding.mode.is_some() {
            return false;
        }
        if binding.action == "linux.mode.block" {
            return !global_only;
        }
        if binding
            .args
            .as_ref()
            .is_some_and(|a| a["when"] == "searching")
            && !searching
        {
            return false;
        }
        match actions::classify(binding, server_actions) {
            actions::Kind::Local(spec) => {
                !global_only
                    || spec.global()
                    || searching && matches!(spec.name, "client.find.next" | "client.find.previous")
            }
            actions::Kind::Forward | actions::Kind::Server { .. } => {
                !global_only && model.connected
            }
            actions::Kind::Skip => false,
        }
    });
    match step {
        Step::None => {
            if exclusive && !global_only {
                s.bindings.consumed.insert(identity, true);
                true
            } else {
                false
            }
        }
        Step::Pending => {
            s.bindings.consumed.insert(identity, true);
            true
        }
        Step::Run { index, sequence } => {
            let binding = map.bindings[index].binding.clone();
            if binding.action == "linux.mode.block" {
                s.bindings.consumed.insert(identity, true);
                return true;
            }
            let mut once = false;
            if let Some(mode) = binding.mode.as_deref()
                && !matches!(
                    binding.action.as_str(),
                    "client.mode.exit" | "client.mode.exit_all"
                )
                && let Some(index) = s
                    .bindings
                    .modes
                    .iter()
                    .rposition(|(name, _)| name == mode)
                    .filter(|index| s.bindings.modes[*index].1)
            {
                s.bindings.modes.remove(index);
                once = true;
            }
            match actions::classify(&binding, server_actions) {
                actions::Kind::Local(spec) => {
                    s.bindings
                        .consumed
                        .insert(identity, sequence || once || !spec.repeats());
                    actions::run(
                        s,
                        &binding.action,
                        binding.args.as_ref().unwrap_or(&serde_json::Value::Null),
                        tx,
                    )
                }
                actions::Kind::Forward => {
                    if sequence || once {
                        // Suppress held-tail repeats after completing the sequence.
                        // forward() still records the press in forwarded, so its release
                        // reaches client.key even though this identity is consumed.
                        s.bindings.consumed.insert(identity, true);
                    }
                    let prefix = binding.keys[..binding.keys.len() - 1].to_vec();
                    forward(s, ev, tx, prefix)
                }
                actions::Kind::Server { repeats } => {
                    s.bindings
                        .consumed
                        .insert(identity, sequence || once || !repeats);
                    let mut request = rexrs::api::client::Action::new(binding.action).origin("key");
                    request.args = binding.args;
                    request.session_id = s.model().selected.clone();
                    if request.session_id.is_some() {
                        request.block_id = s.focused_block();
                    }
                    s.send(
                        s.active,
                        rex_rune::connection::HostCommand::Action {
                            generation: s.keymap_generation,
                            request: Box::new(request),
                        },
                    );
                    true
                }
                actions::Kind::Skip => false,
            }
        }
    }
}

/// Keys a list popup (switcher, Commands, host step, pane menu) takes, from its field or root.
pub fn list_key(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if global_key(s, ev, tx) {
        return true;
    }
    if s.panel.as_ref().is_some_and(|p| p.rename.is_some()) {
        return rename_key(s, ev, tx);
    }
    match ev.key {
        Key::Up | Key::Down => {
            s.navigate_panel(if ev.key == Key::Up { -1 } else { 1 });
            tx.instant();
        }
        // Commands: the highlighted command's shortcut, in Settings.
        Key::Enter
            if ctrl(ev)
                && s.panel
                    .as_ref()
                    .is_some_and(|p| matches!(p.kind, PanelKind::Picker(false))) =>
        {
            s.edit_selected_shortcut(tx)
        }
        Key::Enter => s.submit(tx),
        Key::Escape => s.cancel_panel(tx),
        Key::Function(2) => {
            let selected = s
                .panel
                .as_ref()
                .filter(|p| matches!(p.kind, PanelKind::Picker(true)));
            if let Some(Intent::Select(host, session)) =
                selected.and_then(|p| s.entries().get(p.selected).map(|e| e.action.clone()))
            {
                s.start_rename(host, session);
            }
        }
        // Lists have one control; Tab must not walk focus out to the window behind them.
        Key::Tab => {
            if !ev.mods.shift {
                s.descend_directory();
            }
        }
        _ if ctrl(ev)
            && ev.mods.shift
            && space(ev, tx)
            && s.panel
                .as_ref()
                .is_some_and(|p| matches!(p.kind, PanelKind::Pane)) =>
        {
            s.close_panel(tx)
        }
        _ => return false,
    }
    true
}

/// Keys a terminal's search field takes: Enter selects the next older match and Shift+Enter
/// the next newer one; Escape closes the search.
pub fn search_key(s: &mut Workspace, key: &(HostId, String), ev: &KeyEvent, tx: &mut Tx) -> bool {
    s.last_keypress = Some(std::time::Instant::now());
    if global_key(s, ev, tx) {
        return true;
    }
    match ev.key {
        Key::Escape => s.close_search(key, tx),
        Key::Enter => {
            if let Some(term) = s.terminals.get_mut(key) {
                term.send(Command::SearchStep(!ev.mods.shift));
            }
        }
        _ => return false,
    }
    true
}

/// Keys the switcher row being renamed takes: Enter saves, Esc goes back to browsing.
pub fn rename_key(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    match ev.key {
        Key::Enter => s.finish_rename(true),
        Key::Escape => s.finish_rename(false),
        Key::Tab | Key::Up | Key::Down => {}
        _ => return global_key(s, ev, tx),
    }
    true
}

pub fn on_key(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if s.recording().is_some() {
        return super::recorder::key(s, ev, tx);
    }
    if let Some(panel) = &s.panel {
        if panel.is_list() && tx.focused().is_none() {
            return list_key(s, ev, tx);
        }
        if global_key(s, ev, tx) {
            return true;
        }
        if ev.key == Key::Escape && tx.focused().is_none() {
            s.cancel_panel(tx);
            return true;
        }
        return false;
    }
    // The main view consumes both shortcuts (including sequence prefixes) and PTY keys.
    s.last_keypress = Some(std::time::Instant::now());
    if dispatch(s, ev, tx, false) {
        return true;
    }
    forward(s, ev, tx, Vec::new())
}

fn forward(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx, prefix: Vec<String>) -> bool {
    if tx.focused().is_some() {
        // Both unbound keys and explicit forward bindings return focus to the terminal.
        tx.blur();
    }
    let code = tx.physical_key();
    let action = if tx.key_repeat() {
        Action::Repeat
    } else {
        Action::Press
    };
    let base = tx.key_base();
    let Some(term) = s.active_terminal_mut() else {
        return true;
    };
    if !term.status().live {
        return true;
    }
    term.clear_selection();
    if let Some(payload) = keys::payload(ev, code, base, action) {
        term.send(if prefix.is_empty() {
            Command::Key(payload)
        } else {
            Command::KeyWithPrefix {
                key: payload,
                prefix,
            }
        });
        s.last_keypress = Some(std::time::Instant::now());
        s.forwarded.insert(
            Input {
                event: ev,
                code,
                base,
                repeat: false,
            }
            .identity(),
        );
    }
    true
}

/// A release reaches the PTY only when its press did: keys a shortcut, popup or field took
/// keep their releases local, even when the popup closed in between.
pub fn on_key_up(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let code = tx.physical_key();
    let base = tx.key_base();
    let identity = Input {
        event: ev,
        code,
        base,
        repeat: false,
    }
    .identity();
    let consumed = s.bindings.consumed.remove(&identity).is_some();
    if !s.forwarded.remove(&identity) {
        return consumed || s.panel.is_none();
    }
    if let Some(term) = s.active_terminal_mut()
        && term.status().live
        && let Some(payload) = keys::payload(ev, code, base, Action::Release)
    {
        term.send(Command::Key(payload));
    }
    true
}

pub fn on_ime(s: &mut Workspace, ime: &Ime, _: &mut Tx) -> bool {
    // A shortcut being recorded takes no composed text, and none of it reaches a terminal.
    if super::recorder::ime(s, ime) || s.panel.is_some() {
        // Text inputs handle their own IME first. A modal's unhandled commit (including
        // composition cancelled by the recorder) must never become terminal paste.
        return true;
    }
    let Some(term) = s.active_terminal_mut() else {
        return false;
    };
    match ime {
        Ime::Preedit(text, _) => term.preedit = text.clone(),
        Ime::Commit(text) => {
            term.preedit.clear();
            if term.status().live && !text.is_empty() {
                term.send(Command::Paste(text.clone()));
                s.last_keypress = Some(std::time::Instant::now());
            }
        }
        _ => {}
    }
    true
}
