# Rex Linux vs Rex Beta (macOS): parity gap report

Rex Linux 0.6.0 (commit `ad196ff`, Rune) against Rex Beta 0.1.0 (1932) for macOS, Bleeding Edge, against Rex Server 0.1.0-preview.15 (API revision 13). Assembled 2026-11-11 (PST) from six verified domain analyses in [`gaps/`](gaps/), which rest on the specs in [`beta/`](beta/), [`rune/`](rune/) and [`code/`](code/). Folder guide: [README.md](README.md).

## 1. Verdict

**Rex Linux is a strong engine with a thin surface.** The terminal, protocol, multi-host, keymap, integration and update machinery is at or ahead of the beta. What is missing is almost entirely what a user can see, reach with the mouse and configure: pane controls, a Themes page, a real Settings, shortcut and action coverage, an activity overview, server identity, sounds, onboarding and diagnostics.

The numbers (586 rows across the six verified domain matrices; a capability scored by two domains is counted in both):

| Status | Rows | Share of 586 |
|---|---:|---:|
| parity | 61 | 10% |
| partial | 156 | 27% |
| missing | 257 | 44% |
| linux-ahead | 19 | 3% |
| linux-only | 53 | 9% |
| not-applicable (macOS idioms) | 40 | 7% |

Take away the 53 Linux-only and 40 not-applicable rows and 493 rows apply to Linux. Of those, **80 (16%) are at parity or better, 156 (32%) are partly there and 257 (52%) are absent.**

How hard the gap is to close:

- **Almost all of it is client-only work.** In the raw matrices, 346 of 413 missing-or-partial rows (84%) have no blocker: no new server API, no Rune change, no product decision. 303 of those 346 are effort S or M. The server API already offers the calls for tab reorder, floating panes, pane move and swap, split-before, focus next/previous, reset, clear, directory listing, `server.version` and `server.status`; none is called from `src/` today.
- **The remaining 67 gap rows are blocked.** 45 collide with a product rule (light chrome, a visible tab close button, dimming or recolouring terminal content, terminal animation, ligatures, keyboard-hint footers, writing `shortcuts.json`, self-installing updates), 13 need Rune work (multiple windows, a separate Settings window, full screen, global hotkey, gestures, app icon variants, accessibility tree, live font apply), 4 are unknown, 3 are macOS-only and 2 need a server API that does not exist (live server logs, Web Client Origins). See [section 5](#5-blocked-or-out-of-scope).
- **Linux is ahead where it counts for its own users.** Host-grouped sidebar and multi-host switching, tailnet discovery with a Rex probe, previewed SSH "Start personal server", default-deny per-host remote control, three-layer keymap with chord sequences, per-agent integration status with dry-run previews, signed update checks with a visible result, about 600 installable Ghostty terminal themes (the beta has 41), per-host default shell, and named-target confirmations. Section 3.3 lists all 19 Linux-ahead and 53 Linux-only rows. Keep them while closing gaps.

**The gaps that matter most**, in the order a user hits them:

1. **No pane controls a mouse can reach.** There are no pane header buttons (Split, Float, Zoom, Close), no header or toolbar visibility modes, no Split Left/Up/Best Fit, no pane move, balance or float. A single full-bleed pane has no visible way to split or close it (d1 PN02, PN03; effort M).
2. **The terminal right-click menu has no Copy, Paste, Select All, Reset or Clear** (d1 C01, d3 T63/T64, d4 G02; effort S). It is the cheapest high-impact fix in the report.
3. **Settings is thin and unstructured.** One 760x520 modal with flat rows, clipped content and `Change...` buttons that replace the panel; no cards, no search, no sub-pages. The Terminal page has 5 rows against the beta's roughly 40 controls; Shortcuts shows 28 read-only bindings against 118 actions, with no filter, no unbound actions and no recorder (d2 S11, d3 T01, d4 A01/A02/A04).
4. **No Themes surface.** No Themes page, thumbnails, light and dark selection, editor, import or extended palette; no Auto/Light/Dark chrome and no Tint with theme (the last two need a DESIGN.md decision) (d2 TH01-TH09, I01-I04).
5. **Action coverage and Commands.** The registry has 32 actions against the beta's roughly 100; Commands is 25 ungrouped rows missing Close session, Next/Previous session, Refresh, Go to directory, Change theme and most of the beta's menu (d4 C04-C19, E04).
6. **No Activity overview, tab reorder or tab mute** (d1 A01, TB08, TB09; d5 d5-38, d5-39). Activity is shown only as words on the current session's tabs.
7. **Find bar.** No previous/next/close buttons, it sits over the first output row of a single pane and hides the matches it finds, and the match highlight is a dim accent overlay (d1 F01-F03, d3 T56-T60).
8. **Go to Directory and Insert Path do not exist**, although `terminal.list_dir`, `.pwd` and `.write` are already in the vendored API (d3 T76-T79, d4 D15/D18; effort L).
9. **Server identity, pairing, sounds, onboarding and diagnostics.** `server.version` is never called, so About cannot say which server it talks to; no Server page or Pair QR; no sounds or per-status notification control; no first-run flow; no Diagnostics window at all (d5 d5-01..05, d5-30..33, d5-63; d6 d6.about.server-version, d6.diag.*).

Three beta behaviours should not be copied: a lighter card-on-chrome pane for a single pane, a close button on every tab, and dimming terminal content during find. They conflict with DESIGN.md principles; section 4 and section 5 give the reasoning.

## 2. Scorecard

Counts come from the status column of each domain matrix. "Rows" are the raw matrix rows. "Applicable parity" is (parity + linux-ahead) divided by (rows minus linux-only minus not-applicable).

| Domain | Rows | Parity | Partial | Missing | Linux-ahead | Linux-only | N/A | Applicable parity | One-line read |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| [D1 Workspace chrome](gaps/d1-workspace-chrome.md) | 118 | 17 | 36 | 39 | 6 | 13 | 7 | 23% | Core loop (toolbar, tabs, switcher, palette, split/zoom/close, named confirmations) is at parity and multi-host sidebar is ahead; direct manipulation is missing (pane header buttons, Activity, float, move, tab reorder, mute, find bar). |
| [D2 Settings shell, Interface, Themes](gaps/d2-interface-themes.md) | 86 | 4 | 20 | 54 | 1 | 6 | 1 | 6% | Thin Interface page, four dark chrome palettes, 760x520 modal with no cards, search or sub-pages; no Themes page, editor, light/Auto or Tint. Terminal theme coverage (about 600 Ghostty themes) is richer than the beta. |
| [D3 Terminal](gaps/d3-terminal.md) | 89 | 8 | 16 | 42 | 2 | 13 | 8 | 15% | Engine is strong (Ghostty cells, sprites, selection, links, search) and Linux is ahead on per-host shell; the settings surface is 5 rows against about 40 controls, the context menu lacks Copy/Paste/Reset, Go to Directory is absent, and a quarter of the gaps are gated by product rules. |
| [D4 Shortcuts, Commands, menus](gaps/d4-shortcuts-commands-menus.md) | 152 | 24 | 54 | 51 | 7 | 5 | 11 | 23% | Keymap engine is ahead (three layers, chords, remote consent) but Shortcuts shows 28 read-only bindings against 118 actions, the registry has 32 actions against about 100, and Commands is long and ungrouped. No menu bar by design. |
| [D5 Hosts, server, notifications, integrations, onboarding](gaps/d5-hosts-notifications-integrations-onboarding.md) | 75 | 6 | 18 | 33 | 3 | 9 | 6 | 15% | Ahead on host discovery, SSH setup, remote-control model, integrations and signed update checks; missing the Server page, Pair QR, sounds and per-status notifications, onboarding and migration, and form craft. |
| [D6 Developer, diagnostics, updates, About](gaps/d6-developer-diagnostics-updates.md) | 66 | 2 | 12 | 38 | 0 | 7 | 7 | 4% | Almost no developer or diagnostics surface (no Developer page, Diagnostics, API console, Network or GPU); stronger update story; About is thin and `server.version` is never read. |
| **Raw total** | **586** | **61** | **156** | **257** | **19** | **53** | **40** | **16%** | Traceable to rows; includes cross-domain duplicates |

**Counting note.** The six matrices hold 586 rows in total (118 + 86 + 89 + 152 + 75 + 66). Several capabilities were scored independently by more than one domain (settings shell and control vocabulary; terminal menu and find; Activity; server version; theme pickers), so the totals overstate distinct capabilities somewhat. An earlier draft quoted a 431-row "de-duplicated" total; that figure was an artefact of truncated row lists in the orchestration script, not a real de-duplication, and has been removed.

**Measurement caveat (applies to every Linux size in the domain files).** `rune/r01` and `rune/r03` assume the Linux captures are 2x and halve the pixel sizes. They are 1x captures (2560x1410) at Interface zoom 130% (d1, d4 verified this: toolbar 52 pt x 1.3 = 67.6 px). Take Linux sizes from code constants in `src/app/theme.rs` and `popups.rs`, not from the halved numbers. Beta sizes are real points on a 1x display.

## 3. Inventory

Every raw matrix row appears below, grouped by domain, with its ID, a shortened capability name, effort and blocker, and a link to the domain matrix (IDs link to the matrix section that holds the row). Effort: S under a day, M one to three days, L about a week, XL longer. Rows scored by two domains appear under both.



### 3.1 What Rex Linux has

At parity with the beta (61 rows), then present but short of the beta ("partial": the capability exists, a part of it does not; the gap is in 3.2).

#### D1 Workspace chrome ([file](gaps/d1-workspace-chrome.md))

At parity (17):

- [W11](gaps/d1-workspace-chrome.md#2-feature-matrix) Status bar (bottom of window)
- [T01](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar structure
- [T04](gaps/d1-workspace-chrome.md#2-feature-matrix) Commands / Command Palette button
- [T05](gaps/d1-workspace-chrome.md#2-feature-matrix) New tab (+) button
- [TB06](gaps/d1-workspace-chrome.md#2-feature-matrix) Activity words and progress on tabs
- [TB07](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab menu: Rename Tab…
- [TB10](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab menu: Close Tab
- [TB13](gaps/d1-workspace-chrome.md#2-feature-matrix) Next/Previous tab shortcuts
- [S03](gaps/d1-workspace-chrome.md#2-feature-matrix) Create session by typing a name
- [S08](gaps/d1-workspace-chrome.md#2-feature-matrix) Rename Session
- [P04](gaps/d1-workspace-chrome.md#2-feature-matrix) Palette search across commands, sessions and tabs
- [PN06](gaps/d1-workspace-chrome.md#2-feature-matrix) Split Right / Split Down
- [PN11](gaps/d1-workspace-chrome.md#2-feature-matrix) Focus pane L/R/U/D
- [PN16](gaps/d1-workspace-chrome.md#2-feature-matrix) Rename pane / Reconnect terminal
- [PN21](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane style Comfortable \| Compact
- [F04](gaps/d1-workspace-chrome.md#2-feature-matrix) Search options (case, regex, whole word), cross-pane search
- [N03](gaps/d1-workspace-chrome.md#2-feature-matrix) Dialog / sheet style

Present but partial (36):

- [W02](gaps/d1-workspace-chrome.md#2-feature-matrix) Window title
- [W06](gaps/d1-workspace-chrome.md#2-feature-matrix) App menu: About, Check for Updates, Settings, Services, Hide, Hide Others, Show…
- [W07](gaps/d1-workspace-chrome.md#2-feature-matrix) About panel (icon, version, channel, server version, Copy Version Info)
- [W14](gaps/d1-workspace-chrome.md#2-feature-matrix) Font size commands (Increase, Decrease, Reset)
- [T10](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar layout: Above \| Left Side \| Right Side
- [T12](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar tooltips
- [T13](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar icon vocabulary
- [TB01](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab pill appearance
- [TB05](gaps/d1-workspace-chrome.md#2-feature-matrix) Program icons on tabs
- [TB12](gaps/d1-workspace-chrome.md#2-feature-matrix) Rename tab UI
- [TB14](gaps/d1-workspace-chrome.md#2-feature-matrix) Go to tab N
- [TB16](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab filter (Focus Tab Filter)
- [TB18](gaps/d1-workspace-chrome.md#2-feature-matrix) Tabs in a sidebar
- [S05](gaps/d1-workspace-chrome.md#2-feature-matrix) Previous / Next Session commands
- [S06](gaps/d1-workspace-chrome.md#2-feature-matrix) Collapse Session / Collapse Other Sessions
- [S07](gaps/d1-workspace-chrome.md#2-feature-matrix) Close Session
- [P01](gaps/d1-workspace-chrome.md#2-feature-matrix) Command palette panel
- [P07](gaps/d1-workspace-chrome.md#2-feature-matrix) Palette row accessories
- [PN01](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane card
- [PN02](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane header content
- [PN04](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane header behavior (Always show \| Show on hover \| Hidden), Reveal Pane Header ⌥⇧⌘T
- [PN09](gaps/d1-workspace-chrome.md#2-feature-matrix) Zoom pane
- [PN15](gaps/d1-workspace-chrome.md#2-feature-matrix) Close pane
- [PN17](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane padding: Minimum size H/V, Fill with program colors, Include pane header
- [PN18](gaps/d1-workspace-chrome.md#2-feature-matrix) Highlight active pane edges
- [C02](gaps/d1-workspace-chrome.md#2-feature-matrix) Terminal menu: split x4, Float, Zoom, Close Pane
- [C04](gaps/d1-workspace-chrome.md#2-feature-matrix) Select All; Edit menu clipboard group
- [C07](gaps/d1-workspace-chrome.md#2-feature-matrix) Menu separators between groups
- [C08](gaps/d1-workspace-chrome.md#2-feature-matrix) Menu shortcut hints
- [F01](gaps/d1-workspace-chrome.md#2-feature-matrix) Find bar layout
- [F02](gaps/d1-workspace-chrome.md#2-feature-matrix) Find next / previous controls and commands
- [F03](gaps/d1-workspace-chrome.md#2-feature-matrix) Match highlight and dimming
- [G03](gaps/d1-workspace-chrome.md#2-feature-matrix) Theme switcher overlay (View &gt; Customize View &gt; Change Theme…)
- [N01](gaps/d1-workspace-chrome.md#2-feature-matrix) In-app notices position
- [N04](gaps/d1-workspace-chrome.md#2-feature-matrix) Take control / ownership strip
- [N05](gaps/d1-workspace-chrome.md#2-feature-matrix) Accessibility tree

#### D2 Settings shell, Interface, Themes ([file](gaps/d2-interface-themes.md))

At parity (4):

- [S13](gaps/d2-interface-themes.md#2-feature-matrix) Toggle switch
- [S21](gaps/d2-interface-themes.md#2-feature-matrix) Open Settings shortcut
- [I08](gaps/d2-interface-themes.md#2-feature-matrix) Pane style: Comfortable \| Compact
- [M04](gaps/d2-interface-themes.md#2-feature-matrix) Toggle Sidebar

Present but partial (20):

- [S01](gaps/d2-interface-themes.md#2-feature-matrix) Settings container
- [S02](gaps/d2-interface-themes.md#2-feature-matrix) Category list and grouping
- [S10](gaps/d2-interface-themes.md#2-feature-matrix) Server/API version callout
- [S12](gaps/d2-interface-themes.md#2-feature-matrix) Row anatomy
- [S14](gaps/d2-interface-themes.md#2-feature-matrix) Segmented control with leading glyphs
- [S19](gaps/d2-interface-themes.md#2-feature-matrix) Numeric field with up/down stepper
- [S20](gaps/d2-interface-themes.md#2-feature-matrix) Scroll affordance
- [I05](gaps/d2-interface-themes.md#2-feature-matrix) Toolbar layout: Above \| Left Side \| Right Side
- [I09](gaps/d2-interface-themes.md#2-feature-matrix) Pane header behavior: Always show \| Show on hover \| Hidden
- [P01](gaps/d2-interface-themes.md#2-feature-matrix) Minimum padding horizontal and vertical
- [P02](gaps/d2-interface-themes.md#2-feature-matrix) Fill with program colors: Off \| Automatic \| Always
- [P04](gaps/d2-interface-themes.md#2-feature-matrix) Highlight active pane edges
- [TH03](gaps/d2-interface-themes.md#2-feature-matrix) Built-in library
- [TH06](gaps/d2-interface-themes.md#2-feature-matrix) Selected state
- [TH09](gaps/d2-interface-themes.md#2-feature-matrix) Import Ghostty or iTerm theme file
- [SW01](gaps/d2-interface-themes.md#2-feature-matrix) Theme switcher popover (Change Theme...)
- [SW02](gaps/d2-interface-themes.md#2-feature-matrix) Switcher scope
- [SW03](gaps/d2-interface-themes.md#2-feature-matrix) Search behaviour
- [M05](gaps/d2-interface-themes.md#2-feature-matrix) Copy Config File Path / Reload Config
- [X03](gaps/d2-interface-themes.md#2-feature-matrix) Settings window is a real window with close and zoom

#### D3 Terminal ([file](gaps/d3-terminal.md))

At parity (8):

- [T09](gaps/d3-terminal.md#22-typography-and-fonts) Font size stepper in pt (beta default 12, range not shown)
- [T36](gaps/d3-terminal.md#25-keyboard) Scroll Page Up / Down
- [T57](gaps/d3-terminal.md#29-find) Open find: Cmd+F
- [T62](gaps/d3-terminal.md#29-find) Search is incremental, covers scrollback, survives reconnect
- [T65](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Split Right, Split Down
- [T68](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Zoom Pane
- [T69](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Close Pane
- [T82](gaps/d3-terminal.md#211-path-browser-titles-shell) New tabs and splits inherit source working directory

Present but partial (16):

- [T01](gaps/d3-terminal.md#21-settings-page-structure) Terminal page layout: one scrolling page of 11 sections (Typography, Colors…
- [T05](gaps/d3-terminal.md#22-typography-and-fonts) Terminal font: current family shown in its own face + "Choose..."
- [T18](gaps/d3-terminal.md#22-typography-and-fonts) Fallback fonts: ordered list with "Add..." picker (opens on All Fonts)
- [T31](gaps/d3-terminal.md#25-keyboard) Show key sequence hints switch
- [T33](gaps/d3-terminal.md#25-keyboard) Font-size shortcuts: Increase (Cmd+= and Cmd++), Decrease (Cmd+-), Reset (Cmd+0)
- [T42](gaps/d3-terminal.md#27-selection-and-clipboard) Selection foreground: Theme / Cell Foreground / Cell Background
- [T43](gaps/d3-terminal.md#27-selection-and-clipboard) Selection background: Theme / Cell Foreground / Cell Background
- [T47](gaps/d3-terminal.md#27-selection-and-clipboard) Copy: Edit &gt; Copy (Cmd+C, disabled with no selection)
- [T48](gaps/d3-terminal.md#27-selection-and-clipboard) Paste: Edit &gt; Paste (Cmd+V)
- [T52](gaps/d3-terminal.md#28-links) Web, mail and file: links open directly
- [T56](gaps/d3-terminal.md#29-find) Find bar: pill 300x39 top-right, "Find" placeholder, count 1/1, up/down…
- [T58](gaps/d3-terminal.md#29-find) Find Next / Find Previous as actions (Cmd+G, Shift+Cmd+G; disabled until a…
- [T60](gaps/d3-terminal.md#29-find) Dim terminal and yellow #F4D722 rounded match highlight
- [T75](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Edit menu entries reachable without a menu bar (Select All, Insert Path, Find)
- [T84](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) Pane padding (ADDED, omitted by the first draft although the summary counts it)
- [T86](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) Copy from Pane / Paste into Pane / Copy Pane ID actions

#### D4 Shortcuts, Commands, menus ([file](gaps/d4-shortcuts-commands-menus.md))

At parity (24):

- [B04](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Close (Cmd W, context-sensitive)
- [B05](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Command Palette (Shift Cmd P)
- [B06](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Settings (Cmd ,)
- [C01](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) New Session (Shift Cmd N)
- [C02](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Change Session... (Cmd K)
- [C05](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Rename Session
- [C10](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Add Remote Host...
- [C12](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) New Tab (Cmd T)
- [C15](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Rename Tab
- [C16](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Show Next Tab (Shift Cmd \] and Ctrl Tab)
- [C17](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Show Previous Tab (Shift Cmd \[ and Ctrl Shift Tab)
- [D01](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Split Pane Right (Shift Cmd Right, Cmd D)
- [D02](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Split Pane Down (Shift Cmd Down, Shift Cmd D)
- [D05](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Focus Pane Left/Right/Up/Down (Opt Cmd arrows)
- [D09](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Zoom Pane (Shift Cmd Return)
- [D10](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Close Pane (Cmd W)
- [D29](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Find... (Cmd F)
- [D32](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Toggle Sidebar (Cmd /)
- [E02](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Search placeholder
- [E06](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Chevron for sub-step rows
- [E07](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Shortcut hint column
- [E11](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Match highlight in results
- [F09](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) View: Command Palette, Change Session, Refresh Sessions
- [G18](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Toolbar Commands and New Tab buttons

Present but partial (54):

- [A01](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Page structure
- [A03](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Row layout
- [A05](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Priority helper text
- [A09](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Multiple bindings per action
- [A10](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Not-recordable rows
- [A11](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Parameterised read-only sub-rows
- [A14](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Conflict display
- [A18](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Sticky heading / scroll clipping
- [A19](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Settings window shell around the page
- [B02](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Close Window (Shift Cmd W)
- [B10](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Copy Version Info
- [B11](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Copy Debug Info
- [B14](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Check for Updates in Background / Report Update Status
- [B15](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Set Update Automation / Set Update Channel
- [B18](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Install Rex for Agents
- [C03](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Go to Session (not recordable)
- [C06](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Close Session
- [C08](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Collapse Session / Expand Session / Collapse Other Sessions
- [C11](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Remove Host...
- [C13](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Close Tab (Opt Cmd W)
- [C18](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Go to Tab 1..8 and last (Cmd 1..9)
- [D13](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Resize Pane (not recordable)
- [D16](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Go to Anything... (Cmd P)
- [D19](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Copy Pane ID / Rename Pane
- [D20](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Paste into Pane / Copy from Pane
- [D23](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Send Key to Server
- [D30](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Find Next (Cmd G) / Find Previous (Shift Cmd G)
- [D31](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Increase / Decrease / Reset Font Size (Cmd + / - / 0)
- [D36](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Change Theme...
- [D37](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Import Theme File / Switch Appearance Mode (not recordable)
- [D38](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Copy Config File Path / Reload Config
- [E01](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Panel size and placement
- [E04](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Rows in Beta palette
- [E10](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Selection style
- [F02](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) App menu (Rex Beta)
- [F03](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) File: New Window, New Session, Add Remote Host..., New Tab, Go to Directory...…
- [F05](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Edit: Copy, Paste
- [F07](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Edit &gt; Find submenu: Find..., Find Next, Find Previous
- [F10](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) View &gt; Pane submenu (20 items in 4 groups)
- [F11](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) View &gt; Customize View &gt; Change Theme...
- [F14](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Pane Style: Comfortable / Compact
- [F17](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) View: font size, Toggle Sidebar, Show Activity, Focus Tab Filter, Reveal…
- [F19](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Window menu: Diagnostics, Copy Version Info, Copy Debug Info
- [F20](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Window menu: tab nav and Move Tab x4; Previous/Next Session
- [F21](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Help: search, Send Feedback to Apple, Rex Help (Cmd ?)
- [G01](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Tab context menu
- [G02](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Terminal context menu
- [G05](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Menu visual style
- [G06](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Destructive item styling
- [G09](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Rename tab overlay + 'Restore Default Title'
- [G11](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Terminal find bar
- [G13](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Theme switcher overlay
- [G15](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Confirm dialogs naming target
- [G17](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Tab pill close button

#### D5 Hosts, server, notifications, integrations, onboarding ([file](gaps/d5-hosts-notifications-integrations-onboarding.md))

At parity (6):

- [d5-23](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Let agents name sessions
- [d5-27](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Show activity in tabs (global)
- [d5-28](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Show system notifications (global)
- [d5-36](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Terminal notifications via OSC 9 / OSC 777
- [d5-43](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Remote Hosts page structure
- [d5-44](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Add-host entry points

Present but partial (18):

- [d5-02](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Server status card: name, 'Running for 10m', info button with Server Version /…
- [d5-11](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Client label (Client Info &gt; Label, placeholder = machine hostname)
- [d5-12](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Remote Control: other clients may perform actions in this app
- [d5-14](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Server listeners list with copy
- [d5-15](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Settings window form: separate 816x850 window vs modal
- [d5-18](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Sidebar grouping (4 groups) with coloured 20 pt icon tiles
- [d5-19](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Master switch 'Integrate with available agents' - 'Installs a Rex skill and…
- [d5-26](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) In-app notification position
- [d5-32](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Per-status subpages (Working, Blocked, Error, Done): Enable status, Show…
- [d5-41](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Desktop notification quality: urgency per state, replace/close when the tab is…
- [d5-45](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Add host form: URL, Name, Icon
- [d5-48](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Sheet anatomy: icon + title + subtitle header, divider, 'Host Details' card…
- [d5-55](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) About surface
- [d5-56](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Server version in version info and copy
- [d5-61](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Local API URL with copy
- [d5-67](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Toolbar layout Above / Left Side / Right Side
- [d5-68](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Theme picker: 'Default theme' Merino Dark + 26 'Built-in themes' as live…
- [d5-69](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Integrations step: Rex Agent Skill card (Recommended, Installed chip)

#### D6 Developer, diagnostics, updates, About ([file](gaps/d6-developer-diagnostics-updates.md))

At parity (2):

- [d6.upd.check-menu](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) 'Check for Updates...' (app menu, status menu); beta shows no result panel
- [d6.upd.toast](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Update availability notice

Present but partial (12):

- [d6.dev.local-api](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Local API URL (percent-encoded unix:// path) with 'Copy local Rex endpoint'…
- [d6.dev.quick-switcher](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) 'New quick switcher' switch: one list for Go to Anything / Go to Session / View…
- [d6.dev.quicklook](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) 'Quick Look links' switch: Cmd-Y previews on-screen files and links…
- [d6.dev.popup-style](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Settings popup control: right-aligned value + circular chevron badge…
- [d6.dev.card-style](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Inset grouped cards on the page ground with section heading above, footnote…
- [d6.about.window](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) About as a small standalone window (280x243): icon, name, version, channel…
- [d6.about.version](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Client version line ('Version 0.1.0 (1932)')
- [d6.about.channel](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Release channel line ('Bleeding Edge')
- [d6.about.server-popover](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Server 'ⓘ' popover: Server Version, API Version 'Revision 13'
- [d6.about.copy-info](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Copy Version Info (client + channel + server versions)
- [d6.diag.copy](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Trailing copy glyph on nearly every value, accessible name 'Copy &lt;label&gt;'
- [d6.diag.server.status](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Rex Server pane status card ('Rex server is running', green check)


### 3.2 What Rex Beta has that Linux lacks

Missing rows first, then partial rows (tagged, with their effort and blocker). A blocker other than `none` means the row cannot be closed by client work alone; see section 5.

#### D1 Workspace chrome ([file](gaps/d1-workspace-chrome.md))

Missing (39):

- [W03](gaps/d1-workspace-chrome.md#2-feature-matrix) Multiple windows (New Window ⌘N, Close Window ⇧⌘W, Move Tab to New Window, window list in Window… · XL · needs-rune-work
- [W04](gaps/d1-workspace-chrome.md#2-feature-matrix) Full screen toggle · S · needs-rune-work
- [W09](gaps/d1-workspace-chrome.md#2-feature-matrix) Help menu (search, Send Feedback to Apple, Rex Help ⌘?) · S
- [W13](gaps/d1-workspace-chrome.md#2-feature-matrix) Chrome appearance: Color scheme Auto/Light/Dark, Interface style Classic/Glass, Tint with theme · L · product-rule
- [W15](gaps/d1-workspace-chrome.md#2-feature-matrix) Trackpad gestures (pinch to zoom panes, swipe to toggle sidebar) · M · needs-rune-work
- [W16](gaps/d1-workspace-chrome.md#2-feature-matrix) Animation style (Snappy \| Natural \| Off) · S
- [W17](gaps/d1-workspace-chrome.md#2-feature-matrix) Global show/hide hotkey · M · needs-rune-work
- [T06](gaps/d1-workspace-chrome.md#2-feature-matrix) Activity (eye) toggle button · L
- [T11](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar behavior: Always show \| Show on hover \| Hidden; Reveal Toolbar ⌥⌘T · M
- [TB02](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab close x on the pill · S · product-rule
- [TB03](gaps/d1-workspace-chrome.md#2-feature-matrix) Tabs fill available space · S
- [TB04](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab icon options: Show tab icons (0-5), Use color for tab icons, Shell icon theme, live preview · M
- [TB08](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab menu: Mute Tab · M
- [TB09](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab menu / shortcuts: Move Tab Left, Move Tab Right (⌃⇧↑ / ⌃⇧↓ in B075/B092) · M
- [TB15](gaps/d1-workspace-chrome.md#2-feature-matrix) Close Other Tabs, Move Tab to New Window · S
- [S02](gaps/d1-workspace-chrome.md#2-feature-matrix) Session quick-select numbers · S
- [S04](gaps/d1-workspace-chrome.md#2-feature-matrix) Refresh Sessions · S
- [P02](gaps/d1-workspace-chrome.md#2-feature-matrix) Palette footer hints (Change Keybinding ⌘↩, Action ↩) · S · product-rule
- [P03](gaps/d1-workspace-chrome.md#2-feature-matrix) Rebind a command from the palette (⌘↩) · L
- [P05](gaps/d1-workspace-chrome.md#2-feature-matrix) Palette: Go to Directory… ⇧⌘G, Insert Path…, Change Theme…, New Window, Move Tab to New Window · M
- [P08](gaps/d1-workspace-chrome.md#2-feature-matrix) Go to Anything… (⌘P) · M
- [A01](gaps/d1-workspace-chrome.md#2-feature-matrix) Activity popover (grouped by state, row = icon, title, session/tab, working dots, elapsed, Filter) · L
- [A02](gaps/d1-workspace-chrome.md#2-feature-matrix) Show Activity by Session / by Priority · M
- [A04](gaps/d1-workspace-chrome.md#2-feature-matrix) Activity settings: include idle processes, show each process in tabs · S
- [PN03](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane header action buttons (Split Right, Split Down, Float, Zoom, Close) · M
- [PN05](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane drag handle (Move &lt;title&gt; pane) · L
- [PN07](gaps/d1-workspace-chrome.md#2-feature-matrix) Split Left / Split Up · S
- [PN08](gaps/d1-workspace-chrome.md#2-feature-matrix) Split Best Fit · S
- [PN10](gaps/d1-workspace-chrome.md#2-feature-matrix) Float Pane, New Floating Pane, Minimize Floating Window(s) · L
- [PN12](gaps/d1-workspace-chrome.md#2-feature-matrix) Focus Next / Previous Pane · S
- [PN13](gaps/d1-workspace-chrome.md#2-feature-matrix) Move Pane L/R/U/D, Move Pane to New Tab / to Tab… · M
- [PN14](gaps/d1-workspace-chrome.md#2-feature-matrix) Resize pane by keyboard, Balance Splits, Autobalance splits · M
- [PN19](gaps/d1-workspace-chrome.md#2-feature-matrix) Fade inactive panes (Off \| Surface \| Content \| Surface &amp; Content), Fade panes for inactive windows · M · product-rule
- [PN20](gaps/d1-workspace-chrome.md#2-feature-matrix) Focus follows mouse · S
- [C01](gaps/d1-workspace-chrome.md#2-feature-matrix) Terminal right-click menu: Copy, Paste · S
- [C03](gaps/d1-workspace-chrome.md#2-feature-matrix) Reset Terminal · S
- [C09](gaps/d1-workspace-chrome.md#2-feature-matrix) Copy Pane ID, targeted Paste into Pane / Copy from Pane · S
- [G01](gaps/d1-workspace-chrome.md#2-feature-matrix) Go to Directory… (⇧⌘G) · L
- [G02](gaps/d1-workspace-chrome.md#2-feature-matrix) Insert Path… (⌃⌘I) · M

Partial (36):

- [W02](gaps/d1-workspace-chrome.md#2-feature-matrix) Window title · S
- [W06](gaps/d1-workspace-chrome.md#2-feature-matrix) App menu: About, Check for Updates, Settings, Services, Hide, Hide Others, Show All, Quit · S · macos-only
- [W07](gaps/d1-workspace-chrome.md#2-feature-matrix) About panel (icon, version, channel, server version, Copy Version Info) · S
- [W14](gaps/d1-workspace-chrome.md#2-feature-matrix) Font size commands (Increase, Decrease, Reset) · S
- [T10](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar layout: Above \| Left Side \| Right Side · L
- [T12](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar tooltips · S
- [T13](gaps/d1-workspace-chrome.md#2-feature-matrix) Toolbar icon vocabulary · S
- [TB01](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab pill appearance · M
- [TB05](gaps/d1-workspace-chrome.md#2-feature-matrix) Program icons on tabs · M
- [TB12](gaps/d1-workspace-chrome.md#2-feature-matrix) Rename tab UI · S
- [TB14](gaps/d1-workspace-chrome.md#2-feature-matrix) Go to tab N · S
- [TB16](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab filter (Focus Tab Filter) · M
- [TB18](gaps/d1-workspace-chrome.md#2-feature-matrix) Tabs in a sidebar · M
- [S05](gaps/d1-workspace-chrome.md#2-feature-matrix) Previous / Next Session commands · S
- [S06](gaps/d1-workspace-chrome.md#2-feature-matrix) Collapse Session / Collapse Other Sessions · S
- [S07](gaps/d1-workspace-chrome.md#2-feature-matrix) Close Session · S
- [P01](gaps/d1-workspace-chrome.md#2-feature-matrix) Command palette panel · M
- [P07](gaps/d1-workspace-chrome.md#2-feature-matrix) Palette row accessories · S
- [PN01](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane card · S · product-rule
- [PN02](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane header content · M
- [PN04](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane header behavior (Always show \| Show on hover \| Hidden), Reveal Pane Header ⌥⇧⌘T · M
- [PN09](gaps/d1-workspace-chrome.md#2-feature-matrix) Zoom pane · S
- [PN15](gaps/d1-workspace-chrome.md#2-feature-matrix) Close pane · S
- [PN17](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane padding: Minimum size H/V, Fill with program colors, Include pane header · M
- [PN18](gaps/d1-workspace-chrome.md#2-feature-matrix) Highlight active pane edges · S
- [C02](gaps/d1-workspace-chrome.md#2-feature-matrix) Terminal menu: split x4, Float, Zoom, Close Pane · S
- [C04](gaps/d1-workspace-chrome.md#2-feature-matrix) Select All; Edit menu clipboard group · S
- [C07](gaps/d1-workspace-chrome.md#2-feature-matrix) Menu separators between groups · S
- [C08](gaps/d1-workspace-chrome.md#2-feature-matrix) Menu shortcut hints · S
- [F01](gaps/d1-workspace-chrome.md#2-feature-matrix) Find bar layout · M
- [F02](gaps/d1-workspace-chrome.md#2-feature-matrix) Find next / previous controls and commands · S
- [F03](gaps/d1-workspace-chrome.md#2-feature-matrix) Match highlight and dimming · S · product-rule
- [G03](gaps/d1-workspace-chrome.md#2-feature-matrix) Theme switcher overlay (View &gt; Customize View &gt; Change Theme…) · S
- [N01](gaps/d1-workspace-chrome.md#2-feature-matrix) In-app notices position · S
- [N04](gaps/d1-workspace-chrome.md#2-feature-matrix) Take control / ownership strip · M
- [N05](gaps/d1-workspace-chrome.md#2-feature-matrix) Accessibility tree · L · product-rule

#### D2 Settings shell, Interface, Themes ([file](gaps/d2-interface-themes.md))

Missing (54):

- [S03](gaps/d2-interface-themes.md#2-feature-matrix) Themes category · L
- [S05](gaps/d2-interface-themes.md#2-feature-matrix) Developer category · S
- [S07](gaps/d2-interface-themes.md#2-feature-matrix) Search Settings · M
- [S08](gaps/d2-interface-themes.md#2-feature-matrix) Back/Forward capsule and page title · M
- [S09](gaps/d2-interface-themes.md#2-feature-matrix) Sub-page push inside Settings · M
- [S11](gaps/d2-interface-themes.md#2-feature-matrix) Inset card grouping with section headings · M
- [S15](gaps/d2-interface-themes.md#2-feature-matrix) Popup button (value + chevron) for 3+ options · M
- [S16](gaps/d2-interface-themes.md#2-feature-matrix) Disclosure row (label + chevron) · S
- [S17](gaps/d2-interface-themes.md#2-feature-matrix) Picture tiles (selectable preview with ring) · M
- [S18](gaps/d2-interface-themes.md#2-feature-matrix) Slider with ticks and readout · M
- [I01](gaps/d2-interface-themes.md#2-feature-matrix) Color scheme: Auto \| Light \| Dark · L · product-rule
- [I02](gaps/d2-interface-themes.md#2-feature-matrix) Follow the system appearance (Auto) · M
- [I03](gaps/d2-interface-themes.md#2-feature-matrix) Interface style: Classic \| Glass · L
- [I04](gaps/d2-interface-themes.md#2-feature-matrix) Tint with theme · L · product-rule
- [I06](gaps/d2-interface-themes.md#2-feature-matrix) Toolbar behavior: Always show \| Show on hover \| Hidden · M
- [I07](gaps/d2-interface-themes.md#2-feature-matrix) Customize tabs row · S
- [I10](gaps/d2-interface-themes.md#2-feature-matrix) Customize panes row · S
- [I11](gaps/d2-interface-themes.md#2-feature-matrix) Use trackpad gestures · L · needs-rune-work
- [I12](gaps/d2-interface-themes.md#2-feature-matrix) Animation style: Snappy \| Natural \| Off · S
- [I13](gaps/d2-interface-themes.md#2-feature-matrix) App icon: Default (Dynamic) \| Bright \| Twilight · M · needs-rune-work
- [I14](gaps/d2-interface-themes.md#2-feature-matrix) Show app icon: Dock and menu bar \| Dock only \| Menu bar only · M
- [I15](gaps/d2-interface-themes.md#2-feature-matrix) Show window on launch · S
- [T01](gaps/d2-interface-themes.md#2-feature-matrix) Tabs fill available space · S
- [T02](gaps/d2-interface-themes.md#2-feature-matrix) Tab icons live preview · M
- [T03](gaps/d2-interface-themes.md#2-feature-matrix) Show tab icons (0-5 slider) · M
- [T04](gaps/d2-interface-themes.md#2-feature-matrix) Use color for tab icons: Always \| For active panes \| For active tabs \| Never · M
- [T05](gaps/d2-interface-themes.md#2-feature-matrix) Shell icon theme: Default \| Use theme \| Use theme accent · S
- [P03](gaps/d2-interface-themes.md#2-feature-matrix) Include pane header (colour extends into header) · M
- [P05](gaps/d2-interface-themes.md#2-feature-matrix) Fade inactive panes: Off \| Surface \| Content \| Surface &amp; Content · M · product-rule
- [P06](gaps/d2-interface-themes.md#2-feature-matrix) Fade panes for inactive windows · S
- [P07](gaps/d2-interface-themes.md#2-feature-matrix) Autobalance splits · M
- [P08](gaps/d2-interface-themes.md#2-feature-matrix) Focus follows mouse · S
- [TH01](gaps/d2-interface-themes.md#2-feature-matrix) Light Mode / Dark Mode sections · L
- [TH02](gaps/d2-interface-themes.md#2-feature-matrix) Independent light and dark theme selection · M
- [TH05](gaps/d2-interface-themes.md#2-feature-matrix) Theme card thumbnail · M
- [TH07](gaps/d2-interface-themes.md#2-feature-matrix) Use extended theme palette (per appearance) · M
- [TH08](gaps/d2-interface-themes.md#2-feature-matrix) New light / New dark theme · L
- [E01](gaps/d2-interface-themes.md#2-feature-matrix) Theme editor sheet · L
- [E02](gaps/d2-interface-themes.md#2-feature-matrix) Name field, focused on open · S
- [E03](gaps/d2-interface-themes.md#2-feature-matrix) Appearance badge · S
- [E04](gaps/d2-interface-themes.md#2-feature-matrix) Preview toggle (live) · M
- [E05](gaps/d2-interface-themes.md#2-feature-matrix) Terminal colours: Background, Text, Cursor, Cursor Text, Selection, Selected Text · M
- [E06](gaps/d2-interface-themes.md#2-feature-matrix) ANSI Standard 0-7 and ANSI Bright 8-15 · M
- [E07](gaps/d2-interface-themes.md#2-feature-matrix) Colour well: swatch + monospaced hex field · M
- [E08](gaps/d2-interface-themes.md#2-feature-matrix) Extended Palette section: Automatically regenerate, Reverse palette (light only) · M
- [E09](gaps/d2-interface-themes.md#2-feature-matrix) Show/Hide extended palette: Color Cube, Grayscale Ramp, Reset Overrides · L
- [E10](gaps/d2-interface-themes.md#2-feature-matrix) Revert... / Cancel / Save &amp; Apply · S
- [E11](gaps/d2-interface-themes.md#2-feature-matrix) Edit, duplicate, rename, delete an existing custom theme · M
- [SW04](gaps/d2-interface-themes.md#2-feature-matrix) Change Theme... as a command · S
- [M01](gaps/d2-interface-themes.md#2-feature-matrix) Color Scheme / Toolbar Layout / Pane Style / Pane Headers / Interface Style submenus · M
- [M02](gaps/d2-interface-themes.md#2-feature-matrix) Reveal Toolbar, Reveal Pane Header, Toggle Toolbar Layout · S
- [M03](gaps/d2-interface-themes.md#2-feature-matrix) Switch Appearance Mode and Import Theme File commands · S
- [X01](gaps/d2-interface-themes.md#2-feature-matrix) Onboarding Appearance step (Color scheme, Interface style, Toolbar layout tiles and a theme strip) · L
- [X02](gaps/d2-interface-themes.md#2-feature-matrix) Terminal &gt; Colors rows that depend on the theme · M · product-rule

Partial (20):

- [S01](gaps/d2-interface-themes.md#2-feature-matrix) Settings container · M
- [S02](gaps/d2-interface-themes.md#2-feature-matrix) Category list and grouping · S
- [S10](gaps/d2-interface-themes.md#2-feature-matrix) Server/API version callout · S
- [S12](gaps/d2-interface-themes.md#2-feature-matrix) Row anatomy · S
- [S14](gaps/d2-interface-themes.md#2-feature-matrix) Segmented control with leading glyphs · S
- [S19](gaps/d2-interface-themes.md#2-feature-matrix) Numeric field with up/down stepper · S
- [S20](gaps/d2-interface-themes.md#2-feature-matrix) Scroll affordance · S
- [I05](gaps/d2-interface-themes.md#2-feature-matrix) Toolbar layout: Above \| Left Side \| Right Side · L
- [I09](gaps/d2-interface-themes.md#2-feature-matrix) Pane header behavior: Always show \| Show on hover \| Hidden · M
- [P01](gaps/d2-interface-themes.md#2-feature-matrix) Minimum padding horizontal and vertical · S
- [P02](gaps/d2-interface-themes.md#2-feature-matrix) Fill with program colors: Off \| Automatic \| Always · M
- [P04](gaps/d2-interface-themes.md#2-feature-matrix) Highlight active pane edges · S
- [TH03](gaps/d2-interface-themes.md#2-feature-matrix) Built-in library · L
- [TH06](gaps/d2-interface-themes.md#2-feature-matrix) Selected state · S
- [TH09](gaps/d2-interface-themes.md#2-feature-matrix) Import Ghostty or iTerm theme file · M
- [SW01](gaps/d2-interface-themes.md#2-feature-matrix) Theme switcher popover (Change Theme...) · S
- [SW02](gaps/d2-interface-themes.md#2-feature-matrix) Switcher scope · S
- [SW03](gaps/d2-interface-themes.md#2-feature-matrix) Search behaviour · S
- [M05](gaps/d2-interface-themes.md#2-feature-matrix) Copy Config File Path / Reload Config · S
- [X03](gaps/d2-interface-themes.md#2-feature-matrix) Settings window is a real window with close and zoom · M · needs-rune-work

#### D3 Terminal ([file](gaps/d3-terminal.md))

Missing (42):

- [T02](gaps/d3-terminal.md#21-settings-page-structure) Settings search ("Search Settings") · M
- [T03](gaps/d3-terminal.md#21-settings-page-structure) Back/Forward history and sub-pages (Advanced Font Settings opens in-page) · M
- [T04](gaps/d3-terminal.md#21-settings-page-structure) Controls are live and previewed in the page (typography, cursor, selection previews) · M
- [T06](gaps/d3-terminal.md#22-typography-and-fonts) Font picker: "Search fonts", Monospaced / All Fonts segmented, alphabetical list, check on current… · M
- [T07](gaps/d3-terminal.md#22-typography-and-fonts) Apply font family live (no relaunch) · L · needs-rune-work
- [T08](gaps/d3-terminal.md#22-typography-and-fonts) Font style popup: Regular, Italic, Bold, Bold Italic · M
- [T10](gaps/d3-terminal.md#22-typography-and-fonts) Font weight field, disabled "fixed by the font" unless variable · M · needs-rune-work
- [T11](gaps/d3-terminal.md#22-typography-and-fonts) Line height adjustment, % (0 = font default) · S
- [T12](gaps/d3-terminal.md#22-typography-and-fonts) Typography preview well (regular, bold, italic sample, branch glyph) · S
- [T13](gaps/d3-terminal.md#22-typography-and-fonts) "Advanced font settings" navigation row · S
- [T14](gaps/d3-terminal.md#22-typography-and-fonts) Character spacing stepper + unit px / % · M
- [T15](gaps/d3-terminal.md#22-typography-and-fonts) Ligatures switch (default On) · L · product-rule
- [T16](gaps/d3-terminal.md#22-typography-and-fonts) Render bold and italic: Both / Bold only / Italic only / Neither · S
- [T21](gaps/d3-terminal.md#23-colours) Improve text contrast (default On): "Adjust unreadable text against its background." · M · product-rule
- [T22](gaps/d3-terminal.md#23-colours) Force theme palette for text: "Map text colors to the nearest theme color." · M · product-rule
- [T25](gaps/d3-terminal.md#24-cursor) Cursor preview well (~/rex $ plus live cursor) · S
- [T26](gaps/d3-terminal.md#24-cursor) Cursor style: Block / Bar / Underline · S
- [T27](gaps/d3-terminal.md#24-cursor) Blink speed: Off / Slow / Normal / Fast / Custom (+ hidden Blink interval row) · M · product-rule
- [T28](gaps/d3-terminal.md#24-cursor) Force style: "Ignore application requests to change cursor shape." · S
- [T35](gaps/d3-terminal.md#25-keyboard) Jump to Previous / Next Prompt (Cmd+Up / Cmd+Down) · M · unknown
- [T37](gaps/d3-terminal.md#26-scrolling-and-resizing) Smooth scrolling (pixel scrollback), default Off · M · product-rule
- [T38](gaps/d3-terminal.md#26-scrolling-and-resizing) Smooth terminal resizing (reflow on the resize frame, default On) · L · product-rule
- [T39](gaps/d3-terminal.md#27-selection-and-clipboard) Selection preview well (On branch feature/tabs) · S
- [T40](gaps/d3-terminal.md#27-selection-and-clipboard) Copy on selection switch (default Off) · S
- [T41](gaps/d3-terminal.md#27-selection-and-clipboard) Play animation on copy (default On) · S · product-rule
- [T44](gaps/d3-terminal.md#27-selection-and-clipboard) Rounded selection corners · S
- [T49](gaps/d3-terminal.md#27-selection-and-clipboard) Select All in Pane (Cmd+A) · S
- [T50](gaps/d3-terminal.md#27-selection-and-clipboard) Primary selection (select to copy, middle-click to paste) · M
- [T53](gaps/d3-terminal.md#28-links) "Links that open apps": Ask First / Always Open / Never Open for other schemes (ssh://, ...) · M
- [T59](gaps/d3-terminal.md#29-find) Prev/next disabled until there are matches · S
- [T63](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Copy (disabled without selection) · S
- [T64](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Paste · S
- [T66](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Split Left, Split Up · S
- [T67](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Float Pane · L · needs-rune-work
- [T70](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Reset Terminal · S
- [T71](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Clear Terminal (unbound action in the beta) · S
- [T76](gaps/d3-terminal.md#211-path-browser-titles-shell) Go to Directory... overlay (Shift+Cmd+G): path field with clear button, "Go to" header, folder… · L
- [T77](gaps/d3-terminal.md#211-path-browser-titles-shell) Insert Path... overlay (Ctrl+Cmd+I): same browser, placeholder ~/, types the path at the cursor · M
- [T78](gaps/d3-terminal.md#211-path-browser-titles-shell) Show recently used paths (shared per host) · S
- [T79](gaps/d3-terminal.md#211-path-browser-titles-shell) Shortcut badge on the Integrated Path Browser heading · S
- [T80](gaps/d3-terminal.md#211-path-browser-titles-shell) Prefer generated titles (program + directory instead of shell-set titles) · M
- [T87](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) Terminal content exposed to assistive technology · L · needs-rune-work

Partial (16):

- [T01](gaps/d3-terminal.md#21-settings-page-structure) Terminal page layout: one scrolling page of 11 sections (Typography, Colors, Cursor, Keyboard… · M
- [T05](gaps/d3-terminal.md#22-typography-and-fonts) Terminal font: current family shown in its own face + "Choose..." · S
- [T18](gaps/d3-terminal.md#22-typography-and-fonts) Fallback fonts: ordered list with "Add..." picker (opens on All Fonts) · M
- [T31](gaps/d3-terminal.md#25-keyboard) Show key sequence hints switch · S
- [T33](gaps/d3-terminal.md#25-keyboard) Font-size shortcuts: Increase (Cmd+= and Cmd++), Decrease (Cmd+-), Reset (Cmd+0) · S
- [T42](gaps/d3-terminal.md#27-selection-and-clipboard) Selection foreground: Theme / Cell Foreground / Cell Background · S
- [T43](gaps/d3-terminal.md#27-selection-and-clipboard) Selection background: Theme / Cell Foreground / Cell Background · S
- [T47](gaps/d3-terminal.md#27-selection-and-clipboard) Copy: Edit &gt; Copy (Cmd+C, disabled with no selection) · S
- [T48](gaps/d3-terminal.md#27-selection-and-clipboard) Paste: Edit &gt; Paste (Cmd+V) · S
- [T52](gaps/d3-terminal.md#28-links) Web, mail and file: links open directly · M · product-rule
- [T56](gaps/d3-terminal.md#29-find) Find bar: pill 300x39 top-right, "Find" placeholder, count 1/1, up/down buttons, close x · S
- [T58](gaps/d3-terminal.md#29-find) Find Next / Find Previous as actions (Cmd+G, Shift+Cmd+G; disabled until a search runs) · S
- [T60](gaps/d3-terminal.md#29-find) Dim terminal and yellow #F4D722 rounded match highlight · S · product-rule
- [T75](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Edit menu entries reachable without a menu bar (Select All, Insert Path, Find) · S
- [T84](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) Pane padding (ADDED, omitted by the first draft although the summary counts it) · S
- [T86](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) Copy from Pane / Paste into Pane / Copy Pane ID actions · S

#### D4 Shortcuts, Commands, menus ([file](gaps/d4-shortcuts-commands-menus.md))

Missing (51):

- [A02](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Lists every action incl. unbound · M
- [A04](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Filter field · M
- [A06](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Bound pill (click to re-record) · L · product-rule
- [A07](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Unbound pill · L · product-rule
- [A08](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Inline recorder · L · product-rule
- [A12](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Configuration (...) menu · M
- [A15](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Global 'Toggle Visibility' hotkey · M · needs-rune-work
- [B01](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) New Window (Cmd N) · XL
- [B03](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Toggle Full Screen (Ctrl Cmd F) · S · needs-rune-work
- [B07](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Rex Diagnostics (Shift Cmd ,) · L
- [B08](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Open Onboarding Session... · L
- [B12](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Pair Device with QR Code... · M · unknown
- [B16](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Dismiss Notice (Shift Cmd .) · S
- [B17](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Change Shortcut... · M · product-rule
- [B19](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Restart Server... · M · product-rule
- [C04](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Next Session / Previous Session · S
- [C09](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Switch Host · S
- [C14](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Close Other Tabs · S
- [C19](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Move Tab Backward / Forward (Ctrl Shift PgUp/PgDn; menu shows Ctrl Shift Up/Down) · M
- [C20](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Move Tab to New Window · XL
- [C21](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Mute Tab · M
- [C22](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Tab menu Move Tab Left / Right · M
- [D03](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Split Pane Left / Up · S
- [D04](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Split Pane Best Fit (Cmd Return) · S
- [D06](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Focus Next / Previous Pane (Cmd \] / Cmd \[) · S
- [D07](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Move Pane Left/Right/Up/Down (Opt Shift Cmd arrows) · M
- [D08](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Float Pane / New Floating Pane / Minimize Floating Window(s) · L
- [D11](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Close Other Panes · M
- [D12](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Balance Splits · M
- [D14](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Move Pane to New Tab / Move Pane to Tab... · M
- [D15](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Go to Directory... (Shift Cmd G) · L
- [D17](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) View Activity... · L
- [D18](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Insert Path... (Ctrl Cmd I) · L
- [D21](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Select All in Pane (Cmd A) · M
- [D24](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Jump to Previous / Next Prompt (Cmd Up / Down) · L · unknown
- [D27](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Clear Terminal / Reset Terminal · S
- [D28](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Key modes: Enter Mode / Exit Mode / Exit All Modes · L
- [D33](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Toggle Activity (Cmd .) / Show Activity by Session / by Priority · L
- [D35](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Toggle Toolbar Layout / Reveal Toolbar / Reveal Pane Header · L · product-rule
- [E08](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Footer 'Change Keybinding Cmd Return / Action Return' · M · product-rule
- [E09](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Row-level rebinding from the palette · M · product-rule
- [E12](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Server actions listed · M
- [F06](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Edit: Select All / Insert Path... · M
- [F12](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Color Scheme Auto / Light / Dark · L · product-rule
- [F13](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Toolbar Layout: Above / Left Side / Right Side; Always Show / Show on Hover / Hidden · XL · product-rule
- [F15](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Pane Headers: Always Show / Show on Hover / Hidden · M
- [G08](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Quick-select session by number · S
- [G10](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Activity popover · L
- [G12](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Go to Directory / Insert Path overlays · L
- [G14](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Status-bar (menu bar extra) menu · M
- [G16](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Pane header controls and drag handle · M

Partial (54):

- [A01](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Page structure · L
- [A03](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Row layout · M · product-rule
- [A05](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Priority helper text · S
- [A09](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Multiple bindings per action · S
- [A10](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Not-recordable rows · S
- [A11](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Parameterised read-only sub-rows · S
- [A14](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Conflict display · M
- [A18](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Sticky heading / scroll clipping · S
- [A19](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Settings window shell around the page · L
- [B02](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Close Window (Shift Cmd W) · S
- [B10](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Copy Version Info · S
- [B11](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Copy Debug Info · S
- [B14](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Check for Updates in Background / Report Update Status · S
- [B15](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Set Update Automation / Set Update Channel · S
- [B18](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) Install Rex for Agents · S
- [C03](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Go to Session (not recordable) · S
- [C06](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Close Session · S
- [C08](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Collapse Session / Expand Session / Collapse Other Sessions · S
- [C11](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Remove Host... · S
- [C13](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Close Tab (Opt Cmd W) · S
- [C18](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) Go to Tab 1..8 and last (Cmd 1..9) · S
- [D13](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Resize Pane (not recordable) · M
- [D16](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Go to Anything... (Cmd P) · M
- [D19](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Copy Pane ID / Rename Pane · S
- [D20](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Paste into Pane / Copy from Pane · S
- [D23](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Send Key to Server · S
- [D30](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Find Next (Cmd G) / Find Previous (Shift Cmd G) · S
- [D31](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Increase / Decrease / Reset Font Size (Cmd + / - / 0) · M
- [D36](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Change Theme... · S
- [D37](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Import Theme File / Switch Appearance Mode (not recordable) · S · product-rule
- [D38](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Copy Config File Path / Reload Config · S
- [E01](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Panel size and placement · S
- [E04](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Rows in Beta palette · M
- [E10](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Selection style · S
- [F02](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) App menu (Rex Beta) · S
- [F03](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) File: New Window, New Session, Add Remote Host..., New Tab, Go to Directory..., Close, Close Tab… · M
- [F05](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Edit: Copy, Paste · S
- [F07](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Edit &gt; Find submenu: Find..., Find Next, Find Previous · S
- [F10](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) View &gt; Pane submenu (20 items in 4 groups) · L
- [F11](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) View &gt; Customize View &gt; Change Theme... · S
- [F14](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Pane Style: Comfortable / Compact · S
- [F17](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) View: font size, Toggle Sidebar, Show Activity, Focus Tab Filter, Reveal Toolbar/Pane Header, Enter… · S
- [F19](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Window menu: Diagnostics, Copy Version Info, Copy Debug Info · M
- [F20](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Window menu: tab nav and Move Tab x4; Previous/Next Session · S
- [F21](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) Help: search, Send Feedback to Apple, Rex Help (Cmd ?) · S
- [G01](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Tab context menu · M
- [G02](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Terminal context menu · M
- [G05](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Menu visual style · M
- [G06](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Destructive item styling · S
- [G09](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Rename tab overlay + 'Restore Default Title' · S
- [G11](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Terminal find bar · M
- [G13](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Theme switcher overlay · S
- [G15](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Confirm dialogs naming target · S
- [G17](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Tab pill close button · S · product-rule

#### D5 Hosts, server, notifications, integrations, onboarding ([file](gaps/d5-hosts-notifications-integrations-onboarding.md))

Missing (33):

- [d5-01](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Settings &gt; Server category with live status subtitle ('Running') · M
- [d5-03](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Stop Server... (destructive, ellipsis = confirmation) · S · product-rule
- [d5-04](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Remote Address row with Copy button · S
- [d5-05](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Pair sheet: QR code of the remote address, host name, address, reachability hint, Done · M
- [d5-06](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Tailscale account row 'Signed in to Tailscale / Signed in as &lt;login&gt;' + 'Sign Out...' · M · product-rule
- [d5-07](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Toggle 'Serve over Tailscale' - 'Connect to your local Rex server over your Tailnet.' · M · product-rule
- [d5-08](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Toggle 'Limit access to your Tailscale account' - 'Rex will verify the caller is signed in as… · S · product-rule
- [d5-09](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Read-only rows Tags (None) and Control Server (Tailscale) with ? help glyphs · S
- [d5-10](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Preferred Hostname field - 'Used to generate the Tailscale address.' · S · product-rule
- [d5-16](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Search Settings field · M
- [d5-17](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Back/Forward capsule and page title; subpages push history (Notifications &gt; Working) · M
- [d5-20](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Agent brand glyph strip (Claude, Codex, OpenCode, Pi) · S
- [d5-29](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Badge Dock icon · M
- [d5-30](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Play sounds (global) and Sound volume slider with speaker glyph buttons · M
- [d5-31](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Per-status sound menu (Attention, Warning, Error, Success, Ready, Loading, Tap) and Play preview · M
- [d5-33](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Status summary rows with coloured circular icon, channel summary subtitle ('Tabs, Notifications… · S
- [d5-34](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Include idle processes in activity · S
- [d5-35](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Show each process in tabs · M
- [d5-37](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Tailscale sign-in / device approval / failures notifications · M
- [d5-38](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Activity popover from the toolbar eye button (grouped by Working/Blocked/..., row = icon, title… · L
- [d5-39](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Mute a tab (tab context menu) · S
- [d5-42](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Notification permission and health · S
- [d5-46](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Host icon picker: Globe, Server, Cloud, MacBook, Mac mini, Mac Studio, Desktop · M
- [d5-47](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Transport trust footnote · S
- [d5-58](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Update channel Preview / Bleeding Edge · S · product-rule
- [d5-65](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Color scheme Auto / Light / Dark · S · product-rule
- [d5-60](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Rex installation: location, ready status, Show Details (Version, Commit, Final SHA-256, copy… · S · macos-only
- [d5-63](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) First-run onboarding flow (5 pages + 1 sheet), no step counter · XL · product-rule
- [d5-64](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Welcome page: mascot 160 pt, 'Get Started with Rex', copy, Continue, 'Skip for now' · S
- [d5-71](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Migrate config step: Claude Code / Codex / OpenCode / Pi / Skip migration tiles, Copy Prompt… · L
- [d5-72](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Finish page: 'You're all set', three keycap rows from the live keymap, 'Start Using Rex', '&lt;… · M
- [d5-73](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Onboarding navigation: circular Previous, 'Continue -&gt;' pill, skip, re-run · S
- [d5-75](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Experimental 'New quick switcher': Go to Anything, Go to Session and View Activity in one list… · L

Partial (18):

- [d5-02](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Server status card: name, 'Running for 10m', info button with Server Version / API Version popover · M
- [d5-11](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Client label (Client Info &gt; Label, placeholder = machine hostname) · S
- [d5-12](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Remote Control: other clients may perform actions in this app · S · product-rule
- [d5-14](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Server listeners list with copy · S
- [d5-15](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Settings window form: separate 816x850 window vs modal · M
- [d5-18](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Sidebar grouping (4 groups) with coloured 20 pt icon tiles · S
- [d5-19](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Master switch 'Integrate with available agents' - 'Installs a Rex skill and session hooks for… · S
- [d5-26](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) In-app notification position · M
- [d5-32](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Per-status subpages (Working, Blocked, Error, Done): Enable status, Show activity in tabs, Show… · M
- [d5-41](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Desktop notification quality: urgency per state, replace/close when the tab is seen, survive a… · M
- [d5-45](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Add host form: URL, Name, Icon · S
- [d5-48](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Sheet anatomy: icon + title + subtitle header, divider, 'Host Details' card, divider, right-aligned… · M
- [d5-55](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) About surface · S
- [d5-56](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Server version in version info and copy · S
- [d5-61](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Local API URL with copy · S
- [d5-67](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Toolbar layout Above / Left Side / Right Side · S
- [d5-68](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Theme picker: 'Default theme' Merino Dark + 26 'Built-in themes' as live preview cards (Aa, ~/… · M
- [d5-69](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Integrations step: Rex Agent Skill card (Recommended, Installed chip) · M

#### D6 Developer, diagnostics, updates, About ([file](gaps/d6-developer-diagnostics-updates.md))

Missing (38):

- [d6.dev.section](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Developer settings category (own sidebar group, hammer tile, last) · M
- [d6.dev.mode](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Developer Mode switch: 'Expose settings for development and debugging.' · S
- [d6.dev.channel](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Update Channel popup: Preview \| Bleeding Edge, subtitle follows selection ('Every commit that… · L
- [d6.dev.rex-location](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Rex Installation &gt; Location popup: Included with Rex.app \| Source Checkout · M
- [d6.dev.rex-status](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Helper status row: green check 'Included Rex ready' (failure states not captured) · S
- [d6.dev.rex-details](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Show Details / Hide Details disclosure: Version, Commit (mono), Final SHA-256 (mono… · M
- [d6.dev.origins](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Web Client Origins allow-list: built-in 'https://rex.superlogical.com' (Default, not removable)… · M · needs-server-api
- [d6.dev.anchoring](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Terminal anchoring popup: Top \| Bottom (Bottom puts sub-cell slack at the top) · M
- [d6.upd.actions](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Update commands bindable as shortcuts / client actions: Install Update, Relaunch when Idle, Check… · S
- [d6.about.icon](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) App icon (Rexie, 72 pt) · S
- [d6.about.server-version](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Connected server version line ('Rex Server 0.1.0-preview.15') · S
- [d6.about.copy-debug](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Copy Debug Info (Window menu) · S
- [d6.entry.diag-shortcut](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Open Diagnostics: Window &gt; 'Rex Beta Diagnostics' Shift-Cmd-, ; palette/shortcut list · S
- [d6.entry.status-item](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Menu-bar extra: Rex Server status (green dot 'Running'), endpoint submenu with copy, version block… · M
- [d6.diag.window](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Diagnostics window: 960x600 default, sidebar + title/subtitle + inspector toggle, traffic lights · L
- [d6.diag.sidebar](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Sidebar, 144 pt fixed: Rex Binary, Rex Server, Rex API, RexSync, Network, GPU, icons, blue selected… · S
- [d6.diag.inspector](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Per-pane inspector column (~280 pt, (i) toggle 'Show/Hide Inspector'); pushes main content · M
- [d6.diag.binary](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Rex Binary pane: status card 'Included Rex ready', Installation card (Kind, Version, Commit mono… · M
- [d6.diag.server.stop](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Stop Server (card button + toolbar glyph) · M
- [d6.diag.server.logs](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Live Logs (monospace, streaming, 'No server output yet.') + Clear Logs · L · needs-server-api
- [d6.diag.server.inspector](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Inspector 'Shared Server': Executable, Endpoint, PID, Rex Version, Sessions, Started, Log, Tailnet… · M
- [d6.diag.server.build](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Inspector 'Server Build': Version, Commit, Working Tree, Platform, Built ('Not reported') · S
- [d6.diag.api.connection](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Rex API connection card: 'Connected to Rex', endpoint, Host popup (saved hosts, check on current) · M
- [d6.diag.api.invoke](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Invoke: Method field (mono, default server.version), JSON Payload editor, 'Blank input is distinct… · L
- [d6.diag.api.calls](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Calls transcript: 'N recorded', Clear, 'No API calls yet.' · M
- [d6.diag.api.events](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Events stream: 'All Sessions', 'N recorded', Clear, time + pretty JSON + 'Copy Rex event' · L
- [d6.diag.api.inspector](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Inspector: Connection State/Subscription, Transcript Calls/Events, Attached clients (Session popup… · M
- [d6.diag.sync.policy](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) RexSync &gt; 'Terminal Policy' card (11 rows: Scheme, Cursor Style, Cursor Blink, Option Key, Event… · M
- [d6.diag.sync.host](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Per-host card titled by endpoint: ID, Kind, Name, Icon, Connection, Resumes, Last Resume Failure… · M
- [d6.diag.sync.session](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Per-session rows: Session ID/Label/Status, Structure Status, Demands, Attachment Status, Window… · M
- [d6.diag.net.controls](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Network control row: Record switch (off), Record from Launch checkbox, Clear, Window… · XL
- [d6.diag.net.metrics](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Metrics strip: Open, Control setup, Request RTT, First byte, Snapshot ready, Traffic · L
- [d6.diag.net.legend-timeline](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) 12-item toggle legend (Ticket..Failure) and 'Network timeline' with App Events and Throughput lanes · XL
- [d6.diag.net.tables](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Six tables: Requests (+Slowest first), Events, Connections (13 cols), Methods, App Events, APCs… · XL
- [d6.diag.net.privacy](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Privacy rule: trace keeps sizes, timings, names, never message bodies; Record is opt-in · S
- [d6.diag.gpu.usage](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) GPU &gt; Usage table (Rex, WindowServer, Total x Now/Mean/Peak), 'Last 120 Seconds' chart, Other… · L
- [d6.srv.uptime](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Settings &gt; Server: 'Running for 10m' under 'Rex Server', and the Server nav item sub-label 'Running' · S
- [d6.help.menu](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Help menu: 'Rex Help' and 'Send Rex Beta Feedback to Apple' (plus the macOS search field) · S · unknown

Partial (12):

- [d6.dev.local-api](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Local API URL (percent-encoded unix:// path) with 'Copy local Rex endpoint' icon button · S
- [d6.dev.quick-switcher](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) 'New quick switcher' switch: one list for Go to Anything / Go to Session / View Activity (sessions… · M
- [d6.dev.quicklook](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) 'Quick Look links' switch: Cmd-Y previews on-screen files and links; Shift-Cmd-click previews one · M · macos-only
- [d6.dev.popup-style](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Settings popup control: right-aligned value + circular chevron badge, check-marked menu with… · M
- [d6.dev.card-style](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Inset grouped cards on the page ground with section heading above, footnote below · M
- [d6.about.window](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) About as a small standalone window (280x243): icon, name, version, channel, server version, Copy… · M
- [d6.about.version](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Client version line ('Version 0.1.0 (1932)') · S
- [d6.about.channel](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Release channel line ('Bleeding Edge') · S
- [d6.about.server-popover](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Server 'ⓘ' popover: Server Version, API Version 'Revision 13' · S
- [d6.about.copy-info](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Copy Version Info (client + channel + server versions) · S
- [d6.diag.copy](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Trailing copy glyph on nearly every value, accessible name 'Copy &lt;label&gt;' · S
- [d6.diag.server.status](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Rex Server pane status card ('Rex server is running', green check) · M


### 3.3 What Linux has that the beta lacks

`linux-ahead` is a capability both products have where Linux does more; `linux-only` has no beta counterpart (or the beta behaviour was never captured, so "ahead" is not claimed).

#### D1 Workspace chrome ([file](gaps/d1-workspace-chrome.md))

Linux-ahead (6):

- [T02](gaps/d1-workspace-chrome.md#2-feature-matrix) Session chip
- [T03](gaps/d1-workspace-chrome.md#2-feature-matrix) Sidebar toggle button
- [S01](gaps/d1-workspace-chrome.md#2-feature-matrix) Session switcher popover
- [S09](gaps/d1-workspace-chrome.md#2-feature-matrix) Move session up / down
- [S10](gaps/d1-workspace-chrome.md#2-feature-matrix) Host-grouped session sidebar
- [C06](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane menu title and target outline

Linux-only (13):

- [W18](gaps/d1-workspace-chrome.md#2-feature-matrix) Window menu on right-click, maximize on double-click
- [T07](gaps/d1-workspace-chrome.md#2-feature-matrix) Settings button in toolbar
- [T08](gaps/d1-workspace-chrome.md#2-feature-matrix) Close-app button
- [T09](gaps/d1-workspace-chrome.md#2-feature-matrix) Pending key-sequence chip
- [TB11](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab menu: New tab entry
- [TB17](gaps/d1-workspace-chrome.md#2-feature-matrix) Tab overflow handling
- [S11](gaps/d1-workspace-chrome.md#2-feature-matrix) Resizable sidebar
- [P06](gaps/d1-workspace-chrome.md#2-feature-matrix) Palette: commands only Linux has (next/prev tab across hosts, history back/forward, Search terminal, Hide…
- [C10](gaps/d1-workspace-chrome.md#2-feature-matrix) Keyboard-opened pane menu
- [N02](gaps/d1-workspace-chrome.md#2-feature-matrix) Confirmations for destructive closes
- [N06](gaps/d1-workspace-chrome.md#2-feature-matrix) Empty and error states of the workspace
- [N07](gaps/d1-workspace-chrome.md#2-feature-matrix) Drop files onto a pane
- [N08](gaps/d1-workspace-chrome.md#2-feature-matrix) Pane reconnect strip (`Reconnecting · input paused`, Retry)

#### D2 Settings shell, Interface, Themes ([file](gaps/d2-interface-themes.md))

Linux-ahead (1):

- [TH04](gaps/d2-interface-themes.md#2-feature-matrix) Ghostty theme catalogue

Linux-only (6):

- [S06](gaps/d2-interface-themes.md#2-feature-matrix) Updates and About categories
- [I16](gaps/d2-interface-themes.md#2-feature-matrix) Chrome theme (4 dark palettes)
- [I17](gaps/d2-interface-themes.md#2-feature-matrix) Interface zoom 70-150% with Reset
- [I18](gaps/d2-interface-themes.md#2-feature-matrix) Session sidebar switch
- [I19](gaps/d2-interface-themes.md#2-feature-matrix) Notices position: Top right \| Bottom right
- [TH10](gaps/d2-interface-themes.md#2-feature-matrix) Terminal theme semantics: Follow server \| Rex default colours \| Follow chrome

#### D3 Terminal ([file](gaps/d3-terminal.md))

Linux-ahead (2):

- [T74](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Menu header naming the target, shortcut hints
- [T81](gaps/d3-terminal.md#211-path-browser-titles-shell) Shell startup command (global, local only, quoted args, placeholder "Default login shell")

Linux-only (13):

- [T19](gaps/d3-terminal.md#22-typography-and-fonts) --font, --font-fallback, --font-size launch overrides
- [T20](gaps/d3-terminal.md#22-typography-and-fonts) Missing primary font falls back to bundled IBM Plex Mono with a notice
- [T24](gaps/d3-terminal.md#23-colours) Terminal theme (lives on the Themes page in the beta)
- [T29](gaps/d3-terminal.md#24-cursor) Hollow outline cursor when pane is unfocused / not the keyboard owner
- [T45](gaps/d3-terminal.md#27-selection-and-clipboard) Word and line selection (double and triple click), content-pinned selection, edge autoscroll
- [T46](gaps/d3-terminal.md#27-selection-and-clipboard) Shift+drag selects locally in mouse-tracking TUIs
- [T51](gaps/d3-terminal.md#27-selection-and-clipboard) OSC 52 writes, server-driven "selection copied"
- [T54](gaps/d3-terminal.md#28-links) Link affordance: hover underline, hand cursor, tooltip with the real target
- [T55](gaps/d3-terminal.md#28-links) OSC 8 links and wrapped plain-text URLs
- [T61](gaps/d3-terminal.md#29-find) "No matches" status
- [T73](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) Rename pane, Reconnect terminal in menu
- [T83](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) Box-drawing sprites, curly/dotted underlines, Kitty graphics (static), mouse reporting
- [T89](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) Drop or paste files and images onto a pane to upload them to the terminal's machine (ADDED, Linux-only)

#### D4 Shortcuts, Commands, menus ([file](gaps/d4-shortcuts-commands-menus.md))

Linux-ahead (7):

- [A13](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Override source / layers
- [D25](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Scroll Page Up / Down
- [E03](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Sections
- [E05](gaps/d4-shortcuts-commands-menus.md#e-command-palette) Rows only in Linux palette
- [G03](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Session context menu
- [G04](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Host context menu
- [G07](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) Session switcher

Linux-only (5):

- [A16](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Pointer bindings row
- [A17](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) Files section
- [D39](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Pane actions (Ctrl Shift Space)
- [D40](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Reconnect terminal (Ctrl Shift R) / Reconnect host
- [D41](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) Mouse Back / Forward history; Ctrl Tab cycling across hosts

#### D5 Hosts, server, notifications, integrations, onboarding ([file](gaps/d5-hosts-notifications-integrations-onboarding.md))

Linux-ahead (3):

- [d5-21](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Supported agents
- [d5-22](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Per-agent status, Update/Replace/Remove, dry-run preview listing each file
- [d5-57](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Check for updates

Linux-only (9):

- [d5-24](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Check again (re-detect agents)
- [d5-25](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Intro paragraph explaining skill vs hook
- [d5-40](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Notification click raises the window and selects host, session, tab
- [d5-49](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Tailnet discovery list (Rex servers / Tailnet / Offline, probes, 'Rex check failed')
- [d5-50](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Add over SSH (user, socket auto-find, BatchMode) and 'Start personal server...'
- [d5-51](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Connect to Rex form showing server-reported listeners and trust model
- [d5-52](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Edit host (name, endpoint, default shell), Remove host confirmation, per-host default shell
- [d5-53](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Offline/Reconnecting words, Retry, toolbar 'Reconnecting'
- [d5-74](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) Tab progress bar and Bell word

#### D6 Developer, diagnostics, updates, About ([file](gaps/d6-developer-diagnostics-updates.md))

Linux-ahead (0):

- none

Linux-only (7):

- [d6.upd.auto-check](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Automatic update checks switch + daily schedule
- [d6.upd.signed](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Signed release metadata, anti-rollback, unsupported-floor handling
- [d6.upd.guidance](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Install-channel detection and upgrade command with Copy command (pacman, tarball, deb, nix, source)
- [d6.upd.cli](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) --check-update CLI with exit codes 0/10/1
- [d6.about.links](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Repository / Release notes / License buttons
- [d6.diag.frame-stats](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) Per-frame CPU timing (p50/p95/max update + encode) for rendering performance
- [d6.upd.probe](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) rex-update-probe protocol-only binary for the update check


## 4. UI quality

Goal: equal quality and equal capability in a Linux-native idiom. This is not a brief to match macOS pixel for pixel. Rex Linux takes Rune's native rendering and kit defaults (project note "Native Rune look": do not restore old pixels, do not copy beta hex values), keeps its tokens in `src/app/theme.rs`, and follows [DESIGN.md](../../DESIGN.md) and [PRODUCT.md](../../PRODUCT.md). Beta values below are sampled from the 1x captures and are given to describe a design language, not to be pasted into code.

### 4.1 The beta's design language

**Surfaces.** A dark ladder with clear steps rather than one flat surface. Window chrome `#0B0F13`; settings and onboarding content on the same ground; sidebars and sheets `#0F1217`; the pane is a card `#0F1216` with a 1 px `#222529` border inset 8 pt in the chrome (radius about 8-10), so chrome and terminal are two different surfaces; settings cards `#12161A` (radius 12, no border, no shadow); inputs `#1C1D20` with a 1 px `#26292D` border (radius 5-6); neutral capsule buttons `#22262A`. Floating panels (session switcher, palette, activity, rename, find, Go to, Insert Path, theme switcher) share one construction: 1 px outer hairline `#020203`, 1 px inner highlight `#333436`, opaque body `#1C1D20`, radius 10-12 (find bar and search capsule fully rounded), soft shadow. Native menus are translucent system menus (`#2B2E33`, radius about 10).

**Accent and selection.** One saturated blue (`#006FFE` to `#0676FD`) is used for switches, primary buttons (with a dark label), the Back/Forward and focus ring, and as a full-width selection pill (`#0055BF`, radius 8, 5 pt inset, white text, hint `#B3CCEC`) in every list: switcher, palette, activity, theme list, path browser. Selection is unmistakable and identical across lists. Destructive controls are `#FF5658` on `#3A2023`. Colour otherwise appears only as small gradient status circles and the coloured category tiles.

**Type.** SF Pro throughout chrome: 13 pt for row labels, tab titles and menu items; 11 pt helper text (`#767C81`) directly under a label; 10.5 pt footnotes below a card; section headings 13 pt semibold outside the card; page titles 15-16 pt semibold; sheet titles 17 pt bold; onboarding headings 26 pt bold (32 on page 1). Monospace is reserved for terminal text, hex values in the theme editor, commits, hashes, JSON, logs and method names. Copy is short: a label and at most one sentence.

**Density and geometry.** Toolbar 46 pt (Linux 52); tab capsule 242 x 32 with a full radius, 1 px `#3E4044` border and an 18 pt dark rounded-square badge holding the program icon; session chip 123 x 34, one line, no chevron, no host name; palette rows 28 pt in a 330 pt panel (Linux 460); session switcher 196 pt wide (Linux 340); settings rows 38-39 pt with no helper, 45 pt with one, 60 pt with two lines; controls 24 pt tall (segmented 30); switch 36 x 16 hit area; pane header buttons 22 pt on a 24 pt pitch.

**Settings shell.** A real 816 x 850 window with traffic lights, no title strip. Fixed 214 pt sidebar (`#0F1217`) with a capsule search field `Search Settings`, then nine categories in four groups (Server; Interface, Themes; Terminal, Shortcuts, Integrations, Notifications, Remote Hosts; Developer) on a 32 pt pitch with 13 pt gaps between groups, each with a 20 pt coloured rounded-square icon tile; the selected row is a rounded `#191D22` fill with a semibold label. The content header is a Back/Forward capsule (76 x 36) plus the page title; sub-pages (Tabs, Panes, each notification status) push history and change the title. Content is a 561 pt column of inset cards under bold headings, with a divider between every row and a footnote under the card; overlay scrollbar visible only while scrolling.

**Control vocabulary.** Switch; glyph-led segmented control (capsule, 28 pt selected pill); popup button (current value plus a 20 pt circular chevron badge, opening a menu over the control with the current item checked); disclosure row (`>` at the right edge, pushes a sub-page); capsule push buttons 24 pt (neutral, destructive, primary with dark label); bordered stepper with the unit outside; slider with ticks; 92 x 92 picture tiles with a 2-3 pt accent ring for appearance choices; a six-position mini-screen picker for notification position; keycap chips; a `+ New` link button. Dependent controls are disabled with reduced alpha, never hidden, except that the `Sound` row appears only while `Play sound` is on.

**Iconography.** Thin monochrome line glyphs (about 15 pt, `#A3A9AF`) on a 34 pt pitch in the toolbar; 20 pt coloured rounded-square tiles for settings categories; 24 pt gradient circles with white glyphs for the four statuses (Working blue, Blocked orange, Error red, Done green); blue folder icons in the path browser; per-program badges on tabs.

**Motion.** Snappy (default), Natural or Off, applied to chrome; native fades for menus and sheets. The beta also animates terminal content (cursor blink, smooth scrolling, copy flash), which Linux must not.

**Empty and error states.** A centred glyph, a 14 pt semibold title, one sentence and one action (Remote Hosts: globe, `No Remote Hosts`, `Add a Rex HTTP or HTTPS address...`, `Add Remote Host...`). The Network pane uses grey skeleton bars instead (not to be copied).

**Dialogs.** Sheets 480 pt wide, radius 16, a 64 pt header (28-32 pt icon, 17 pt bold title, 13 pt subtitle), a divider, grouped body, a divider, and a footer with a tinted Cancel and a filled default action.

### 4.2 What Linux is on purpose (keep)

DESIGN.md sets a calmer language. These are decisions, not defects, and the report recommends keeping them: a single dark surface where a lone pane is full bleed and reads as one surface with the toolbar (no card-on-chrome for one pane); one working toolbar row; "quiet when healthy" (words, never status dots or "Connected" labels; the one exception is activity words and progress bars on tabs); selection that is never fill alone; 12 pt popover radius with springs from the anchor (`POPOVER`, `SNAPPY`, `STILL`); Inter at 10-13 pt with 10.5 pt small-caps group labels; 15 pt authored stroke icons on a 20 x 20 box; no terminal dimming, recolouring or animation; no keyboard-hint footers; no tab close button (until decided); terminal search match as an overlay; dark chrome only (until decided).

### 4.3 Cross-cutting differences and the Linux-native change

| Area | Beta | Linux now | Change (Rune views and tokens) |
|---|---|---|---|
| Settings shell | Real 816 x 850 window, fixed 214 pt sidebar with search, grouped icon tiles, Back/Forward, sub-pages | 760x520 modal clamped to 92% x 88%; 190 pt text-only nav with an `x`; 150 px dead space on Interface, clipped rows on Terminal and Integrations; no search or history | `settings()` in `settings_view.rs`: width about 816 (clamped to 92% of the viewport), height `min(content, 88%)` with a 480 floor, `scroll_edge_fade(12.)` on the body (already used in `sidebar.rs`); search field above the list; group gaps and 20 pt authored stroke icons in a tinted rounded square; Back/Forward pair wired to a page stack in `workspace.rs` (`open_section`); sub-pages for Tabs, Panes and notification statuses. Keep the modal frame (Rex draws one window) |
| Grouping | Cards under bold headings with a divider between every row and a footnote below | Flat rows on one surface; letter-spaced caps group labels; dividers placed by hand and missing in places | `section(title, card, footnote)` and `card(rows)` helpers in `settings_view.rs` (fill `white(0.035-0.04)` over `POPOVER`, `CARD_RADIUS` 12, 10 pt padding, automatic inset dividers); headings `medium(13.)` in `INK`; keep caps only in popups. No page places a divider by hand |
| Row rhythm and alignment | Fixed 39/45 pt rows; every control right-aligned 10 pt from the card edge | Controls and value readouts right-align at different x; rows without helper are shorter | `setting()` (`settings_view.rs:14-35`): `min_h(39.)`, 45 when a detail exists, a fixed trailing slot with equal `min_w` per control kind, `segmented_fixed(width)` |
| Control vocabulary | Popup buttons, glyph segments, disclosure rows, sliders, picture tiles, six-position picker | `Change...` pills that close Settings and open a detached top-centre popup; text-only segments; round -/+ steppers | `select_row()` / `value_popup(label, value, options)` (kit `Select` or a pill with a trailing chevron) opening a `list_popup` anchored to the control's rect (`panel.anchor`, already honoured in `popups.rs`); `disclosure_row`; `picture_tile` drawn from Rune elements (miniature window, no bitmaps); `slider` from `El::on_step` and `on_drag` (the kit has no Slider); glyph segments for toolbar layout and pane style (icons already exist as 20x20 strokes). Keep the searchable popup only for the 600-entry terminal theme list, opened over Settings, not instead of it |
| Selection and lists | Saturated accent pill with white text, identical in every list | Grey-blue `#222A38`-`#363F4C` fill, barely distinct from hover; menus flat with no separators; destructive items look like the rest; hint and chevron columns ragged on some rows | New `SELECTION` token (`ACCENT` at about 18-28% plus a 1 pt `ACCENT` outline, text stays `INK`) used by `list_popup` for selected rows (`popups.rs` around `:237`), satisfying DESIGN.md "never fill alone"; a separator row kind in `list_popup`; a `danger` flag (text in `DANGER`) placed after a separator; always reserve a 12 pt chevron column. Recommend this Linux-native treatment over a solid blue pill (design call, section 5) |
| Shortcut notation | Compact glyph strings, aligned right | `Ctrl Shift D` in menus, `^K` in the sidebar, hard-coded tooltips that lie after a rebind | One formatter in `widgets.rs` (keycap chips, `MUTED`, 10.5 pt) used by menus, Commands, switcher, tooltips and the sidebar hint, fed from `effective_keymap()` as Commands details already are (`panel.rs:861-873`); remove hard-coded strings in `toolbar.rs:44,292-359` and `sidebar.rs:440` |
| Density and shape | Toolbar 46 pt, tab capsule 32 pt, session chip 123 x 34, palette 330, switcher 196 | Toolbar 52 pt, tab 28 pt (radius 8, bare glyph), chip 210 x 34 two lines with chevron, Commands 460, switcher 340 | Tab pill 30-32 pt with radius h/2 and a 20 pt icon badge (white 6% rounded square) behind `program_icon`; chip sized to content (min 120) and the host line hidden when there is one connected host; `TOOLBAR` 52 to 46-48; Commands about 380 pt, switcher about 300 pt (`popups.rs` constants). Needs a design call; DESIGN.md sizes change with it |
| Pane chrome | Always-on header with title and five 22 pt buttons; single pane is still a card | 30 pt text-only header, absent for a single pane; flat single pane | Trailing icon cluster in `header()` (`panes.rs:88-151`: 24 pt targets, `MUTED` at rest, `INK` on hover, `DANGER` on Close hover, unavailable actions at 35% opacity), a `pane_header` setting (Always, On hover, Hidden). Keep the single-surface pane by default |
| Type | SF Pro; 13/11/10.5 scale; monospace for code-like values | Inter 10-13 pt; helper paragraphs of up to four lines; paths, commit and hex in the UI face | Helpers rewritten to one sentence of at most 70 characters with explanation moved to a footnote (`settings_view.rs`); a `mono(size)` TextStyle on the bundled IBM Plex Mono for commits, endpoints, paths, ids, hex and JSON (`theme.rs`, `fonts.rs`); pane title 12.5 pt regular; one `format_version()` (no `v` prefix) |
| Colour | Saturated accent plus coloured status circles and tiles | Monochrome chrome, `ACCENT` only for focus, selection and current emphasis, words for state | `status_working/blocked/error/done` tokens per palette (a mixed tint, not a gradient) with 24 pt circle glyphs (`DOTS`, `BAN`, `ALERT`, `CHECK` via `icon!`) in the Notifications page, Activity popover and tab pills; `SEARCH_MATCH` and `SEARCH_CURRENT` tokens (current at 80% `ACCENT` with `ON_ACCENT` text, others 40%) still drawn as overlays so cell colours never change; keycap chips raised to `INK` 11 pt on `CHIP` (MUTED on CARD passes 4.5:1, the chips do not). Do not hard-code beta hexes |
| Iconography | Coloured category tiles, program badges, thin line toolbar glyphs | Authored 20x20 1.5 stroke monochrome icons; sliders glyph for Settings; one monochrome program icon per tab | Gear for Settings (`toolbar.rs:343`); agent glyphs as 16 pt leading icons in Integrations rows; folder icon for the path browser; optional tab icon count and colour modes (Tabs sub-page); host icon choice (Globe, Server, Cloud, Laptop, Desktop, Mini PC) |
| Motion | Snappy / Natural / Off | Fixed springs | `reduce_motion` / `animation` setting scaling `SNAPPY`/`POP` in `theme.rs:204-216` and using `tx.instant()` when Off; honour `prefers-reduced-motion` from the portal if Rune exposes it; terminal content stays unanimated |
| Empty and error states | Glyph, title, one sentence, one action; coloured status glyph | Grey sentences ("Waiting for connection...") with no icon or action; probe errors mixed into helper text | `empty_state(icon, title, body, action)` in `widgets.rs` for discovery, Remote Hosts, Integrations "rex not found", Activity ("Nothing needs you") and Diagnostics ("Recording is off"); three tiers for diagnostics (status line in `WARNING`/`ACCENT` with a glyph, muted helper, monospace block with copy) and a `Collapsible` for probe details |
| Dialog anatomy | Header with icon, title, subtitle; grouped body; footer | Title plus stacked labelled fields; primary button under the last paragraph; low-contrast placeholder, weak focus | `sheet(icon, title, subtitle, body, footer)` over `modal_frame` in `dialogs.rs`; 64 pt header and `white(0.06)` dividers; placeholder `mix(muted(), ground(), 0.45)` and a 2 pt `ACCENT` focus ring on fields; right-aligned primary; keep 338 pt only for yes/no confirmations |
| Previews | Live preview wells, theme cards (`Aa`, `~/` chip, cursor pill, eight dots), tab and cursor previews | None; font changes apply after relaunch | `preview_well()` (inset 6, 1 px border, radius 8, terminal font and background tokens) and `theme_swatch(&TerminalTheme)` in a new `themes_view.rs`; previews read local palette data and never recolour terminals; the cursor preview is steady (no blink) |
| Search | `Search Settings`, Filter Shortcuts, `Filter or create...` | Settings, Shortcuts and Themes have no search | Row registry (label, helper, section) driving a nav filter; pinned `Filter shortcuts...` accepting `ctrl+k` style queries; diacritic-insensitive theme search (`rose` should find Rosé Pine, `panel.rs:469`) |

### 4.4 Specific view and token changes, in one list

New tokens in `theme.rs`: `SELECTION`, `SEARCH_MATCH`, `SEARCH_CURRENT`, `status_*` (four), a `mono` text style, a `reduce_motion` branch for `SNAPPY`/`POP`/`STILL`, preview-well and keycap-chip values, a 22 pt icon-badge constant. New icon assets via `icon!`: gear, chevron up/down, folder, ban, alert, check, dots, host kinds, agent marks.

New shared widgets in `widgets.rs` and `settings_view.rs`: `section`, `card`, `setting` (fixed slot), `segmented_fixed`, `select_row`/`value_popup`, `disclosure_row`/`disclosure`, `picture_tile`, `slider`, `preview_well`, `empty_state`, `copy_value_row`, `keycaps` formatter, `status_icon`. New view files: `themes_view.rs`, `server_page.rs`, `pair.rs`, `directory.rs` (path browser), `onboarding.rs`, `diagnostics.rs` plus `diagnostics/*.rs`.

Edits to existing views: `toolbar.rs` (tab pill, chip, tooltips, Activity button, gear icon), `panes.rs` (header cluster, find bar, visibility modes), `popups.rs` (separators, danger rows, chevron column, selection), `panel.rs` (menus, Commands groups, Settings sections), `sidebar.rs` (count badge shown only above one tab, worst-state word, tab row menu), `dialogs.rs` (`sheet`, Use default title), `about.rs`, `updates.rs`, `integrations.rs`, `discover.rs`.

### 4.5 Do not copy

Beta defects and idioms the Linux version should avoid: the transparent Diagnostics title bar that overprints row text (use an opaque 52 pt header with a hairline when scrolled); the Network pane's collapse at 960 pt (truncated controls, hyphenated legend words, hidden columns; use wrapping rows and flow chips); mid-word wrapping of hostnames and Swift debug strings (`HostID(rawValue: "...")`) in the inspector and RexSync (middle ellipsis, raw ids, tooltip plus copy); skeleton-bar empty states (use text); three always-on disabled switches with no explanation (Linux already explains disabled switches; keep it); a Check for Updates with no feedback (Linux shows a result); the Manage Permissions sheet missing from the accessibility tree; the terminal dimming, cursor blink, smooth scrolling and copy animation; the card-on-chrome single pane; Glass and traffic lights.

### 4.6 Quality items that need a DESIGN.md decision

These change a written rule. Each is flagged again in the roadmap; the recommendation is the report's, not a decision.

| Decision | Recommendation |
|---|---|
| Selection treatment: solid accent pill (beta) vs tinted fill plus outline | Tinted fill plus outline (`SELECTION`); satisfies "never fill alone" and stays in the native Rune idiom |
| Tab shape and close button: 30-32 pt capsule with a badge and a visible `x` | Capsule and badge yes; `x` only on the active tab and on hover, or keep the current "no close button" rule; owner call |
| Toolbar height 52 to 46-48 and popover widths | Yes, reclaim 4-6 pt of terminal height; owner call on exact values |
| Card-on-chrome for a single pane | No. Keep the single surface; record it as an intentional deviation in README or DESIGN.md |
| "Keep 190 pt keycap columns" and the 760x520 Settings size | Amend: label left, keycaps right in cards; Settings about 816 wide |
| Keyboard-hint footer in the palette (beta: `Change Keybinding`, `Action`) | Keep the rule; use `Ctrl Enter` on a highlighted row to open its binding editor, with a tooltip |
| Light chrome, Auto, Tint with theme | Reverse "no light chrome" only with a light `Palette`, contrast checks and the portal colour-scheme signal; see M6 |
| Take control strip described in DESIGN.md but stubbed in code (`connection.rs:1917`) | Implement or delete the DESIGN.md paragraph; README must stay honest |
| Status words and green checks inside Diagnostics (`Connected to Rex`) | Allowed only inside the explicit Diagnostics surface, in `LIVE`; chrome stays quiet; document the exception |

## 5. Blocked or out of scope

Rows that cannot be closed by client work alone, grouped by why. The raw matrices hold 95 rows with a blocker other than `none` among missing, partial and not-applicable rows (67 gap rows and 28 not-applicable rows), plus 11 not-applicable rows with no blocker. Nothing needs a new server API except live server logs and Web Client Origins: every other server-side claim was checked against `vendor/rexrs` (`session.move_window`, `float_block`, `new_layer`, `set_layer_state`, `swap_blocks`, `move_block`, `focus_next_pane`, `new_split` with `side`, `resize_pane`, `terminal.list_dir`, `.pwd`, `.reset`, `.clear`, `server.version`, `server.status`, `client.set_label`). Per-tab mute and Balance splits have no server method and are client-side by design.

### 5.1 Forbidden or constrained by product rules (53 rows)

| Row | Capability | Status | Rule or reason |
|---|---|---|---|
| [d1 W13](gaps/d1-workspace-chrome.md#2-feature-matrix) | Chrome appearance: Color scheme Auto/Light/Dark, Interface style Classic/Glass… | missing | DESIGN.md: dark chrome only, no light palette, no Glass. Reversing needs a decision (M6.1) |
| [d1 TB02](gaps/d1-workspace-chrome.md#2-feature-matrix) | Tab close x on the pill | missing | DESIGN.md: tabs have no close button (close by menu, middle-click or Ctrl Shift W). Owner call; proposal is an x on the active and hovered tab only |
| [d1 P02](gaps/d1-workspace-chrome.md#2-feature-matrix) | Palette footer hints (Change Keybinding ⌘↩, Action ↩) | missing | DESIGN.md: no keyboard-hint footers. Alternative: Ctrl Enter on a highlighted row opens its binding editor |
| [d1 PN01](gaps/d1-workspace-chrome.md#2-feature-matrix) | Pane card | partial | DESIGN.md: a single pane is full bleed and one surface with the toolbar. Keep as an intentional deviation |
| [d1 PN19](gaps/d1-workspace-chrome.md#2-feature-matrix) | Fade inactive panes (Off \| Surface \| Content \| Surface &amp; Content), Fade panes… | missing | Content fade dims terminal output; the client never recolours terminal content (AGENTS.md). A header-only or surface fade is allowed |
| [d1 F03](gaps/d1-workspace-chrome.md#2-feature-matrix) | Match highlight and dimming | partial | Dimming the terminal during find is a local recolour. The match highlight itself is allowed as an overlay |
| [d1 N05](gaps/d1-workspace-chrome.md#2-feature-matrix) | Accessibility tree | partial | PRODUCT.md puts AT-SPI out of scope; README says terminal accessibility is not implemented |
| [d2 S04](gaps/d2-interface-themes.md#2-feature-matrix) | Server category (status, Tailscale, client info) | not-applicable | Linux does not host a server; server identity belongs on a read-only Server page (d5) |
| [d2 I01](gaps/d2-interface-themes.md#2-feature-matrix) | Color scheme: Auto \| Light \| Dark | missing | DESIGN.md: no light chrome palette |
| [d2 I04](gaps/d2-interface-themes.md#2-feature-matrix) | Tint with theme | missing | Tint with theme for a light terminal theme needs a light chrome, which DESIGN.md forbids |
| [d2 P05](gaps/d2-interface-themes.md#2-feature-matrix) | Fade inactive panes: Off \| Surface \| Content \| Surface &amp; Content | missing | Content fade recolours terminal output (see d1 PN19); surface fade is allowed |
| [d2 X02](gaps/d2-interface-themes.md#2-feature-matrix) | Terminal &gt; Colors rows that depend on the theme | missing | Terminal colours are server-owned; the client declares a theme but never recolours output |
| [d3 T15](gaps/d3-terminal.md#22-typography-and-fonts) | Ligatures switch (default On) | missing | DESIGN.md: ligatures and kerning are off so the cell grid stays exact |
| [d3 T21](gaps/d3-terminal.md#23-colours) | Improve text contrast (default On): "Adjust unreadable text against its… | missing | The client never recolours terminal output; a legibility transform needs an explicit exception like cursor-text |
| [d3 T22](gaps/d3-terminal.md#23-colours) | Force theme palette for text: "Map text colors to the nearest theme color." | missing | The client never recolours terminal output; palette forcing needs the same explicit exception |
| [d3 T27](gaps/d3-terminal.md#24-cursor) | Blink speed: Off / Slow / Normal / Fast / Custom (+ hidden Blink interval row) | missing | Terminal content never animates (cursor blink draws steadily) |
| [d3 T37](gaps/d3-terminal.md#26-scrolling-and-resizing) | Smooth scrolling (pixel scrollback), default Off | missing | Terminal content never animates (no smooth scrolling) |
| [d3 T38](gaps/d3-terminal.md#26-scrolling-and-resizing) | Smooth terminal resizing (reflow on the resize frame, default On) | missing | Size ownership is automatic and server-driven; reflow timing is not a client decision |
| [d3 T41](gaps/d3-terminal.md#27-selection-and-clipboard) | Play animation on copy (default On) | missing | Terminal content never animates (no copy animation) |
| [d3 T52](gaps/d3-terminal.md#28-links) | Web, mail and file: links open directly | partial | links.rs:5-9 refuses file: on purpose (remote output can name a local path); reversing is a security decision |
| [d3 T60](gaps/d3-terminal.md#29-find) | Dim terminal and yellow #F4D722 rounded match highlight | partial | Dimming the terminal is a local recolour; the yellow highlight alone is allowed as an overlay |
| [d4 A03](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) | Row layout | partial | DESIGN.md "keep 190 pt keycap columns"; amend in the same change as the card layout |
| [d4 A06](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) | Bound pill (click to re-record) | missing | PRODUCT.md: Settings never writes the override file. Needs a GUI override layer decision (M4.4) |
| [d4 A07](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) | Unbound pill | missing | As A06 |
| [d4 A08](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) | Inline recorder | missing | As A06 |
| [d4 B13](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) | Install Update (Shift Cmd U) / Relaunch when Idle (Opt Shift Cmd U) / Install… | not-applicable | PRODUCT.md: no self-installing updates; Linux shows a signed notice and a copyable upgrade command |
| [d4 B17](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) | Change Shortcut... | missing | As A06 |
| [d4 B19](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) | Restart Server... | missing | The client never stops or reconfigures a server on attach (AGENTS.md); an explicit confirmed stop is a decision |
| [d4 D35](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) | Toggle Toolbar Layout / Reveal Toolbar / Reveal Pane Header | missing | DESIGN.md: one toolbar row; fixed top toolbar |
| [d4 D37](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) | Import Theme File / Switch Appearance Mode (not recordable) | partial | Chrome is dark only (DESIGN.md); import and appearance switching are theme-editor work |
| [d4 E08](gaps/d4-shortcuts-commands-menus.md#e-command-palette) | Footer 'Change Keybinding Cmd Return / Action Return' | missing | DESIGN.md: no keyboard-hint footers |
| [d4 E09](gaps/d4-shortcuts-commands-menus.md#e-command-palette) | Row-level rebinding from the palette | missing | Depends on the GUI override layer decision (M4.4) |
| [d4 F12](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) | Color Scheme Auto / Light / Dark | missing | DESIGN.md: no light chrome palette |
| [d4 F13](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) | Toolbar Layout: Above / Left Side / Right Side; Always Show / Show on Hover /… | missing | DESIGN.md: one toolbar row; fixed top toolbar |
| [d4 G17](gaps/d4-shortcuts-commands-menus.md#g-context-menus-and-overlays) | Tab pill close button | partial | DESIGN.md: tabs have no close button (same as d1 TB02) |
| [d5 d5-03](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Stop Server... (destructive, ellipsis = confirmation) | missing | Attachment-only: server writes need an explicit confirmed action naming the effect; recommend not building by default |
| [d5 d5-06](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Tailscale account row 'Signed in to Tailscale / Signed in as &lt;login&gt;' + 'Sign… | missing | As d5-03 (Tailscale sign-out) |
| [d5 d5-07](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Toggle 'Serve over Tailscale' - 'Connect to your local Rex server over your… | missing | As d5-03 (Serve over Tailscale) |
| [d5 d5-08](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Toggle 'Limit access to your Tailscale account' - 'Rex will verify the caller… | missing | As d5-03 (Limit access to Tailscale account) |
| [d5 d5-10](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Preferred Hostname field - 'Used to generate the Tailscale address.' | missing | As d5-03 (Preferred Hostname) |
| [d5 d5-12](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Remote Control: other clients may perform actions in this app | partial | Linux keeps per-host, default-deny remote control with a pinned allowed set (AGENTS.md); align copy and placement only |
| [d5 d5-58](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Update channel Preview / Bleeding Edge | missing | PRODUCT.md: stable signed releases only; no client Preview channel (M6.4) |
| [d5 d5-59](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Download / install updates automatically | not-applicable | PRODUCT.md: no binary download, no self-update (re-scored from missing to match d6.dev.auto-download and d6.dev.auto-install) |
| [d5 d5-63](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | First-run onboarding flow (5 pages + 1 sheet), no step counter | missing | Attachment-only: a first-run flow must be client-local and never create a session. Buildable as M4.5; the rule shapes it |
| [d2 I02](gaps/d2-interface-themes.md#2-feature-matrix) | Follow the system appearance (Auto) | missing | Needs the light palette of I01 first; same rule (blocker corrected from none by the completeness review) |
| [d2 P07](gaps/d2-interface-themes.md#2-feature-matrix) | Autobalance splits | missing | An automatic client rebalance of a shared server layout conflicts with AGENTS.md (split geometry is server-owned); the explicit Balance Splits action is client-only (d1 PN14, M4.2) |
| [d6 d6.dev.channel](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | Update Channel popup: Preview \| Bleeding Edge | missing | Widens the update trust surface; decision D9 |
| [d5 d5-65](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Color scheme Auto / Light / Dark | missing | DESIGN.md: dark chrome only (re-scored from not-applicable to match d1 W13, d2 I01, d4 F12) |
| [d5 d5-66](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Interface style Classic / Glass | not-applicable | DESIGN.md: dark chrome only, no Glass |
| [d6 d6.dev.auto-download](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | 'Download new updates when available' switch (ON, forced/disabled) | not-applicable | PRODUCT.md: no binary download, no self-update |
| [d6 d6.dev.auto-install](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | 'Install updates automatically' switch: relaunch while not visible, sessions… | not-applicable | PRODUCT.md: no self-update |
| [d6 d6.dev.cursor-shaders](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | 'Cursor shaders' switch: Ghostty-style shader cursor that follows focus between… | not-applicable | Terminal content never animates |
| [d6 d6.diag.binary.launch](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | Inspector 'Launch' &gt; Command rex server start (mono, copy) | not-applicable | The client never starts a server except through the explicit confirmed Start personal server |

### 5.2 Needs Rune work (13 rows)

| Row | Capability | Status | What is missing |
|---|---|---|---|
| [d1 W03](gaps/d1-workspace-chrome.md#2-feature-matrix) | Multiple windows (New Window ⌘N, Close Window ⇧⌘W, Move Tab to New Window… | missing | One Workspace per app today; Rune has multi-window factories but selection, history, panels and focus must be split out first (XL, not a written product rule) |
| [d1 W04](gaps/d1-workspace-chrome.md#2-feature-matrix) | Full screen toggle | missing | No fullscreen setter in vendor/rune |
| [d1 W15](gaps/d1-workspace-chrome.md#2-feature-matrix) | Trackpad gestures (pinch to zoom panes, swipe to toggle sidebar) | missing | No gesture API in Rune |
| [d1 W17](gaps/d1-workspace-chrome.md#2-feature-matrix) | Global show/hide hotkey | missing | Rune has portal GlobalShortcuts but it is unused; needs validation on Hyprland and GNOME |
| [d2 I11](gaps/d2-interface-themes.md#2-feature-matrix) | Use trackpad gestures | missing | No gesture API in Rune |
| [d2 I13](gaps/d2-interface-themes.md#2-feature-matrix) | App icon: Default (Dynamic) \| Bright \| Twilight | missing | set\_app\_icon\_png runs before startup only; Wayland reads the desktop entry |
| [d2 X03](gaps/d2-interface-themes.md#2-feature-matrix) | Settings window is a real window with close and zoom | partial | A separate top-level window needs Rune multi-window support (as d1 W03); the in-app 816-wide modal is client-only (M1.8) |
| [d3 T07](gaps/d3-terminal.md#22-typography-and-fonts) | Apply font family live (no relaunch) | missing | Font registry is built at startup |
| [d3 T10](gaps/d3-terminal.md#22-typography-and-fonts) | Font weight field, disabled "fixed by the font" unless variable | missing | Variable font axes are not exposed |
| [d3 T67](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) | Float Pane | missing | Floating layer rendering (panes domain); session.float\_block exists |
| [d3 T87](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) | Terminal content exposed to assistive technology | missing | Nothing exposes the cell grid to AT-SPI; README says terminal accessibility is not implemented |
| [d4 A15](gaps/d4-shortcuts-commands-menus.md#a-settings--shortcuts-page) | Global 'Toggle Visibility' hotkey | missing | As d1 W17 |
| [d4 B03](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) | Toggle Full Screen (Ctrl Cmd F) | missing | As d1 W04 |

### 5.3 Needs a server API that does not exist (3 rows)

| Row | Capability | Status | What is missing |
|---|---|---|---|
| [d5 d5-62](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Web Client Origins allow-list | not-applicable | No Web Client Origins method in help.json |
| [d6 d6.dev.origins](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | Web Client Origins allow-list: built-in 'https://rex.superlogical.com'… | missing | No Web Client Origins method in vendor/rexrs/codegen/help.json |
| [d6 d6.diag.server.logs](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | Live Logs (monospace, streaming, 'No server output yet.') + Clear Logs | missing | No server log method in help.json |

### 5.4 Unknown (could not be classified from the evidence) (4 rows)

| Row | Capability | Status | Open question |
|---|---|---|---|
| [d3 T35](gaps/d3-terminal.md#25-keyboard) | Jump to Previous / Next Prompt (Cmd+Up / Cmd+Down) | implemented | M3.7: Ctrl Shift Up/Down; OSC 133 verified through disposable Rex snapshots/live streams. [Evidence](prompt-navigation.md). |
| [d4 B12](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) | Pair Device with QR Code... | missing | No pairing method in the generated API; d5 decoded the beta QR as the plain remote address, so d5 d5-05 treats it as client-only |
| [d4 D24](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) | Jump to Previous / Next Prompt (Cmd Up / Down) | missing | As d3 T35 |
| [d6 d6.help.menu](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | Help menu: 'Rex Help' and 'Send Rex Beta Feedback to Apple' (plus the macOS… | missing | The beta Help target URL was not captured |

### 5.5 macOS-only (22 rows)

| Row | Capability | Status | Reason |
|---|---|---|---|
| [d1 W06](gaps/d1-workspace-chrome.md#2-feature-matrix) | App menu: About, Check for Updates, Settings, Services, Hide, Hide Others, Show… | partial | macOS app menu (Services, Hide, Show All); About, Settings and Quit map to Commands |
| [d1 W12](gaps/d1-workspace-chrome.md#2-feature-matrix) | Dock/menu-bar presence, Show window on launch, app icon variants | not-applicable | Dock and menu-bar presence; tray is optional (M5.6) |
| [d1 A03](gaps/d1-workspace-chrome.md#2-feature-matrix) | Dock badge | not-applicable | Dock badge; launcher-entry badge is an option in M3.4 |
| [d1 C05](gaps/d1-workspace-chrome.md#2-feature-matrix) | AutoFill, Writing Tools, Dictation, Emoji &amp; Symbols | not-applicable | AutoFill, Writing Tools, Dictation, Emoji: supplied by the compositor or IME on Linux |
| [d3 T17](gaps/d3-terminal.md#22-typography-and-fonts) | Font smoothing switch | not-applicable | macOS font smoothing; fontconfig and Rune decide antialiasing |
| [d3 T23](gaps/d3-terminal.md#23-colours) | Use Display P3 colour space | not-applicable | Display P3; no colour-managed wide-gamut path in Rune on Wayland |
| [d3 T32](gaps/d3-terminal.md#25-keyboard) | Option key: Automatic / Composes characters / Sends Alt / Left sends Alt /… | not-applicable | Option key modes; Linux Alt is Meta and compose uses AltGr and dead keys |
| [d3 T72](gaps/d3-terminal.md#210-context-menu-and-terminal-commands) | AutoFill submenu (Contact, Passwords, Credit Card) | not-applicable | AutoFill submenu |
| [d3 T85](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) | Quick Look of a path | not-applicable | Quick Look is a macOS service |
| [d3 T88](gaps/d3-terminal.md#212-rendering-fidelity-not-in-beta-captures) | Edit menu items that are macOS system services: Undo, Redo, Cut, Delete… | not-applicable | Edit-menu system items (Undo, Cut, Writing Tools, Dictation, Emoji) |
| [d4 B09](gaps/d4-shortcuts-commands-menus.md#b-window-and-app-actions-shortcuts-page-rows) | Grant System Permissions... | not-applicable | System permissions (TCC) have no Linux equivalent beyond a notification-service health line |
| [d4 D26](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) | Quick Look (Cmd Y) | not-applicable | Quick Look is a macOS service |
| [d4 F01](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) | Global menu bar (File, Edit, View, Window, Help) | not-applicable | Global menu bar; Commands plus context menus are the Linux equivalent |
| [d4 F08](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) | Edit: Writing Tools, AutoFill (Contact, Passwords, Credit Card), Start… | not-applicable | Writing Tools, AutoFill, Dictation |
| [d4 F16](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) | Interface Style: Classic / Glass; Tint with theme | not-applicable | Glass and Tint are macOS materials |
| [d4 F18](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) | Window menu: Minimize, Minimize All, Zoom, Fill, Center, Move &amp; Resize, Full… | not-applicable | Window menu (tiling, Fill, Center) is the compositor's job |
| [d5 d5-13](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Server shown in the menu-bar status item (header with green dot, version block… | not-applicable | macOS menu-bar status item; an optional tray is the Linux equivalent (M5.6) |
| [d5 d5-60](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Rex installation: location, ready status, Show Details (Version, Commit, Final… | missing | Bundled helper provenance; Linux equivalent is the local rex CLI path and version (M5.1) |
| [d5 d5-70](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | System Permissions card and Manage Permissions sheet (Full Disk Access, Screen… | not-applicable | macOS TCC permissions; server.apple.\* calls are useless on a Linux server |
| [d6 d6.dev.quicklook](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | 'Quick Look links' switch: Cmd-Y previews on-screen files and links… | partial | Quick Look is a macOS service; a list of on-screen links is the Linux reading (M5.7) |
| [d6 d6.entry.app-menu](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | App menu: About, Check for Updates…, Settings…, Services, Hide/Hide Others/Show… | not-applicable | macOS app menu |
| [d6 d6.diag.gpu.footnote](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | Explains WindowServer compositing and points to script/gpu-usage.py | not-applicable | WindowServer compositing is macOS-only |

### 5.6 Not applicable (platform idiom or no need; 11 rows)

| Row | Capability | Why not applicable |
|---|---|---|
| [d1 W01](gaps/d1-workspace-chrome.md#2-feature-matrix) | Window frame and controls | Borderless window with own Close; no min/max by design (DESIGN.md); compositor owns the frame |
| [d1 W05](gaps/d1-workspace-chrome.md#2-feature-matrix) | Global menu bar (Rex Beta, File, Edit, View, Window, Help) | No menu bar; Commands plus context menus replace it |
| [d1 W08](gaps/d1-workspace-chrome.md#2-feature-matrix) | Window management menu (Minimize, Zoom, Fill, Center, Move &amp; Resize, Full… | Window management belongs to the compositor |
| [d3 T30](gaps/d3-terminal.md#24-cursor) | IME preedit at the cursor | Linux IME preedit already draws at the cursor |
| [d3 T34](gaps/d3-terminal.md#25-keyboard) | Send Key presets (Cmd+Backspace as ctrl+u, Cmd+arrows as ctrl+a/e… | Linux Ctrl and Alt chords reach programs natively; pane.send\_key exists |
| [d4 C07](gaps/d4-shortcuts-commands-menus.md#c-session-host-tab-actions) | Refresh Sessions (Cmd R) | Models stay synced; no manual refresh needed (the beta command exists; consider a Commands row anyway, M1.6) |
| [d4 D22](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) | Send Key mappings (Cmd Delete, Cmd Left/Right, Opt Left/Right) | macOS line-edit idioms; Home, End and Ctrl+arrows already work |
| [d4 D34](gaps/d4-shortcuts-commands-menus.md#d-pane-keys-find-view-actions) | Focus Tab Filter | No tab filter in Linux: tabs live in the toolbar or sidebar |
| [d4 F04](gaps/d4-shortcuts-commands-menus.md#f-menu-bar-rex-beta-app-menus) | Edit: Undo, Redo, Cut, Delete | Undo, Redo, Cut, Delete are not meaningful in a terminal |
| [d5 d5-54](gaps/d5-hosts-notifications-integrations-onboarding.md#2-feature-matrix) | Scan a pairing QR to add a host | No camera flow; paste an https address in Add host. No rule forbids a QR scan |
| [d6 d6.upd.last-checked](gaps/d6-developer-diagnostics-updates.md#2-feature-matrix) | 'Last checked' time on the update status (Linux polish item, not a beta feature) | Not a beta feature; kept as a Linux polish item (Updates page could show Last checked) |

### 5.7 Decisions the owner needs to make

Product rules are decisions, not laws of nature. Each item below unlocks the rows named; the recommendation is the report's.

| # | Decision | Unlocks | Recommendation |
|---|---|---|---|
| D1 | List selection treatment: solid accent pill (beta) or tinted fill plus outline | M1.2 and every list | Tinted fill plus outline; no decision needed to start M1.2 with that default |
| D2 | Tab capsule, icon badge and visible close x (DESIGN.md "no close button") | d1 TB02, G17, delta 9 | Capsule and badge yes; x on active and hover only, or keep the rule |
| D3 | Reverse "no light chrome" for Auto, Light and Tint with theme | M6.1, d2 I01, I02, I04, d4 F12 | Decide after M3.6; needs a light palette, contrast checks and the portal colour-scheme signal |
| D4 | Shortcut recorder writes a GUI override layer (PRODUCT.md line 30 says Settings never writes the override file) | M4.4, d4 A06-A08, B17, E09 | Fourth layer in the app settings file; never touch `shortcuts.json`; Copy as JSON for hand edits |
| D5 | Stop Server, Tailscale sign-out, Serve, Limit access, Preferred Hostname | d5 d5-03, 06, 07, 08, 10; d4 B19; d6 server.stop | Do not build by default; if wanted, a confirmed Manage server disclosure for a local `unix://` host only |
| D6 | Terminal dimming during find, cursor blink, smooth scrolling, copy animation, contrast and palette forcing, ligatures | d1 F03, d3 T15, T21, T22, T27, T37, T41, d6 cursor shaders | Keep steady and unrecoloured; document the choice in README |
| D7 | Multiple windows | d1 W03, d4 C20 | Not a written rule; XL Rune and state-model work; plan separately after M1-M3 |
| D8 | `file:` links open locally | d3 T52 | Confirmation for remote-host terminals only, after a security review |
| D9 | Preview update channel | d5 d5-58, d6 d6.dev.channel | Ask before building: it widens the update trust surface |
| D10 | Automatic Autobalance of splits (the client rewrites a shared server layout whenever panes change) | d2 P07, part of d1 PN14 | Do not build; keep explicit Balance Splits (M4.2) so a user action, not the client, changes the layout |
| D10 | Toolbar position and visibility (Left, Right, Hover, Hidden) vs the single working row | M2.7, M4.3, d4 D35, F13 | Hover and Hidden yes with a keyboard reveal; Left/Right later and only as a rail shared with the sidebar |
| D11 | Diagnostics gated behind Developer Mode | M5.2 | Reachable from Commands always; Developer Mode hides only the experimental switches |
| D12 | Keep the name "RexSync" for the Hosts pane | M5.3 | Only if the server docs use it |
| D13 | Take control strip: implement or remove from DESIGN.md | d1 N04 | Either; README must stay honest |

README honesty (AGENTS.md): do not document the Server page, sounds, onboarding, Diagnostics or any M-item until it is implemented and checked. README line 233 about `https://user@host` login servers is not backed by client code (c04 section 7); do not extend it in onboarding copy.

## 6. Roadmap

Milestones are ordered by impact per effort. Effort: S under a day, M one to three days, L about a week, XL longer. Every item follows [AGENTS.md](../../AGENTS.md): explicit user actions only, never create or destroy a session on attach, failed input never replayed, server-owned layout and terminal content, a `Spec::remote()` decision for each new action (default `Never`, pinned test unchanged), headless tests in `src/app/tests.rs`, a rendered check with `-- --snap out.png` for UI, and no README claim before the feature is checked. Row IDs in "Closes" are `domain id` (for example d1 PN03); domain work-item numbers are in the matching file's section 5.

Dependency summary: M1 has no dependency except where noted inside it. M2 needs M1.8 (Settings scaffolding). M3 needs M1.6 (Commands) and M1.8. M4 needs M1.3 (pane header) and M3 where noted. M5 needs M1.7 and M1.8. M6 is gated by decisions and by Rune.

### M1. Everyday workspace (high impact, no blocker, effort S/M)

The first milestone. Everything is client-side, uses API calls the vendored SDK already generates, and fixes what a mouse user hits first.

**M1.1 Terminal context menu verbs (S-M).**
- Scope: add Copy (disabled without selection), Paste and Select All (`Terminal::select_all` already exists in `vendor/ghostty-vt`), Reset terminal (`terminal.reset`), Clear terminal (`terminal.clear`) to the pane menu, grouped by separators (clipboard, layout, lifecycle). Mirror Copy, Paste, Reset, Clear as Commands rows and actions (`pane.reset`, `pane.clear`, `pane.select_all`).
- Files: `src/app/panel.rs` (pane menu near `:716-734`), `src/app/actions.rs` (`Spec::remote()`: `Never`; update the allowed-set test only if a view-only action is added), `src/app/workspace.rs`, `src/connection.rs` (Command::Reset, Command::Clear), `src/terminal/selection.rs`, `src/app/tests.rs`.
- Depends on: M1.2 for separators (the verbs can ship first without them).
- Acceptance: right-click shows Copy disabled with no selection and enabled after dragging text; Paste inserts clipboard text; Reset clears screen and scrollback with the program still running (verified against a disposable server); the call goes to the clicked pane only and is never replayed after a failure; `--snap` shows the groups.
- Also add `Copy pane ID` (a client-only clipboard write of the block id) to the pane menu and Commands. Closes: d1 C01-C04, C09 (Copy pane ID and Paste/Copy as actions), d3 T47, T48, T49, T63-T66, T70, T71, T75, T86, d4 D19, D20, G02, F05, F06, F10.

**M1.2 Menu structure pass (M).**
- Scope: a separator row kind and a `danger` flag in `list_popup`; destructive rows (`Close pane...`, `Close tab...`, `Close session...`, `Remove host...`) after a separator in `DANGER`; always reserve the chevron column so hints align; one keycap formatter fed from `effective_keymap()` replacing hard-coded tooltips and the sidebar `^K`; a `SELECTION` token for selected rows (tinted fill plus 1 pt outline).
- Files: `src/app/popups.rs:82-439`, `src/app/panel.rs` (session menu `:367`, host menu `:420`, tab menu `:563`, pane menu `:716`), `src/app/theme.rs`, `src/app/widgets.rs`, `src/app/toolbar.rs:44,292-359`, `src/app/sidebar.rs:440`.
- Depends on: nothing. Design call on the selection treatment (section 4.6); the tinted-outline default needs none.
- Acceptance: every menu has separators between groups; destructive rows are `DANGER`; a test rebinds `session.switch` and checks the tooltip; selected rows are distinguishable from hover in a snapshot.
- Closes: d1 C07, C08, T12, T13, d4 (deltas 9-11), d3 (deltas 10, 11).

**M1.3 Pane header actions and visibility (M).**
- Scope: a trailing icon cluster in the pane header (Split right, Split down, Zoom, Close; Float when M4.1 lands), 24 pt targets, unavailable actions at 35% opacity, Close opens the existing named confirm; a `pane_header` setting (Always, On hover, Hidden) with a `Zoomed` chip; a single pane keeps full bleed unless Always.
- Files: `src/app/panes.rs` (`header`, `full_bleed`), `src/settings.rs`, `src/app/settings_view.rs`, `src/app/theme.rs`, `src/app/tests.rs`.
- Depends on: nothing.
- Acceptance: with two panes the header shows the icons; Close opens the named confirm; Always shows the header on a single pane; On hover reveals it when the pointer enters the pane's top 36 pt; snapshot attached.
- Closes: d1 PN02, PN03, PN04, PN09, d4 G16, F15.

**M1.4 Find bar upgrade (M).**
- Scope: previous, next and close buttons (disabled with no matches), `No matches` in `DANGER`, a counter that does not remove the close affordance, bindable Find next and Find previous actions that work with the field closed, bar placement that never covers the selected match (offset one row or use the pane header row in Comfortable), stronger match overlay tokens.
- Files: `src/app/panes.rs:552-595`, `src/app/input.rs:177-194`, `src/app/term.rs:761-776`, `src/app/theme.rs`, `src/app/actions.rs`, `src/keymap.rs`, `src/terminal/search.rs`.
- Depends on: nothing. Search never claims size ownership; no terminal dimming.
- Acceptance: with a match in the right prompt (screenshot 47 state) the match is visible and not under the bar; arrows step through matches and are disabled with `No matches`; Enter and Shift+Enter still work; Find next works with the field closed.
- Closes: d1 F01-F03, d3 T56, T58, T59, T60 (highlight only), d4 D30, G11.

**M1.5 Tab management (M).**
- Scope: Move tab left/right (`session.move_window`, `before_window_id`), Mute tab (client-local flag read by `activity.rs` notification and word gating), Close other tabs, `Use default title` in the rename dialog, tab menu separators with Close last, the same menu on sidebar tab rows, a `Close other tabs` command.
- Files: `src/app/panel.rs:563-575`, `src/app/workspace.rs` (`Intent::MoveWindow`, `CloseOtherWindows`), `src/app/sidebar.rs:221-286`, `src/app/activity.rs`, `src/settings.rs`, `src/keymap.rs`, `src/app/dialogs.rs:54-150`.
- Depends on: nothing.
- Acceptance: Move Left/Right changes order on the server and in another client and is disabled at the ends; a muted tab raises no desktop notification and shows no word; tests for ordering and mute.
- Closes: d1 TB08, TB09, TB12, TB14, TB15 (without New Window), d4 C19, C21, C22, G01, d5 d5-39.

**M1.6 Commands completeness and structure (M).**
- Scope: add Close session, Next/Previous session, Refresh sessions, Collapse other sessions, Close other tabs, Change theme, Increase/Decrease/Reset font size and interface zoom, Reset terminal, Find next/previous, Copy version info, Rex Help, Report an issue; group as Session, Tab, Pane, Host, View, App while keeping one flat list when filtering; about 20 rows on first open; hints follow the effective keymap; ids pinned by a test.
- Files: `src/app/panel.rs:514-859`, `src/app/popups.rs`, `src/app/widgets.rs`, `src/app/actions.rs`.
- Depends on: M1.1, M1.5 for the rows they add; M1.2 for keycaps.
- Acceptance: every item of the beta File, Edit, View and Help menus (excluding macOS system items) has a Commands row; group headers visible; the registry-to-Commands list cannot silently shrink.
- Closes: d1 P01, P05, P07, S04-S07, d4 E04, E12, F03, F09, F11, F14, F17, G13.

**M1.7 Server identity, About and version info (S-M).**
- Scope: after each control connection is ready, issue read-only `server.version` and `server.status` once per host (never attach, never create); store `HostModel.server_info`; show `Rex Server <version>` and the supported API revision range in About and in Copy version info; add Copy debug info (no addresses, no secrets) and an About command; centred About header with icon, one `format_version()` everywhere.
- Files: `src/connection.rs`, new `src/connection/server_info.rs`, `src/app/about.rs`, `src/app/updates.rs`, `src/app/panel.rs`, `src/app/workspace.rs`.
- Depends on: nothing (SDK types already generated).
- Acceptance: a mock server returning version and status shows the version in About and the copy text; an older API that lacks a field shows `Not reported`; a refused request is quiet; copied debug text contains no endpoint strings (test).
- Closes: d6 d6.about.server-version, d6.about.copy-info, d6.about.copy-debug, d6.about.icon, d5 d5-56, d5 d5-09, d4 B10, B11, F21.

**M1.8 Settings scaffolding (M).**
- Scope: `section`, `card`, fixed-slot `setting`, `segmented_fixed`, `select_row`/`value_popup` anchored to the control, `disclosure`, `mono` style; Settings panel about 816 wide with `scroll_edge_fade`; one-sentence helper copy; Updates and About rebuilt on the helpers with no behaviour change; dividers placed only by the card.
- Files: `src/app/settings_view.rs`, `src/app/theme.rs`, `src/app/widgets.rs`, `src/app/popups.rs`, `src/app/panel.rs`, `src/app/updates.rs`, `src/app/about.rs`.
- Depends on: nothing. Do this before the other settings items so later pages inherit it.
- Acceptance: `-- --snap` of every section at 100% and 130% zoom shows no clipped control, controls right-aligned to one x, a divider between every pair of rows (grep: no hand-placed dividers); a test opens a picker from Terminal and checks the popup rect abuts the control; existing tests pass.
- Closes: d2 S01, S02, S11, S12, S14, S15, S16, S20, d3 T01 (structure), d5 d5-15, d5 d5-18 (partly), d6 d6.dev.card-style, d6.dev.popup-style.

**M1.9 Small fixes (S each).**
- Window title `<tab title> - <session> · <host>` (d1 W02, `workspace.rs:742-756`); a gear for Settings (`toolbar.rs:343`); sidebar count badge shown only above one tab (`sidebar.rs:118-219`); session chip sized to content and one line when a single host is connected (`toolbar.rs:19-74`); discovery list truncation and group icons (d5 d5-49, `discover.rs:187-276`); Integrations footnote and state chips (d5 d5-19, d5-20, d5-23, `integrations.rs:486-670`); Split left/up/best fit and Focus next/previous pane (`session.new_split` with `side:"before"`, `session.focus_next_pane`; d1 PN07-PN09, PN12); font-size actions that leave `Ctrl =/-/0` as interface zoom (d3 T33); `Rex help` command (d1 W09); fix or delete the DESIGN.md "Take control" paragraph (d1 N04).
- Depends on: nothing. Acceptance: each has a test or a snapshot; the discovery test uses a 40-character name with long metadata and shows the name intact, metadata ellipsised and the chevron present.

### M2. Settings that control things (M/L, no blocker; needs M1.8)

**M2.1 Shortcuts page lists every action with filter and key search (M).** Scope: iterate `actions::SPECS`, show unbound actions as `Not set`, label-left keycap-right cards, pinned `Filter shortcuts...` accepting `ctrl+k` style queries via `Chord::parse`, merge alternates into one row, render a literal plus as `Plus`, raise chip contrast, an `Also bound to <action>` caption from the shadowing already computed in `keymap.rs:618-681`. Files: `settings_view.rs:592-728`, `actions.rs`, `keymap.rs`, `theme.rs`, DESIGN.md (replace the 190 pt rule). Depends on: M1.8. Acceptance: one row per registered action; filtering `ctrl+shift+d` lists Split pane right; an unbound action reads `Not set`; no half-clipped rows in a snapshot. Closes: d4 A01-A05, A09, A14, A18.

**M2.2 Fill the action registry from existing API (L).** Scope: `session.next/previous`, `client.tab.close_others`, `client.tab.move(direction)`, `pane.split` left/up/best-fit, `pane.focus_next/previous`, `pane.swap/move(direction)`, `pane.close_others`, `pane.clear`, `pane.reset`, `pane.select_all`, `client.search.next/previous`, `session.select` by number (the beta shows Cmd 1, Cmd 2 on switcher rows, B089; d1 S02), with defaults (tab goto on Ctrl+Alt+1..9 rather than Alt+digit, Ctrl+Alt+[ and ], Ctrl+Shift+Alt+arrows for move, F3 and Shift+F3 while search is open). Files: `actions.rs`, `keymap.rs:370-414`, `workspace.rs`, `connection.rs`. Depends on: M2.1 for visibility; M1.1 and M1.5 already add some. Acceptance: each new action has a headless test, a `remote()` entry, appears in Settings and Commands, and round-trips through `shortcuts.json`. Every new action is `Remote::Never`, including navigation ones such as `pane.focus_next` (the exact allowed set of eight view-only actions stays pinned unless the owner decides to widen it). Closes: d1 S02, d4 C04, C14, C18, C19, C21, D03, D04, D06, D07, D11, D13, D14, D21, D27, D30, F20.

**M2.3 Settings navigation: search, groups, history (M).** Scope: a `Search Settings` field over a row registry (label, helper, section) that jumps to and highlights the row; grouped sidebar with 20 pt authored icons in tinted tiles in the beta's order; a Back/Forward pair and a page stack (`settings_stack`), sub-pages for Notifications statuses and Tabs/Panes, mouse Back/Forward and Alt+Left/Right; a Themes section. Files: `settings_view.rs`, `panel.rs:181-205`, `workspace.rs` (`open_section`). Depends on: M1.8. Acceptance: typing `sound` lists Notifications and Enter opens it; typing `padding` lists Pane padding; Notifications > Blocked > Back returns with scroll position kept. Closes: d2 S07, S08, S09, d5 d5-16, d5-17, d6 (delta 15).

**M2.4 Anchored pickers and the theme switcher as a command (M).** Scope: font and theme pickers open over Settings anchored under their row, not instead of it; diacritic-insensitive and highlighted theme search; switcher width 340 and max height 420 with a right-hand check; `Change theme...` in the action registry and Commands with an Appearance group; rename the two Theme rows to `Window colors` and `Terminal theme`. Files: `panel.rs:469`, `popups.rs`, `actions.rs`, `settings_view.rs`. Depends on: M1.8. Acceptance: `Ctrl Shift P`, type `rose` lists Rosé Pine; choosing a theme keeps Settings open; the action can be bound in `shortcuts.json`. Closes: d2 S03 (partly), SW01-SW04, d3 (deltas 5, 6).

**M2.5 Terminal page restructure and selection settings (M).** Scope: sections with headings, cards, footnotes; `preview_well` for typography, cursor and selection (steady cursor); move the `Saved in .../workspace.json` footer to About; Copy on selection (default off, never claims size ownership), selection foreground and background modes, rounded corners; optional primary-selection and middle-click paste when the program is not tracking the mouse; cursor style (Block, Bar, Underline) and Force style, sequence-hint switch; blink stays off. Files: `settings_view.rs:274-373`, `widgets.rs`, `settings.rs`, `term.rs` (selection `:768-811`, cursor `:1115-1144`), `panes.rs:306-309`, `terminal/selection.rs`. Depends on: M1.8. Acceptance: copy-on-select pastes into another app; Cell Foreground inverts the selection; with Bar and Force on, `printf '\e[2 q'` still draws a bar; old config files load. Closes: d3 T01, T04, T12, T25, T26, T28, T31, T39-T44 (animation excluded), T50.

**M2.6 Font picker, style, fallback list, metrics (M-L).** Scope: fontconfig enumeration cache, a font picker panel (search, Monospaced | All, check), the family shown in its own face, Font style popup, ordered fallback list with Add, an Advanced font page with line height, character spacing and Render bold/italic (cell metrics and sprites scale together; verify PTY resize reporting). Files: `fonts.rs`, `panel.rs`, `popups.rs`, `settings_view.rs`, `settings.rs`, `term.rs:156-166`, `sprites.rs`. Depends on: M2.5. Acceptance: Hack Nerd Font Mono saves and the helper says it applies on reopen; the Monospaced segment lists only mono families; +20% line height changes the rows and columns reported to the server and sprites still touch cell edges; `--font-fallback` still overrides. Closes: d3 T05, T06, T08, T11, T13, T14, T16, T18.

**M2.7 Interface switches that need only client work (L).** Scope: Animation style and `reduce_motion`, toolbar behaviour (Always, On hover, Hidden) with a Reveal Toolbar action, separate horizontal and vertical padding, active-pane edge highlight, Focus follows mouse (focus path only, never `session.own_block`), header-only fade (not content fade), Show window on launch. Files: `toolbar.rs`, `panes.rs`, `settings.rs`, `settings_view.rs`, `theme.rs:204-216`, `keymap.rs`, `mod.rs`. Depends on: M1.3, M1.8. Acceptance: a test per persisted key; hover reveals within 150 ms; hidden toolbar frees its 52 pt and remains draggable and closable by keyboard; hover focus moves keyboard focus but does not send `session.own_block`. Autobalance (d2 P07) is deliberately left out: rewriting a shared server layout on every change is gated by decision D10; explicit Balance Splits ships in M4.2. Closes: d1 T10 (without Left/Right), T11, W16, PN18-PN20 (content fade excluded), d2 I06, I09, I12, P01, P04, P08, d3 T84.

**M2.8 Tab appearance options (M).** Scope: Tabs fill available space, icon colour mode (Always, Active pane, Active tab, Never), tab icon count and shell icon theme (a 0-5 stepper if the slider is skipped), a live preview box. Files: `toolbar.rs`, `activity.rs`, `settings_view.rs`, `settings.rs`. Depends on: M2.3 (sub-page). Acceptance: each setting visibly changes the track in a snapshot; defaults leave 0.6.0 pixels unchanged. Closes: d1 TB01, TB03, TB04, d2 I07, I10, T01-T05.

**M2.9 Theme thumbnails and a library page (M).** Scope: `theme_swatch` (112 x 88, `Aa`, `~/` chip, cursor pill, eight dots), a Themes section with Default, Built-in, Custom and Ghostty groups as a searchable wrapped grid (not a 600-item strip), selection ring, a four-dot strip in switcher rows. Files: new `themes_view.rs`, `terminal_theme.rs`, `settings_view.rs`. Depends on: M2.4. Acceptance: each card shows Aa, chip, pill and eight dots; scrolling 600 themes stays smooth. Closes: d2 TH01 (partly), TH05, TH06, d5 d5-68.

**M2.10 Generated titles (M).** Scope: `generated_titles` setting; tab and pane title from program plus directory (`terminal.process`, `pwd`) when the shell title is the default; names the user gave and program-set titles still win. Files: `connection/activity.rs:232`, `toolbar.rs`, `panes.rs`, `settings.rs`. Acceptance: a tab running `vim` in `~/rex` reads `vim - ~/rex`; a renamed tab keeps its name. Closes: d3 T80.

### M3. New surfaces on existing server API (L, no blocker)

**M3.1 Go to Directory and Insert Path (L).** Scope: a palette-style path browser over `terminal.list_dir` (dirs, optional files, `prefix`, `limit`), first row the typed directory, children sorted case-insensitively by the client; Go to Directory sends `cd -- <quoted>` plus newline only when the foreground process is a shell; Insert Path pastes the quoted path with no Enter; per-host recents; Commands rows; defaults `Ctrl Shift G` and `Ctrl Alt I` unless they conflict with Find next. Files: new `src/app/directory.rs`, `panel.rs`, `popups.rs`, `connection.rs`, `actions.rs` (remote `Never`), `settings.rs`, `settings_view.rs`. Depends on: M1.6. Acceptance: against a disposable server, typing `/tmp/` lists children; Enter changes the shell's directory; Insert Path puts `'/tmp/a b'` at the prompt without executing; a failed list shows an inline error and never retries; nothing replays; quoting is tested. Closes: d1 G01, G02, d3 T76-T79, d4 D15, D18, G12.

**M3.2 Activity popover (L).** Scope: an eye button with an On state; a popover grouped Working, Needs you, Error, Done with row = program icon, title, `<session> · <host>`, elapsed time since state change (`StatusRecord.updated_at` is parsed away today, no server work), Filter field, click navigates to the tab; empty state `Nothing needs you`; worst-state word on sidebar session rows and the chip; prefetch views for all sessions (measure with `REX_FRAME_STATS`). Files: `toolbar.rs`, `popups.rs`, `activity.rs`, `workspace.rs:888-904`, `sidebar.rs`. Depends on: M1.5 (mute), status tokens. Acceptance: a running agent in a background tab appears under Working with elapsed time; selecting it navigates; honours Show activity in tabs; no status dots. Closes: d1 T06, A01, A02, d4 D17, D33, G10, d5 d5-38.

**M3.3 Read-only Server page and Pair QR (M + M).** Scope: `SettingsSection::Server` for the active host (status, version popover, Remote Address with Copy, Tailscale state read-only, listeners); a Pair sheet drawing a QR of `https://<dns_name>` from `El` rects (small encoder crate or about 200 vendored lines; `--locked` stays working). Files: new `server_page.rs`, `pair.rs`, `panel.rs:187,209`, `Cargo.toml`/`Cargo.lock`, `dialogs.rs`. Depends on: M1.7. Acceptance: with a local server the page shows version, uptime from `started_at`, address and a working copy; an older server shows `Not reported`; `zbarimg` on a `--snap` capture decodes the exact URL. Closes: d5 d5-01, d5-02, d5-04, d5-05, d5-09, d5-14, d6 d6.srv.uptime.

**M3.4 Notifications: statuses, sounds, health (L + M).** Scope: grouped cards (Activity in tabs, Desktop notifications, Messages from programs, Play sounds, volume), four status rows with colour icons and channel summaries pushing a status sub-page (Enable status, Activity in tabs, Desktop notification, Play sound, Sound popup, preview); freedesktop sound-theme names via the notification `sound-name` hint or `paplay`/`pw-play` (no bundled third-party audio); six-position notice picker (default stays Top right); notification health line and `Send test notification`; urgency (critical for Blocked and Error), replace id per tab, close when seen, notifier thread survives a failed `Notify`. Files: `settings.rs:24-68`, `settings_view.rs:212-272`, `notify.rs:10-116`, `activity.rs:246-342`, `widgets.rs` (slider), new `subpage.rs`. Depends on: M2.3, M1.8. Acceptance: toggling a status off silences exactly that status in a headless outbox test; two Blocked events in one tab produce one notification; settings round-trip with old files; with no notification service the page says so. Closes: d5 d5-26, d5-29..d5-36, d5-41, d5-42.

**M3.5 Sheet anatomy and host forms (M).** Scope: `sheet()`; rebuild Add host, Add over SSH, Connect to Rex, Edit host and Client label with header, grouped fields, the transport-trust footnote (shown for http/https/ws, not unix or ssh), diagnostics in three tiers with mono paths, focus ring and placeholder contrast; make Client label live with `client.set_label`; host icon choice. Files: `dialogs.rs`, `discover.rs:333-454`, `workspace.rs:1265-1327`, `connection.rs` (`SetLabel`), `settings.rs:11-22`, `panel.rs:455-461`, `sidebar.rs`. Depends on: M1.8. Acceptance: each form shows header, dividers and footer; editing the label updates `client.list` without restart (live probe on a disposable server); old files load unchanged. Closes: d5 d5-11, d5-45..d5-48, d5-46.

**M3.6 Built-in themes and light/dark pairing (L).** Scope: Merino Light/Dark and a curated set from a licence-clear source, persisted `light_theme` and `dark_theme` chosen by effective appearance, declared once per connection through `terminal.set_theme`. Files: `terminal_theme.rs`, `settings.rs`, `workspace.rs`. Depends on: M2.9. Acceptance: round-trip through `workspace.json`; switching appearance declares the matching theme once. Closes: d2 TH02, TH03, I01 (data only).

**M3.7 Jump to previous/next prompt — implemented 2026-11-11.** The disposable-server spike verified OSC 133 in snapshots, live output and reattachment. `client.prompt.previous/next` now navigate the local viewport with Ctrl Shift Up/Down and Commands; the original roadmap's `pane.prompt_previous/next` spellings are aliases. All are `Remote::Never`; the allowed set is unchanged. Ghostty's existing prompt-scroll operation handles boundaries/continuations; no new protocol or mark index. [Protocol evidence and checks](prompt-navigation.md). Original scope: first check whether OSC 133 prompt marks reach the client terminal model (`vendor/ghostty-vt/src/screen.rs:314,380` expose `semantic_prompt`; no decoded-frame use was found), against a disposable server. If they do, add `pane.prompt_previous` and `pane.prompt_next` that move the viewport (view-only, no PTY input, `Remote::Never`) and default to Ctrl Shift Up and Down unless those conflict. If they do not, record the finding and stop; do not add a server request. Files: `src/terminal/`, `src/app/term.rs`, `actions.rs`, `keymap.rs`. Depends on: nothing. Acceptance: with a shell emitting OSC 133, the actions step the viewport between prompts and do nothing otherwise; a headless test feeds marks through the terminal model. Closes: d3 T35, d4 D24 (medium impact, `unknown` blocker until the spike answers).

### M4. Layout vocabulary (L, no blocker; M4.1 and M4.2 need M1.3)

**M4.1 Floating panes (L).** Scope: Float pane, New floating pane, Minimize and restore (`session.float_block`, `new_layer`, `set_layer_state`, `raise/lower/resize/close_layer`); layer chrome and drag/resize in `panes.rs`; `layout.rs` already places layers; header Float button. Depends on: M1.3. Acceptance: Float turns the focused pane into a layer; a floated pane appears for another client; minimize hides it; tests cover placement and minimized layers. Closes: d1 PN10, d3 T67, d4 D07, D08.

**M4.2 Pane move, swap, balance, keyboard resize (L).** Scope: drag handle on the pane header with `El::on_drag`, `session.swap_blocks`, `session.move_block`, Balance splits (0.5 ratios through `session.resize_pane`, once per boundary), 5% arrow-key resize, Move pane to new tab. Depends on: M1.3. Acceptance: dragging a header onto another pane swaps or moves via the API; balance sets 0.5 ratios; no layout invented locally. Closes: d1 PN05, PN13, PN14, d4 D07, D08.

**M4.3 Toolbar layout Left/Right (L).** Scope: a vertical rail reusing the sidebar, unified with Tab position. Depends on: M2.7. Acceptance: three layouts render without overlapping window controls. Closes: d1 T10, d2 I05. Design call: one toolbar row (DESIGN.md) vs the beta's side layouts.

**M4.4 Shortcut recorder and GUI override layer (L, decision).** Scope: store recorder output in the app settings file as a fourth keymap layer above `shortcuts.json`, never touching the user file; recorder control with focus ring, Cancel and Remove; conflict warning (`Also bound to ...`) before saving; chord and sequence capture that ignores IME-consumed keys, Escape cancels; Configuration menu (status, Copy path, Copy overrides as JSON, Reset all with confirm); `Ctrl Enter` in Commands opens the highlighted action's editor. Files: `settings_view.rs`, `settings.rs`, `keymap.rs` (`Overrides`), `input.rs`, `panel.rs`. Depends on: M2.1; PRODUCT.md line 30 must change. Acceptance: record `Ctrl+Shift+J` on Split right, it works at once and survives restart, `shortcuts.json` is byte-identical, Reset all restores defaults. Closes: d4 A06-A08, A12, B17, E09, and E08 by the `Ctrl Enter` alternative.

**M4.5 Client-local onboarding and migration (XL + L).** Scope: a full-window overlay that never creates or attaches a session or block, shown once when `onboarded` is unset (default true for existing files) and from Commands `Welcome to Rex...`; Welcome, Appearance (tab position, zoom, chrome and terminal theme cards; no Color scheme or Glass), Integrations (Rex Agent Skill status plus a notification-service check in place of System Permissions), Migrate (agent tiles, `Copy Prompt` with a Linux-specific prompt, an explicit split only on click), Finish (three keycap rows from the live keymap). Files: new `onboarding.rs`, `migrate_prompt.md`, `settings.rs`, `panel.rs`, `keymap.rs`. Depends on: M1.8, M2.9, M3.5, M3.4 for sounds copy. Acceptance: a fresh `--config` temp file shows page 1; Skip writes `onboarded`; the flow never calls `session.create`; `tests.rs` covers focus, Esc and Previous/Continue. Closes: d5 d5-63..d5-73.

**M4.6 Links policy (M, decision).** Scope: allow `file:` links with confirmation for remote-host terminals, `links_open_apps` Ask first | Always | Never for other schemes, a confirm dialog naming the URL; needs a security review of `links.rs:5-9`. Depends on: M2.5. Closes: d3 T52, T53.

### M5. Developer and diagnostics (M-XL, no blocker except noted)

**M5.1 Developer section and Rex CLI (M).** Settings > Developer with a Developer Mode switch that gates only experimental extras; Rex CLI card (detected path, `rex version --json`, SHA-256 of the file, Copy per field, optional path override); Local API card with Copy. Files: `settings_view.rs`, `panel.rs`, new `developer.rs`, `integrations.rs:281-323`, `settings.rs`. Depends on: M1.8. Acceptance: tests with a fake `rex` shell script on a scratch PATH; no server contacted with `--autostart=false`. Closes: d6 d6.dev.section, d6.dev.mode, d6.dev.local-api, d6.dev.rex-*.

**M5.2 Diagnostics shell (M).** `PanelKind::Diagnostics` as a large dialog with a six-pane sidebar, opaque header, inspector toggle (inspector collapses to an overlay under 900 pt), `app.diagnostics` bound to `Ctrl Shift ,` (check conflicts) and a Commands row. Depends on: M1.8. Acceptance: opens from Commands and the shortcut, Esc closes, focus returns to the pane; remote control reports `not_supported`. Closes: d6 d6.diag.window, d6.diag.sidebar, d6.diag.inspector, d6.entry.diag-shortcut.

**M5.3 Hosts pane (the RexSync equivalent) (M).** Per saved host: label, kind, connection state and last error, resumes, epoch, client id, token presence only, instance and host id (needs M1.7), Tailscale state, per-session rows, Terminal policy card from local settings; copy button on every value; wrapping per section 4.5. Depends on: M1.7, M5.2. Acceptance: two mock hosts (one disconnected) are listed; no secrets.

**M5.4 Rex Server pane (M).** Status, session count, started, pid, listeners with admission text (`tailnet.rs:215-262`), Tailscale state, DNS and HTTPS, build block, `server.whoami`. No Stop Server and no logs. Depends on: M1.7, M5.2. Acceptance: snapshot shows every field or `Not reported`.

**M5.5 Rex API console (L).** Method field, JSON payload (blank is distinct from `{}`), Send, call transcript, event ring buffer (500) fed from the control connection (record unknown events), All Sessions scope, Clear, Copy event, Attached clients. Hazard confirm naming the host for `session.create`, `session.destroy` and `block.close`; `server.stop` is refused by the console unless decision D5 is taken (AGENTS.md: the client never stops a server as a side effect, and D5 says not to build Stop Server by default); never replays. Depends on: M5.2, M5.3. Acceptance: test sends `server.version` and a blank-payload call through a mock and shows an event; the allowed-set remote test is untouched.

**M5.6 Client log, update actions, optional tray (S + S + M).** An in-memory logger for the `log` facade shown in Diagnostics; `app.check_updates` and `app.dismiss_update_notice` as registry actions (`Remote::Never`, which the server sees as not available remotely); an opt-in StatusNotifierItem through `rune::tray` (Rex Linux version, server version, Copy version info, Check for updates, Settings, Quit; no running dot). Closes: d4 G14, d6 d6.upd.actions, d6.entry.status-item.

**M5.7 Rendering pane and developer experimentals (L + M).** Frame p50/p95/max from `FrameStats`, adapter and backend, optional `/proc/<pid>/fdinfo` GPU share with an honest empty state (adapter info may need Rune); Terminal anchoring Top/Bottom (only the sub-cell slack placement moves; selection and mouse hit-testing use the same offset); a Commands list of on-screen links for Quick Look links. Depends on: M5.2.

**M5.8 Network recorder (XL).** Opt-in metadata-only recorder in `transport.rs`, `connection.rs`, `wire.rs` with a ring buffer by window and a chart spike; never stores bodies; zero cost when off. Depends on: M5.2. Acceptance: mock-server test records one connect, one request and one failure with the right spans.

### M6. Decision-gated and Rune-dependent

Do not start these without the named decision. Several change DESIGN.md or PRODUCT.md. Exception: M6.2 (theme editor) has no blocker; it sits here only because of its size and its dependency on M2.9 and M3.6, and it can start as soon as those land.

- **M6.1 Light chrome, Auto and Tint with theme (L + L).** Needs the "no light chrome" rule reversed. Add a light `Palette`, a `color_scheme` setting, read `Cx::system_dark()` (Rune already follows the portal `color-scheme` signal), a contrast-clamped palette builder deriving ground, card, ink and accent from the active terminal theme. Depends on: M3.6. Acceptance: toggling the desktop colour scheme flips chrome without restart; contrast unit tests pass. Closes: d1 W13, d2 I01, I02, I04, d4 F12, d5 d5-65.
- **M6.2 Theme editor, extended palette, import (XL + M).** Editor sheet with 22 colour fields (hex field plus swatch), Preview through `terminal.set_theme`, save as a Ghostty file under `$XDG_CONFIG_HOME/ghostty/themes`, Revert, Cancel, Save and Apply; LAB generation of indices 16-255 with Reverse and a cube and ramp view; import of Ghostty and `.itermcolors` files. Depends on: M2.9. Closes: d2 TH07-TH09, E01-E11.
- **M6.3 Rune work.** Multiple windows (XL; needs window-scoped selection, history, panels and focus split out of the single `Workspace` first; not a documented product rule), full screen (S; no setter in `vendor/rune`), global show/hide hotkey through the portal GlobalShortcuts (M), trackpad gestures (M), app icon variants (M), live font apply (L), accessibility tree for the terminal grid (L). Each needs a vendor/rune patch with a README entry. Closes: d1 W03, W04, W15, W17, d4 A15, B03, d2 I11, I13, X03 (a separate Settings window; the in-app equivalent is M1.8), d3 T07, T87. Note: Rune already ships `Tray` and portal GlobalShortcuts, so the tray (M5.6) is not Rune work; the global hotkey is listed here because it is unvalidated on Hyprland and GNOME rather than missing from Rune.
- **M6.4 Product decisions.** Glass (L, window-level blur); release channel Preview (L, widens the update trust surface); Stop Server and Tailscale writes behind a confirmed Manage server disclosure for a local `unix://` host (L); Web Client Origins and live server logs (need new server API); terminal dimming, cursor blink, smooth scrolling, copy animation, ligatures, contrast and palette forcing (recommendation: do not build; document the steady behaviour in README).

## 7. Evidence index

Every beta screenshot (001-137) and every Linux screenshot (00-49) with the domain file that covers it. "Domain file" is the file whose subject the screenshot belongs to; "Also cited in" lists other gap files that cite it. Coverage was computed by scanning the six gap files for each screenshot number (prefixed `B`/`L`, in ranges, or as bare zero-padded numbers in `d3`, which does not use prefixes). Beta screenshots live in [`../../rex-beta-ui-2026-11-10/screenshots/`](../../rex-beta-ui-2026-11-10/screenshots/) (1x, points equal pixels) and Linux screenshots in [`../../rex-rune-2026-11-11/screenshots/`](../../rex-rune-2026-11-11/screenshots/) (2560x1410 at Interface zoom 130%). Both directories are untracked in git at the time of writing.

**Defect found and fixed by the completeness review:** Linux screenshot 39 (`rename-pane`) was covered by no domain file; d1 TB12 now cites it. The capability is scored (d1 PN16 cites L38) but the Rename pane dialog itself (title, `Pane name` field, Cancel and Save, no `Use default title`) is analysed only in the supporting spec [rune/r03](rune/r03-menus-overlays.md). Add it to d1's tab and pane rename analysis (d1 delta 21 covers only the tab dialog). Beta screenshots have no uncovered number. Beta screenshots 087 and 088 (Window > Move & Resize, Full Screen Tile) are covered as macOS window-management idioms (d4 F18, d1 W08).

#### Rex Beta screenshots 001-137

| # | Screenshot | Domain file | Also cited in | Note |
|---:|---|---|---|---|
| 001 | [main-terminal](../../rex-beta-ui-2026-11-10/screenshots/001-main-terminal.png) | [d1](gaps/d1-workspace-chrome.md) | - |  |
| 002 | [menu-rex-beta](../../rex-beta-ui-2026-11-10/screenshots/002-menu-rex-beta.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 003 | [menu-rex-services](../../rex-beta-ui-2026-11-10/screenshots/003-menu-rex-services.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 004 | [about-rex-beta](../../rex-beta-ui-2026-11-10/screenshots/004-about-rex-beta.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 005 | [settings-overview](../../rex-beta-ui-2026-11-10/screenshots/005-settings-overview.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d2](gaps/d2-interface-themes.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 006 | [settings-server-details](../../rex-beta-ui-2026-11-10/screenshots/006-settings-server-details.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d2](gaps/d2-interface-themes.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 007 | [settings-interface](../../rex-beta-ui-2026-11-10/screenshots/007-settings-interface.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 008 | [settings-customize-tabs](../../rex-beta-ui-2026-11-10/screenshots/008-settings-customize-tabs.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 009 | [menu-tab-icon-color](../../rex-beta-ui-2026-11-10/screenshots/009-menu-tab-icon-color.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 010 | [menu-shell-icon-theme](../../rex-beta-ui-2026-11-10/screenshots/010-menu-shell-icon-theme.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 011 | [settings-customize-panes](../../rex-beta-ui-2026-11-10/screenshots/011-settings-customize-panes.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md), [d3](gaps/d3-terminal.md) |  |
| 012 | [menu-pane-fill](../../rex-beta-ui-2026-11-10/screenshots/012-menu-pane-fill.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 013 | [menu-pane-fade](../../rex-beta-ui-2026-11-10/screenshots/013-menu-pane-fade.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 014 | [menu-toolbar-behavior](../../rex-beta-ui-2026-11-10/screenshots/014-menu-toolbar-behavior.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 015 | [menu-pane-header-behavior](../../rex-beta-ui-2026-11-10/screenshots/015-menu-pane-header-behavior.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 016 | [settings-interface-lower](../../rex-beta-ui-2026-11-10/screenshots/016-settings-interface-lower.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 017 | [menu-show-app-icon](../../rex-beta-ui-2026-11-10/screenshots/017-menu-show-app-icon.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 018 | [settings-themes](../../rex-beta-ui-2026-11-10/screenshots/018-settings-themes.png) | [d2](gaps/d2-interface-themes.md) | - |  |
| 019 | [new-light-theme-editor](../../rex-beta-ui-2026-11-10/screenshots/019-new-light-theme-editor.png) | [d2](gaps/d2-interface-themes.md) | - |  |
| 020 | [theme-extended-palette](../../rex-beta-ui-2026-11-10/screenshots/020-theme-extended-palette.png) | [d2](gaps/d2-interface-themes.md) | - | extended palette cube and ramp clipped in the capture |
| 021 | [new-dark-theme-editor](../../rex-beta-ui-2026-11-10/screenshots/021-new-dark-theme-editor.png) | [d2](gaps/d2-interface-themes.md) | - |  |
| 022 | [settings-terminal](../../rex-beta-ui-2026-11-10/screenshots/022-settings-terminal.png) | [d3](gaps/d3-terminal.md) | [d2](gaps/d2-interface-themes.md) |  |
| 023 | [terminal-font-picker](../../rex-beta-ui-2026-11-10/screenshots/023-terminal-font-picker.png) | [d3](gaps/d3-terminal.md) | - |  |
| 024 | [terminal-font-picker-all](../../rex-beta-ui-2026-11-10/screenshots/024-terminal-font-picker-all.png) | [d3](gaps/d3-terminal.md) | - |  |
| 025 | [settings-advanced-font](../../rex-beta-ui-2026-11-10/screenshots/025-settings-advanced-font.png) | [d3](gaps/d3-terminal.md) | - |  |
| 026 | [menu-character-spacing-unit](../../rex-beta-ui-2026-11-10/screenshots/026-menu-character-spacing-unit.png) | [d3](gaps/d3-terminal.md) | - |  |
| 027 | [menu-font-rendering](../../rex-beta-ui-2026-11-10/screenshots/027-menu-font-rendering.png) | [d3](gaps/d3-terminal.md) | - |  |
| 028 | [fallback-font-picker](../../rex-beta-ui-2026-11-10/screenshots/028-fallback-font-picker.png) | [d3](gaps/d3-terminal.md) | - |  |
| 029 | [menu-font-style](../../rex-beta-ui-2026-11-10/screenshots/029-menu-font-style.png) | [d3](gaps/d3-terminal.md) | - |  |
| 030 | [menu-cursor-style](../../rex-beta-ui-2026-11-10/screenshots/030-menu-cursor-style.png) | [d3](gaps/d3-terminal.md) | - |  |
| 031 | [menu-cursor-blink](../../rex-beta-ui-2026-11-10/screenshots/031-menu-cursor-blink.png) | [d3](gaps/d3-terminal.md) | - |  |
| 032 | [menu-option-key](../../rex-beta-ui-2026-11-10/screenshots/032-menu-option-key.png) | [d3](gaps/d3-terminal.md) | - |  |
| 033 | [settings-terminal-middle](../../rex-beta-ui-2026-11-10/screenshots/033-settings-terminal-middle.png) | [d3](gaps/d3-terminal.md) | [d2](gaps/d2-interface-themes.md) |  |
| 034 | [menu-selection-foreground](../../rex-beta-ui-2026-11-10/screenshots/034-menu-selection-foreground.png) | [d3](gaps/d3-terminal.md) | - |  |
| 035 | [menu-selection-background](../../rex-beta-ui-2026-11-10/screenshots/035-menu-selection-background.png) | [d3](gaps/d3-terminal.md) | - |  |
| 036 | [menu-link-handling](../../rex-beta-ui-2026-11-10/screenshots/036-menu-link-handling.png) | [d3](gaps/d3-terminal.md) | - |  |
| 037 | [settings-terminal-lower](../../rex-beta-ui-2026-11-10/screenshots/037-settings-terminal-lower.png) | [d3](gaps/d3-terminal.md) | - |  |
| 038 | [settings-shortcuts](../../rex-beta-ui-2026-11-10/screenshots/038-settings-shortcuts.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 039 | [menu-shortcuts-configuration](../../rex-beta-ui-2026-11-10/screenshots/039-menu-shortcuts-configuration.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md) |  |
| 040 | [shortcut-recorder](../../rex-beta-ui-2026-11-10/screenshots/040-shortcut-recorder.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md) |  |
| 041 | [shortcuts-page-02](../../rex-beta-ui-2026-11-10/screenshots/041-shortcuts-page-02.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 042 | [shortcuts-page-03](../../rex-beta-ui-2026-11-10/screenshots/042-shortcuts-page-03.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 043 | [shortcuts-page-04](../../rex-beta-ui-2026-11-10/screenshots/043-shortcuts-page-04.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md) |  |
| 044 | [shortcuts-page-05](../../rex-beta-ui-2026-11-10/screenshots/044-shortcuts-page-05.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md) |  |
| 045 | [shortcuts-page-06](../../rex-beta-ui-2026-11-10/screenshots/045-shortcuts-page-06.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md) |  |
| 046 | [shortcuts-page-07](../../rex-beta-ui-2026-11-10/screenshots/046-shortcuts-page-07.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md) |  |
| 047 | [shortcuts-page-08](../../rex-beta-ui-2026-11-10/screenshots/047-shortcuts-page-08.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md) |  |
| 048 | [shortcuts-page-09](../../rex-beta-ui-2026-11-10/screenshots/048-shortcuts-page-09.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md) |  |
| 049 | [shortcuts-page-10](../../rex-beta-ui-2026-11-10/screenshots/049-shortcuts-page-10.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md) |  |
| 050 | [shortcuts-page-11](../../rex-beta-ui-2026-11-10/screenshots/050-shortcuts-page-11.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md) |  |
| 051 | [shortcuts-page-12](../../rex-beta-ui-2026-11-10/screenshots/051-shortcuts-page-12.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md) |  |
| 052 | [settings-integrations](../../rex-beta-ui-2026-11-10/screenshots/052-settings-integrations.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - |  |
| 053 | [settings-notifications](../../rex-beta-ui-2026-11-10/screenshots/053-settings-notifications.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 054 | [settings-remote-hosts](../../rex-beta-ui-2026-11-10/screenshots/054-settings-remote-hosts.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - |  |
| 055 | [settings-developer](../../rex-beta-ui-2026-11-10/screenshots/055-settings-developer.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 056 | [menu-update-channel](../../rex-beta-ui-2026-11-10/screenshots/056-menu-update-channel.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 057 | [menu-rex-installation](../../rex-beta-ui-2026-11-10/screenshots/057-menu-rex-installation.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 058 | [menu-terminal-anchoring](../../rex-beta-ui-2026-11-10/screenshots/058-menu-terminal-anchoring.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 059 | [settings-developer-lower](../../rex-beta-ui-2026-11-10/screenshots/059-settings-developer-lower.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 060 | [developer-installation-details](../../rex-beta-ui-2026-11-10/screenshots/060-developer-installation-details.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 061 | [add-remote-host](../../rex-beta-ui-2026-11-10/screenshots/061-add-remote-host.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 062 | [menu-remote-host-icon](../../rex-beta-ui-2026-11-10/screenshots/062-menu-remote-host-icon.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - |  |
| 063 | [notifications-working](../../rex-beta-ui-2026-11-10/screenshots/063-notifications-working.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 064 | [notifications-blocked](../../rex-beta-ui-2026-11-10/screenshots/064-notifications-blocked.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 065 | [notifications-error](../../rex-beta-ui-2026-11-10/screenshots/065-notifications-error.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 066 | [notifications-done](../../rex-beta-ui-2026-11-10/screenshots/066-notifications-done.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 067 | [menu-done-sound](../../rex-beta-ui-2026-11-10/screenshots/067-menu-done-sound.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - |  |
| 068 | [menu-error-sound](../../rex-beta-ui-2026-11-10/screenshots/068-menu-error-sound.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - |  |
| 069 | [menu-blocked-sound](../../rex-beta-ui-2026-11-10/screenshots/069-menu-blocked-sound.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - |  |
| 070 | [settings-notifications-lower](../../rex-beta-ui-2026-11-10/screenshots/070-settings-notifications-lower.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - |  |
| 071 | [pair-device](../../rex-beta-ui-2026-11-10/screenshots/071-pair-device.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | QR decodes to the plain remote address |
| 072 | [menu-file](../../rex-beta-ui-2026-11-10/screenshots/072-menu-file.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 073 | [menu-edit](../../rex-beta-ui-2026-11-10/screenshots/073-menu-edit.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 074 | [menu-view](../../rex-beta-ui-2026-11-10/screenshots/074-menu-view.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 075 | [menu-window](../../rex-beta-ui-2026-11-10/screenshots/075-menu-window.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 076 | [menu-help](../../rex-beta-ui-2026-11-10/screenshots/076-menu-help.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 077 | [menu-edit-find](../../rex-beta-ui-2026-11-10/screenshots/077-menu-edit-find.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 078 | [menu-edit-writing-tools](../../rex-beta-ui-2026-11-10/screenshots/078-menu-edit-writing-tools.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 079 | [menu-edit-autofill](../../rex-beta-ui-2026-11-10/screenshots/079-menu-edit-autofill.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 080 | [menu-view-pane](../../rex-beta-ui-2026-11-10/screenshots/080-menu-view-pane.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 081 | [menu-view-customize-view](../../rex-beta-ui-2026-11-10/screenshots/081-menu-view-customize-view.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 082 | [menu-view-customize-view-color-scheme](../../rex-beta-ui-2026-11-10/screenshots/082-menu-view-customize-view-color-scheme.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 083 | [menu-view-customize-view-toolbar-layout](../../rex-beta-ui-2026-11-10/screenshots/083-menu-view-customize-view-toolbar-layout.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 084 | [menu-view-customize-view-pane-style](../../rex-beta-ui-2026-11-10/screenshots/084-menu-view-customize-view-pane-style.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 085 | [menu-view-customize-view-pane-style-pane-headers](../../rex-beta-ui-2026-11-10/screenshots/085-menu-view-customize-view-pane-style-pane-headers.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 086 | [menu-view-customize-view-interface-style](../../rex-beta-ui-2026-11-10/screenshots/086-menu-view-customize-view-interface-style.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 087 | [menu-window-move-and-resize](../../rex-beta-ui-2026-11-10/screenshots/087-menu-window-move-and-resize.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 088 | [menu-window-full-screen-tile](../../rex-beta-ui-2026-11-10/screenshots/088-menu-window-full-screen-tile.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 089 | [session-switcher](../../rex-beta-ui-2026-11-10/screenshots/089-session-switcher.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 090 | [command-palette](../../rex-beta-ui-2026-11-10/screenshots/090-command-palette.png) | [d1](gaps/d1-workspace-chrome.md) | [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 091 | [activity-popover](../../rex-beta-ui-2026-11-10/screenshots/091-activity-popover.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | Activity popover; tree gives toolbar and pane-header bounds |
| 092 | [tab-context-menu](../../rex-beta-ui-2026-11-10/screenshots/092-tab-context-menu.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 093 | [rename-tab](../../rex-beta-ui-2026-11-10/screenshots/093-rename-tab.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 094 | [terminal-context-menu](../../rex-beta-ui-2026-11-10/screenshots/094-terminal-context-menu.png) | [d1](gaps/d1-workspace-chrome.md) | [d3](gaps/d3-terminal.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 095 | [terminal-context-autofill](../../rex-beta-ui-2026-11-10/screenshots/095-terminal-context-autofill.png) | [d1](gaps/d1-workspace-chrome.md) | [d3](gaps/d3-terminal.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 096 | [diagnostics-overview](../../rex-beta-ui-2026-11-10/screenshots/096-diagnostics-overview.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 097 | [diagnostics-rex-binary](../../rex-beta-ui-2026-11-10/screenshots/097-diagnostics-rex-binary.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 098 | [diagnostics-rex-api](../../rex-beta-ui-2026-11-10/screenshots/098-diagnostics-rex-api.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 099 | [diagnostics-rexsync](../../rex-beta-ui-2026-11-10/screenshots/099-diagnostics-rexsync.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 100 | [diagnostics-network](../../rex-beta-ui-2026-11-10/screenshots/100-diagnostics-network.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 101 | [diagnostics-gpu](../../rex-beta-ui-2026-11-10/screenshots/101-diagnostics-gpu.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d3](gaps/d3-terminal.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 102 | [diagnostics-inspector](../../rex-beta-ui-2026-11-10/screenshots/102-diagnostics-inspector.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 103 | [inspector-rex-binary](../../rex-beta-ui-2026-11-10/screenshots/103-inspector-rex-binary.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 104 | [inspector-rex-server](../../rex-beta-ui-2026-11-10/screenshots/104-inspector-rex-server.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 105 | [inspector-rex-api](../../rex-beta-ui-2026-11-10/screenshots/105-inspector-rex-api.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 106 | [inspector-rexsync](../../rex-beta-ui-2026-11-10/screenshots/106-inspector-rexsync.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 107 | [inspector-network](../../rex-beta-ui-2026-11-10/screenshots/107-inspector-network.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 108 | [network-events](../../rex-beta-ui-2026-11-10/screenshots/108-network-events.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 109 | [network-connections](../../rex-beta-ui-2026-11-10/screenshots/109-network-connections.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 110 | [network-methods](../../rex-beta-ui-2026-11-10/screenshots/110-network-methods.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 111 | [network-app-events](../../rex-beta-ui-2026-11-10/screenshots/111-network-app-events.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 112 | [network-apcs](../../rex-beta-ui-2026-11-10/screenshots/112-network-apcs.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d3](gaps/d3-terminal.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 113 | [menu-diagnostics-host](../../rex-beta-ui-2026-11-10/screenshots/113-menu-diagnostics-host.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | Diagnostics Host menu |
| 114 | [diagnostics-api-events](../../rex-beta-ui-2026-11-10/screenshots/114-diagnostics-api-events.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 115 | [rexsync-page-02](../../rex-beta-ui-2026-11-10/screenshots/115-rexsync-page-02.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 116 | [rexsync-page-03](../../rex-beta-ui-2026-11-10/screenshots/116-rexsync-page-03.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 117 | [rexsync-page-04](../../rex-beta-ui-2026-11-10/screenshots/117-rexsync-page-04.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 118 | [rexsync-page-05](../../rex-beta-ui-2026-11-10/screenshots/118-rexsync-page-05.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 119 | [rexsync-page-06](../../rex-beta-ui-2026-11-10/screenshots/119-rexsync-page-06.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 120 | [rexsync-page-07](../../rex-beta-ui-2026-11-10/screenshots/120-rexsync-page-07.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) |  |
| 121 | [rexsync-page-08](../../rex-beta-ui-2026-11-10/screenshots/121-rexsync-page-08.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md) |  |
| 122 | [rexsync-page-09](../../rex-beta-ui-2026-11-10/screenshots/122-rexsync-page-09.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d2](gaps/d2-interface-themes.md) |  |
| 123 | [rexsync-page-10](../../rex-beta-ui-2026-11-10/screenshots/123-rexsync-page-10.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md) |  |
| 124 | [rexsync-page-11](../../rex-beta-ui-2026-11-10/screenshots/124-rexsync-page-11.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d2](gaps/d2-interface-themes.md) |  |
| 125 | [terminal-find](../../rex-beta-ui-2026-11-10/screenshots/125-terminal-find.png) | [d3](gaps/d3-terminal.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 126 | [terminal-find-results](../../rex-beta-ui-2026-11-10/screenshots/126-terminal-find-results.png) | [d3](gaps/d3-terminal.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 127 | [go-to-directory](../../rex-beta-ui-2026-11-10/screenshots/127-go-to-directory.png) | [d3](gaps/d3-terminal.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 128 | [insert-path](../../rex-beta-ui-2026-11-10/screenshots/128-insert-path.png) | [d3](gaps/d3-terminal.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 129 | [theme-switcher](../../rex-beta-ui-2026-11-10/screenshots/129-theme-switcher.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 130 | [status-bar-menu](../../rex-beta-ui-2026-11-10/screenshots/130-status-bar-menu.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 131 | [status-bar-server-submenu](../../rex-beta-ui-2026-11-10/screenshots/131-status-bar-server-submenu.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d6](gaps/d6-developer-diagnostics-updates.md) | submenu did not open in the capture (COVERAGE.md) |
| 132 | [onboarding-session](../../rex-beta-ui-2026-11-10/screenshots/132-onboarding-session.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |
| 133 | [onboarding-appearance](../../rex-beta-ui-2026-11-10/screenshots/133-onboarding-appearance.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d2](gaps/d2-interface-themes.md), [d3](gaps/d3-terminal.md), [d4](gaps/d4-shortcuts-commands-menus.md) |  |
| 134 | [onboarding-permissions](../../rex-beta-ui-2026-11-10/screenshots/134-onboarding-permissions.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 135 | [system-permissions](../../rex-beta-ui-2026-11-10/screenshots/135-system-permissions.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | sheet missing from the beta accessibility tree |
| 136 | [onboarding-migration](../../rex-beta-ui-2026-11-10/screenshots/136-onboarding-migration.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d3](gaps/d3-terminal.md), [d4](gaps/d4-shortcuts-commands-menus.md), [d6](gaps/d6-developer-diagnostics-updates.md) |  |
| 137 | [onboarding-finish](../../rex-beta-ui-2026-11-10/screenshots/137-onboarding-finish.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d1](gaps/d1-workspace-chrome.md) |  |

All 137 beta screenshots are covered by at least one domain file.

#### Rex Linux screenshots 00-49

| # | Screenshot | Domain file | Also cited in | Note |
|---:|---|---|---|---|
| 00 | [workspace-terminal](../../rex-rune-2026-11-11/screenshots/00-workspace-terminal.png) | [d1](gaps/d1-workspace-chrome.md) | [d2](gaps/d2-interface-themes.md) | |
| 01 | [settings-interface](../../rex-rune-2026-11-11/screenshots/01-settings-interface.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 02 | [settings-terminal](../../rex-rune-2026-11-11/screenshots/02-settings-terminal.png) | [d3](gaps/d3-terminal.md) | [d1](gaps/d1-workspace-chrome.md), [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 03 | [settings-notifications](../../rex-rune-2026-11-11/screenshots/03-settings-notifications.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 04 | [settings-integrations](../../rex-rune-2026-11-11/screenshots/04-settings-integrations.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 05 | [settings-remote-hosts](../../rex-rune-2026-11-11/screenshots/05-settings-remote-hosts.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d6](gaps/d6-developer-diagnostics-updates.md) | |
| 06 | [settings-shortcuts](../../rex-rune-2026-11-11/screenshots/06-settings-shortcuts.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 07 | [settings-updates](../../rex-rune-2026-11-11/screenshots/07-settings-updates.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 08 | [settings-about](../../rex-rune-2026-11-11/screenshots/08-settings-about.png) | [d6](gaps/d6-developer-diagnostics-updates.md) | [d1](gaps/d1-workspace-chrome.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 09 | [interface-theme-picker](../../rex-rune-2026-11-11/screenshots/09-interface-theme-picker.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md), [d3](gaps/d3-terminal.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 10 | [font-family-and-fallbacks](../../rex-rune-2026-11-11/screenshots/10-font-family-and-fallbacks.png) | [d3](gaps/d3-terminal.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 11 | [terminal-theme-picker](../../rex-rune-2026-11-11/screenshots/11-terminal-theme-picker.png) | [d2](gaps/d2-interface-themes.md) | [d1](gaps/d1-workspace-chrome.md), [d3](gaps/d3-terminal.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 12 | [default-shell](../../rex-rune-2026-11-11/screenshots/12-default-shell.png) | [d3](gaps/d3-terminal.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 13 | [theme-search](../../rex-rune-2026-11-11/screenshots/13-theme-search.png) | [d2](gaps/d2-interface-themes.md) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 14 | [settings-terminal-lower](../../rex-rune-2026-11-11/screenshots/14-settings-terminal-lower.png) | [d3](gaps/d3-terminal.md) | [d2](gaps/d2-interface-themes.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 15 | [settings-integrations-lower](../../rex-rune-2026-11-11/screenshots/15-settings-integrations-lower.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 16 | [agent-integration-preview](../../rex-rune-2026-11-11/screenshots/16-agent-integration-preview.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 17 | [all-agents-setup-preview](../../rex-rune-2026-11-11/screenshots/17-all-agents-setup-preview.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 18 | [client-label](../../rex-rune-2026-11-11/screenshots/18-client-label.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 19 | [edit-host](../../rex-rune-2026-11-11/screenshots/19-edit-host.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 20 | [remove-host-confirmation](../../rex-rune-2026-11-11/screenshots/20-remove-host-confirmation.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 21 | [add-host-tailnet-discovery](../../rex-rune-2026-11-11/screenshots/21-add-host-tailnet-discovery.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 22 | [add-host-manual-endpoint](../../rex-rune-2026-11-11/screenshots/22-add-host-manual-endpoint.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 23 | [add-host-ssh](../../rex-rune-2026-11-11/screenshots/23-add-host-ssh.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | - | |
| 24 | [discovered-server-listeners](../../rex-rune-2026-11-11/screenshots/24-discovered-server-listeners.png) | [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | [d6](gaps/d6-developer-diagnostics-updates.md) | |
| 25 | [shortcuts-page-2](../../rex-rune-2026-11-11/screenshots/25-shortcuts-page-2.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | - | |
| 26 | [shortcuts-page-3](../../rex-rune-2026-11-11/screenshots/26-shortcuts-page-3.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | - | |
| 27 | [shortcuts-page-4](../../rex-rune-2026-11-11/screenshots/27-shortcuts-page-4.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | - | |
| 28 | [shortcuts-page-5](../../rex-rune-2026-11-11/screenshots/28-shortcuts-page-5.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | - | |
| 29 | [shortcuts-page-6](../../rex-rune-2026-11-11/screenshots/29-shortcuts-page-6.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | - | |
| 30 | [workspace-session-sidebar](../../rex-rune-2026-11-11/screenshots/30-workspace-session-sidebar.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 31 | [host-context-menu](../../rex-rune-2026-11-11/screenshots/31-host-context-menu.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 32 | [session-context-menu](../../rex-rune-2026-11-11/screenshots/32-session-context-menu.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 33 | [rename-session](../../rex-rune-2026-11-11/screenshots/33-rename-session.png) | [d1](gaps/d1-workspace-chrome.md) | - | |
| 34 | [close-session-confirmation](../../rex-rune-2026-11-11/screenshots/34-close-session-confirmation.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 35 | [tab-context-menu](../../rex-rune-2026-11-11/screenshots/35-tab-context-menu.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 36 | [rename-tab](../../rex-rune-2026-11-11/screenshots/36-rename-tab.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 37 | [close-tab-confirmation](../../rex-rune-2026-11-11/screenshots/37-close-tab-confirmation.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 38 | [pane-context-menu](../../rex-rune-2026-11-11/screenshots/38-pane-context-menu.png) | [d1](gaps/d1-workspace-chrome.md) | [d3](gaps/d3-terminal.md), [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 39 | [rename-pane](../../rex-rune-2026-11-11/screenshots/39-rename-pane.png) | [d1](gaps/d1-workspace-chrome.md) | - | Fixed by the completeness review: d1 TB12 now analyses the Rename pane dialog (also in [rune/r03](rune/r03-menus-overlays.md), section 39) |
| 40 | [close-pane-confirmation](../../rex-rune-2026-11-11/screenshots/40-close-pane-confirmation.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 41 | [command-palette](../../rex-rune-2026-11-11/screenshots/41-command-palette.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | [d1](gaps/d1-workspace-chrome.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 42 | [command-search](../../rex-rune-2026-11-11/screenshots/42-command-search.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | - | |
| 43 | [session-switcher](../../rex-rune-2026-11-11/screenshots/43-session-switcher.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md), [d5](gaps/d5-hosts-notifications-integrations-onboarding.md) | |
| 44 | [session-switcher-search](../../rex-rune-2026-11-11/screenshots/44-session-switcher-search.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 45 | [new-session-host-chooser](../../rex-rune-2026-11-11/screenshots/45-new-session-host-chooser.png) | [d4](gaps/d4-shortcuts-commands-menus.md) | - | |
| 46 | [terminal-search](../../rex-rune-2026-11-11/screenshots/46-terminal-search.png) | [d3](gaps/d3-terminal.md) | [d1](gaps/d1-workspace-chrome.md), [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 47 | [terminal-search-results](../../rex-rune-2026-11-11/screenshots/47-terminal-search-results.png) | [d3](gaps/d3-terminal.md) | [d1](gaps/d1-workspace-chrome.md), [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 48 | [terminal-search-no-matches](../../rex-rune-2026-11-11/screenshots/48-terminal-search-no-matches.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |
| 49 | [session-inline-rename](../../rex-rune-2026-11-11/screenshots/49-session-inline-rename.png) | [d1](gaps/d1-workspace-chrome.md) | [d4](gaps/d4-shortcuts-commands-menus.md) | |

49 of 50 Linux screenshots are covered by a gap file. Uncovered: L39.

### Supporting evidence not indexed above

The beta folder also holds accessibility trees under `evidence/` (200 files, including `-display.png` full-display captures for 002, 003, 062, 067, 068, 069), `captures.jsonl`, `capture.py` and `COVERAGE.md` (known gaps: Check for Updates produced no panel; the status-item server submenu did not open; the Left and Right toolbar layouts, light appearance, Glass, multi-tab and multi-pane layouts and floating panes were never captured). The Linux folder holds `manifest.json`, `navigation/` and `thumbnails/`. Beta behaviour that no screenshot shows (hover reveal, history stack, Go to Anything, tab activity states) rests on the first-draft wording `not captured` in the rows that depend on it.


## Completeness review

A critic pass on 2026-11-11 (PST) looked for what is missing rather than what is present. Source files were not touched.

### What was checked

- **Screenshot coverage.** Every number in `rex-beta-ui-2026-11-10/screenshots` (001-137) and `rex-rune-2026-11-11/screenshots` (00-49) has a domain file in section 7. Linux 39 was the one exception and is now fixed. 45 beta screenshots were opened, mostly from those with the fewest citations (065, 111, 122, 020, 026, 027, 029-032, 034, 036, 069, 070, 079, 113, 116, 056-062, 054, 082, 087, 130, 091, 134, 136, 110, 101, 102, 133, 039, 131, 137, 106, 012, 014, 084, 123, 135, 066), plus Linux 38, 39, 41, 42 and 45. Every control, menu option and label seen in them (cursor, font style, Option key, link handling, sound lists, notification position and statuses, update channel, Rex installation, Terminal anchoring, host icons, Terminal Policy card in RexSync, GPU and Network panes, permission sheet, status-item menu, Shortcuts Configuration menu, Customize View submenus, Window menu) was found in at least one matrix. A further automated pass compared every label in the beta accessibility trees for the settings, menu and overlay captures against the gap files; the unmatched remainder was accessibility identifiers, colour values, per-theme names and per-row copy buttons, not features.
- **Uncaptured states.** Linux split and zoom layouts, update-available, empty workspace and failure states are not in the Linux captures (rune README). They were checked in code (`panes.rs`, `updates.rs`, `workspace.rs`). None is wrongly scored as a missing beta feature; two Linux-side descriptions were wrong or incomplete and are fixed (below).
- **Roadmap.** Every high-impact, unblocked, missing or partial row is covered by a milestone. The roadmap was read against AGENTS.md rule by rule.
- **Cross-domain consistency.** About 90 rows that two or more domains score for the same capability were compared on status, blocker, effort and impact.

### Changes made

Domain files (`gaps/`):

- d1 PN04 `missing` to `partial` and d1 PN21 `partial` to `parity`, to agree with d2 I09 and I08. d2 I09 said Comfortable always shows the 30 pt header; the code draws a single pane full-bleed with no header (`full_bleed`, `panes.rs:84`), so the text is corrected.
- d5 d5-65 `not-applicable` to `missing` (d1 W13, d2 I01 and d4 F12 score the same capability as a product-rule gap) and d5 d5-59 `missing` to `not-applicable` (d6.dev.auto-download and auto-install do). Net raw d5 counts unchanged.
- Blockers: d2 I02 and d2 P07 `none` to `product-rule`; d6.dev.channel `none` to `product-rule` (matches d5-58 and D9); d4 G14 and d1 W10 `needs-rune-work` to `none` (Rune ships `Tray`, `vendor/rune/crates/rune/src/tray.rs`); d2 X03 `none` to `needs-rune-work` (a separate Settings window is the same Rune multi-window blocker as d1 W03).
- Effort and impact: d5-39 (mute a tab) aligned with d1 TB08 and d4 C21 (M, low); d4 D24 effort aligned with d3 T35 (M).
- Rows added or extended: d1 N08 (pane reconnect strip, `Reconnecting · input paused` and Retry, linux-only); d1 N06 now lists every empty state (`Your workspace starts here`, `Choose a session`, `Host unavailable` with Retry, `Start personal server...`); d1 TB12 now analyses the Rename pane dialog (L39); d5-63 and the onboarding paragraph acknowledge the empty-workspace card; d1 PN01, PN02 and d6.upd.toast say plainly that the Linux side is from code, not a capture.

Master report:

- Scorecard, the 585-row reconciliation (now 586), sections 3.1-3.3 lists and counts, section 5.1 (now 53 rows), 5.2 (now 13 rows), the section 5 intro and the verdict bullets (346 of 413 gap rows unblocked, 67 blocked: 45 product rule, 13 Rune, 4 unknown, 3 macOS-only, 2 server API) were recomputed for the changes above. The de-duplicated 431 figures could not be recomputed (the de-duplication key was never recorded) and are unchanged; the direction of change is: one more Linux-only capability (N08), PN21 moves from partial to parity.
- Roadmap: M2.7 no longer includes Autobalance (new decision D10); M5.5 no longer lets the console send `server.stop` (contradicted D5 and AGENTS.md); M6 intro says M6.2 (theme editor) has no blocker and is only sequenced late for size; M2.2 gains session-by-number selection (d1 S02) and a note that every new action is `Remote::Never`, including navigation ones, so the pinned eight-action allowed set does not change; M1.1 gains Copy pane ID and cites d1 C09, d3 T86, d4 D19, D20; new M3.7 (prompt jump spike, d3 T35 and d4 D24); M5.6 wording `not_supported` replaced by `Remote::Never`; M6.3 notes the tray is not Rune work.
- Section 7: Linux 39 row and defect paragraph. `README.md` in this folder: row count, Linux uncaptured-state caveat, defect note.

### What remains uncertain

- **Totals.** The verdict now quotes raw matrix rows (586); no de-duplication across domains was done.
- **Impact scores differ across domains for the same capability** (for example Change Theme is `low` in d1 G03 and `high` in d2 SW01; mute tab was `medium` in d5 and `low` elsewhere). Only status and blocker conflicts were reconciled; impact reflects each domain's framing.
- **Linux layouts never captured.** Every size and behaviour for split, zoomed, floating, update-available, failure and first-run states on the Linux side comes from code reading. A rendered `-- --snap` pass of those states would close the gap; the beta side has no counterpart captures either (COVERAGE.md).
- **Beta behaviour that no screenshot shows** (Left and Right toolbar layouts, light and Glass, multi-tab and multi-pane layouts, tab activity states, the Activity popover with idle processes, Go to Anything) remains `not captured`; those rows may still change when the beta is exercised further.
- **Jump to prompt (M3.7) — resolved 2026-11-11.** OSC 133 row/cell semantics survive real Rex API 13 snapshots, live streams and reattachment; viewport-only navigation is implemented. [Evidence](prompt-navigation.md).
- **Global hotkey and tray on Linux desktops.** Rune has portal GlobalShortcuts and `Tray`; behaviour under GNOME and Hyprland is unvalidated.
- **Quick Look.** d3 T85 and d4 D26 treat it as macOS-only, while d6.dev.quicklook (partial) and M5.7 propose a list of on-screen links as a Linux counterpart. Not reconciled; the counterpart is a different feature, so both readings are defensible.
- **Rows left at their original line numbers.** File and line citations in the domain files were not re-verified beyond the ones edited here.
