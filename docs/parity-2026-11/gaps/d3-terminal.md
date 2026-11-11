# D3: Terminal settings and terminal behaviour (Rex Linux 0.6.0 vs Rex Beta 0.1.0 for macOS)

Domain: fonts, advanced font, colours, cursor, keyboard/option key, scrolling/resizing, selection, links, find, context menu, Go to Directory, Insert Path, titles, shell.

Inputs: beta `b04-terminal.md`, `b09-overlays.md` (plus `b05-shortcuts.md`, `b08-menubar.md` for the Edit, View and Find actions); Linux `r01-workspace-settings.md`, `r03-menus-overlays.md`, `c01-settings-themes-terminal.md`, `c03-keys-shortcuts.md`. Screenshots opened side by side: beta 022, 033, 126, 127; Linux 02, 38, 47. Source and `vendor/rexrs` were grepped to confirm what exists.

Screenshot references: beta numbers are `rex-beta-ui-2026-11-10/screenshots/NNN-*`; Linux numbers are `rex-rune-2026-11-11/screenshots/NN-*`. Line numbers are from the repo at HEAD ad196ff.

## 1. Summary

Linux covers the terminal engine well (Ghostty decoding, exact cell grid, box-drawing sprites, native selection gestures, OSC 8 and URL links, incremental scrollback search, clipboard and upload), and in places is ahead of the beta (per-host default shell, "No matches" status, link tooltip, Shift+drag override, bundled font fallback chain). Its *settings* surface for the terminal is far thinner. The Terminal page has five rows (font family form, font size, pane padding, read-only fallback fonts, terminal theme) plus per-host shell rows; the beta has about 40 controls in 11 sections: font style/weight/line height, a typography preview, advanced font (character spacing, ligatures, bold/italic rendering, smoothing, fallback list), colour legibility switches, cursor style/blink/force, option key, smooth scrolling and resizing, selection colours and copy-on-select, link policy, path browser, generated titles and startup command. The pane right-click menu lacks Copy, Paste, Select All, Reset Terminal, Clear and Split Left/Up. Go to Directory and Insert Path do not exist, although the vendored server API already has `terminal.list_dir`, `terminal.pwd`, `terminal.reset`, `terminal.clear` and `terminal.write`, so most of this is client-only work. Find works (Enter and Shift+Enter step, Esc closes) but has no previous/next or close buttons and its bar covers live terminal text (screenshots 46 to 48). Linux has no terminal accessibility (README line 495), whereas the beta exposes the whole screen as a read-only text field (T87).

Three clusters of gaps are blocked by product rules rather than effort: cursor blink, smooth scrolling and the copy animation ("terminal content never animates"); the contrast/palette-forcing switches ("client never recolours terminal output"); and the exact-grid model that rules out ligatures. Those need an explicit product decision before any work. A fourth cluster is not applicable on Linux: Option key, Display P3, macOS font smoothing, AutoFill, Quick Look.

Overall, of the 89 rows below (after verification and 6 added rows), 8 are at parity, 2 are linux-ahead, 13 are linux-only, 16 are partial, 42 are missing and 8 are not applicable. Most missing items are small (S/M) because the engine already does the rendering and only needs a setting. (The earlier draft said "roughly 15 of about 60 at parity or ahead, 10 partial, about 30 missing"; that did not match its own 83 rows.)

## 2. Feature matrix

Status: parity | partial | missing | linux-ahead | linux-only | not-applicable. Blocker: none | needs-server-api | needs-rune-work | macos-only | product-rule. Effort S (under 1 day), M (1 to 3 days), L (about a week), XL (more). "Beta" cites beta screenshots; "Linux" cites Linux screenshots and file:line.

### 2.1 Settings page structure

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T01 | Terminal page layout: one scrolling page of 11 sections (Typography, Colors, Cursor, Keyboard, Scrolling, Resizing, Selection, Links, Integrated Path Browser, Titles, Shell), each a heading, a rounded card and a grey footnote | 022, 033, 037 | Flat list of 5 rows + DEFAULT SHELL inside the 760x520 modal, no cards or headings (02, 14; `settings_view.rs:274-373`) | partial | none | M | high |
| T02 | Settings search ("Search Settings") | 022 | none (c01 section 7) | missing | none | M | medium |
| T03 | Back/Forward history and sub-pages (Advanced Font Settings opens in-page) | 025 | Pickers and forms replace the panel (09, 10, 11, 12) | missing | none | M | low |
| T04 | Controls are live and previewed in the page (typography, cursor, selection previews) | 022, 033 | none; font family needs relaunch (`settings_view.rs:275-293`) | missing | none | M | medium |

### 2.2 Typography and fonts

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T05 | Terminal font: current family shown in its own face + "Choose..." | 022 (`Menlo`, button) | "Font family" row, active family in helper text, "Change..." opens free-text form (02, 10; `settings_view.rs:275-293`) | partial | none | S | medium |
| T06 | Font picker: "Search fonts", Monospaced / All Fonts segmented, alphabetical list, check on current, scroll to selection, Nerd Fonts included | 023, 024 | none; free text, no list, no validation (10) | missing | none (fontconfig `fc-list :spacing=mono`) | M | high |
| T07 | Apply font family live (no relaunch) | implied by 022 | "Changes apply when you reopen the client" (`fonts.rs` loads once; c01 4.2) | missing | needs-rune-work (font registry is built at startup) | L | medium |
| T08 | Font style popup: Regular, Italic, Bold, Bold Italic | 022, 029 | none; regular/bold/italic/bold-italic faces resolved by `fc-match` (`fonts.rs:50-82`) | missing | none | M | low |
| T09 | Font size stepper in pt (beta default 12, range not shown) | 022 (`Font size`, AX textfield) | stepper, 8 to 48 pt, default 15, live (02; `settings_view.rs:295-304`, `workspace.rs:2012-2022`) | parity | none | S | high |
| T10 | Font weight field, disabled "fixed by the font" unless variable | 022 | none | missing | needs-rune-work (variable font axes) | M | low |
| T11 | Line height adjustment, % (0 = font default) | 022 | fixed 1.4 x size (`term.rs:156-162`) | missing | none | S | medium |
| T12 | Typography preview well (regular, bold, italic sample, branch glyph) | 022, 025 | none | missing | none | S | medium |
| T13 | "Advanced font settings" navigation row | 022, 025 | none | missing | none | S | low |
| T14 | Character spacing stepper + unit px / % | 025, 026 | none | missing | none (cell width = advance + delta in `term.rs:152-166`) | M | low |
| T15 | Ligatures switch (default On) | 025 | permanently off: `'liga' 0, 'calt' 0, 'clig' 0, 'kern' 0` (`term.rs:152`, DESIGN.md "Typography") | missing | product-rule (CORRECTED from needs-rune-work: DESIGN.md line 53 states "ligatures and kerning are off so the grid stays exact"; also rows are placed per cell, so shaping across cells conflicts with the grid) | L | medium |
| T16 | Render bold and italic: Both / Bold only / Italic only / Neither | 025, 027 | bold and italic always rendered via faces (`term.rs:547-568`) | missing | none | S | low |
| T17 | Font smoothing switch | 025 | none | not-applicable | macos-only (fontconfig/Rune decide antialiasing; "Native Rune look" rule) | S | low |
| T18 | Fallback fonts: ordered list with "Add..." picker (opens on All Fonts) | 025, 028 | comma-separated text in the Fonts form; settings row is read-only "Automatic" (02, 10; `settings_view.rs:323-328`) | partial | none | M | medium |
| T19 | `--font`, `--font-fallback`, `--font-size` launch overrides | none | `connection.rs:43-83` | linux-only | none | S | low |
| T20 | Missing primary font falls back to bundled IBM Plex Mono with a notice | none | `fonts.rs:84-136`, helper text `settings_view.rs:275-286` | linux-only | none | S | low |

### 2.3 Colours

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T21 | Improve text contrast (default On): "Adjust unreadable text against its background." | 022 | none (c01 section 7: minimum contrast absent) | missing | product-rule (client never recolours terminal output; needs an explicit render-time exception like cursor-text) | M | medium |
| T22 | Force theme palette for text: "Map text colors to the nearest theme color." | 022 | none | missing | product-rule | M | low |
| T23 | Use Display P3 colour space | 022 | none | not-applicable | macos-only (no colour-managed wide gamut path in Rune/Wayland) | S | low |
| T24 | Terminal theme (lives on the Themes page in the beta) | b03 (other domain) | "Theme" row + searchable picker with Server/Built in/Dark/Light groups (02, 11, 13; `settings_view.rs:330-346`) | linux-only | none | S | medium |

### 2.4 Cursor

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T25 | Cursor preview well (`~/rex $` plus live cursor) | 022, 023 | none | missing | none | S | low |
| T26 | Cursor style: Block / Bar / Underline | 022, 030 | follows the program's DECSCUSR only; hollow when unfocused (`term.rs:1115-1130`) | missing | none (local draw choice, like cursor-text) | S | medium |
| T27 | Blink speed: Off / Slow / Normal / Fast / Custom (+ hidden Blink interval row) | 022, 031 | steady cursor; blink requests ignored (`terminal.rs:403`) | missing | product-rule (terminal content never animates) | M | low |
| T28 | Force style: "Ignore application requests to change cursor shape." | 022 | none | missing | none | S | low |
| T29 | Hollow outline cursor when pane is unfocused / not the keyboard owner | not captured | `term.rs:1115-1130` | linux-only | none | S | low |
| T30 | IME preedit at the cursor | n/a (native text input) | `term.rs:1131-1160`, `input.rs:290-312` | not-applicable | none | S | low |

### 2.5 Keyboard

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T31 | Show key sequence hints switch | 022 | pending-sequence chip always shown in toolbar (`toolbar.rs:300`); no switch | partial | none | S | low |
| T32 | Option key: Automatic / Composes characters / Sends Alt / Left sends Alt / Right sends Alt | 022, 032 | Alt always goes to `client.key` as `alt` with text kept (`keys.rs:97-112`); AltGr/dead keys compose via the layout | not-applicable | macos-only (Linux equivalent is already Alt = Meta; compose is a separate key. At most a README note.) | S | low |
| T33 | Font-size shortcuts: Increase (Cmd+= and Cmd++), Decrease (Cmd+-), Reset (Cmd+0) | b05 View card, b08 074 | `Ctrl =` / `Ctrl -` / `Ctrl 0` scale the whole UI (70 to 150%), not the terminal font (`keymap.rs:390-397`) | partial | none | S | medium |
| T34 | Send Key presets (Cmd+Backspace as ctrl+u, Cmd+arrows as ctrl+a/e, Option+arrows as alt+b/f) | Shortcuts 048 (`Send Key` rows: Command Delete = `key: ctrl+u`, Command Left/Right Arrow = `key: ctrl+a`/`ctrl+e`, Option Left/Right Arrow = `key: alt+b`/`alt+f`) | `pane.send_key` action exists, no default presets (`actions.rs:94`); not needed (Ctrl/Alt chords reach programs natively) | not-applicable (CORRECTED from parity: Linux ships no presets and does not need them; only the mechanism matches) | none | S | low |
| T35 | Jump to Previous / Next Prompt (Cmd+Up / Cmd+Down) | Shortcuts 047, 048 (`Jump to Previous Prompt` = Command Up Arrow, `Jump to Next Prompt` = Command Down Arrow) | none; scrolling is by lines (`keymap.rs:410`) | missing | unknown (CORRECTED from needs-server-api: the client decodes a local Ghostty terminal and `vendor/ghostty-vt/src/screen.rs:314,380` exposes row `semantic_prompt` and cell `semantic_content` from OSC 133, and `selection.rs:265` has `select_output`; so this is probably client-only if the snapshot carries prompt marks. Not verified against a live server.) | M | medium |
| T36 | Scroll Page Up / Down | b05 (unbound) | `Shift PageUp/PageDown`, 20 lines (`keymap.rs:410`) | parity | none | S | low |

### 2.6 Scrolling and resizing

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T37 | Smooth scrolling (pixel scrollback), default Off | 033 | line accumulation only (`panes.rs:473-492`) | missing | product-rule (no terminal motion) | M | low |
| T38 | Smooth terminal resizing (reflow on the resize frame, default On) | 033 | server owns size; display crops locally, never resizes (`terminal.rs:310-327`) | missing | product-rule (size ownership is automatic and server-driven) | L | low |

### 2.7 Selection and clipboard

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T39 | Selection preview well (`On branch feature/tabs`) | 033, 037 | none | missing | none | S | low |
| T40 | Copy on selection switch (default Off) | 033 | none; copy only via `Ctrl Shift C` or server `selection_copied` events (`clipboard.rs`, c03 "Absent") | missing | none | S | high |
| T41 | Play animation on copy (default On) | 033 | none | missing | product-rule (terminal content never animates) | S | low |
| T42 | Selection foreground: Theme / Cell Foreground / Cell Background | 033, 034 | declared theme `selection-foreground` only, else cell text unchanged (`term.rs:798-811`) | partial | none | S | low |
| T43 | Selection background: Theme / Cell Foreground / Cell Background | 033, 035 | declared theme `selection-background`, else palette `terminal_selection` (`term.rs:998-1002`) | partial | none | S | low |
| T44 | Rounded selection corners | 033 | square per-row rectangles (confirmed in code, `term.rs:768-774` fills a plain `el()` per row with no radius; no screenshot shows a selection) | missing | none | S | low |
| T45 | Word and line selection (double and triple click), content-pinned selection, edge autoscroll | not captured | `selection.rs:101-195`, `term.rs:415-423` | linux-only | none | S | low |
| T46 | Shift+drag selects locally in mouse-tracking TUIs | not captured | `panes.rs:329-335`; also right-click opens the pane menu even over a mouse-tracking program and Shift+right-click goes to the program (`panes.rs:451-475`) | linux-only | none | S | low |
| T47 | Copy: Edit > Copy (Cmd+C, disabled with no selection) | 073, 094 | `Ctrl Shift C`, no menu item (`panel.rs:717-726`) | partial | none | S | high |
| T48 | Paste: Edit > Paste (Cmd+V) | 073, 094 | `Ctrl Shift V` (also files and images upload), no menu item | partial | none | S | high |
| T49 | Select All in Pane (Cmd+A) | 073 (Edit > Select All, enabled), Shortcuts 046 to 048 (`Select All in Pane`, unbound) | none | missing | none (CORRECTED: no vendor patch needed. `vendor/ghostty-vt/src/selection.rs:236` already has `Terminal::select_all`; it returns a snapshot that must be installed with `set_selection` (line 212) through the selection controller in `src/terminal/selection.rs`) | S | medium |
| T50 | Primary selection (select to copy, middle-click to paste) | n/a (macOS has none) | absent; middle button only reaches a tracking program (`panes.rs:306-309`) | missing | none | M | medium |
| T51 | OSC 52 writes, server-driven "selection copied" | not captured | `clipboard.rs:22,29`, 8 MiB cap, focused pane only | linux-only | none | S | low |

### 2.8 Links

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T52 | Web, mail and `file:` links open directly | 037 footnote | http, https, ftp, mailto only; `file:` refused on purpose, because the output may come from a remote host and `file:` would name a path on this machine (`links.rs:5-9, 89-95`; a test feeds `file:///etc/passwd`, line 219) | partial | product-rule (CORRECTED from none: the refusal is a documented security decision; work item 9 needs an explicit decision, not just effort) | M | medium |
| T53 | "Links that open apps": Ask First / Always Open / Never Open for other schemes (ssh://, ...) | 036, 037 | all other schemes refused, no setting | missing | none | M | low |
| T54 | Link affordance: hover underline, hand cursor, tooltip with the real target | not captured | Ctrl held: underline, hand, tooltip "Ctrl+click to open <URL>" (`term.rs:1164-1212`) | linux-only | none | S | low |
| T55 | OSC 8 links and wrapped plain-text URLs | not captured | `links.rs:20-160` | linux-only | none | S | low |

### 2.9 Find

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T56 | Find bar: pill 300x39 top-right, "Find" placeholder, count `1/1`, up/down buttons, close x | 125, 126 (pixel-checked: pill about 300x40 with hairline, placeholder `Find`, `1/1`, divider, up and down chevrons dim when empty, `x`; its AX tree is not captured because the bar is an overlay) | 280x34 field, magnifier, "Search terminal", trailing `esc` hint when empty / `N/M` / "No matches", no buttons, no close button; Esc does close it and clears highlights (`input.rs:176-189`, `workspace.rs:949`) (46, 47, 48; `panes.rs:552-585`) | partial | none | S | medium |
| T57 | Open find: Cmd+F | b05 050 | `Ctrl Shift F` (`keymap.rs:388`) | parity | none | S | high |
| T58 | Find Next / Find Previous as actions (Cmd+G, Shift+Cmd+G; disabled until a search runs) | 077 (Find submenu: `Find…`, `Find Next` disabled, `Find Previous` disabled); Shortcuts 048 to 050: Find… = Command F, Find Next = Command G, Find Previous = Shift Command G. Note the beta binds Shift+Cmd+G to both Find Previous and Go to Directory… (045 to 050), a conflict in the beta itself | Enter (next older match) / Shift+Enter (next newer) inside the field only (`input.rs:176-189`); no bindable action (`actions.rs` SPECS has only `client.search`) | partial | none | S | medium |
| T59 | Prev/next disabled until there are matches | 125 | no buttons | missing | none | S | low |
| T60 | Dim terminal and yellow `#F4D722` rounded match highlight | 126 | accent overlay at 25% (all) and 55% (current), matches hidden behind the bar (47; `term.rs:761-767`) | partial | product-rule for dimming (local recolour); none for the highlight | S | medium |
| T61 | "No matches" status | not captured (125 is an empty query, 126 has results; the beta's no-result state was never captured) | "No matches" in the field (48; `panes.rs:558`) | linux-only (CORRECTED from linux-ahead: the beta's behaviour is unobserved, so "ahead" is unsupported) | none | S | low |
| T62 | Search is incremental, covers scrollback, survives reconnect | implied only (the beta's incremental and scrollback behaviour was not captured) | `terminal/search.rs:1-165`; re-searches after a fresh snapshot (line 79); one test is `#[ignore]`d because Ghostty's match order on a restored, resized terminal differs (line 307) | parity | none | S | medium |

### 2.10 Context menu and terminal commands

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T63 | Copy (disabled without selection) | 094 (`Copy`, disabled) | missing (38); same capability as T47, counted twice (menu bar vs context menu) | missing | none | S | high |
| T64 | Paste | 094 | missing (38); same capability as T48 | missing | none | S | high |
| T65 | Split Right, Split Down | 094 | Split right, Split down with shortcut hints (38; `panel.rs:717-726`) | parity | none | S | high |
| T66 | Split Left, Split Up | 094 | missing | missing | none (`session.new_split` takes a top-level `side` of `before` or `after`, `vendor/rexrs/src/api/generated/session.rs` around line 2406; CORRECTED: the earlier cite said `before`/`after` in `layout`, which are the child slots of a `split` node. Linux only ever sends `side: after`, `panel.rs:717`, `workspace_probe.rs:144`) | S | low |
| T67 | Float Pane | 094 | missing; rexrs has `session.float_block` | missing | needs-rune-work (floating layer rendering; belongs to the panes domain) | L | low |
| T68 | Zoom Pane | 094 | Zoom pane / Restore pane (38) | parity | none | S | medium |
| T69 | Close Pane | 094 | Close pane... with confirm (38, 40) | parity | none | S | high |
| T70 | Reset Terminal | 094 | missing; `terminal.reset` exists, unused in `src` | missing | none | S | medium |
| T71 | Clear Terminal (unbound action in the beta) | b05 049 | missing; `terminal.clear` exists, unused | missing | none | S | medium |
| T72 | AutoFill submenu (Contact, Passwords, Credit Card) | 094, 095 | none | not-applicable | macos-only (a Linux equivalent would be IME/emoji pickers supplied by the compositor; nothing to build) | S | low |
| T73 | Rename pane, Reconnect terminal in menu | not in the 094 menu; the beta does have `Rename Pane` as a shortcut-bindable action (Shortcuts 049, unbound) so Rename pane is not Linux-only | 38 | linux-only (Reconnect terminal only; Rename pane is partial) | none | S | low |
| T74 | Menu header naming the target, shortcut hints | none (no hints in 094) | header `Shell . shuv@shuvbot:~`, hints (38) | linux-ahead | none | S | low |
| T75 | Edit menu entries reachable without a menu bar (Select All, Insert Path, Find) | 073, 077 | Commands palette lists only Search terminal among terminal-text verbs (41); `pane.copy` and `pane.paste` exist as bindable actions (`actions.rs:85-86`) but are not palette entries | partial | none | S | medium |

### 2.11 Path browser, titles, shell

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T76 | Go to Directory... overlay (Shift+Cmd+G): path field with clear button, "Go to" header, folder rows, typed directory first with home badge | 127 | none | missing | none (`terminal.list_dir` with `prefix`, `terminal.pwd`, `terminal.write`; only when the foreground is a shell) | L | medium |
| T77 | Insert Path... overlay (Ctrl+Cmd+I): same browser, placeholder `~/`, types the path at the cursor | 128 | none | missing | none (`list_dir` with `files: true`, then paste; no Enter) | M (after T76) | medium |
| T78 | Show recently used paths (shared per host) | 037 | none | missing | none (client-side store per host in `workspace.json`) | S | low |
| T79 | Shortcut badge on the Integrated Path Browser heading | 037 | none | missing | none | S | low |
| T80 | Prefer generated titles (program + directory instead of shell-set titles) | 037 | tab label shows the terminal title (00); a "Shell" tab uses the focused pane title (DESIGN.md Layout) | missing | none (`terminal.process`, `terminal.pwd`, `activity.rs:232`) | M | medium |
| T81 | Shell startup command (global, local only, quoted args, placeholder "Default login shell") | 037 | per-host "Default shell" rows via a form (02, 12; `settings_view.rs:347-364`) | linux-ahead | none (beta is local-only; Linux covers every host) | S | medium |
| T82 | New tabs and splits inherit source working directory | 037 footnote | `connection.rs:405` | parity | none | S | high |

### 2.12 Rendering fidelity (not in beta captures)

| ID | Capability | Beta | Linux | Status | Blocker | Effort | Impact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T83 | Box-drawing sprites, curly/dotted underlines, Kitty graphics (static), mouse reporting | not captured | `sprites.rs`, `term.rs:855-927`, `graphics.rs` | linux-only | none | S | medium |
| T84 | Pane padding (ADDED, omitted by the first draft although the summary counts it) | Customize Panes page 011: separate `Horizontal padding in points` (10) and `Vertical padding in points` (6) steppers plus "Extends background colors into the padding area." (another domain, see d2) | one `Pane padding` stepper, 0 to 32 pt, default 8, on the Terminal page (02, 14; `settings_view.rs:305-322`) | partial | none | S | low |
| T85 | Quick Look of a path (ADDED) | Shortcuts 048: `Quick Look` = Command Y | none | not-applicable | macos-only (Quick Look is a macOS service; only meaningful once Go to Directory / Insert Path exist) | S | low |
| T86 | Copy from Pane / Paste into Pane / Copy Pane ID actions (ADDED) | Shortcuts 046 to 049: `Copy from Pane`, `Paste into Pane`, `Copy Pane ID`, all unbound by default | `pane.copy` ("Copy selection") and `pane.paste` actions with `Ctrl Shift C` / `Ctrl Shift V` (`actions.rs:85-86`, `keymap.rs:386-387`); no Copy Pane ID action | partial | none | S | low |
| T87 | Terminal content exposed to assistive technology (ADDED) | 125 to 128 AX tree: the pane is a read-only `textfield` named `Terminal` whose value is the whole screen text | none; README line 495 says accessibility (AT-SPI) is not implemented | missing | needs-rune-work (Rune's default `accessibility` feature is on per the user's note, but nothing exposes the cell grid as text) | L | medium |
| T88 | Edit menu items that are macOS system services: Undo, Redo, Cut, Delete, Writing Tools, Start Dictation, Emoji & Symbols (ADDED) | 073, 078 | none | not-applicable | macos-only (a Linux compositor or IME supplies emoji and dictation) | S | low |
| T89 | Drop or paste files and images onto a pane to upload them to the terminal's machine (ADDED, Linux-only) | not captured | `upload.rs`, `app/upload.rs:36` (`drop_files`), paste handling noted in T48 | linux-only | none | S | medium |

## 3. Beta UI specification

All values are in points (1x), dark appearance, from `b04-terminal.md` and `b09-overlays.md`, cross-checked against 022, 033, 126, 127.

### 3.1 Settings window frame (for context)

- Window 816x850, large corner radius (~16), soft shadow. Sidebar 214 wide, fill `#0F1217`. Content fill `#0B0F13`. Sidebar row "Terminal" (icon: dark green square with green `>_`) selected: `#191D22`, ~8 radius, semibold.
- Header (y 0 to 52): glass capsule ~76x36 at (223, 9) with Back (enabled) and Forward (disabled) 38x36 buttons and a thin divider; page title right of it, bold ~15 pt `#DADADB`. Content scrolls under the header with a fade/blur edge. Overlay scrollbar (handle 6 wide, hidden at rest).
- Content column: cards x 235 to 795 (561 wide): 20 left margin from sidebar, 21 right. Section heading at x 245.

### 3.2 Common components

- **Section heading**: bold ~13 pt `#E1EAF1`, 16 line box; 30 below the previous card (or footnote), 10 above its card.
- **Card**: fill `#12161A`, no border, radius ~12. Rows inset 10 left and right. Row heights: 38 to 39 single line, 45 to 46 with helper. Divider 1 px `#26292D`, inset 10 both sides.
- **Row label** ~13 pt regular `#E1EAF1`. **Helper** inside a row ~10.5 pt `#7A8086`, 4 below the label. **Footnote** below a card ~10.5 pt `#7A8086`, 10 below, wraps at ~516.
- **Switch** 36x16: on track `#0676FC`, off `#292D30` (CORRECTED from `#282C2F`; sampled at two points of the 022 Force-theme-palette switch), knob white. Inactive window: on-track `#323639`.
- **Popup button** (right-aligned, 10 from card edge, 24 tall): value text `#DBDBDC` ~13 pt then an ~18 pt circular badge `#24282C` with a stacked up/down chevron.
- **Push button** capsule fill `#22262A`, label `#E1EAF1` ~13 pt, 24 tall (Choose... 78 wide, Add... 55 wide).
- **Stepper field**: ~82x23, fill `#1C1D20`, 1 px border `#26292D`, radius ~6, leading dim glyph, value, up/down arrows, unit label (`pt`, `%`, `px` in `#7A8086`) outside the field. Disabled field: fill `#1C2125`, no border, text `#5B5F64`.
- **Preview well**: inset 6 from card edges, 1 px border `#24282D`, fill `#0F1216` in the interior (CORRECTED from `#0F1217`; `#0F1217` is only the sidebar and picker fill, the well matches the terminal background `#0F1216`), radius ~8, 10 left padding, Menlo 12 in `#E2EAF1`.
- **Text input** (Startup command): 386 wide, ~22 tall, fill `#1C1D20`, 1 px border, radius ~5, placeholder monospaced `#5B5F64`.
- **Keycap badge** (path browser heading): 32x18 outline `#71777D`, 8 right of the heading text.

### 3.3 Terminal page, top to bottom (022, 033, 037)

1. **Typography** heading. Card 1 (169 tall):
   - Row "Terminal font": label left; right: the family name set in itself (`Menlo`), then **Choose...** button.
   - Four-column strip separated by 1 px `#26292D` vertical dividers (at x ~437, 562, 668 window-relative). Column headers ~11 pt `#767C81`: **Font style** (popup field 180x23, fill `#22262A`, radius 6, value `Regular` with chevron), **Font size** (stepper, glyph `AA`, value `12`, unit `pt`), **Weight** (disabled, value `400`, accessibility name "Font weight, fixed by the font"), **Line height** (stepper, line-spacing glyph, value `0`, unit `%`).
   - Preview well 549x58, three lines: `~/rex <branch glyph> main $ swift build && swift test` / `main.swift:4: warning: 'run() -> Bool' is deprecated` / **`Build complete!`** *`(1.24 sec)`*.
   - Card 2 (38 tall, 10 gap): **Advanced font settings** with a dim `>` chevron `#45494E` at the right.
2. **Colors** card (3 rows x 46): **Improve text contrast** (helper "Adjust unreadable text against its background.", On); **Force theme palette for text** ("Map text colors to the nearest theme color.", Off); **Use Display P3 color space** ("Read theme and program colors as wide-gamut Display P3.", Off).
3. **Cursor** card: preview well 549x30 with `~/rex $ ` and a blue block cursor `#0071F6`; **Cursor style** popup `Block` (options Block, Bar, Underline); **Blink speed** popup `Normal` (options Off, separator, Slow, Normal, Fast, separator, Custom); (hidden **Blink interval** row for Custom); **Force style** ("Ignore application requests to change cursor shape.", Off).
4. **Keyboard** card: **Show key sequence hints** (On); **Option key** popup `Automatic (by keyboard layout)` (options Automatic (by keyboard layout), Composes characters, Sends Alt, Left sends Alt, Right sends Alt). Footnote: "Automatic sends Alt on U.S. layouts and types special keys elsewhere. Applies to all terminals on this Mac, including remote hosts."
5. **Scrolling** card: **Smooth scrolling** (Off). Footnote: "Move through terminal history smoothly with a trackpad or Magic Mouse. Programs that handle scrolling themselves, and scrolling while Find is open, still move by whole lines."
6. **Resizing** card: **Smooth terminal resizing** ("Reflow text on the frame a pane changes size, without waiting for the server.", On).
7. **Selection** card: preview well 549x30 (`On branch feature/tabs`, `feature/tabs` drawn inverted `#E2EAF1` fill with `#0F1216` text, rounded); **Copy on selection** (Off); **Play animation on copy** (On); **Foreground** popup `Theme` (Theme, Cell Foreground, Cell Background); **Background** popup `Theme` (same options); **Rounded corners** (On).
8. **Links** card: **Links that open apps** popup `Ask First` (Ask First, Always Open, Never Open). Footnote: "Web, mail, and file links open directly. Other links, such as ssh://, launch the app registered for them, and any program in a terminal can print one."
9. **Integrated Path Browser** heading + keycap `^Cmd I`; card: **Show recently used paths** (On). Footnote: "Insert Path types the path of a file or folder at the cursor. Recently used paths are shared by every session on the same host."
10. **Titles** card: **Prefer generated titles** (On). Footnote: "Show the running program and its directory instead of titles that shells set. Titles that programs set for themselves, and names you give tabs, still appear."
11. **Shell** card: **Startup command** text field, placeholder `Default login shell`. Footnote: "Executable and arguments. Use quotes to group arguments. Applies to new terminals on this Mac. Remote hosts use their own default shell. New tabs and splits use the source terminal's working directory when available. Otherwise, they use the server's default directory."

Bottom padding after the last footnote ~20.

### 3.4 Advanced Font Settings sub-page (025)

No headings; three cards 10 apart, starting 12 below the header. Card 1: typography preview; **Character spacing** (stepper with `Abc` glyph, value `0`, unit popup `px` with options `px`, `%`); **Ligatures** (On); **Render bold and italic** popup `Both` (Both, separator, Bold only, Italic only, separator, Neither). Card 2: **Font smoothing** (Off). Card 3 (45 tall): **Fallback fonts**, helper "Used in order for characters the terminal font doesn't have.", **Add...** button.

### 3.5 Font picker popover (023, 024, 028)

Popover ~287x371, fill `#0F1217`, outer stroke `#020304`, inner rim `#272A2E`, radius ~14, shadow. Top to bottom: search capsule 278x30 fill `#272A2E`, placeholder "Search fonts", focused with blue caret `#0073FF`; segmented control 278x24 (track `#212429`, selected thumb `#3E4248` with dark 1 px outline) labelled **Monospaced** | **All Fonts**; hairline `#202327`; list with 28 pt rows, names in the system UI font (not previewed), 13 pt `#E1EAF1`, 15 left inset. Selected row: fill `#0055BF`, radius ~8, inset 4, white text, check at right (no check on the Add... picker's default highlight). The terminal-font picker opens on Monospaced (Andale Mono, Courier New, Hack Nerd Font Mono, Menlo, MesloLG* Nerd Font Mono, Monaco...); the fallback picker opens on All Fonts. The Settings window goes inactive while it is open.

### 3.6 Floating panel family (find, go to, insert path)

Shared style: 1 px outer hairline `#020203`, 1 px inner highlight `#333436`, body `#1C1D20`, radius 10 to 12 (find bar fully rounded, ~19), soft shadow, opaque. Search field fill `#333437`, 30 tall, radius ~8, grey magnifier, placeholder `#8F8F91`, blue caret. Rows 28 pitch, 13 pt SF `#ECECEC`; selected row pill `#0055BF`, radius 8, inset 5. Section header 11 to 12 pt semibold grey. Dividers `#2A2B2D`.

### 3.7 Find bar (125, 126)

~300x39 pill at the pane's top-right, 10 inside the right edge, 5 below the pane header. Left to right: text field (placeholder `Find`, no magnifier, caret blue), match counter `1/1` grey (only when there are results), 1 px divider `#2B2D30`, previous `^` and next `v` chevron buttons (disabled/dim until matches), close `x` `#64696E`. With results the terminal dims from `#0F1216` to `#080A0C` with dimmer text, and the match is drawn as a yellow `#F4D722` rounded rectangle (radius ~3) with dark text. Edit > Find submenu: Find... Cmd+F; Find Next Cmd+G (disabled until active); Find Previous (disabled; the Shortcuts page binds Shift+Cmd+G).

### 3.8 Go to Directory / Insert Path (127, 128)

Panel 330x501 centred, ~29 below the toolbar. Path field with magnifier; Go to Directory shows `/Users/shuv/` with a circular clear button (`x` in a grey circle) at the right; Insert Path shows placeholder `~/` and no clear button. Header `Go to` (the same header in both). Rows 28 pitch: blue Finder folder icon (~16x13, `#4FBCED`), name at x 580. First row is the typed directory itself (home folder badge) and selected; children alphabetical, case-insensitive. No shortcut hints, no chevrons.

### 3.9 Terminal context menu (094, 095)

Native menu 132 wide, items 24, separators 11, no shortcut hints. Items: Copy (disabled without selection); Paste; --; Split Right; Split Left; Split Down; Split Up; Float Pane; Zoom Pane; --; Close Pane; Reset Terminal; --; AutoFill > (Contact..., Passwords..., Credit Card...). System accent highlight `#0069FF`.

## 4. UI quality deltas

Numbered; each states the difference, evidence and a Linux fix in Rune terms. "Kit" means the rune-kit widgets pinned in `theme.rs`.

1. **No grouping or cards on the Terminal page.** Beta groups settings into heading, card and footnote; Linux is one flat list: the first five rows do have hairline dividers (`divider()` calls inside `terminal()` between the first five rows, visible in 02 and 14), but the per-host rows under the DEFAULT SHELL caps header have none (14), and there are no cards, section headings or footnotes (beta 022 vs Linux 02, 14; r01 polish notes). CORRECTED: the first draft said there were no dividers between them. Fix: in `src/app/settings_view.rs` `terminal()` (lines 274-373) build each section as a `section(title, rows, footnote)` helper: 10 pt heading in `INK`, card using the existing card surface from `theme.rs` (radius 12, 1 px row dividers through the existing `divider()`, which must also be added between the per-host shell rows), footnote in `MUTED` 10.5 pt. Reuse across Interface and Notifications so dividers are consistent.
2. **Helper text carries what should be a footnote.** The terminal Theme helper is a four-line paragraph (02), the beta puts such prose below the card (033 Keyboard, Scrolling; 037 Links, Shell). Fix: shorten each Linux helper to one line and move explanation to a footnote under the card (`settings_view.rs`, new footnote element).
3. **Font choice is a text field, not a picker.** Beta 023/024 give a searchable, segmented list with the current font checked; Linux 10 is a free-text form with no validation. Fix: new `Panel::FontPicker` in `src/app/panel.rs` modelled on the existing terminal theme picker (11): search field (the picker already has a focused field with `esc` hint), a two-segment control Monospaced | All fonts (kit segmented control already used for density), rows with check in the left gutter, rows 28 pt like the Linux theme picker. List from `fc-list : family spacing` (cache once). Show the chosen family set in its own face in the Terminal page row (`settings_view.rs`, use the resolved family with the font slot).
4. **No live preview anywhere in Terminal settings.** Beta shows typography, cursor and selection preview wells that react to the controls (022, 033). Linux has none, and its font change only applies after relaunch (02). Fix: add a `preview_well()` widget in `src/app/widgets.rs` (inset 6, 1 px border, radius 8, terminal font via the terminal font slot, terminal background token); use it for typography, cursor and selection. For live size/line-height this needs `term.rs` cell metrics to read settings.
5. **Controls vocabulary mismatch.** Beta uses a popup button with a value + chevron badge for every enumeration, steppers as bordered fields with a unit outside, and switches; Linux uses `Change...` pills that open a replacement picker, round -/+ steppers, and segmented controls only for two-way choices (02, 01). Fix: for 3+ option enumerations (cursor style, selection colours, link policy, bold/italic render) use the kit's popup/select if it exists in `widgets.rs`, else a `Pill` with trailing chevron opening a small anchored list popup (`popups.rs`), anchored to the button, not top-centre (r01 cross-cutting 4).
6. **Pickers and forms replace Settings and detach from their button.** Beta pickers are popovers anchored to Choose...; Linux 09 to 12 replace the panel (CORRECTED detail: the pickers, 09, 11 and 13, are top-centre list popups; the font and shell forms, 10 and 12, are small centred dialogs). Fix: stack the font and theme pickers over Settings (anchor to the pressed control's rect) in `popups.rs`/`panel.rs`; keep the Settings panel visible but dimmed.
7. **Find bar covers the text it searches and has no prev/next.** Beta 126 puts a 300x39 pill below the pane header with prev/next chevrons, counter and a close button; Linux 46/47 pins a 280x34 field 6 pt in the corner over the first prompt row, so the matches on the right prompt are hidden. Fix: in `src/app/panes.rs` `search_field()` (552-585) add up/down icon buttons (existing 20x20 stroke icons) disabled while `total == 0`, a divider, a close button, and keep the counter; when matches fall under the bar rect, offset the bar to the opposite corner or shift by one row, and reserve the first row in compact single pane layout.
8. **Match highlight is weak.** Beta uses a high-contrast yellow rounded rectangle (126); Linux uses accent at 25 and 55% (47), which on Midnight is barely distinguishable from selection (`#2f4a6e`). Fix: raise alpha for the current match (e.g. 80%) and use `WARNING` as the current-match colour; add a small radius to match rects in `term.rs:769-776`. This keeps the overlay model and does not recolour cells.
9. **Terminal context menu lacks the basic verbs.** Beta 094 starts with Copy and Paste and has separators and a Reset item; Linux 38 has only layout verbs, and the shortcut column and chevron column misalign (r03). Fix: in `panel.rs:717-726` add Copy (disabled with no selection), Paste, Select All, then a separator group of Split right / Split left / Split down / Split up / Zoom, then Rename, Reconnect, Reset terminal, Clear terminal, then Close pane... in `DANGER` text. Add a divider entry kind (`Entry::separator`) and right-align hints and chevrons in a single trailing column.
10. **Destructive entries are unstyled and ungrouped.** Beta separates Close Pane with a separator; Linux puts Close pane... directly under Reconnect (38). Fix: as 9, plus `DANGER` ink on destructive entries in menus and Commands.
11. **Search/path overlays differ in selection styling.** Beta uses an unmistakable `#0055BF` pill with white text (127); Linux selection fill is `#222A38` on `#0C1524` (11, 43), a barely visible neutral. Fix: bump the keyboard-selected row to `ACCENT` at ~18% plus left accent bar or accent outline in `theme.rs` (kit selection token); DESIGN.md already says keyboard selection must never be fill alone.
12. **No Go to Directory / Insert Path surface.** Beta has a complete directory browser (127/128). Fix: new `Panel::PathBrowser { mode }` in `panel.rs` reusing the Commands list popup (search field, 28 pt rows) with folder icons added to `theme.rs`, a clear button in the field and a `Go to` caps header (the Linux caps header style already exists). First row = typed directory.
13. **Settings page is not scannable from the keyboard or by search.** Beta has Search Settings (022). Fix: add a filter field above the section list in `settings_view.rs` that filters rows across sections by label and helper (rows already carry label + helper strings via `setting()`).
14. **Value readouts are inconsistent.** Linux right-aligns "15 pt", "8 pt", "Automatic" and "Follow server" at different x (02); beta aligns all trailing controls 10 pt from the card edge (022). Fix: give `setting()` (`settings_view.rs:14-35`) a fixed trailing column width and right-align inside it.
15. **Read-only value row with hidden editor.** Fallback fonts shows "Automatic" with no control (02) while the editor is under Font family. Fix: add an `Edit...` pill on the row that opens the same form or, better, an ordered list editor (T18).
16. **Copy and wording.** Beta rows are verbs and nouns in title case with helpers in sentence case; Linux mixes "Change..." everywhere so the button says nothing about what changes. Fix: name buttons by object where the row is a picker (`Choose font...`, `Choose theme...`) in `settings_view.rs`.
17. **Cursor preview impossible without motion.** The beta preview blinks (022 vs 023 differ). Linux must not animate terminal content: draw the preview cursor steadily (DESIGN.md), and omit the blink setting unless the product rule changes.
18. **Footer path and bookkeeping rows.** Linux shows "Saved in /home/shuv/.config/rex-linux/workspace.json" in the Terminal page (14). Fine as a developer aid, but beta has no such row on this page; move to About or Advanced (`settings_view.rs:365-372`).

## 5. Recommended work items (ordered)

Order puts cheap, high-impact fixes first, then the settings surface, then features that depend on new surfaces, then the decision-gated items.

1. **Pane context menu verbs and structure** (T47, T48, T63 to T66, T70, T71, T75; deltas 9, 10)
   - Scope: add Copy (disabled without selection), Paste, Select All (after item 5), Split left/up, Reset terminal (`terminal.reset`), Clear terminal (`terminal.clear`); separators; destructive styling; aligned trailing column. Mirror Reset/Clear/Copy/Paste/Find next/Find previous as Commands entries and actions (`pane.reset`, `pane.clear`, `client.search.next`, `client.search.previous`).
   - Files: `src/app/panel.rs` (717-726), `src/app/actions.rs` (65-98, `Spec::remote()` default deny, update the pinned test), `src/app/workspace.rs`, `src/connection.rs` (Command::Reset, Command::Clear), `src/keymap.rs` only if defaults are added.
   - Dependencies: none. Effort S to M.
   - Acceptance: right-click shows Copy disabled with no selection and enabled after dragging text; Paste inserts clipboard text; Reset clears screen and scrollback with the program still running (verified against a disposable server); `cargo test --locked --all-features` passes with the remote allow-set test unchanged (new actions are `Never`); `--snap` shows separators and a red Close pane.

2. **Find bar polish** (T56, T58, T59, T60; deltas 7, 8)
   - Scope: prev/next/close buttons, disabled state, collision avoidance, stronger match colours, bindable next/previous actions (default `F3` / `Shift F3` or `Ctrl Shift G`).
   - Files: `src/app/panes.rs` (552-585), `src/app/term.rs` (769-776), `src/app/actions.rs`, `src/keymap.rs`, `src/terminal/search.rs` (API for next/previous).
   - Dependencies: none. Effort S to M.
   - Acceptance: with query `shuv` in the right prompt (screenshot 47 state) the match is visible and not under the bar; arrows step through matches and are disabled with "No matches"; Enter/Shift+Enter still work.

3. **Terminal settings page restructure into cards, footnotes, previews** (T01, T04, T12, T25, T39; deltas 1, 2, 4, 14, 16, 18)
   - Scope: `section()` helper, preview wells, trailing column alignment, shorter helpers, move the config path footer.
   - Files: `src/app/settings_view.rs`, `src/app/widgets.rs`, `src/app/theme.rs` (preview well tokens), `src/app/tests.rs`.
   - Dependencies: none (other items add rows to it). Effort M.
   - Acceptance: `--snap` of Settings > Terminal shows headed cards with dividers and footnotes, every trailing control aligned; headless test confirms section order.

4. **Selection: copy on selection, foreground/background modes, rounded corners, preview** (T40, T42, T43, T44, T39; plus T50 primary selection)
   - Scope: settings `copy_on_selection` (default false), `selection_foreground`/`selection_background` enums (Theme, Cell Foreground, Cell Background), `selection_rounded` (default true). Copy on selection writes to the clipboard on mouse release only for a user-made selection, keeps the size-ownership rule (selecting must not claim ownership). Optional `primary_selection` writes the Wayland/X11 primary selection and enables middle-click paste in `panes.rs` (only when the program is not tracking the mouse).
   - Files: `src/settings.rs`, `src/app/settings_view.rs`, `src/app/term.rs` (798-811, 998-1002), `src/app/panes.rs` (306-309), `src/terminal/selection.rs`.
   - Dependencies: item 3 for the card. Effort M.
   - Acceptance: with copy-on-select on, drag-select then paste in another app yields the text; with Cell Foreground background the selection inverts; tests for settings round-trip and defaults (old files load).
   - Skip "Play animation on copy" unless the product rule is relaxed (T41).

5. **Font picker, style, typography preview, fallback list** (T05, T06, T08, T12, T18; deltas 3, 6, 15)
   - Scope: fontconfig enumeration cache, `Panel::FontPicker` (search, Monospaced | All, check), Terminal font row shows family in its own face, Font style popup, ordered fallback list with Add... (picker opens on All Fonts) and remove/reorder, typography preview.
   - Files: `src/app/fonts.rs`, `src/app/panel.rs`, `src/app/popups.rs`, `src/app/settings_view.rs`, `src/settings.rs`.
   - Dependencies: item 3. Effort M.
   - Acceptance: picking "Hack Nerd Font Mono" saves it and the helper reads "applies when you reopen"; picker lists only mono families on the Monospaced segment; fallback order is saved and `--font-fallback` still overrides; screenshot compared with beta 023/028.

6. **Line height, character spacing, render bold/italic** (T11, T13, T14, T16)
   - Scope: settings `line_height_pct`, `char_spacing` (px or %), `bold_italic` (Both/Bold only/Italic only/Neither); cell metrics in `term.rs` (156-166) and sprites must scale with cell size; "Advanced font settings" sub-page.
   - Files: `src/app/term.rs`, `src/app/sprites.rs`, `src/app/settings_view.rs`, `src/settings.rs`.
   - Dependencies: items 3 and 5. Effort M to L (PTY size depends on cell metrics, so verify resize reporting).
   - Acceptance: line height +20% grows rows and rows/cols reported to the server change accordingly; sprites still touch cell edges (no seams); tests in `term.rs` for cell metrics.

7. **Cursor settings** (T25, T26, T28, T31)
   - Scope: `cursor_style` (Block/Bar/Underline), `cursor_force` (ignore program DECSCUSR), preview well (steady); sequence-hint switch. Blink stays off.
   - Files: `src/app/term.rs` (1115-1144), `src/settings.rs`, `src/app/settings_view.rs`, `src/app/toolbar.rs` (300).
   - Dependencies: item 3. Effort S.
   - Acceptance: with Bar + Force on, `printf '\e[2 q'` (block) still draws a bar; default behaviour unchanged for existing config files.

8. **Go to Directory and Insert Path with recents** (T76 to T79; deltas 11, 12)
   - Scope: `Panel::PathBrowser`, host command wrappers for `terminal.list_dir` (dirs, optional `files`, `prefix`), Go to Directory writes `cd -- <quoted path>` plus newline only when the foreground process is a shell (use `terminal.program_status` or `process`), Insert Path pastes the quoted path without Enter. Per-host recents (`workspace.json`) with a switch. Actions `pane.goto_directory`, `pane.insert_path`, default keys proposal `Ctrl Shift G` and `Ctrl Alt I` (check against `keymap.rs` conflicts and `Find next` choice in item 2); both added to Commands. Explicit user action only (attachment rule).
   - Files: `src/connection.rs` + `src/connection/` (new host command), `src/app/panel.rs`, `src/app/popups.rs`, `src/app/actions.rs` (remote `Never`), `src/settings.rs`, `src/app/settings_view.rs`.
   - Dependencies: item 3. Effort L.
   - Acceptance: against a disposable server, typing `/tmp/` lists children; Enter on a child changes the shell's directory; Insert Path puts `'/tmp/a b'` at the prompt without executing; list is truncated with the prefix narrowing; no keys are replayed on failure.

9. **Links policy** (T52, T53)
   - Note: `links.rs:5-9` refuses `file:` deliberately (remote output could name a local path); treat reversing that as a product decision, not plain effort.
   - Scope: allow `file:` (open the containing path via `xdg-open`) with confirmation for remote-host terminals, settings `links_open_apps` Ask First / Always / Never for other schemes, confirm dialog naming the URL. Needs a security review of `links.rs:7-9`.
   - Files: `src/links.rs`, `src/app/workspace.rs` (768-795), `src/app/dialogs.rs`, `src/app/settings_view.rs`.
   - Dependencies: item 3. Effort M.
   - Acceptance: Ctrl+click on `ssh://x` shows an Ask dialog by default; Never silently refuses with a notice; `file://` on a remote host does not open locally without confirmation. README "Links" section updated.

10. **Font-size actions** (T33)
   - Scope: actions `client.font.increase/decrease/reset` changing `font_size`; palette entries; leave `Ctrl =/-/0` as interface zoom (document), propose `Ctrl Alt =` / `Ctrl Alt -` / `Ctrl Alt 0` or leave unbound.
   - Files: `src/app/actions.rs`, `src/keymap.rs`, `src/app/workspace.rs:2012`.
   - Effort S. Acceptance: palette entry changes the size by 1 pt within 8 to 48, one notice at the limit.

11. **Generated titles** (T80)
   - Scope: setting `generated_titles` (default On in beta); tab and pane title = program + directory from `terminal.process`/`pwd` when the shell title is the default; names the user gave and titles programs set still win.
   - Files: `src/connection/activity.rs` (232), `src/app/toolbar.rs`, `src/app/panes.rs`, `src/settings.rs`.
   - Effort M. Acceptance: a tab running `vim` in `~/rex` reads `vim - ~/rex`; renamed tab keeps its name.

12. **Product decisions needed before work** (T15 ligatures per DESIGN.md line 53, T21, T22, T27, T37, T38, T41, T52 `file:` links)
   - Contrast and palette forcing: a render-time legibility transform conflicts with "the client never recolours terminal output" but is the same kind of local overlay as cursor-text. Cost M, benefit: unreadable dark-blue on Midnight in Powerline themes. Recommend a user decision; if accepted, implement in `term.rs` colour resolution behind `improve_contrast` with WCAG-style ratio 4.5.
   - Cursor blink, smooth scrolling, copy animation: rule "terminal content never animates". Recommend keeping Linux steady (equivalent to Blink Off/Smooth scrolling Off, both beta defaults or near) and documenting the decision in README rather than building.
   - Smooth terminal resizing: server owns geometry; recommend not building.

13. **Deferred, engine work** (T07, T10, T35, T67; T15 is a product decision and T49 is small)
   - Live font family apply and weight (rune font registry), ligatures (shaping across the grid, large and conflicts with the cell model), prompt navigation (CORRECTED: not shown to need a server API; ghostty-vt exposes OSC 133 row and cell semantics locally, so verify against a live server before calling it blocked), Select All (CORRECTED: no patch needed, `Terminal::select_all` already exists in the vendored `ghostty-vt`; it moves out of this deferred item into item 4's selection work), Float Pane (floating layer rendering in the panes domain; `session.float_block` exists).

Not applicable on Linux and not to be built: Option key (T32), Display P3 (T23), macOS font smoothing (T17), AutoFill (T72), Quick Look. Platform equivalents already exist: Alt is Meta, compose via AltGr and dead keys, IME preedit at the cursor, `xdg-open` for links.

## Verification log

Verified 2026-11-11 (Pacific Time) by an adversarial pass against primary evidence only: beta screenshots and AX trees under `rex-beta-ui-2026-11-10/`, Linux screenshots under `rex-rune-2026-11-11/screenshots/`, source at HEAD ad196ff, `vendor/rexrs`, `vendor/ghostty-vt`. Tools: flattened AX trees, ImageMagick pixel sampling (beta screenshots are 1x: the 022 screenshot maps tree point (x, y) to pixel (x - 1048, y - 62)).

### Corrections made

1. **Summary counts** were wrong (claimed about 60 capabilities, 15 at parity or ahead, 10 partial, 30 missing against 83 actual rows). Replaced with counts of the verified table (89 rows).
2. **T35 Jump to Previous/Next Prompt**: blocker `needs-server-api` was unsupported. `vendor/ghostty-vt/src/screen.rs:314,380` exposes OSC 133 row and cell semantics and `selection.rs:265` has `select_output`; the client holds a local Ghostty terminal. Changed to `unknown` (probably client-only; not verified on a live server), effort L to M. Beta shortcut values (Command Up Arrow / Down Arrow) confirmed in Shortcuts 047, 048.
3. **T49 Select All**: the claim "may need a `ghostty-vt` vendor patch" is false. `Terminal::select_all` exists at `vendor/ghostty-vt/src/selection.rs:236`. Effort M to S; item 13 and the not-to-build text updated.
4. **T15 Ligatures** blocker `needs-rune-work` changed to `product-rule`: DESIGN.md line 53 says ligatures and kerning are off so the grid stays exact.
5. **T52 `file:` links** blocker `none` changed to `product-rule`: `links.rs:5-9` documents refusing `file:` because output can come from a remote host; a test at line 219 pins it. Work item 9 and item 12 annotated.
6. **T61 "No matches"**: `linux-ahead` was unsupported because the beta's no-result state was never captured (125 is an empty query). Changed to `linux-only`.
7. **T34 Send Key presets**: `parity` changed to `not-applicable`; Linux ships no presets, only the `pane.send_key` mechanism. Beta evidence cited precisely (Shortcuts 048 rows).
8. **T66 Split Left/Up** evidence: `session.new_split` has a top-level `side` (`before` or `after`), not `before`/`after` in `layout` (`vendor/rexrs/src/api/generated/session.rs` near line 2406). Blocker `none` still correct; Linux only ever sends `side: after`.
9. **T56 Find bar**: Linux closes with Esc (`input.rs:176-189`, `workspace.rs:949`); "no close" meant no close button. Added beta pixel evidence (125/126 crops: `Find` placeholder, `1/1`, divider, chevrons dim when empty, `x`).
10. **T58 Find Next/Previous**: added beta evidence from Shortcuts (Find Next = Command G, Find Previous = Shift Command G) and the beta's own conflict: Go to Directory… is also Shift Command G (045 to 050).
11. **T60, T42, T43 line cites** drifted: search overlay is `term.rs:761-767`, selection fill `768-774`, selection colours `783` and `992-995`. T60 updated; T44 now confirmed from code (plain rectangles, no radius) instead of inferred.
12. **T62 parity** now notes the beta behaviour is "implied only" and that one search test is `#[ignore]`d for Ghostty order on a restored, resized terminal.
13. **T73**: Rename pane is not Linux-only; the beta has a `Rename Pane` action (Shortcuts 049). Only Reconnect terminal is Linux-only.
14. **T75**: noted that `pane.copy` and `pane.paste` exist as bindable actions (`actions.rs:85-86`) but are not palette entries. **T63/T64** noted as the same capability as T47/T48.
15. **T46**: added that right-click opens the pane menu even over a mouse-tracking program, with Shift+right-click going to the program (`panes.rs:451-475`).
16. **UI delta 1**: the claim that Linux has no dividers is wrong for the first five rows (`divider()` at `settings_view.rs` lines 294, 305, 322, 329; visible in 02 and 14). Only the per-host shell rows lack them. **UI delta 6**: pickers are top-centre popups but the font and shell forms are centred dialogs.
17. **Hex corrections in section 3**: switch off track is `#292D30`, not `#282C2F`; preview well interior is `#0F1216`, not `#0F1217`.
18. **Added omitted rows**: T84 pane padding (beta Customize Panes 011 has separate horizontal 10 and vertical 6 pt padding; Linux has one 0 to 32 pt stepper), T85 Quick Look (Command Y, Shortcuts 048), T86 Copy from Pane / Paste into Pane / Copy Pane ID actions, T87 terminal accessibility (beta AX exposes the screen as a read-only `Terminal` text field; README line 495 says AT-SPI is not implemented), T88 macOS Edit-menu system items (Undo, Redo, Cut, Delete, Writing Tools, Start Dictation, Emoji & Symbols), T89 Linux file drop and paste upload (`upload.rs`).
19. **Noted discrepancy**: the UI and README call the font size "pt" but DESIGN.md line 53 says "8 to 48 px"; the code range is 8 to 48 (`settings.rs:268`, `workspace.rs:2013`) and the default is 15 (`settings.rs:207`); beta default is 12 pt.

### Claims checked and confirmed

- Beta 022/023/033 AX labels, verbatim: `Search Settings`, `Terminal font` + `Choose...`, `Font style` (Regular), `Font size` (12, `pt`), `Font weight, fixed by the font` (400, disabled), `Line height adjustment in percent` (0, `%`), `Typography preview: regular, bold, and italic terminal text.`, `Advanced font settings`, `Improve text contrast` (on) with helper text, `Force theme palette for text`, `Use Display P3 color space`, `Cursor preview`, `Cursor style` Block, `Blink speed` Normal, `Blink interval` (disabled), `Force style`, `Show key sequence hints`, `Option key` Automatic (by keyboard layout), `Smooth scrolling`, `Smooth terminal resizing`, `Copy on selection`, `Play animation on copy` (on), `Foreground`/`Background` Theme, `Rounded corners` (on), `Links that open apps` Ask First, `Show recently used paths`, `Prefer generated titles`, `Startup command`, with the footnotes quoted in section 3.3. The 11 section headings match.
- Menus from trees: font style (Regular, Italic, Bold, Bold Italic; 029), cursor style (Block, Bar, Underline; 030), blink (Off, Slow, Normal, Fast, Custom with separators; 031), option key (five entries; 032), selection foreground and background (Theme, Cell Foreground, Cell Background; 034, 035), link handling (Ask First, Always Open, Never Open; 036), character spacing unit (px, %; 026), render bold and italic (Both, Bold only, Italic only, Neither; 027).
- Context menu 094 tree: Copy (disabled), Paste, separator, Split Right, Split Left, Split Down, Split Up, Float Pane, Zoom Pane, separator, Close Pane, Reset Terminal, separator, AutoFill; 132 wide, rows 24, separators 11. Edit menu 073: Select All, Insert Path…, Find submenu (Find…, Find Next disabled, Find Previous disabled), AutoFill (Contact…, Passwords…, Credit Card…). View menu 074: Reset, Increase, Decrease Font Size.
- Shortcuts: Find… Command F, Find Next Command G, Go to Directory… Shift Command G, Insert Path… Control Command I (also the `⌃⌘I` keycap, 32x18, 8 pt right of the heading), Increase Font Size Command + and Command =, Decrease Command -, Reset Command 0, Zoom Pane Shift Command Return, Scroll Page Up/Down and Clear/Reset Terminal unbound, Select All in Pane unbound.
- Geometry from trees: window 816x850, sidebar splitter 214, Back/Forward 38x36 at (223, 9) window-relative, cards 561 wide at x 235, switch 36x16, Choose... 78x24, Add... 55x24, popups 24 tall, previews 549x58 and 549x30, Startup command 386 wide, row pitch 46 in Colors, 38 single and 45 with helper, 30 pt gap above headings.
- Pixel samples (beta 022, 023, 033, 126, 127): sidebar `#0F1217`, content `#0B0F13`, card `#12161A`, switch on `#0676FC`, row divider `#26292D`, popup badge `#24282C`, push button `#22262A`, well border `#24282D`, text-input fill `#1C1D20`, blue cursor `#0071F6`, selection pill `#0055BF`, picker fill `#0F1217`, search capsule `#272A2E`, segmented thumb `#3E4248`, selection preview inverted fill `#E2EAF1`, find match `#F4D722`, find dim `#0F1216` to `#080A0C`, find and path panel body `#1C1D20`, search field `#333437`, hairline `#020203`/`#010102`, inner highlight `#333436`. The find pill measures about 300 wide and 40 tall including hairline (spec said 39); the 127 and 128 panel measures 326x496 inside its hairline (spec 330x501), both within tolerance.
- Linux: T01 modal is 760x520 (`settings_view.rs:826`); Terminal page has Font family, Font size, Pane padding, Fallback fonts, Theme and per-host Default shell rows plus the `Saved in ...workspace.json` footer (02, 14); font changes apply only after reopening (helper text); no search, no previews, no cursor, line-height, spacing, contrast or selection settings (grep of `src/settings.rs` and `settings_view.rs`).
- Linux pane menu (38; `panel.rs:717-726`): Split right, Split down, Zoom/Restore pane, Rename pane, Reconnect terminal, Close pane with confirm, header `Shell · shuv@shuvbot:~`, shortcut hints. No Copy, Paste, Select All, Reset, Clear, Split left/up.
- `terminal.list_dir`, `pwd`, `process`, `program_status`, `reset`, `clear`, `write` exist in `vendor/rexrs/src/api/generated/terminal.rs`; `list_dir` takes `path`, `prefix`, `limit`, `files` as the matrix says and leaves sorting to the client; `session.float_block` exists at `session.rs:1133`. `Reset`/`Clear` are not referenced anywhere in `src/`. No server prompt-mark method exists in `vendor/rexrs`.
- Defaults: `Ctrl+=`, `Ctrl+-`, `Ctrl+0` bind to `client.zoom.*`, titled "Enlarge interface" and so on, and drive `ui_scale` (`keymap.rs:390-396`, `workspace.rs:798-815`), not the terminal font; `Ctrl Shift F` opens find (`keymap.rs:388`); `Shift PageUp/PageDown` scroll 20 lines (`keymap.rs:410`).
- Cursor shape follows the program (`terminal.rs:402-408`) with a hollow block when unfocused (`term.rs:1118-1120`); blink ignored by design. Ligatures off via `FEATURES` (`term.rs:152`). Cell height is `ceil(size * 1.4)` (`term.rs:154-166`).
- `Ctrl+click` link tooltip `Ctrl+click to open <URL>` (`term.rs:1177`); schemes limited to https, http, ftp, mailto (`links.rs:5-9`).
- New tabs and splits inherit the source directory (`connection.rs:405` and `activity.rs:232` `shell_dir`); matrix parity for T82 holds.
- Matrix and screenshots for find: 46 to 48 show the 280-wide field in the top-right, the `esc`, `1/2` and `No matches` trailing text, and the matches partly under the bar in 47.
