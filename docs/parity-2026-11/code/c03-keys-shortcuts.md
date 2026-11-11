# C03 Keys, shortcuts, input (Rex Linux, from code)

Source: worktree of rex-rune at `ad196ff`. Paths are relative to the repo root. Line numbers are from reading the code; verify before quoting.

## Pipeline

- Rune key event -> `input::on_key` (`src/app/input.rs:207`): with a panel open, list popups use `list_key` (`input.rs:136`), other panels only take `global_key` (`input.rs:27`) then Escape; with the main view, `dispatch` (`input.rs:31`) runs shortcuts first, then `forward` (`input.rs:229`) sends everything else to the focused PTY via `client.key`.
- Key release: `on_key_up` (`input.rs:273`) forwards a release only if its press was forwarded (`s.forwarded` identity set); keys consumed by a shortcut/popup keep releases local.
- Hold-to-repeat: a consumed identity suppresses repeats unless the action `repeats()` (`actions.rs:148`): zoom in/out/reset, pane.focus, tab.next/previous, pane.scroll, tab.next_global/previous_global.
- Matching (`src/app/bindings.rs:65`): ctrl+cmd both count as `ctrl` (`bindings.rs:68`; Rex `cmd`/`super`/`meta`/`win` mean the logo key); character chords match the layout character (`key_base`), with a US-position physical fallback for non-Latin layouts and shifted punctuation (`bindings.rs:85-101`); named chords at `bindings.rs:102`; physical chords `[KeyT]` by W3C code, Super mapped to Meta (`bindings.rs:77`).
- Sequences (`bindings.rs:137`): `>`-separated chords, Escape cancels (`:174`), modifier-only presses keep pending (`:171`), a failed tail is never retried as a fresh shortcut (doc `:163`), context change (host/session/pane/panel/focus/keymap generation/connection) clears the pending sequence (`input.rs:46-59`).
- Pending sequence chip in toolbar, e.g. `Ctrl B …`, tooltip "waiting for the next key; Escape cancels" (`src/app/toolbar.rs:300`).
- Keymap layers: Linux defaults -> host Rex config -> client overrides; the active host's reply drives dispatch, local defaults+overrides are the offline/failed fallback (`input.rs:66-71`, `workspace.rs:637`, `src/connection/keymap.rs`).
- Declaration to server: `client.keymap` with `server_actions("all")`, defaults and overrides layers (`workspace.rs:630-632`); re-registered on reconnect and on `keymap_changed` (`connection.rs:1120`).

## Default bindings (all 33, `src/keymap.rs:370-414`)

Layer name `linux`. Every default action is registered in `actions::SPECS` (test at `actions.rs:555`).

| Keys (spelling) | Action | Args | Title (`actions.rs:161`) | Category | file:line |
| --- | --- | --- | --- | --- | --- |
| ctrl+k | session.switch | - | Switch session | Navigate | keymap.rs:373 |
| ctrl+shift+p | client.palette.toggle | - | Commands | Interface | keymap.rs:374 |
| ctrl+, | client.settings | - | Settings | Interface | keymap.rs:375 |
| ctrl+shift+b | client.sidebar.toggle | - | Toggle sidebar | Interface | keymap.rs:376 |
| ctrl+shift+n | session.new | - | New session | Create and close | keymap.rs:377 |
| ctrl+shift+t | client.tab.new | - | New tab | Create and close | keymap.rs:378 |
| ctrl+shift+w | client.close | - | Close focused pane or tab (pane if >1 pane, else tab; asks first, `workspace.rs:1394`) | Create and close | keymap.rs:379 |
| ctrl+tab | client.tab.next_global | - | Next tab across hosts | Navigate | keymap.rs:380 |
| ctrl+shift+tab | client.tab.previous_global | - | Previous tab across hosts | Navigate | keymap.rs:381 |
| ctrl+pageup | client.tab.previous | - | Previous tab (server `session.focus_previous_window`, no wrap) | Navigate | keymap.rs:382 |
| ctrl+pagedown | client.tab.next | - | Next tab (server `session.focus_next_window`) | Navigate | keymap.rs:383 |
| ctrl+shift+enter | pane.zoom | - | Zoom pane | Panes | keymap.rs:384 |
| ctrl+shift+space | pane.actions | - | Pane actions (opens pane menu) | Panes | keymap.rs:385 |
| ctrl+shift+c | pane.copy | - | Copy selection | Text and view | keymap.rs:386 |
| ctrl+shift+v | pane.paste | - | Paste | Text and view | keymap.rs:387 |
| ctrl+shift+f | client.search | - | Search terminal | Text and view | keymap.rs:388 |
| ctrl+shift+r | pane.reconnect | - | Reconnect terminal | Panes | keymap.rs:389 |
| ctrl+=, ctrl+shift+=, ctrl++, ctrl+[NumpadAdd] | client.zoom.in | - | Enlarge interface | Interface | keymap.rs:390-393 |
| ctrl+-, ctrl+[NumpadSubtract] | client.zoom.out | - | Shrink interface | Interface | keymap.rs:394-395 |
| ctrl+0, ctrl+[Numpad0] | client.zoom.reset | - | Reset interface size | Interface | keymap.rs:396-397 |
| ctrl+shift+d | pane.split | direction=right | Split pane | Panes | keymap.rs:401 |
| ctrl+alt+d | pane.split | direction=down | Split pane | Panes | keymap.rs:401 |
| ctrl+alt+left/right/up/down | pane.focus | direction=left/right/up/down | Focus pane | Panes | keymap.rs:404-409 |
| shift+pageup | pane.scroll | lines=-20 | Scroll terminal | Text and view | keymap.rs:410 |
| shift+pagedown | pane.scroll | lines=20 | Scroll terminal | Text and view | keymap.rs:410 |

Plus a pointer default: mouse Back / Forward buttons -> `client.history.back` / `client.history.forward` (`src/app/mod.rs:98-109`; ignored while a panel is open or a drag is active).

Zoom actions are "global": they also fire while a field or dialog is focused (`actions.rs:142`, `input.rs:75`); all other actions are main-view only (`global_only` gate).

## Action registry (32 actions, `src/app/actions.rs:62-95`)

No default binding, available to bind: `client.history.back` (:73, Back in history), `client.history.forward` (:74), `session.select` (:79, args `session_id` required, `window_id` optional; "Show session"), `client.tab.goto` (:89, `index` integer >=1 or -1 for last), `client.tab.close` (:90), `client.tab.rename` (:91), `session.rename` (:92), `pane.close` (:93), `pane.send_key` (:94, `key` = single synthesizable chord, bypasses Lua bindings via `block.event`, press+release, `keys.rs:187`).

Argument validation before dispatch (`actions.rs:262`): split direction right|down; focus left|right|up|down; scroll i32 lines; empty object or null for argless; unknown keys rejected. Invalid args -> binding skipped plus a diagnostic (`actions.rs:529-547`).

Remote consent (default deny, `Spec::remote()` `actions.rs:109`): remotely allowed view-only: tab.next, tab.previous, tab.goto, pane.focus, session.select (navigation) and sidebar.toggle, pane.zoom, pane.scroll. Everything else `Never(reason)` (paste, copy, send_key, new session/tab, split, close, rename, dialogs, zoom interface, global tab/history, reconnect). Action declarations tell the server which are not remote-available (`actions.rs:508`).

Server (Lua/block) actions: any action the server lists with kind `lua` or `block` is dispatched to the host via `HostCommand::Action` with `origin("key")`, args, session and focused block (`actions.rs:451`, `input.rs:112-127`). `client.key.forward` forwards the key to the PTY with the server's original prefix (`input.rs:102`, `Command::KeyWithPrefix`).

## Override file and rebinding (no recording UI)

- File: `$XDG_CONFIG_HOME/rex-linux/shortcuts.json`, or `<config>.shortcuts.json` beside a custom `--config` (`src/settings/shortcuts.rs:22-29`). The app never writes it except the one-time nivox `keybindings` import (`shortcuts.rs:104`, `settings.rs:273`). Object form `{"shortcuts":[...]}` or bare array; missing/empty means none.
- Entry: `{"keys": "ctrl+b>c", "action": "name"|null, "args": {...}}` (`keymap.rs:417-424`, `Overrides::parse` `keymap.rs:460`). `action: null` unbinds exactly that chord/sequence in lower layers (not its prefix); binding-then-null folds away (`keymap.rs:496-503`). Later entries win. Bad entries are skipped with a diagnostic.
- Chord grammar (`Chord::parse` `keymap.rs:243`): modifiers ctrl|control, alt|opt|option, shift, cmd|command|super|meta|win (all = logo key); duplicate modifiers rejected; literal `+` key handled (`ctrl++`); named keys enter, escape, tab, space, backspace, delete, insert, left, right, up, down, home, end, pageup, pagedown, f1-f12 (`keymap.rs:32`); single characters (Unicode ok); physical `[Code]` from a W3C code list of ~190 names (`keymap.rs:64-240`). Canonical spelling order ctrl, alt, shift, cmd (`keymap.rs:294`).
- Layout: unknown action diagnostics for `client.`/`session.`/`pane.` prefixes; prefix-vs-key shadowing diagnostics (`keymap.rs:618-681`).
- Reload: on window focus and when Settings opens, gated by file mtime+length (`workspace.rs:604-607`, `workspace.rs:712`, README "hand edits reload").
- Parse problems: one-time toast for the file, persistent Problems list; server `settings`-source diagnostics toast once per distinct set per host (`workspace.rs:682`).
- Settings -> Shortcuts (`src/app/settings_view.rs:592-728`): read-only list of effective bindings grouped Navigate / Create and close / Panes / Text and view / Interface / Server (`:596-603`); keycaps via rune-kit `Keybind`; "then" between sequence steps; "also numpad" merge of numpad +/-/0 with the main variants (`:569-583`); four default `ctrl+alt+arrow` rows collapsed into one "Ctrl Alt Arrows - Focus adjacent pane" row (`:539-561`); source label ("shortcuts.json", server layer name, or "Server"); "Pointer: Mouse Back / Forward - Navigation history" row (`:658`); Problems list; server and local file paths (selectable text); footer text "Edit the overrides file, then return to Rex to reload. Escape cancels a sequence; unbound keys reach the terminal." (`:722`).
- Commands/menus show the highest binding per action+args as a trailing detail (`workspace.rs:663`, `panel.rs:861`); toolbar tooltips hard-code a few ("Toggle sidebar - Ctrl Shift B" `toolbar.rs:292`, "Switch session - Ctrl K" `toolbar.rs:44`). Keycaps use "Super" for logo (`keymap.rs:320`).

## Key handling to the PTY

- Every unbound key goes to `client.key` as a Ghostty-named payload (`src/app/keys.rs:148-183`): `key` from the physical code (`KeyA` -> `key_a`, `ArrowUp` -> `arrow_up`, `Numpad0` -> `numpad_0`, `F12` -> `f12`; `keys.rs:7-36`), falling back to a logical name when no physical code (`keys.rs:39`); `mods` string shift/ctrl/alt/super (`keys.rs:92`, Rune `cmd` counts as ctrl off macOS); `text` (Enter `\n`, Tab `\t`, none with Ctrl/Super; Alt keeps text, `keys.rs:108`); `consumed_mods` = shift when the layout used it (`keys.rs:128`); `unshifted_codepoint` from the layout base key; `composing` true for dead keys; `action` press | repeat | release (release carries empty text).
- Physical-position key naming means non-US layouts work (README :38); Caps Lock cased-key heuristic documented at `keys.rs:119-127`.
- Keys with no Rex name are dropped (`payload` returns None, `keys.rs:154`).
- Forwarding gates: needs an active terminal with `status().live`; otherwise the key is swallowed (`input.rs:241-246`); a typing key clears the local selection (`input.rs:247`) and bumps `last_keypress` (used for remote-action quiet period); blurs any Rune-focused field so the terminal gets focus (`input.rs:230`). Failed input is not replayed.
- Escape in the main view goes to the PTY unless a sequence is pending.
- `pane.send_key` synthesis (`keys.rs:187-233`): parses a chord, builds press+release payloads with no invented text for physical spellings, `insert` via wire name; sent as `Command::SendKey` (`block.event`), bypassing Lua bindings.
- Scrolling keys: Shift+PageUp/PageDown default to `pane.scroll` +/-20 lines local scrollback (`Command::Scroll`, `actions.rs:372`).

## IME and dead keys

- Dead keys: Rune's dead-key events forwarded (`composing` flag, `keys.rs:180`; vendored Rune patch, README :485); shift-consumed heuristic notes dead-key text (`keys.rs:160`).
- IME: `on_ime` (`input.rs:296`): `Preedit` shown as underlined text at the cursor (`term.rs:1143-1160`, field `preedit` `term.rs:45`); `Commit` sent as `Command::Paste` (bracketed paste aware path) if the pane is live; cursor flagged `ime_cursor` only when the pane owns the keyboard (`term.rs:1129`, `panes.rs:286`). Preedit is cleared on commit.
- README states input methods beyond dead-key composition are not hands-on validated (README :493).

## Mouse

- Pointer to program: if the program tracks the mouse and Shift is not held, left/middle press, motion (cell-change only) and release become `block.event` mouse payloads (`panes.rs:335-390`, `term.rs:391`); Shift is stripped from the reported mods (`term.rs:396`). Hover motion without button (mode 1003) only for the focused, live pane (`panes.rs:433-445`). Wheel over a tracking program sends up to 8 wheel events per gesture (`term.rs:406`, `panes.rs:486`); otherwise wheel scrolls local scrollback; Shift+wheel always scrolls local history (`panes.rs:475-494`); precise (trackpad) wheel is divided by cell height and UI scale.
- Right-click opens the pane menu always; Shift+right-click sends right press+release to a tracking program (`panes.rs:454-474`). Press on a pane focuses it (`panes.rs:336`, `focus_block`).
- Ctrl+click opens `http`, `https`, `ftp`, `mailto` links (OSC 8 or text URLs) with `xdg-open`, on release over the same link; hover with Ctrl underlines, hand cursor, tooltip with the real target (`panes.rs:340-372`, `workspace.rs:768`, `src/links.rs`). Works over mouse-tracking programs. Other schemes refused.
- Cursor shape: I-beam normally, default arrow when a program tracks the mouse, pointer over a link (`panes.rs:299-305`).
- Mouse Back/Forward buttons: history navigation (`mod.rs:98`).
- Middle-click on a tab pill closes it behind the confirm (`toolbar.rs:110-113`). Right-click tab / session / host / pane opens menus. Double-click on the sidebar edge resets width (`sidebar.rs:538`) and on the toolbar drag region toggles maximize (`toolbar.rs:120`).
- Drag-and-drop files on a pane (`panes.rs:238-248`) with a drop-highlight; upload implemented (`upload.rs:36`).

## Selection and clipboard

- Selection: left drag selects locally; double-click = word (including shell paths), triple-click = line (`term.rs:415`, `panes.rs:394-406`); selection stays anchored to content during scroll; drag past the grid edge autoscrolls (README :432; selection phases sent to the server as `Command::Selection` with geometry, `term.rs:449`); Shift+drag selects locally in a mouse-tracking TUI. A bare focus click does not copy (`term.rs:425`). Selection cleared on typing (`input.rs:247`) and on forwarded mouse press (`panes.rs:352`).
- Copy: `pane.copy` (Ctrl+Shift+C) writes `copy_text()` (the server's `selection_text` if its revision matches, else the client's `selected_text`) to the system clipboard (`actions.rs:361`, `term.rs:502`). Server-driven copy: `selection_copied` and OSC 52 `clipboard_written` events copy to the clipboard, <=8 MiB, only for the active session's focused pane while the window is focused and no dialog open (`src/connection/clipboard.rs:22,29`; `workspace.rs:532-545`). Clipboard reads by programs are not supported (clipboard.rs doc).
- Paste: `pane.paste` (Ctrl+Shift+V) reads the clipboard (`upload.rs:50`): copied files -> upload; else text -> `Command::Paste` (bracketed-paste aware, server side); else image -> uploaded as `clipboard.png`. Result goes only to the pane that asked if still focused and no panel open.
- File upload: requires API revision 11+, bytes over the Rex connection, shell-quoted paths typed as a paste, no Enter, folders rejected, failures never retried (README :436).
- Terminal search: Ctrl+Shift+F opens a field inside the pane (`workspace.rs:924`); Enter = next older match, Shift+Enter = next newer, Escape closes (`input.rs:177-194`); count shown "2/5" / "No matches" / "esc" (`panes.rs:552`); matches highlighted in accent.

## Other keyboard behaviours

- Popups (switcher, Commands, host step, pane menu): Up/Down navigate, Enter submits, Escape cancels, Tab swallowed, F2 renames the highlighted session in the switcher (`input.rs:143-160`); Ctrl+Shift+Space closes the pane menu (`input.rs:163-171`); Enter/Escape in a rename row (`input.rs:197`).
- Tab/Shift-Tab cycles fields in dialogs; confirm dialogs default to Keep running (DESIGN.md).
- Global tab cycle (Ctrl+Tab): cached sidebar order across hosts, wrap, 150 ms settle, history is local to window and survives reconnects (`navigation.rs:293-360`, README :249).
- Window focus loss clears the pending sequence, consumed/forwarded sets and ends any selection drag (`workspace.rs:602-620`).

## Absent or partial

- No shortcut recording/rebinding UI: Settings -> Shortcuts is read-only; rebinding is hand-editing `shortcuts.json` (README :422 "a recording editor ... not implemented"; `settings_view.rs:722`).
- No key modes (modal bindings): bindings with a `mode` are skipped (`actions.rs:438`, `keymap.rs:597`, README :422).
- Server actions are not listed in Commands (README :422); they only run via key bindings. Pane-resize bindings and actions: none registered.
- Cross-server `client_call_requested` relaying not implemented (README :423, issue #20).
- No default bindings for: tab goto (e.g. Ctrl+1..9), tab/session rename, pane close, send_key, history back/forward keys (mouse only), pane-resize, select-all, clear scrollback, scroll to top/bottom, font size change, new window, quit.
- No context menu (right-click) Copy/Paste entries: pane menu has split right/down, zoom/restore, rename, reconnect, close only (`panel.rs:717-726`); no "Select all".
- No primary-selection (middle-click paste) and no copy-on-select setting in the client; copying on selection depends on server `selection_copied` events.
- Middle-click does not paste; it goes to a tracking program or does nothing.
- Terminal-requested clipboard reads (OSC 52 query) unsupported by design.
- Links: only http/https/ftp/mailto via Ctrl+click; no right-click "Copy link/Open link", no plain-click open.
- Hold-to-repeat not available for non-listed actions (new tab, split, search etc. fire once per hold).
- IME: only preedit text drawn at the cursor and commit sent as paste; no IME candidate-window positioning beyond Rune's `ime_cursor`, no preedit cursor/segments (the second preedit field is ignored, `input.rs:301`); input methods beyond dead keys not validated (README :493).
- `keycaps()` uses "Super"; no Cmd/Option naming and no per-platform keycap glyphs.
- Shift+Enter, Ctrl+Enter etc. depend purely on Ghostty encoding by the server; the client does not do its own CSI-u mapping.
- Physical Wayland pointer gestures, multi-seat, X11 window-menu are noted as unvalidated (README :493).
- Local keyboard "no terminal" case: with no live terminal, ordinary keys are swallowed silently (`input.rs:241-246`).

## Constraints

- Terminal content, colours and split geometry are server-owned; the client only declares a theme via `terminal.set_theme` and never recolours or invents layout (AGENTS.md "Conventions", PRODUCT.md Principles, DESIGN.md).
- Ordinary attachment must never start/stop/reconfigure a server or create/destroy sessions; creation is explicit (AGENTS.md, PRODUCT.md). Shortcut-driven New session/tab/split are explicit user actions.
- Failed input is never replayed; a transport failure pauses affected panes and never queues input (AGENTS.md, PRODUCT.md, DESIGN.md).
- Effective workspace shortcuts consume their keys; every binding must be removable or replaceable; ordinary keys flow through `client.key` and releases are sent only when the press was (AGENTS.md). Only explicit `pane.send_key` uses `block.event` to bypass Lua bindings.
- The active host's effective map drives dispatch, Settings and Commands; offline local defaults still work (PRODUCT.md "Keyboard first").
- Remote availability decided in `Spec::remote()`, default deny, consent per saved host; replies stay on connection epoch; allowed-set test pinned (AGENTS.md; README "Remote control"). Remote requests cannot type, use the clipboard, or open dialogs.
- Clipboard: the client never sends the clipboard to a remote program without a user paste action; programs cannot read it; terminal-initiated writes only from the focused, visible pane of a focused window (README :434).
- Size ownership is automatic and selecting, copying or navigating must not claim ownership (DESIGN.md "Do and don't").
- Terminal content never animates; chrome motion uses springs in `theme.rs` (AGENTS.md, DESIGN.md).
- DESIGN.md: no keyboard-hint footers, status dots, terminal size readouts; shortcuts are shown in muted keycaps; Settings -> Shortcuts keeps 190 pt keycap column and 12.5 pt labels; right-click captures the pane under the pointer; links open only on Ctrl+click.
- Native Rune look: take Rune defaults/rendering, accessibility on; do not restore old pixels (user memory note). Linux modifier convention: Ctrl+Shift for terminal-safe app shortcuts so plain Ctrl chords reach programs; `cmd` in config means Super, never Ctrl (`keymap.rs:10`), so macOS Cmd-based defaults cannot be copied one to one.
- Keep README claims honest: do not document unimplemented features (AGENTS.md).
- Out of scope per PRODUCT.md: complete Kitty graphics, AT-SPI accessibility, distribution packages.
