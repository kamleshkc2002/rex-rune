# c02: Rex Linux workspace chrome (code inventory)

Source: this repo at branch `t3/0a321b52` (HEAD ad196ff, version 0.6.0). Paths are relative to the repo root; line numbers are for the files as read on 2026-11-11. Scope: sessions, tabs, panes, splits, zoom, sidebar, toolbar, switcher, Commands, context menus, dialogs, search, drag/drop, uploads, status indicators. Settings, themes and terminal rendering are in c01. Written from the code; PRODUCT.md, DESIGN.md and AGENTS.md were used only for the Constraints section and to flag where they disagree with the code.

Abbreviations: `ws` = `src/app/workspace.rs`, `panel` = `src/app/panel.rs`, `panes` = `src/app/panes.rs`, `popups` = `src/app/popups.rs`, `dialogs` = `src/app/dialogs.rs`, `sidebar` = `src/app/sidebar.rs`, `toolbar` = `src/app/toolbar.rs`, `actions` = `src/app/actions.rs`, `nav` = `src/app/navigation.rs`, `act` = `src/app/activity.rs`.

## 1. Window and model

- One native borderless window, single window only (no multi-window, no window-per-session): `AppSpec`/`WindowOptions{borderless:true}` `src/app/mod.rs:219-241`. Minimum size 640x480 (`mod.rs:220,239`); `--window-size WxH` default 1440x900 (`mod.rs:200-209`).
- Layout, top to bottom: 52 pt toolbar (`theme.rs:198`), then body row = optional sidebar + sidebar resize edge + pane area (`mod.rs:62-86`). Toasts and the popup/dialog overlay float above (`mod.rs:87-95`).
- Window edges and corners resize via 6 pt bands (12 pt corners), hidden when maximized; the corner above Close is excluded (`mod.rs:39-56,117-131`). Empty toolbar space drags the window (`toolbar.rs:289,320`); right-click on toolbar background opens the compositor window menu (`toolbar.rs:290`). No minimize/maximize buttons (DESIGN.md).
- Window title `"<session> — <host> — Rex"`, "Choose session" when none (`ws:742-756`).
- Hierarchy: saved hosts (`Settings.hosts`) -> server sessions -> Rex windows shown as "tabs" -> blocks shown as panes in layers (`ws:33-57`, `layout.rs`). Every saved host gets a connection at launch, not only the active one (`ws:294-296`); the model of the inactive hosts keeps syncing (`ws:432`).
- Interface zoom (chrome and terminal together) 70-150% in 10% steps, effective scale limited to keep a 640x480 layout floor (`ws:797-834`). Notice "Zoom N%" with " . minimum/maximum" or " . showing X% to fit this window" suffix (`ws:821-832`).
- Window-focus handling: on focus-in, reload shortcut file, clear claims and claim visible panes (`ws:603-622`); on focus-out, ends selections, clears sequence/forwarded keys.
- Frame statistics: `REX_FRAME_STATS=1` prints per-frame p50/p95/max CPU to stderr every 120 frames or 2 s (`src/app/stats.rs:1-46`). Developer-only; no UI.

## 2. Hosts

- Host kinds by endpoint: `unix://` = monitor icon ("this machine"), anything else = server icon (`panel:455-461`).
- Add host: opens "Add host" list of tailnet machines, search field "Search your tailnet…" (`discover.rs:69-76`), groups "Rex servers" / "Tailnet" / "Offline", rows show added-as, endpoint, SSH method, OS, hostname (`discover.rs:187-270`); footer "Enter an endpoint…" (`discover.rs:272-275`); empty text "Looking at your tailnet…" / scan error / "No machine by that name on your tailnet." (`discover.rs:279-287`). Skips this machine and iOS devices (`discover.rs:196`). Machines with a found Rex server connect in one click (`add_machine`, `discover.rs:289+`); others open an SSH form ("Add SSH host", `Form::AddSsh`, `panel:173`). Reached from sidebar "Add host", switcher footer, Commands "Add host…" (`sidebar.rs:485-499`, `panel:628,847`).
- Add host by endpoint form "Add host by endpoint": fields Host name (placeholder `my-host`), Endpoint (`https://…`, `ssh://user@host[/socket]`, `unix:///path`); primary button "Connect"; help text explains bare host tried on 443 then 7297 (`ws:1267-1271`, `dialogs.rs:103-110,127-131`). Validation: name required, `validate_host`, duplicate endpoint refused "This endpoint is already in the workspace" (`ws:1867-1899`).
- Discovered-endpoint form "Connect to Rex" with server-reported listener list (scrollable, max 240 pt) (`discover.rs:296-317`, `dialogs.rs:93-102`).
- Edit host "Edit <label>": Host name, Endpoint, Default shell; changing endpoint drops the connection, resets `remote_control` to false, reconnects under a new host id, forgets old terminals (`ws:1320-1331,389-414`); note text under the form (`dialogs.rs:77-84`). Reached from host menu, offline session menu, Commands "Edit host <label>…", Settings → Remote Hosts.
- Remove host: confirm "Remove <label>?" with buttons "Keep host" (default) / "Remove host"; says sessions keep running; last host cannot be removed (`ws:1332-1335,363-387`, `dialogs.rs:197-207`, `panel:449`).
- Default shell per host: form "Default shell on <host>", one program name, whitespace/control chars rejected ("Enter one shell name or path, without arguments."); empty clears; applies to new sessions, tabs and splits, not running terminals (`ws:1252-1260,1861-1866`, `dialogs.rs:111-118`). Commands entry shows current value as detail (`panel:651-654`).
- Retry/reconnect: `HostCommand::Reconnect` from sidebar "Retry connection" row, host/session menus, switcher row, empty state, Commands "Reconnect host" (`ws:1205,1336`).
- Start personal server over SSH: dialog "Start a personal server on <label>?" with exact command, effects, default "Not now"; offered only when SSH connected but no default server (`ssh_setup.rs:19-31,42,222-228`). Entry points: empty-state pill, host menu ("Review first"), Commands (`panes.rs:628-637`, `panel:439-446,679-686`). See also PRODUCT.md.
- Remote control per host: Commands entry "Turn on/off remote control for <host>" toggles `Host.remote_control` (default off) (`panel:640-650`, `src/app/remote.rs:23-26`); per-host switch also in Settings → Remote Hosts; client label form "Client label" (`ws:1248-1251`).
- Host order is the saved order; no reorder UI. Host collapse state `collapsed` is persisted (`sidebar.rs:77-81`).
- Hosts are addressed by stable `HostId` in menus/history so a removal cannot retarget (`ws:28-31`).

## 3. Sessions

- Create session (always explicit, never on attach): `create_session(host, label, tx)` sends `HostCommand::CreateSession` -> `session.create` with `initial_windows:[{window_label:"Shell", layout: shell block}]`, optional `label`, host's default shell and the declared terminal theme (`ws:1342-1358`, `src/connection.rs:1731-1747`). Server picks a unique label when none typed. Creating on a host also activates that host and expands it (`ws:1350-1355`). Disabled while the host is offline (`ws:1343`).
- Create entry points: shortcut `Ctrl Shift N` (`session.new`, `keymap.rs:377`), toolbar none, sidebar panel header + button ("New session… . choose a host" -> `Create session on…` list) (`sidebar.rs:464-470`), per-host `+` button in sidebar host rows (`sidebar.rs:27-48`), sidebar "New session" row for an empty connected host (`sidebar.rs:296-327,358-361`), host menu "New session", switcher group-header `+` and footer, Commands "New session on <host>", empty-state "New session" pill (`panes.rs:613-619`).
- Select/switch session: `select_visit` -> correlated `HostCommand::Navigate{session, window:None}`; preserves the server-side active window; 30 s timeout notice "Navigation timed out. Try again." (`nav.rs:191-203,150-171`, `nav.rs:8-9`). Selecting a session on an offline host sends `HostCommand::Select` instead (`nav.rs:264-273`). Last session per host is persisted (`ws:477-486`).
- Rename session: form "Rename session" field "Session name" (current via Commands/shortcut `session.rename`, or any sidebar session via its menu) -> `session.set_label`; inline rename in switcher row (Enter saves, Esc cancels, empty/unchanged ignored) (`ws:1286-1289,1501-1509,1931-1934,2045-2087`).
- Close session: confirm "Close <label>?" naming tab count and host, default "Keep running", danger "Close and end programs"; sends `session.destroy` against the named session, not the selected one (`ws:1524-1548`, `dialogs.rs:152-159`). From the switcher, Cancel returns to the switcher (`ws:1655-1663`).
- Reorder sessions: sidebar session menu "Move up" / "Move down" -> `session.move` with `before_session_id`, disabled at the ends (`ws:1510-1523`, `panel:385-396`). No drag reorder.
- Session row in the sidebar: 32 pt (24 pt label when Sidebar tab position is on), terminal glyph, name, tab-count number, current = fill + accent glyph + "(current)" in the a11y label; right-click opens the session menu; targeted-by-menu row gets a 1.5 pt accent outline (`sidebar.rs:118-219`).
- Sessions from other clients appear through the server's session events; each client can attach to the same sessions (PRODUCT.md).
- Sessions have no per-session activity word, badge or unread marker in the sidebar or switcher; activity is shown per tab only (see 11).

## 4. Tabs (Rex windows)

- Terminology: server windows are "tabs". Default new-tab label is `Shell` (`src/connection.rs:23`); a tab still named `Shell` or the server's generated `Window XXXXXXXX` shows its focused pane's terminal title while that program is foreground (`act:46-67`).
- New tab: `session.new_window` with `window_label:"Shell"`, a shell layout, `focus:true`; inherits the focused pane's shell working directory via `NewTerminal`/`cwd_from` (best effort) (`ws:1272-1279,995-1005`, `src/connection.rs:1779-1795`). Shortcut `Ctrl Shift T`; toolbar "+" button; tab menu; Commands "New tab"; empty-state "New tab" pill.
- Select tab: click pill, sidebar tab row, `Intent::Tab` -> `Navigate{session, window}` (`ws:1188-1192,1137-1147`). Shortcuts: `Ctrl PageUp/PageDown` previous/next within the session (`session.focus_previous_window/next_window`, server-controlled) (`actions.rs:328-329`, `keymap.rs:382-383`); `Ctrl Tab`/`Ctrl Shift Tab` next/previous across hosts and sessions (sidebar order, wrapping, 150 ms settle when crossing sessions/hosts) (`nav.rs:329-399`); `client.tab.goto` with `index` (1-based, -1 = last) has no default binding (`actions.rs:377-387,419-425`).
- History: window-local visit history, 128 entries, back/forward by mouse Back/Forward buttons or the `client.history.back/forward` actions (no default key; Commands "Back in history"/"Forward in history" show "Mouse Back"/"Mouse Forward" as detail); skips closed/offline targets; ignored while a panel is open or a divider is dragging (`nav.rs:34-100,293-327`, `mod.rs:96-109`, `panel:597-606`).
- Rename tab: form "Rename tab" (double-click the pill, tab menu, Commands, `client.tab.rename`); starts from the displayed name; `session.set_window_label` (`ws:1290-1298,1935-1937`, `toolbar.rs:120-122`).
- Close tab: confirm "Close this tab?" ("Ends every program in this tab on the server. Closing the app itself leaves programs running."), default "Keep running"; middle-click pill, tab menu, Commands, `client.tab.close`, `Ctrl Shift W` (when the tab has one pane) -> `session.close_window` (`ws:1299-1304,1392-1407`, `toolbar.rs:112-115`). The title does not name the tab or host (DESIGN.md says it should; only pane and session confirms name targets).
- No tab reorder (drag or menu), no tab pinning, no tab duplicate, no "move tab to another session", no detach to window.
- Tab pill (toolbar): 96-240 pt wide, 28 pt tall, program icon 15 pt (with program name tooltip), label (ellipsized, full text tooltip when truncated), activity word (see 11), 2 pt progress bar; active = lit fill with sliding matched highlight, inactive hover 5%; no close button (`toolbar.rs:76-229`). Strip scrolls horizontally with edge fade; when it overflows, previous/next scroll chevrons appear (`toolbar.rs:179-227,371-376`); active tab is scrolled into view after selection/resize/sidebar change (`mod.rs:146-158`).
- Tab menu (right-click pill): see 8.
- Tabs in the sidebar (Sidebar tab position, only while the sidebar is open): pill-like 32 pt rows under each session, indented 12 pt, active = fill + gradient border, with program icon, label, activity word and progress bar; click selects (`sidebar.rs:221-286`, `ws:1151-1161`). No context menu, no middle-click close, no double-click rename on sidebar tab rows (handlers absent). The sidebar prefetches views for every expanded host's sessions in this mode (`ws:888-904`).
- Empty session: "This session has no tabs" card with "New tab" (`panes.rs:606-612`).

## 5. Panes and splits

- Split creation: `session.new_split` with `anchor_id`, `direction` horizontal (right) or vertical (down), `side:"after"`, a new shell layout, `focus:true`, working directory inherited from the source block (`ws:2123-2125,1280-1285,1569-1578`). Shortcuts `Ctrl Shift D` (right) and `Ctrl Alt D` (down); pane menu "Split right"/"Split down". Always splits the anchor "after" (no split-before/left/up).
- Focus pane by direction: `pane.focus` args left/right/up/down -> `session.focus_direction`; defaults `Ctrl Alt Arrows` (`keymap.rs:404-409`, `actions.rs:355`). Click on a pane focuses it (`panes.rs:335-338`), as does clicking its header (`panes.rs:114-121`).
- Close pane: confirm "Close <pane> in <tab>?" ("Ends the program in this pane on <host>. Closing the app itself keeps programs running."), default "Keep running"; `block.close`. Shortcut `Ctrl Shift W` closes the focused pane when the tab has more than one, else the tab (`ws:1392-1407,1596-1609`). `pane.close` action has no default binding.
- Rename pane: form "Rename pane", field "Pane name", `session.set_block_label`; starts from the server label, not the displayed title (`ws:1587-1595,1938-1947`).
- Reconnect terminal: `Command::Reconnect` per pane; `Ctrl Shift R`; strip "Retry" button (`ws:1610-1613`, `panes.rs:153-211`).
- Resizing: drag the divider handle (10 pt hit area, resize cursor), live preview, ratio clamped 0.1-0.9, sent once on release as `session.resize_pane` (`direction, before_block_id, after_block_id, ratio`); Esc/cancel aborts (`panes.rs:732-797`, `ws:2024-2040`). No keyboard resize, no equalize/swap/rotate/move-pane, no drag-to-rearrange pane, no pane resize bindings (README: "pane-resize bindings … are not implemented").
- Layout is server-owned: `layout::placements` converts the server's split tree to normalized rects; horizontal/vertical, nested, ratio; floating and tiled layers share window coordinates; `minimized` layers skipped, unknown layer states drawn (`src/layout.rs:90-175`, tests `:196-216`). The client cannot create floating layers or minimize/restore them (no UI).
- Density: Comfortable (default) = rounded 12 pt cards, 30 pt header (pane name, activity word, exception strip, progress), 8 pt gutters, 10 pt outer margin; Compact = edge to edge, 1 pt separators, no headers/borders/shadows, per-pane activity hidden (`panes.rs:26-75,213-273,801-832`, `theme.rs:198-202`). A single unzoomed pane draws full bleed with no card/header and its strip floats top-right (`panes.rs:83-86,543-547`). Setting in Settings → Interface (`settings_view.rs:119-139`).
- Pane focus cues: Comfortable focused card has accent gradient border + glow; targeted card (menu/confirm/drop hover) has solid accent border; terminal cursor is a block when this pane has the keyboard, hollow otherwise (`panes.rs:257-271,276-288`). Compact has no border; focus is the cursor only. Popup/dialog on a pane draws a 1.5 pt target outline over it (`popups.rs:40-80`).
- Pane header: name = `pane_name` (server label, or terminal title while label is the flavor default), right-click opens pane menu; tooltip with full name when truncated (`panes.rs:88-151`, `act:69-79`).
- Terminal grid input in a pane: left-drag selects (double = word, triple = line), Ctrl+click opens links, mouse reporting to programs unless Shift, middle button only while tracking, wheel scrolls local scrollback or reports to the program, Shift+right-click goes to a tracking program (`panes.rs:322-494`). Pane padding 0-32 pt (default 8) in Settings.
- Status of terminals not yet streaming: "Opening terminal…" / "Waiting for host…" placeholder (`panes.rs:531-541`).

## 6. Zoom

- Pane zoom (server-side): `session.zoom_block` toggles the window's `zoomed_block_id`; `Ctrl Shift Enter` (`pane.zoom`) zooms the focused pane, pane menu "Zoom pane"/"Restore pane" focuses the target then zooms (`ws:1305-1309,1579-1586`, `keymap.rs:384`). A zoomed window shows exactly one placement at [0,0,1,1] and no dividers (`layout.rs:91-99`). Zoomed pane is drawn as a card with header in Comfortable (full-bleed only when not zoomed). No separate zoom indicator, badge or "Restore" affordance in the toolbar or header beyond the pane menu label; zoom state is per window on the server so other clients see it.
- Interface zoom (`client.zoom.in/out/reset`): `Ctrl =`, `Ctrl Shift =`, `Ctrl +`, `Ctrl NumpadAdd` in; `Ctrl -`, `Ctrl NumpadSubtract` out; `Ctrl 0`, `Ctrl Numpad0` reset; global (also reach dialogs) and repeat (`keymap.rs:390-397`, `actions.rs:142-146`). Not in Commands; Settings → Interface has stepper + Reset.

## 7. Toolbar (52 pt, single row)

Left to right (`toolbar.rs:281-368`):
- Sidebar toggle button (`SIDEBAR` icon, 30 pt), tooltip "Toggle sidebar . Ctrl Shift B" (hard-coded text, not the effective binding).
- Identity chip (210 x 34): host-kind icon, session name (bold), host name in muted; when the host is down the second line becomes "<host> . Reconnecting" in WARNING; chevron; click opens the switcher, right-click opens the current session's menu; tooltip "<session> . <host> . Switch session . Ctrl K" (hard-coded) (`toolbar.rs:19-74`).
- Tab track (hidden when tabs live in the sidebar): see 4.
- Pending-sequence chip: `Ctrl B …` style label (max 170 pt, full text tooltip, "Escape cancels") while a multi-key shortcut is mid-sequence (`toolbar.rs:300-318`).
- Drag space (>= 48 pt).
- Right-end bare icon buttons on a 36 pt pitch with round 30 pt hover fill: Commands (`COMMAND` icon; tooltip "Commands, sessions and tabs . Ctrl Shift P"), New tab ("+"; "New tab . Ctrl Shift T"), Settings ("Settings . Ctrl ,"), 1x16 divider, Close app (hover tint DANGER 22%; "Close app . server programs keep running"; calls `tx.close_window()` with no confirmation) (`toolbar.rs:321-366`).
- Absent from the toolbar: split buttons, zoom indicator, search button, connection/host status icon, notification bell, update button (update appears as a toast, a Commands row and Settings → Updates), reconnect button.

## 8. Context menus (popovers anchored to the pointer; 260 pt wide, 28 pt rows, title row 26 pt)

Shared shape: `list_popup` (`popups.rs:82-439`); clamped to viewport; dismissed by outside click (consumed), Esc, or re-pressing `Ctrl Shift Space` for the pane menu (`popups.rs:31-37`, `input.rs:163-171`). Disabled rows are shown dimmed (not hidden). Shortcut hints on the right come from the host's effective keymap (`panel:861-873`).

- Pane menu (right-click anywhere on a pane card, its header, or `Ctrl Shift Space` for a keyboard-anchored one; title "<tab> . <pane>"; target outlined) (`ws:1439-1449`, `panes.rs:122-124,235-237,454-473`): Split right; Split down; Zoom pane / Restore pane; Rename pane…; Reconnect terminal; Close pane… (`panel:716-734`). Detail hints for split/zoom/reconnect/close come from bindings; Rename has none. Closes itself on selection, except Rename/Close which open dialogs (`panel:116`). A pane that vanishes dismisses the menu with notice "That pane is no longer available." (`ws:552-556`). Right-click over a mouse-tracking program opens the menu; Shift+right-click sends the click to the program (`panes.rs:451-473`).
- Tab menu (right-click a toolbar pill; title = tab name): Rename tab…; Close tab…; New tab (disabled when host offline) (`panel:563-575`, `ws:1374-1390`). Not available on sidebar tab rows.
- Session menu (right-click a sidebar session row, or the identity chip; title "<host> . <session>[ . Offline]"): Switch to session (hidden if current); Rename session…; Move up; Move down; Close session… (all disabled while the host is offline); offline adds Retry connection and Edit host… (`panel:367-416`, `ws:1451-1479`). Row outlines while its menu/dialog is open.
- Host menu (right-click a sidebar host header; title "<host>[ . Offline]"): New session (disabled offline); Default shell… (shows current shell as detail); Retry connection (offline only); Start personal server… (only when applicable, detail "Review first"); Edit host…; Remove host… (disabled for the only host) (`panel:420-452`, `ws:1360-1371`).
- Switcher row context: right-click a session row in the switcher starts inline rename (`popups.rs:243-247`).
- Terminal grid has no text context menu (no Copy/Paste/Select all/Open link menu items): right-click opens the pane menu instead. Copy/paste are keyboard only (`Ctrl Shift C`, `Ctrl Shift V`).
- Toolbar background right-click: native window menu only (`toolbar.rs:290`).
- No menu for: sidebar tab rows, the sidebar background, toast, search field, host-icon in identity chip.

## 9. Switcher (Ctrl K, sidebar "Find a session", identity chip)

- 340 pt popover anchored under the toolbar at the left; search field "Filter sessions or hosts…" with `esc` hint; title "Switch session" is not displayed when the field exists (`ws:1667-1691`, `popups.rs:93-117,368-411`).
- Content: one group per host (header = host label, or "<label> . Offline"), host-kind icon, a `+` create button in the header of each connected host (tooltip "New session on <label>"); session rows with a check on the current one; a connected host with no sessions shows a "New session" row (or `New session "<q>"`); an offline host shows a "Retry connection" row (`panel:736-801`). Selection starts on the current row (`ws:1685-1690`).
- Filter matches `"<session label> <host label>"`; a typed query names a new session: footer rows `New session "<q>" on <host>`, `Create "<q>" on another host…` (opens the "Create "<q>" on…" host list), and `Add host…` (footer separated by a rule) (`panel:831-856`).
- Row actions while highlighted: pencil (Rename) and × (Close session…) buttons replace the detail; F2 or right-click renames in place; close routes to the confirm dialog and returns to the switcher if cancelled (`popups.rs:204-249`, `input.rs:150-160`, `ws:2045-2106`).
- Keyboard: Up/Down move through enabled rows, Enter activates, Esc cancels/goes back, Tab is swallowed (`input.rs:136-175`). Hover moves the highlight.
- Scroll hints with counts: "N more sessions", "<host> offline", "N offline hosts below", "Earlier sessions above" (`popups.rs:296-352`). Max height bounded by viewport.
- Not present: recent-sessions ordering, session activity badges, numbered quick-select, fuzzy ranking (plain substring contains), preview of tabs.

## 10. Commands list (Ctrl Shift P; Commands button) (`panel:514-859`)

460 pt popover, 30 pt rows, search "Search commands, sessions, and tabs…"; query filters label + detail + group (substring, case-insensitive). Group "Workspace", in order:

1. Switch session… (shortcut shown: Ctrl K)
2. Next tab across hosts (Ctrl Tab)
3. Previous tab across hosts (Ctrl Shift Tab)
4. Back in history (detail "Mouse Back")
5. Forward in history (detail "Mouse Forward")
6. Search terminal… (Ctrl Shift F)
7. New session on <host> (Ctrl Shift N; disabled if host offline)
8. New tab (Ctrl Shift T; disabled if offline or no view)
9. Rename session… (disabled if offline or no view)
10. Show sidebar / Hide sidebar (Ctrl Shift B)
11. Turn on / Turn off remote control for <host>
12. Default shell for <host>… (current shell as detail)
13. Settings… (Ctrl ,)
14. Check for updates
15. Add host…
16. Edit host <host>…
17. Reconnect host
18. (conditional) Update Rex Linux (<version>)… with channel id as detail, when a verified update exists
19. (conditional) Start personal server… (detail host) when applicable
20. (conditional, when a tab is current) Rename tab… ; Close tab…

Group "Pane . <name>" (when a pane is focused, or the target of a pane menu): Split right; Split down; Zoom pane/Restore pane; Rename pane…; Reconnect terminal; Close pane…

With a non-empty query only: group "Sessions . all hosts" (every session on every host, detail "<host> . Session", current marked) and group "Tabs . <host> / <session>" (tabs of the current session, detail "Tab") (`panel:738-822`).

Not in Commands (exist only as shortcuts/actions): Copy selection, Paste, Scroll terminal, Focus pane (direction), Go to tab N, Close focused pane or tab (`client.close`), Next/Previous tab within session, interface zoom in/out/reset, Pane actions, Send key, Toggle Commands. Not present at all: font size change, theme switch, notification toggles, remote host list, session close/move, "Close session", "New window". Server-registered `lua`/`block` actions work from keybindings but README says "server actions in Commands" are not implemented. Entries have no icons.

## 11. Status indicators

Quiet-when-healthy design (DESIGN.md "Quiet when healthy", "Don't add status dots"): connected hosts and live panes show nothing.

- Host offline: sidebar host header dims; session list replaced by note "Waiting for connection…" (empty) plus an accent "Retry connection" row (tooltip = status string); identity chip second line "<host> . Reconnecting" (WARNING); switcher group "<host> . Offline"; menu titles " . Offline" (`sidebar.rs:358-384`, `toolbar.rs:27-31`, `panel:748-753`). Empty state "Host unavailable" + status text + "Retry connection" (`panes.rs:599-605`). There is no coloured dot and no "Connected" label by design.
- Pane not live: exception strip "Reconnecting . input paused[ . N rows hidden]" with "Retry" pill, in WARNING; text hidden when the card is narrower than 360 pt; tooltip is the raw status string (`panes.rs:153-211`). Input is not queued while paused (AGENTS.md).
- Size ownership: automatic. This window claims panes on focus (`session.own_block mode:take`, `ws:574-601`), and non-passive input (presses, typing) claims the block (`src/connection.rs:1836-1845`). An owner sends `Resize`; a non-owner never resizes (`term.rs:329-352`). There is no "Take control" button and no ownership indicator; `Command::TakeControl` is a stub (`src/connection.rs:111,1917`) and DESIGN.md's description of a Take control strip is not implemented (only "Retry" exists).
- Tab/pane activity (OSC 7501 records folded Blocked > Error > Working > Done > Idle, `act:85-136`): word after the label: "Needs approval" / "Question" / "Needs sign-in" / "Needs input" (WARNING), "Error" (DANGER), "Bell" (WARNING, latched until the tab is seen), "Paused[ N%]" (WARNING), unseen "Done" (ACCENT); working/idle show nothing (`act:152-182`). Progress (OSC 9;4 or record percentage) is a 2 pt bar along the pill's bottom, ACCENT / WARNING / DANGER, indeterminate = whole track at 35% (`act:187-224`). The program's message is the word's tooltip. Gated by Settings → "Show activity in tabs" (`notifications.show_in_tabs`, default true) (`toolbar.rs:93-94,171`, `sidebar.rs:274-283`). In Comfortable density the pane header repeats the trouble word and progress for that pane alone (`panes.rs:88-151`); Compact hides them.
- Program icons on tabs: Claude (`claude`/`claude-code`), Codex, OpenCode, Pi (also `omp`), Vim (`nvim`/`vim`/`vi`), Git (`git`/`lazygit`/`tig`), Cargo (`cargo`/`rustc`/`rustup`), SSH (`ssh`/`mosh`), terminal glyph for shells and unknown (`act:138-150`).
- Desktop notifications (freedesktop, D-Bus worker): for tabs not in front, on live changes to Working (off by default), Blocked, Error, Done (on by default), plus program messages (OSC 9/777) (`act:288-376`, `src/app/notify.rs`, `src/settings.rs:46-66`); summary "<program> . <word>", body message + "<session> . <host>"; clicking raises the window (activation token) and selects the tab (`act:427-437`). Tab "seen" = window focused, host/session active, window active (`act:244-253`).
- Toasts ("notices"): 340 pt wide stack, at most 3 (oldest non-sticky dropped), identical text deduped, errors sticky until dismissed, information auto-clears after 6 s, X button dismisses (`ws:836-871`, `dialogs.rs:217-299`). Corner = Settings → Notices (Top right default / Bottom right). Sources: app notices, the host's `notice` (request failures), terminal notices (e.g. "Input not sent", "Search failed", "Upload failed"), remote-control notices naming host/requester/action, keymap problem counts, update available.
- Update toast: accent dot, channel guidance, "What's new" (opens notes URL), "Updates…" (Settings → Updates), dismiss-for-this-version X; background check failures silent (`src/app/updates.rs:269-350`).
- Sidebar footer: "<N> sessions" total count across hosts (`sidebar.rs:411,500-505`). Session row tab-count number.
- Keymap: pending key sequence chip; "N shortcut problem(s) on <host>; see Settings → Shortcuts." notice once per distinct problem set (`ws:682-710`).
- No status bar anywhere; no terminal size readout (DESIGN.md "Don't").

## 12. Sidebar

- Optional, toggle `Ctrl Shift B`, toolbar button, Settings switch, Commands; open state and width persisted (default closed on first run, `settings.rs:160,203`); default 236 pt, range 200-360; drag the 10 pt edge to resize, double-click resets (`ws:185-193`, `sidebar.rs:521-542`). Slides in/out with spring (`sidebar.rs:513-517`).
- Panel (rounded, 12 pt): header "Sessions" + `+` (opens the "Create session on…" host picker), "Find a session" search chip with `⌃K` hint (hard-coded) that opens the switcher, scrolling host groups, footer with "Add host" and total session count; scroll thumb and hint "More sessions and hosts below"/"Earlier sessions and hosts above" (`sidebar.rs:329-519`).
- Host group: disclosure chevron (collapse/expand, persisted), host-kind icon + uppercase label (click makes it the active host and expands it), per-host `+` (new session; empty slot kept when offline), right-click = host menu (`sidebar.rs:19-116`). Inactive/offline hosts use muted text.
- Session rows (see 3); "New session" row on empty connected host; "Waiting for connection…" + "Retry connection" when offline.
- Sidebar tab mode: sessions become 24 pt labels with tab rows beneath (see 4); tabs leave the toolbar only while the sidebar is open (`ws:1151-1153`).
- No search inside sidebar (uses the switcher), no keyboard navigation in the sidebar (rows are click-only; doc comment `sidebar.rs:1-2`), no drag-reorder, no host reorder, no pinning/favorites, no session activity badges, no per-host status dot.
- Prefetch of views for tab listing only when Sidebar tabs mode is on (`ws:888-904`).

## 13. Dialogs and panels (`dialogs.rs`)

- Modal frame: centered, 520 pt wide (<= 90% viewport), max 85% height, 22 pt padding, over a 50% black scrim without blur; scrim click = back/cancel; Esc = back; target pane outlined above the scrim (`dialogs.rs:13-48`).
- Forms: Cancel (neutral) / primary (`Save`, or `Connect` for add-host/discovered/SSH forms, default button); first field focused; inline error text in DANGER; helper notes (`dialogs.rs:54-150`).
- Form list: Terminal fonts (`Form::Fonts`: family + comma-separated fallbacks; applied on reopen), Client label, Default shell, Rename session / tab / pane, Add host by endpoint, Connect to Rex (discovered), Add SSH host (name, SSH user, socket), Edit host (`panel:160-177`, `ws:1235-1251,1263-1334`).
- Confirmations: Close tab, Close session, Close pane (buttons "Keep running" default tab index 1 / "Close and end programs"), Remove host ("Keep host"/"Remove host") (`dialogs.rs:152-207`).
- Start personal server dialog (`ssh_setup.rs`), agent-integration change preview (`AgentChange`, `src/app/integrations.rs`), Settings (760x520, eight sections: Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts, Updates, About) (`panel:179-227`, `settings_view.rs`; details in c01). Forms/pickers opened from Settings return to it (`ws:1700-1745`).
- List panels: terminal theme picker ("Terminal theme", filter "Filter themes…"), chrome theme picker ("Theme"), Add host list, host picker (`panel:229-264`).
- Closing a panel hands focus back to the terminal; Tab cycles fields; Esc backs out; opening any panel ends in-progress selections and clears pending key sequences (`ws:1693-1795`).
- Client actions returned from the server while a panel is open are skipped with notice "Returned client action skipped because its workspace context changed." (`ws:509-524`).

## 14. Search (terminal)

- `Ctrl Shift F` (`client.search`, also Commands "Search terminal…") opens a per-pane field "Search terminal" pinned 6 pt inside the pane's top-right (280 x 34, popover colour, magnifier icon) (`panes.rs:550-595`, `ws:923-928`). Searches the pane's local Ghostty replica (screen + scrollback) as you type (`Command::Search`), keeping up with new output; Enter selects next older match, Shift+Enter next newer; Esc closes and clears; counter `esc` (empty) / `i/N` / "No matches" (`input.rs:177-194`, `src/terminal/search.rs`, `src/connection.rs:2013-2032`). Matches are overlays: ACCENT 25%, selected 55%; the viewport scrolls to the selected match. While the field has the keyboard the pane cursor is hollow. Failures arrive as a "Search failed: …" toast.
- Not present: case-sensitivity, whole-word or regex toggles; search across panes/tabs/sessions; search history; Find next/previous shortcuts outside the field; highlight-all toggle; in-switcher scrollback search. Sessions/tabs are searched only via the switcher/Commands text filters.

## 15. Drag and drop

- File drop onto a pane: `on_drop_files` -> `drop_files(block, paths)`; the target pane is focused if needed; pane gets an accent border while a drag hovers (`panes.rs:238-251`, `src/app/upload.rs:299-309`). Ignored while a panel is open or when empty.
- Dropped data other than files (text, URLs, images, tabs, sessions) is not handled. Dropping on the toolbar, sidebar or tab pills does nothing.
- Dividers are draggable (see 5); the sidebar edge is draggable (see 12); the toolbar's empty space drags the window.
- No drag and drop of tabs, sessions, hosts or panes (no reorder or move via drag), no drag out to a new window.

## 16. Uploads (`src/app/upload.rs`, `src/upload.rs`, `src/connection.rs:1566-1630`)

- Triggers: dropped files, `Ctrl Shift V` (`pane.paste`) with files on the clipboard (copied files upload; else text pastes via `block.event paste`; else a clipboard image uploads as `clipboard.png`) (`src/app/upload.rs:313-337`). The clipboard read is asynchronous and applies only if the same pane is still focused and no panel opened.
- Requires API revision >= 11, else "this host's Rex server is too old to receive files; update Rex there"; only live terminals ("Files can be uploaded only to a live terminal.") (`src/upload.rs:349`, `src/connection.rs:1579`, `src/app/upload.rs:285-288`).
- Protocol: for each file `terminal.upload_begin` with `upload:<uuid v4>` id, name, exact size, and `batch`/`index`/`count` for multi-file drops, 8 s timeout, never retried; then each file streams on its own data ticket over the `rex.data.bytes.v1` WebSocket in 64 KiB binary chunks ended by an empty message; Rex closes 1000 after typing the batch's paths into the terminal; 120 s completion timeout; size change during upload is an error (`src/upload.rs:348-473`, `src/connection.rs:1593-1630`). Only regular files (folders/special files refused with a message).
- Feedback: none on success except the paths appearing in the terminal; no progress bar, no per-file status, no cancel, no queue UI. Failure = sticky toast "Upload failed: …".
- Block is fixed at drop/paste time, later focus changes don't redirect (`src/app/upload.rs:274-277`).
- No download, no file browser, no "upload file…" menu item or command, no folder upload, no size limit enforced client-side.

## 17. Keyboard shortcuts (Linux defaults) (`src/keymap.rs:370-414`)

`Ctrl K` session.switch . `Ctrl Shift P` palette . `Ctrl ,` settings . `Ctrl Shift B` sidebar . `Ctrl Shift N` new session . `Ctrl Shift T` new tab . `Ctrl Shift W` close pane/tab . `Ctrl Tab`/`Ctrl Shift Tab` next/previous tab across hosts . `Ctrl PageUp/PageDown` previous/next tab in session . `Ctrl Shift Enter` zoom pane . `Ctrl Shift Space` pane actions . `Ctrl Shift C` copy . `Ctrl Shift V` paste/upload . `Ctrl Shift F` search . `Ctrl Shift R` reconnect pane . interface zoom as in 6 . `Ctrl Shift D` / `Ctrl Alt D` split right/down . `Ctrl Alt Left/Right/Up/Down` focus pane . `Shift PageUp/PageDown` scroll +-20 lines. No default bindings: `client.history.*`, `client.tab.goto`, `session.rename`, `client.tab.rename/close`, `pane.close`, `session.select`, `pane.send_key`.
- Every binding removable/replaceable through the user-owned `shortcuts.json` (sequences with `>`); host Rex config layers beneath (`ws:624-680`); Settings → Shortcuts lists effective bindings. 32 registered client actions (`actions.rs:62-95`), each with a `remote()` decision: view-only subset (tab next/previous/goto, pane focus, session select, sidebar toggle, pane zoom, scroll) allowed only with per-host consent and outside panels/typing; everything else `Never` (`actions.rs:101-141`, `src/app/remote.rs`).
- Mouse: Back/Forward navigate history (`mod.rs:96-109`); middle-click tab closes (with confirm); double-click tab renames; double-click sidebar edge resets width; Ctrl+click link opens via `xdg-open`.
- Input flow: shortcuts first; other keys go to the focused pane via `client.key`; releases only for forwarded presses (`input.rs:207-294`); IME preedit/commit supported (`input.rs:296-311`).

## Absent or partial

- Take control: no button/strip; `Command::TakeControl` is a no-op stub (`src/connection.rs:1917`); DESIGN.md says a non-owner shows "Take control" (not implemented). Ownership is only claimed on focus-in and on input.
- Tab and session management gaps: no tab reorder, move-to-session, duplicate, pin; no drag reorder of sessions/hosts (menu Move up/down for sessions only); no host reorder; no sidebar tab context menu or rename/close gestures; no session-level activity indicators.
- Pane management gaps: no keyboard resize, equalize, swap, rotate, break-out to tab, move pane, split-before; no floating layer creation/minimize/restore (minimized layers merely hidden); no pane "zoomed" badge.
- Commands is limited to the 20 workspace entries and 6 pane entries listed in 10; no copy/paste/zoom-interface/theme/font/notification commands, no server actions.
- Terminal context menu absent (right-click opens the pane menu); no Copy/Paste/Select all/Search/Open link menu.
- Tooltips on the toolbar/sidebar show hard-coded shortcuts (`toolbar.rs:294,328,337,346`, `sidebar.rs:440`) even if the user rebinds them; only Commands and pane/tab menu details follow the effective keymap.
- Tab-close confirm does not name the tab or host (`ws:1299-1304`) though DESIGN.md says confirmations name them.
- Search: substring only, no options, single pane.
- Uploads: no progress, cancel, menu command, or success feedback; text/URL/image drag and drop unsupported (clipboard image paste uploads as `clipboard.png`).
- Single window; no detach, no multiple windows, no window restore of layout beyond last host/session and sidebar width/state.
- Close app is immediate; no "quit" confirmation (by design, programs keep running).
- Kitty graphics partial; AT-SPI accessibility absent (PRODUCT.md "Out of scope").
- `client.history.*` has no default keyboard binding (mouse only). `client.tab.goto` has none.
- Cross-server `client_call_requested` relaying not implemented (README:423).
- Mouse Back/Forward only act outside panels and during no divider drag.
- API revisions 10-13 supported; uploads need 11+ (PRODUCT.md, `src/upload.rs:349`).

## Constraints

- Server owns terminal content, colours and split geometry; the client draws chrome and may only declare a theme via `terminal.set_theme`, never recolour output or invent layout (AGENTS.md, PRODUCT.md, DESIGN.md). Pane placements, zoom and ratios come from the server; the client sends only `session.resize_pane`, `session.zoom_block`, `session.new_split` (anchor/side/direction), `block.close`.
- Attachment never starts/stops/reconfigures a server and never creates or destroys a session; creation and destruction are explicit user actions with named targets. A missing default server on SSH may be started once after a separate preview and confirmation (PRODUCT.md, AGENTS.md).
- Failed input is never replayed; a transport failure pauses panes (`input paused`), never queued (AGENTS.md, DESIGN.md).
- Destructive confirmations name the target and default to **Keep running**; closing the app leaves programs running.
- Terminal content never animates; chrome motion uses the springs in `theme.rs`; no status dots, "Connected" labels, terminal size readouts or keyboard-hint footers (DESIGN.md "Don't"). Healthy state shows nothing; words only for trouble plus program activity (Done/Bell/Paused/Error and progress bars).
- One working row: a single 52 pt toolbar with session identity, tabs, commands and window controls; navigation hosts -> sessions opens on demand, optional sidebar; Sidebar tab position moves tabs out of the toolbar only while the sidebar is open. No minimize/maximize.
- Server windows are called tabs; popups never open another popup from a row (switcher renames in place); panel sizes: switcher 340, Commands 460, menus 260; dialogs centered over a scrim; Settings 760x520 (DESIGN.md).
- Keyboard first: every workspace shortcut is removable/replaceable; ordinary keys flow through `client.key`; only explicit `pane.send_key` bypasses Lua bindings; sequences never leak prefixes. Remote availability is default-deny in `Spec::remote()`, per-host consent, replies bound to connection epoch; the allowed set test is pinned (AGENTS.md).
- Size ownership is automatic: selecting, copying or navigating must not claim ownership (DESIGN.md "Don't").
- Chrome palettes are dark only (Midnight default, Rosé Pine, Rosé Pine Moon, Gruvbox Dark); no light chrome (DESIGN.md). Interface zoom 70-150%; terminal font size is independent in DESIGN.md but the zoom setting text says chrome and terminals together (see c01).
- Links open only on Ctrl+click; clipboard reads by programs are unsupported; terminal clipboard writes limited to 8 MiB from the focused pane of the active session (README:434).
- Linux specifics: tiling WM friendly (no min/max), Wayland and X11, desktop notifications via freedesktop D-Bus, links via `xdg-open`, files dropped as local paths (uploaded, not linked).
- Keep README claims honest and add tests for protocol/state changes (AGENTS.md); vendored Rune/rexrs/ghostty-vt patches must stay minimal.
- For parity with macOS Rex Beta this means: native menubar, multiple windows, tab drag/reorder, a Take-control affordance and richer search options would need server-compatible, explicit-action designs; none may auto-create sessions, recolour output or alter server-owned layout.
