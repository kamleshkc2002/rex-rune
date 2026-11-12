# Changelog

## Unreleased

### Changed

- **Activity coverage:** after each connection, read every session's program status once, selected session first and the rest paced in the background, up to three reads at a time. Already-waiting tabs in unopened sessions now feed the Activity list, sidebar word and toolbar chip without notifications or read receipts. The list names reads in progress, failed reads and disconnected hosts. No polling or cap; see [measurements and limits](docs/parity-2026-11/M3.2-status-sweep.md).

- **Activity list:** the filter also matches a row's state (`error`, `needs`, `working`); opening the list from the toolbar chip starts on the first tab that is not the one in front; and while a saved host is not connected the list's one line says so, so an empty list beside a host that is down does not read as all clear.

### Added

- **Session activity in the switcher (partial M3.2):** the Switch session list shows the same word a sidebar session row does for a session whose tabs are out of sight (a needs-you word, Error, Bell or Paused, an unseen Done), so it can be read with the sidebar closed. The session on screen, muted tabs and the Activity in tabs switch keep it quiet; the word stays beside Rename and Close on the highlighted row. Nothing is sent to a server and nothing is marked seen. A host that is not connected no longer shows a word on its sidebar or switcher rows, matching the Activity list and chip. The identity chip itself is unchanged: the toolbar activity chip already counts what needs you. See [checks and limits](docs/parity-2026-11/M3.2-session-words.md).

- **Recently used paths (partial M3.1):** Insert path and Go to directory offer the last ten paths used on each saved host, shared by its sessions on this computer. Settings → Terminal → Show recently used paths is on by default; turning it off forgets the lists. Choosing a recent still only pastes, never presses Enter or checks that the path exists. See [checks and limits](docs/parity-2026-11/M3.1-recent-paths.md).

- **Activity toolbar chip (partial M3.2):** a chip in the toolbar opens Activity while known, unmuted tabs other than the one on screen need you (`3 need you`) or, with none waiting, have errors (`2 errors`). Working, Done and Bell keep the toolbar quiet, and so does the Activity in tabs switch. It never floats over the terminal, is not drawn below 720 pt of window width, waits for a key sequence or a download chip in windows under 1000 pt, never marks tabs seen and sends nothing to servers. Coverage now includes the per-connection status pass described above; an absent chip is not an all-clear while reading, after failed reads, or for missing events and suppressed activity. See [checks and limits](docs/parity-2026-11/M3.2-activity-chip.md).

### Fixed

- **Update instructions on Omarchy:** the update notice and Settings → Updates no longer offer `sudo pacman -Syu rex-linux-bin` as an alternative on Omarchy, where Omarchy's update guard refuses a direct system upgrade. They name `omarchy update` only. Arch without Omarchy is unchanged.

## 0.7.5 - 2026-11-12

### Changed

- **README screenshots:** refreshed the workspace image and added current Settings screenshots with reproducible native capture notes (#77).

### Added

- **Session activity in the sidebar (partial M3.2):** a sidebar session row whose tabs are out of sight now shows the most pressing word one of them would: a needs-you word, Error, Bell or Paused, or an unseen Done. The session on screen and sessions with their tabs listed under them are unchanged; muted tabs and the Activity in tabs switch keep it quiet. See [checks and limits](docs/parity-2026-11/M3.2-session-words.md).

- **Activity list (partial M3.2):** Show activity… in Commands groups known Needs you, Error, unseen Done and Working tabs across connected hosts and sessions, with filtering and navigation to a tab. Muted tabs stay quiet; browsing marks nothing seen and sends nothing to servers. The list explains its incomplete coverage. The action is unbound and configurable in Shortcuts. See [checks and limits](docs/parity-2026-11/M3.2-activity-list.md).

- **Go to directory… (partial M3.1):** Commands → Go to directory… browses the folders on the focused terminal's machine and types `cd` with the chosen folder, shell-quoted, at the prompt. It does not press Enter: the client cannot see what the prompt already holds, so running the line stays with you. Same captured pane, errors and refusals as Insert path; local-only and unbound by default. Recent paths remain deferred. See [checks and limits](docs/parity-2026-11/M3.1-go-to-directory.md).

- **Insert path… (partial M3.1):** browse the focused terminal's files and folders from Commands, then paste one POSIX-shell-quoted absolute path without Enter. The captured pane never changes; browsing only lists paths, errors stay inline and nothing retries automatically. The action is local-only and unbound by default. Recent paths remain deferred. See [checks and limits](docs/parity-2026-11/M3.1-insert-path.md).

- **Server settings (partial M3.3):** a read-only page shows the active host's cached version, API, process, listeners and built-in Tailscale facts, with explicit missing/stale states and sanitized address copy. Opening the page makes no requests or server changes. Pairing and server controls remain deferred. See [checks and limits](docs/parity-2026-11/M3.3-server-settings.md).

- **Notification quality and health (partial M3.4):** activity notifications replace the previous one for their tab, use critical urgency for Needs you/Error and close when the tab is seen. Failed sends stay dropped without ending the worker. Settings → Notifications reports service health and offers an explicit test, even with desktop notifications off. Program messages remain separate. See [checks and limits](docs/parity-2026-11/M3.4-notification-quality.md).

## 0.7.4 - 2026-11-11

One-click updates for eligible installer users, optional header fading, toolbar tab fill, and configurable tab icons with a preview.

### Added

- **Tab icons (partial M2.8):** **Show tab icons** in Interface → Sidebar and tabs saves a 0–5 count with a small live preview. The default preserves the existing single monochrome icon; additional icons use distinct cached pane programs, focused first, in toolbar and sidebar tabs. **Use colour for tab icons** adds Always / Active pane / Active tab / Never (default Never), tinting only the existing glyphs with an inert active/inactive preview. Shell-theme styling remains deferred. See [count checks](docs/parity-2026-11/M2.8-tab-icons.md) and [colour checks and limits](docs/parity-2026-11/M2.8-tab-icon-colours.md).

- **Tab fill (partial M2.8):** default-off **Tabs fill available space** in Interface → Sidebar and tabs shares spare toolbar track width without changing sidebar rows, terminal geometry or ownership. Existing scroll arrows, tab menus, live chips and native drag space remain. See [checks and limits](docs/parity-2026-11/M2.8-tab-fill.md).

- **Header fading (partial M2.7):** two default-off Interface → Panes switches fade header names and available buttons on inactive panes or in inactive windows. Activity, progress, exceptions and terminal content stay unchanged; no server/layout/ownership change from preferences. Focus follows mouse remains deferred. See [checks and limits](docs/parity-2026-11/M2.7-header-fading.md).
- **One-click installer updates:** Settings → Updates offers **Update now** for verified, user-writable install.sh installations under `~/.local`, with download progress, errors and **Restart to finish**. Signed manifest hashes and exact sizes are checked before transactional replacement of the client, launcher, icons and receipt; failures restore the previous files. **Copy command** remains available. Optional **Install updates automatically** is off by default and never restarts the app for you. Package-managed and source builds keep their existing guidance.

## 0.7.3 - 2026-11-11

Toolbar visibility, terminal display and selection controls, font picker and metrics, and pane padding and active-pane outline preferences.

### Added

- **Pane ergonomics (partial M2.7):** independent horizontal/vertical padding (0–32 pt) and an Active pane outline choice (Auto/On/Off). Defaults preserve legacy uniform padding and density-following decoration. These preferences never claim ownership, alter shared split layout or dim/recolour terminal output. Focus follows mouse and header fading remain deferred. See [checks and limits](docs/parity-2026-11/M2.7-panes.md).

- **Font controls (M2.6):** cached searchable font-family picker with Monospaced/All filters, current checks and requested-face samples; font style and an Advanced font settings subpage with line height, character spacing, program bold/italic choices and an ordered editable fallback list. Family changes still apply on reopen and CLI overrides still win. Live metrics share one grid across drawing, input and owned-pane resize reports; defaults and ownership boundaries are unchanged. See [verification and limits](docs/parity-2026-11/M2.6.md).

- **Terminal display and selection controls (M2.5):** steady Typography, Cursor and Selection previews; default cursor shape and an optional Force style; opt-in copy on selection, PRIMARY selection/middle-click paste, and rounded selection corners; cell-derived selection colour choices and a switch for shortcut-sequence hints. Existing title/font keys and default rendering remain unchanged. Cursor policy is local, program mouse reporting takes precedence over PRIMARY paste, and copying never claims pane ownership. See [verification and limits](docs/parity-2026-11/M2.5.md).

- **Toolbar visibility (focused M2.7):** Settings → Interface → Toolbar visibility offers Always show (unchanged default), Show on hover and Hidden. Hidden modes give the 52 pt row to the terminals; hovering the top edge or **Reveal toolbar** (`Ctrl Alt Shift T`, Commands and Shortcuts) floats it without resizing their content. The action is local-only and temporary. Downloads and pending key sequences still have visible chips; notices and window-manager Move/Close remain available. This recovers only the toolbar slice of preserved W2, not its other interface changes. See [scope and checks](docs/plans/m2.7-toolbar-visibility.md).

## 0.7.2 - 2026-11-11

Searchable Shortcuts and a shortcut recorder, previous and next shell prompt navigation, Settings search and Back/Forward navigation, and opt-in generated pane and tab titles adapted from Matt Porter's work.

### Added

- **Shortcut recorder (M4.4):** replace, remove or record a chord/sequence on a Settings → Shortcuts row; focus, Cancel, IME isolation and conflict warnings stay local. An app-owned `recorded_shortcuts` layer in workspace.json wins above shortcuts.json without rewriting it, retaining named modes and conditional arguments. Configuration offers path/JSON copy and confirmed Reset all; Commands supports Change shortcuts… and Ctrl Enter to edit a valid command binding. Argument-only references remain read-only.

- **Generated titles (M2.10):** Settings → Terminal → Prefer generated titles names default tabs and panes from the foreground program and directory (`vim - ~/rex`), while user names and program-set titles win. Off by default, including old settings files; toolbar, sidebar, pane menus and window titles agree. Adapted the untitled-pane naming and home-directory retention from [Matt Porter (@ohporter)'s `e94fe1d`](https://github.com/ohporter/rex-rune/commit/e94fe1da4725b80a412c27be6fd01a00c1ec1caa), using per-terminal metadata and the shared title resolver rather than a pane-only fallback.
- **Settings search and navigation (M2.3, focused recovery):** Search Settings finds existing pages and labelled settings by their text and section; Enter or a click opens a result and briefly highlights its row. The grouped sidebar gains authored icons. Back/Forward buttons, Alt+Left/Right and mouse Back/Forward revisit pages at their previous scroll positions, within Settings only. The read-only Shortcuts filter remains separate. No new preference pages or subpages are exposed.
- **Previous / next prompt (M3.7, #44):** `Ctrl Shift Up/Down` and Commands navigate shell OSC 133 prompt groups in the local viewport, including restored scrollback. `client.prompt.previous/next` now work; `pane.prompt_previous/prompt_next` are aliases. The new default keys are consumed by Rex rather than sent to the terminal; remove or replace either binding in `shortcuts.json` to restore program input. Unmarked terminals do not move. Navigation never sends PTY input, claims ownership, changes server layout or animates; all spellings remain remote-denied. Verified on a disposable Rex API 13 server through snapshots, live output and reattachment.
- **Searchable Shortcuts reference (M2.1):** filter Settings → Shortcuts by action, category or key chord; supported unbound actions read **Not set**, alternate bindings share one row, and bound aliases retain their names. Sources, mode-scoped conflicts, unsupported diagnostics and config paths remain visible. A literal `+` reads **Plus** in keycaps. The reference remains available alongside the M4.4 recorder.

## 0.7.1 - 2026-11-11

Expanded Rex action compatibility for shared keybindings, a single installer summary, and a relay design document. Desktop notifications gain the Rex icon and the window remembers its size, both harvested from Matt Porter.

### Added

- **Shared Rex keybinding compatibility (#44):** config reload and config-path copy, directional pane aliases, server-owned pane resize/move/balance, close other panes, moving to an existing tab, host actions, page scrolling and host-defined key modes. Settings → Shortcuts also lists unbound actions and marks aliases. All new actions are local-only; the remote allowed set is unchanged. Prompt navigation and moving a pane to a new tab are recognized with explicit unsupported diagnostics, not implemented yet; #44 remains open.
- **Rex icon in desktop notifications:** embeds the 128 px icon as the freedesktop `image-data` hint, so notifications show it without an installed icon file. Harvested from [Matt Porter (@ohporter)](https://github.com/ohporter/rex-rune/commit/f73baf449b84a52a44eee2ee741cfb23ef34fb1d).
- **Remember window size:** reopen at the last non-maximized size, saved once resizing settles for one second; `--window-size` still overrides it and the 640×480 minimum still applies. Adapted from [Matt Porter (@ohporter)](https://github.com/ohporter/rex-rune/commit/97d7e3195585ca1b17031054484a33a5956e3533) onto the current Settings.

### Fixed

- **`install.sh` no longer prints the post-install summary twice:** `packaging/install/install.sh` now suppresses the duplicate summary from the bundled `scripts/install-desktop.sh` (`REX_INSTALL_QUIET_SUMMARY=1`) and prints the three-line summary (with both the `rex-linux` binary path and the `.desktop` launcher path) once, while running `scripts/install-desktop.sh` directly still prints it once.

## 0.7.0 - 2026-11-11

File downloads from Rex API 13 servers, Rune bumped to `d70929a` with accessibility on, a fix for window focus (an unfocused window wrote terminal copies to the clipboard), and the first milestone of macOS-app parity: grouped menus, pane header buttons, a find bar, tab management, every command in Commands and a rebuilt Settings.

### Added

- **File downloads (Rex API 13):** **Download file…** in the pane menu and Commands (action `pane.download`, no default key, never available to `client.perform`) saves a file from the terminal's machine into the XDG download folder, or `downloads.directory` in `workspace.json`, which **Change** in the dialog sets. Errors before any bytes stay in the dialog: Rex's own words for the file errors (no such file, a folder, a FIFO, no permission), the client's own sentence for a server older than API 13. A toolbar chip and a Downloads popover show progress, **Cancel** and, for a finished file, **Show in folder** (the file manager's `ShowItems`, else `xdg-open` on the folder). Files are never overwritten (`name (1).ext`), never opened by the client and never made executable. There is no **Open**: a file's name and content cannot pin which program a desktop runs, and the default program for a type is not Rex's to vet, so open a saved file from your file manager. Files of 1 GiB or more, or that would leave under 256 MiB free, ask first with **Cancel** focused; three transfers per host at once; growth and stall limits; nothing is retried or resumed; partial files are removed on cancel, failure and exit, and a killed app's at the next launch. Quitting cancels running downloads without asking, and nothing is resumed. A file that changes while it is sent can be saved partly updated or shorter than Rex reported (the notice says when the size differs); the download folder must already exist. Checked end to end with `scripts/download-acceptance.py`; see the README's Downloading files section.
- **AT-SPI accessibility tree**, from Rune's new default `accessibility` feature, which stays on. Checked with an AT-SPI client script on a private bus; not tried with Orca or another screen reader. Chrome buttons, tabs, switches, segmented controls, text fields (with caret, selection and edit events), dialogs and notices have roles, names and states, and notices raise announcements. The terminal pane exposes its visible rows as plain labels only: no text interface, caret, selection or focus. See the README's Accessibility section for what was observed and what is missing.
- **Accessibility names and states from that check:** the active toolbar and sidebar tab reports selected; dialogs are named by their title; each terminal pane is named `Terminal <pane name>`; and the switcher and Commands report the highlighted row (as an active descendant), so an AT-SPI client is told which row is highlighted as you arrow through the list.
- **Server version in About:** each host connection now asks its Rex server for its version once with a read-only `server.version` request, and keeps the status the session list already carries (process, sessions, listeners, Tailscale state). Settings → About shows `Rex Server <version>` for the active host with the negotiated API revision and the range the server supports; a server that doesn't answer is shown as not reported and nothing else changes.
- **Copy debug info**, next to **Copy version info** in Settings → About: the version info plus the install channel, display server, desktop, interface theme and zoom, terminal font, and each saved host's kind, connection state, API revision and server version. Neither copy contains a host name, address, session name, client id or credential. Commands gains **About Rex Linux**, **Copy version info** and **Copy debug info**, and About links to the issue tracker.
- **Pane menu verbs:** right-clicking a pane now offers Copy, Paste, Select all, Copy pane ID, Clear terminal and Reset terminal, beside splits on all four sides (right, left, down, up). Every row acts on the pane that was clicked; Copy needs a selection, and Paste, Clear and Reset a live terminal. Clear and Reset send Rex's `terminal.clear` / `terminal.reset` once and are never sent again after a failure; they are covered by tests against a scripted server, not yet tried against a real one.
- **Pane header buttons and a Pane headers setting:** a pane's header ends in Split right, Split down, Zoom (or Restore) and Close, which act on that pane, and shows a `Zoomed` chip. Settings → Interface → Pane headers offers Auto (the default, and what earlier versions did), Always, On hover and Hidden; **Reveal pane headers** in Commands floats them until it is run again.
- **Find bar:** terminal search has a match counter, Find next, Find previous and Close buttons, and moves to another corner rather than cover the selected match. It takes whole rows and columns of the terminal, so no sliver of a row shows beside it. Unselected matches are a light tint with an outline, so dim terminal text stays readable on them. Find next and Find previous are actions too and reopen the last search on a pane whose bar is closed.
- **Tab management:** the tab menu, on toolbar tabs and now on sidebar tab rows, gains Mute tab (no activity word, progress bar or notification from that tab on this client), Move left and right (`Ctrl Shift PageUp` / `PageDown`) and Close other tabs…; the rename dialog offers **Use default title** for a tab you named. Sidebar tab rows take middle-click to close and double-click to rename.
- **Commands lists every command**, under Session, Tab, Pane, Host, View and App headings that name what is on screen, about twenty rows at a time, and searches them as one list, best match first. New rows include Next and Previous session, Refresh sessions, Close session…, Focus next and previous pane, Split best fit, Collapse other hosts, the theme lists, pane density and tab position, interface zoom, terminal font size, About Rex Linux, Rex help and Report an issue. A host's own actions that need no arguments are listed under Server actions.
- **Terminal font size actions:** Increase, Decrease and Reset terminal font size, in Commands and bindable in `shortcuts.json` (`client.font.increase` / `decrease` / `reset`), change the size by a point or reset it. They have no default keys, so `Ctrl Alt +` / `−` / `0` still reach terminal programs; `Ctrl +` / `−` / `0` stay interface zoom.
- **Keycap hints everywhere from the keymap:** menu rows, Commands, the switcher, the sidebar's search chip and toolbar tooltips show each action's current binding, so a rebind in `shortcuts.json` shows everywhere at once. 26 actions are new this cycle; all are bindable, listed on Settings → Shortcuts once bound, and refused to remote callers.

### Changed

- **Clearer client-only install output and docs:** `packaging/install/install.sh` and `scripts/install-desktop.sh` now print the exact installed desktop client path (`rex-linux`), state explicitly that they do not install or change the `rex` server, and report the detected `rex` server path on `PATH` (or note that a Rex server must be installed separately). The README and release/operator docs also state at the top of the install section that every install channel (`rex-linux-bin`, `.deb`, Nix, tarball and `install.sh`) installs only `rex-linux`, its launcher and icons, never touching `rex` (and explain why `which -a rex` can show both `/usr/bin/rex` and `/bin/rex` on `usrmerge` systems).
- **Rune bumped to `d70929a`** (anomalyco/rune `dev`, 920 commits after `100b7d9`), with the vendored window-chrome patches rebased onto it. Two patches are dropped because Rune now provides them: `Tx::toggle_maximize` (now `Tx::toggle_maximized`) and the `fc-list` system-font discovery. The Linux window icon moved into Rune's `rune::set_app_icon`, whose Linux support is still a Rex patch. Scrolling and redraw cost far less: on the acceptance machine, p95 frame CPU while scrolling btop fell from about 35 ms to about 8 ms.
- **Rex now takes Rune's rendering as it is.** Translucent 1 pt rims on chips, popovers, pills and cards are slightly brighter, because borders draw over the fill; at 1.25× and 1.5× they are exactly one device pixel wide. Text line boxes are no longer rounded up to device pixels, so Settings rows are 1 pt tighter. Text uses Rune's new default rasteriser, which is heavier at 1× and unchanged at 2×. Terminal cell size and column positions are unchanged.
- **Accessibility costs a little while the bus is enabled.** The release binary is about 13 MB larger after the bump, about 5 MB of it the accessibility feature. The tree is built on every redraw whenever `org.a11y.Status IsEnabled` is set, even with no screen reader attached: during a 20 Hz full-screen redraw the app used about 0.71 CPU cores with it enabled and 0.59 with it disabled (three rounds on a busy machine; the pre-bump build used 0.66).
- **About page:** a centred header with the app icon, version, install channel and the active server's version replaces the version rows; commit and build date share one row. Versions are printed one way everywhere (`0.6.0`, no `v` on Settings → Updates).
- **Settings layout:** Settings is a wider dialog (816×680, clamped to the window and kept under the toolbar) whose pages are grouped into headed cards with a divider between every row, one-sentence helpers and controls aligned to one edge. The window palette, terminal theme and notice position are popup buttons whose lists open over Settings beside the control instead of replacing it; font family, client label and each host's default shell are rows that open their form; pane density and tab position are segmented controls with glyphs. Long pages fade at the scrolled edge and show a scroll thumb. Settings and saved preferences are unchanged. Settings → Shortcuts lists each binding by the name Commands uses, with its keys on the right as the same keycaps menus show.
- **Add host list:** a machine's name stays whole and what the check found is cut with an ellipsis instead (its tooltip has the rest), so a long name no longer collapses to `…` or pushes text past the list. Rows line up in columns (status, system, chevron); **Rex check failed** and a refused hello are in the warning colour and come first; a machine that is already added has an **Added** chip in place of the check mark before its name; and every group (Rex servers, Tailnet, Offline) has an icon. The list is 520 pt wide.
- **Integrations page:** each agent is one row with its mark, a state chip (Installed, Update available, Not installed, Not found, Conflict, Unreadable) and the actions that state allows. The skill several agents share is explained once under the list instead of on every row, **Check again** and **Set up all detected…** are right-aligned with the primary last, and the footnotes are shorter. Previews, per-agent actions and the session-naming default (off) are unchanged.
- **Settings button:** the toolbar's Settings glyph is a gear instead of two sliders.
- **Menus are grouped.** Pane, tab, session and host menus separate their groups with hairlines and end with the rows that close or remove something, in red. The keyboard-selected row is a tinted fill with an outline. Context menus are 280 pt wide.
- **New default bindings take keys from terminal programs:** `Ctrl Shift PageUp` / `PageDown` (move tab) no longer reach the focused pane. Each can be removed or rebound in `shortcuts.json`.
- **Window title** is `<tab> - <session> · <host>` (`Rex · <host>` before a session is chosen). A sidebar session row counts its tabs only above one, and the toolbar's identity chip is sized to its words, with the host line only while a second host is connected or the host is reconnecting.
- **Close confirmations name the tab** (`Close <tab>?`) and its host. The pane menu opens on its first row that can run, so `Ctrl Shift Space` then `Enter` pastes (or copies, with a selection) instead of splitting right.
- **Interface zoom actions are named** Increase, Decrease and Reset interface zoom, and their hint is `Ctrl +` rather than the numpad key.

### Fixed

- **An unfocused Rex window no longer overwrites the system clipboard, and its cursor is hollow.** Rune delivers native window focus to the view's first element, but Rex registered its handler on a child of it, so the app never learned that its window lost focus: terminal OSC 52 copies were still applied (the window-focus gate for them never closed) and the focused pane kept a solid block cursor. Window focus now also drives other behavior that was dead before: a bell, Done or notification in the visible tab while the window is unfocused shows the tab's word and can fire a desktop notification, a drag selection ends, the key-sequence state resets and the input method is turned off on blur, and on refocus shortcut override files are reloaded (as the README has long said) and the window re-claims size ownership (with two Rex windows of different sizes on one session, the terminal reflows when you switch between them). Checked on X11 and on a private Wayland compositor; the 0.6.0 release behaves the same way, so this is not new in the Rune bump. Found while looking into a report that OSC 52 copy doesn't work in the Codex CLI (reported by akakabuto).
- DESIGN.md described a **Take control** strip for panes another client had sized; the client never had one. It now says what happens: the window claims its panes when it comes forward and typing or pasting claims a pane, with **Retry** on a reconnecting pane.

### Documentation

- **README: Clipboard (OSC 52).** What copies reach the system clipboard and when, that reads are never answered and large payloads are dropped, and what to do for programs that only send OSC 52 when an `SSH_*` variable is set (the Codex CLI, Neovim 0.10.x, LazyVim) or prefer the server's own clipboard. Rex Linux cannot set environment variables in a remote terminal; the page gives the server-side workarounds that were checked. Reported by akakabuto. This does not change how Codex decides where to copy.

## 0.6.0 - 2026-11-10

### Added

- **In-app update notice and Settings → Updates:** on startup (after 30 seconds) and at most once every 24 hours while running, checks the signed GitHub release manifest (`latest.json` and `latest.json.minisig`) against the embedded CI and backup Minisign public keys (`packaging/keys/release.pub` and `packaging/keys/release-backup.pub`). When a newer stable release is verified, shows a non-blocking update toast with **What's new** (opens the release page in your browser without rendering remote HTML or markdown), **Updates…** and **Dismiss for this version**. Guidance matches the detected install channel (`REX_LINUX_CHANNEL`, `/usr/share/rex-linux/channel` and `~/.local/share/rex-linux/install.json`): `sudo pacman -Syu rex-linux-bin` on Arch (`omarchy update` on Omarchy), the verified `install.sh` one-command upgrade for tarball/script installs, `.deb` upgrade instructions for Debian/Ubuntu, and `nix flake update rex-rune` / `nix profile upgrade rex-linux` for Nix. Includes `rex-linux --check-update`, a **Check now** action, and opt-out via Settings → Updates (`updates.check` in `workspace.json`) or `REX_LINUX_NO_UPDATE_CHECK=1`; source builds never check automatically, background check failures back off for 24 hours silently, and the client never self-updates or downloads binaries.
- **Pane density and padding:** Settings → Interface → **Pane density** adds a **Compact** mode alongside **Comfortable** (default), removing split pane headers, rounded card borders, shadows, gaps and outer margins in favour of 1 pt split separators while keeping split geometry, divider dragging and floating reconnect controls. Settings → Terminal → **Pane padding** adjusts uniform terminal content padding from 0 to 32 pt (default 8 pt) live without restarting. Adapted from [Loric ANDRE (@LoricAndre)](https://github.com/LoricAndre) ([LoricAndre/rex-rune](https://github.com/LoricAndre/rex-rune), `fd4e753`, `e0be986`, `d3a9cbe`).

### Changed

- **Uniform default pane padding:** terminal content now uses uniform 8 pt padding on all four sides in every pane by default (replacing the previous 12 pt horizontal, 8 pt bottom, and 0 pt split-card top padding), so single panes and split cards share identical inset geometry at a given **Pane padding** setting.

## 0.5.3 - 2026-11-10

### Added

- Signed Arch pacman repository at `https://pkg.shuv.dev/arch/$arch`, with x86_64 and aarch64 packages, signed repository databases and package signatures. See the [setup and migration instructions](docs/auto-update-operator-walkthrough.md#5-the-arch-repository-u2).
- Install channel markers and installer receipts; About identifies the installation channel. Nix installation guidance is available in `docs/nix.md`.

## 0.5.2 - 2026-11-10

### Added

- Signed one-command user installer, published as immutable `install.sh` and `install.sh.minisig` release assets. Embedded CI and backup keys verify release checksums with minisign or OpenSSL 3+ before any downloaded code runs. Supports upgrades, checks, explicit version pins and uninstall; refuses automatic downgrades. Repository-key fallback is opt-in only (`--trust-repo-keys`).

### Changed

- Support Rex API revisions **10–13**, including the official Arch snapshot `0.0.1794323808-g757adf`. Refresh the vendored SDK for the additive `terminal.download_begin` method; existing attachment, upload gating and per-host remote-control consent remain unchanged. File-download UI is not enabled. See [API-13 compatibility notes](docs/server-api13.md).

## 0.5.1 - 2026-11-10

### Added

- Settings → About with app version, build commit/date, connected server API revision, repository/release/license links and Copy version info for bug reports.

## 0.5.0 - 2026-11-10

### Added

- Opt-in Remote control per saved host (off even locally), eight view-only client actions, explicit refusals with server-command alternatives, host/target/panel/typing guards, five-second stale-request limits and reconnect-scoped replies. Every performed request names its host and requester in a six-second in-app notice.
- `session.select session_id=… [window_id=…]` shows a session on the requesting host, with acknowledgement, a five-second deadline and history integration. `client.tab.goto index=-1` selects the last tab. Checked live against private preview.14 servers, including a real SSH host.
- A client label in Settings → Remote Hosts, defaulting to the hostname and sent in HELLO; edits apply after reopening.

- Signed releases with minisign signatures for `SHA256SUMS` and `latest.json`, protected release approval, package provenance attestations, and committed CI and offline recovery public keys (#21, #28). No client auto-update is included.

### Changed

- Update GitHub Actions checkout to 7.0.1, upload-artifact to 7.0.2, and mise-action to 5.2.7 (Dependabot #22–#24).

### Verifying downloads

Verify the metadata signatures before checking package hashes. See the [README's minisign public keys and verification instructions](https://github.com/shuv1337/rex-rune/blob/v0.5.0/README.md#verify-a-download). Releases through 0.4.2 remain unsigned.

## 0.4.2 - 2026-11-09

### Added

- Offer a separately confirmed, one-time personal-server start when SSH finds no default Rex server. Preview the target and command, default to Not now, check local/remote API-12 support, report failures without retrying, and reconnect through the unchanged `ssh -L` transport. Ordinary attachment never starts servers; installation remains manual.
- Add host reads API-12 `server.status` listeners, showing the verified endpoint and a review of bound addresses and admission rules. Older servers and unavailable metadata retain attachment without guessed access rules.
- New tabs and splits start in the source pane's shell directory on its server, including shells that do not report OSC 7; a pane-menu split inherits from the pane the menu names. Missing or slow directory reads fall back to the server's default; new sessions are unchanged. (@kitten-lily)

### Fixed

- Clicking a disabled control, divider or padding inside a dialog no longer dismisses it. Empty space preserves field focus; clicking outside still closes the dialog. (@kitten-lily)

## 0.4.1 - 2026-11-09

### Changed

- Support Rex server 0.1.0-preview.14 (Linux build `7e460d0`), negotiating API revisions **10–12**. Refresh the vendored SDK capture and generated types for optional listener metadata in `server.status` and `session.bootstrap`; uploads still require revision 11.
- When SSH finds no Rex socket, explain attachment-only behavior and suggest the manual `rex server install ssh://host` setup. Socket lookup explicitly disables autostart; existing OpenSSH forwarding, aliases and custom sockets remain supported.

### Fixed

- Answer remote `client_perform_requested` actions with `not_supported` and an explanation instead of leaving agents waiting for a timeout. Remote interface control remains unimplemented; local shortcuts and their queued actions are unchanged.

See [preview.14 compatibility notes](docs/server-preview14.md) for the exact protocol diff, test results and deferred features.

## 0.4.0 — 2026-11-09

Changes since 0.3.2, including work adapted from [Joe Goldin](https://github.com/joegoldin) ([joegoldin/rex-rune](https://github.com/joegoldin/rex-rune)), [Lily](https://github.com/kitten-lily) ([kitten-lily/rex-rune](https://github.com/kitten-lily/rex-rune)) and [Andrea Zito (nivox)](https://github.com/nivox). Thank you; their contributions are credited below and in the commits.

### Added

- **Tailnet host discovery:** Add host lists and searches your Tailscale machines, using Tailscale IPs when MagicDNS is unavailable. Online machines are checked for Rex at the addresses the official clients use; connect to a detected server with one press, or enter an SSH login and optional socket path. Manual endpoints and multiple logins to the same machine remain supported.
- **SSH socket forwarding:** connect with `ssh://[user@]host[:port][/socket]` using an existing SSH login, including Tailscale SSH, without reconfiguring Rex. The client discovers the remote Rex socket when no path is given and forwards it with `ssh -L`. Each client instance keeps its own forwards; disconnects show SSH's reason next to Retry.
- **Global navigation and visit history:** Ctrl Tab and Ctrl Shift Tab cycle through tabs across connected hosts and sessions in sidebar order. Mouse Back/Forward and Commands revisit this window's history, including across reconnects, skipping unavailable targets. Rapid cross-session cycling briefly settles before attachment without enabling extra prefetch; Ctrl PageUp/PageDown still use the server's current-session navigation. (@joegoldin)
- **Configurable shortcuts through Rex's `client.keymap`:** bindings in a host's Rex config now apply, layered with Linux defaults and user overrides in `~/.config/rex-linux/shortcuts.json`. Rebind or remove any workspace shortcut, including Ctrl Tab, to let it reach terminal programs. Supports multi-key sequences, layout-aware logical and physical keys, Lua forwarding and server actions, plus explicit `pane.send_key`, tab selection and pane resizing. (@nivox)
- Settings → Shortcuts shows effective bindings, their sources, file paths and Problems; Commands follows rebinding and a toolbar chip shows pending sequences. Override files reload on window focus or opening Settings and are not rewritten by workspace saves. Invalid files leave defaults working, and unavailable server keymaps use local defaults plus overrides. Custom bindings from nivox's format import once without copying defaults or replacing an existing override file. (@nivox)
- **Terminal search:** Ctrl Shift F searches the focused pane's screen and scrollback with Ghostty's search. Matches are highlighted; Enter and Shift Enter step through them, and Escape returns the keyboard to the terminal. Search-field typing and clicks stay out of the running program. (@kitten-lily)
- **Activity, progress and bells:** OSC 9;4 progress appears beneath tab labels and split-pane names, with OSC 7501 percentages as a fallback and words for Paused or Error. Each split pane shows its own activity and, unless renamed, its program's terminal title. An unseen terminal bell marks its tab Bell until viewed. Progress clears with its reporting program but survives while a child holds the foreground. (@kitten-lily)

### Changed

- **Terminal fidelity:** programs can select block, bar or underline cursors; curly, dotted, dashed and double underlines use their requested colours, and overlines are supported. Box-drawing and block characters meet on whole device pixels without seams, and decoration thickness respects fractional display scaling. (@kitten-lily)
- A pane owned by this client waits up to half a second for a frame at its own size before drawing, avoiding a flash of the previous client's dimensions without waiting indefinitely. (@kitten-lily)
- Settings → Terminal identifies saved font changes that need a client restart and font families that are not installed; saving a family change also shows a restart notice.

### Fixed

- SSH connections no longer get stuck reconnecting when SSH config enables `ControlMaster auto`. Forwards stay off shared control masters and keep their login open for the connection's lifetime.
- Silently dropped server connections, such as after laptop sleep, are detected within about 30 seconds using TCP keepalives so hosts and terminals can reconnect. (@kitten-lily)
- Shift Space, Shift Tab and Shift Enter reach Kitty-keyboard programs such as Claude Code as distinct keys. Shift is retained unless the keyboard layout used it to produce the character. Held within-session navigation works again, stale consumed-key state no longer swallows fresh presses, and forwarded terminal input releases focus from UI controls. (@kitten-lily for modified-key handling)
- Search highlights and selected matches stay correctly positioned in clipped or scrolled panes. Closing search during a host reconnect clears its highlights, and a failed search reports a notice instead of dropping the terminal stream. (@kitten-lily)
- Back/Forward retains the right history position after failed or superseded navigation; expired queued visits no longer attach later, and selecting a reconnecting host's session no longer bounces back. (@joegoldin for the navigation foundation)
- Terminal sizes, themes and activity are restored before refreshing shortcuts after reconnect. Local action-request errors no longer disconnect the host, returned actions can finish after pane-focus updates, and changed overrides also reach hosts that were offline.
- The X11 smoke test now holds modifiers through key releases and checks pointer selection at the correct cell boundary. It requires an explicit disposable server and stubs tailnet discovery rather than contacting real tailnet machines.

## 0.3.2 — 2026-11-06

- First prebuilt Linux releases for x86-64 and ARM64: user-installable tarballs and Debian/Ubuntu packages, with SHA-256 checksums.
- Automated native builds and tests on Ubuntu 24.04. No application behavior changes since 0.3.1.

## 0.3.1 — 2026-11-06

- Only Rex API revisions 10 and 11 are supported; servers older than revision 10 are refused with a message to update Rex. The revision-7 and revision-8 compatibility paths and `scripts/revision8-check.py` are gone.

## 0.3.0 — 2026-11-06

Changes since the 0.2.0 public release on 2026-11-01. Many of them come from two forks; thank you to [Joe Goldin](https://github.com/joegoldin) ([joegoldin/rex-rune](https://github.com/joegoldin/rex-rune)) and [Lily](https://github.com/kitten-lily) ([kitten-lily/rex-rune](https://github.com/kitten-lily/rex-rune)), credited on each item below and as authors or co-authors of the commits.

### Terminal

- Drop files on a pane, or paste copied files or a screenshot with Ctrl+Shift+V, and Rex uploads them to the machine that terminal runs on and types their paths, local or remote, with no SSH setup. Needs a Rex server with API revision 11; older servers say so. ([#2](https://github.com/shuv1337/rex-rune/issues/2), requested by @joegoldin)
- The client speaks Rex API revisions 7 to 11, and doesn't draw floating layers another client minimized.
- Inline images with the Kitty graphics protocol, restored when you reattach. Adds a patched copy of `ghostty-vt` under `vendor/`. (@joegoldin)
- Copies made by terminal programs (OSC 52) reach your clipboard, accepted only from the focused pane while the window is focused. Remote programs still can't read your clipboard. (@joegoldin)
- Double-click selects a word or shell path and triple-click a line; a selection stays attached to its text while output scrolls, and dragging past the edge autoscrolls. (@joegoldin)
- Ctrl+click opens web and mail links, including OSC 8 hyperlinks; the tooltip shows the real target. `file://` links are never opened. (@kitten-lily)
- Full-screen programs no longer flicker while they redraw (synchronized output). (@kitten-lily)
- Tabs with a server-generated name show their program's title, as tabs called "Shell" already did. (@joegoldin)
- Terminal themes: Settings → Terminal → Theme sends a theme to the server with Rex's `set_theme`, so programs that ask for colours get the ones on screen. Other clients viewing a terminal see the theme while this client owns it, as they see its size. Choose Rex's default colours, Midnight, any Ghostty theme you have installed, or follow the chrome palette. The default, Follow server, changes nothing. (@kitten-lily)
- The terminal font family and fallback families are saved in Settings → Terminal; only the families you name are loaded. (@joegoldin, @kitten-lily)

### Keyboard

- Dead-key accents are typed into the terminal, including an accent followed by Space. (@kitten-lily)
- Programs can tell Shift+Enter, Shift+Tab and Shift+Backspace from the unshifted keys. (@kitten-lily)
- Key releases are sent correctly when nothing in the window has focus. (@kitten-lily, @joegoldin)
- Alt+arrows reach terminal programs (word movement in shells and editors); switching panes moved to Ctrl+Alt+arrows. (@joegoldin)
- Ctrl+Shift+V pastes only into the pane that asked, even if focus moves before the clipboard read returns.

### Window and navigation

- Chrome palettes, switchable live in Settings → Interface → Theme: Midnight (the default, unchanged), Rosé Pine, Rosé Pine Moon and Gruvbox Dark. (@joegoldin, @kitten-lily)
- Program activity on tabs (OSC 7501: "Needs approval", "Error", "Done" and so on), program icons, and desktop notifications for tabs you aren't looking at.
- Settings in sections (Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts), including adding, editing and removing hosts.
- A single pane draws edge to edge; tabs and toolbar restyled.
- Tabs in the sidebar: Settings → Interface → Tab position → Sidebar lists each session's tabs under it while the sidebar is open. (@joegoldin)
- The sidebar opens at launch as you left it. (@kitten-lily)
- A Settings button in the toolbar; the toolbar's buttons share one plain hover style, and the minimize and maximize buttons are gone.
- Edit a host (name, endpoint, default shell) from its sidebar menu, from Commands or from an offline session's menu.
- Rename and close sessions in place in the switcher and from the session chip.
- The pane menu opens on right-click even over programs that track the mouse.

### Integrations and packaging

- Settings → Integrations installs, updates or removes Rex's skill and session hooks for Claude Code, Codex, OpenCode and Pi with the local `rex skill`, always previewed first.
- OMP (oh-my-pi) in Settings → Integrations. It shares Pi's skill directory, so it uses Rex's Pi setup until Rex knows OMP. (@kitten-lily)
- `rex` is also found in `~/.local/bin` (or `$XDG_BIN_HOME`), not only on `PATH`. (@kitten-lily)
- A Nix flake with pinned packages and desktop integration; see [docs/nix.md](docs/nix.md). (@joegoldin)
- Optional mise tasks, and `scripts/install-desktop.sh --copy` / `--uninstall`. (@kitten-lily)
