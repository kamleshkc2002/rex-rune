# Rex Linux 0.6.0: workspace terminal, Settings, pickers and forms (screenshots 00–17)

Source: `rex-rune-2026-11-11/screenshots/00…17`, captured 2026-11-11 PST on Omarchy/Hyprland. Build: 0.6.0, commit `ad196ff`, build date 2026-11-10 PST, server API revision 13.

## Measurement conventions

- Originals are 2560 × 1410 px at 2× device scale. Sizes below are **original pixels (px)** with the **logical size (pt ≈ px/2)** in brackets where useful.
- Interface zoom was **130%** in every capture (Settings › Interface), so logical sizes are already scaled by 1.3. Dividing by 2.6 gives an approximate 100%-zoom size (inferred).
- Colours were sampled from pixels with ImageMagick. Anti-aliased text was sampled at its brightest pixel, so text colours are approximate.
- There was no accessibility tree. All text was read from the pixels.

## Shared tokens seen across these screens

| Token | Hex | Where sampled |
| --- | --- | --- |
| Terminal / window background (Midnight) | `#001528` | 00, terminal area and toolbar, which share one colour |
| Modal scrim over the workspace | `#000B14` | 01, outside the Settings panel. The workspace is dimmed to about 50% |
| Panel / modal / popup surface | `#0C1523` (popup `#0C1524`) | Settings sidebar and content, form modals, theme pickers |
| Settings sidebar ↔ content divider | `#141D2A` (1 px) | 01, x ≈ 1033 |
| Row divider | approximately `#141D2A`, 1 px | between Settings rows |
| Selected nav row / selected popup row | `#222A37` / `#222A38` | 01 sidebar, 09 "Midnight" row |
| Secondary button fill ("Change…", "Install…", "Edit…", "Cancel", "Check again") | `#19222F` | 04, 01 |
| Stepper −/+ circle and close (×) circle fill | `#061F32` (stepper ring), close `#061F32`/`#112435` | 01 |
| Segmented control, selected segment | `#001528` fill with a light 1 px outline | 01 |
| Segmented control, unselected segment | `#161E2E` | 01 |
| Toggle on | track `#81C6FE` (light blue), knob `#F2F5F9` on the right | 03 |
| Toggle off | track `#253C51`, white knob on the left | 03 |
| Primary button ("Set up all detected…", "Save", "Set up") | horizontal gradient from about `#81BCFE` (left) to `#90A9FC` (right), dark label of about `#061220` | 04, 10, 16 |
| Destructive button ("Remove…") | fill `#24212E`, 1 px reddish outline, label `#FB8888` | 05 |
| Key chip (Shortcuts) | fill `#152D42`/`#1C3448`, label `#8D9EB2` | 06 |
| Session pill (toolbar) | `#0E2234` | 00 |
| Tab pill (toolbar, active) | `#24303A` | 00 |
| Search field in the picker | `#161F2D` | 11 |
| Primary text (headings, labels) | about `#E2E8F0`–`#E8EEF6` | 01 |
| Secondary / helper text, unselected nav, section caps | about `#92A2B7` / `#93A3B8` / `#91A0B5` | 01, 11 |
| Right-aligned meta text in a picker ("Declares nothing", "Midnight") | about `#B2B8C2` | 11 |
| Toolbar icon glyphs | about `#91A1B6` | 00 |

Typography: the chrome uses one proportional sans (looks like Inter or a similar grotesk, inferred) at about three sizes: panel title (largest, semibold), row label (medium weight, white) and helper (regular, smaller, blue-grey). Section headers such as `DEFAULT SHELL`, `AGENTS`, `NAVIGATE` and `SERVER` are small, bold, all caps, letter-spaced and blue-grey. Monospace appears only in terminal content. Paths and URLs in Settings (`unix:///…`, `~/.agents/skills/rex`) use the proportional font.

Corner radii: Settings panel and modals about 20–24 px (10–12 pt). Buttons are fully pill-shaped. Nav selection and popup selection about 10–12 px. Toolbar pills about 12 px. Key chips about 4–6 px. Panels have a soft, wide drop shadow and a faint lighter 1 px rim (`#182129` at the panel edge).

---

## 00 — Workspace terminal (`00-workspace-terminal.png`)

**What it is:** The main window with one session (`still-atoll` on host `shuvbot-rex`) and one tab. The session sidebar is hidden. This is the default state after launch and attaching.

**Controls, left to right (toolbar, about 66 px [33 pt] tall, the same colour as the terminal `#001528`, with no separator line):**
1. **Sidebar toggle**: icon-only button with a rectangle glyph split by a vertical line on the left (a "sidebar" glyph). Colour `#91A1B6`.
2. **Session pill**: a rounded rectangle about 270 × 41 px, fill `#0E2234`, containing:
   - a server-stack glyph (two stacked rounded rectangles),
   - title **`still-atoll`** (semibold, white) over the subtitle **`shuvbot-rex`** (small, `#93A3B8`),
   - a small chevron-down `⌄` at the right edge. It opens a session/host menu (inferred from the chevron).
3. **Tab pill (active tab)**: about 205 × 41 px, fill `#24303A`, with a slightly lighter 1 px outline. It holds a prompt glyph `>_` and the label **`shuv@shuvbot:~`** (the terminal title, semibold). It has no close button or activity badge.
4. Right cluster: **⌘** (command-key glyph, Commands palette), **+** (new tab/session), **sliders/tune glyph** (Settings), a thin vertical separator, and **×** (close window). All are icon-only with no visible labels or tooltips.

**Terminal content:** A server-owned zsh prompt (a powerline-style theme): Apple logo, `>` chevron, a blue house icon with `~`, and a white block cursor. The right prompt shows a green `✔`, `‹ shuv@shuvbot` in bold orange/amber, `‹ 00:21:37`, and a clock glyph. The terminal font is a monospace (JetBrainsMono Nerd Font per screenshot 02). The terminal fills the whole window under the toolbar with no visible pane header, border or gap. With one pane in "Comfortable" density no chrome is visible.

**Layout:** The toolbar items start about 16 px from the left. The gap between the session pill and the tab pill is about 14 px. The right icons are spaced about 46 px apart. Terminal content starts about 22 px from the left (8 pt pane padding × zoom, inferred).

**Behaviour implied:** The session pill switches session or host. The tab pill selects the tab. The right icons open Commands, create something new, open Settings and close the window.

**Polish notes:** The toolbar has no divider from the terminal, so chrome and content merge (it may be intentional). The right-hand icons have no hover affordance in this still frame. The `⌘` glyph is a macOS idiom on a Linux client, where the shortcut is Ctrl.

---

## 01 — Settings › Interface (`01-settings-interface.png`)

**What it is:** A modal Settings panel centred over the dimmed workspace, opened with the toolbar sliders icon (inferred) or Commands. Interface is the first section and is selected.

**Panel geometry:** About 987 × 674 px (≈494 × 337 pt), centred, with a radius of about 22 px. The left nav column is about 247 px (≈124 pt) wide and has the same fill `#0C1523` as the content. It is separated only by a 1 px `#141D2A` vertical line. A close button (× in a small dark circle of about 33 px) sits at the content's top-right.

**Nav sidebar (top to bottom):** heading **Settings** (semibold, larger), then **Interface** (selected: `#222A37` rounded fill, white semibold text), **Terminal**, **Notifications**, **Integrations**, **Remote Hosts**, **Shortcuts**, **Updates** and **About** (all `#93A3B8`, regular). Nav rows are about 44 px tall.

**Content (title: `Interface`):**
| Row label | Helper text (verbatim) | Control | Current value |
| --- | --- | --- | --- |
| Theme | "The window's colours. Until you pick a terminal theme, terminals take the matching one (Follow chrome)." | Value text + secondary button **Change…** | **Midnight** |
| Interface zoom | "Chrome and terminals together, 70–150%. Ctrl + and Ctrl − also work." | Stepper: circular **−** button, value, circular **+** button, then pill button **Reset** | **130%** |
| Pane density | "Compact uses straight separators, without pane headers or gaps." | Two-segment control: **Compact** \| **Comfortable** | **Comfortable** |
| Session sidebar | "Hosts and their sessions beside the terminals. Ctrl Shift B." | Toggle | Off |
| Tab position | "In the toolbar, or under each session in the sidebar while it's open." | Two-segment control: **Top** \| **Sidebar** | **Top** |
| Notices | "Where in-app notices appear." | Two-segment control: **Top right** \| **Bottom right** | **Top right** |

**Visual spec:** Rows are label over helper, left-aligned at about 64 px from the content edge, with the control right-aligned to about 30 px from the panel edge. Row pitch is about 60–75 px. Helper text wraps to 2 lines at about 430 pt. The selected segment shows as a darker `#001528` pill with a light outline. Unselected segments are `#161E2E` with blue-grey text, and the whole group has a thin outline. The stepper buttons are dark circles with a hairline ring.

**Behaviour implied:** Theme opens the Interface theme popup (09). Zoom steps the whole UI and terminals (70–150%), and Reset returns it to 100% (inferred). Density switches pane chrome. Sidebar toggles the session sidebar, also with Ctrl Shift B. Tab position moves tabs into the sidebar while it is open. Notices sets the toast corner.

**Polish notes:**
- **Dividers are inconsistent.** There is a 1 px divider under Theme, Interface zoom and Pane density, and between Tab position and Notices, but **none between Session sidebar and Tab position**.
- Segmented controls have different widths per row (Compact/Comfortable is wider than Top/Sidebar), so their left edges don't align.
- The stepper's −/+ circles use a different style (dark ring) from the pill buttons beside them (Reset/Change…).
- About 150 px of empty space sits under Notices, so the panel height does not fit the content.

---

## 02 — Settings › Terminal, top (`02-settings-terminal.png`)

**What it is:** The Terminal section, scrolled to the top.

**Content (title: `Terminal`):**
| Row label | Helper text (verbatim) | Control | Value |
| --- | --- | --- | --- |
| Font family | "Active: JetBrainsMono Nerd Font. Changes apply when you reopen the client." | **Change…** button | (in the helper) |
| Font size | "8–48 pt, for every terminal." | Stepper − / value / + | **15 pt** |
| Pane padding | "0–32 pt around each terminal's content. Applies immediately to every pane." | Stepper − / value / + | **8 pt** |
| Fallback fonts | "Used for missing glyphs, followed by installed system fallbacks. Changes apply when you reopen the client." | Read-only value (no button) | **Automatic** |
| Theme | "Declared to Rex for panes this client owns or nobody owns. Follow server declares nothing until this client has declared a theme once; from then on it resets those panes to Rex's colours, once each per connection." | Value + **Change…** | **Follow server** |

Section header **DEFAULT SHELL** (caps), then one row per saved host:
| Host | Helper | Control |
| --- | --- | --- |
| shuvdev | "New sessions, tabs and splits start zsh" | **Change…** |
| shuvbot-rex | "New sessions, tabs and splits use the server's default shell" | **Change…** |
| shuvmbp | "New sessions, tabs and splits use the server's default shell" | **Change…** (row clipped by the panel bottom) |

**Behaviour implied:** Font family opens the Terminal fonts form (10), which also edits fallbacks. Size and padding step within the stated ranges. Theme opens the searchable terminal theme picker (11). Each host's Change… opens the Default shell form (12).

**Polish notes:**
- The Theme helper is a dense 4-line paragraph, the longest in Settings and hard to parse.
- Fallback fonts shows a value with no control. It is edited only through Font family › Change…, which isn't obvious.
- Default shell rows have **no dividers**, while the rows above do.
- The last row (shuvmbp) is clipped at the panel's bottom edge with no scroll fade or scrollbar visible.
- Stepper values ("15 pt", "8 pt") and "Automatic"/"Follow server" are right-aligned at different x positions.

---

## 03 — Settings › Notifications (`03-settings-notifications.png`)

**Content (title: `Notifications`):**
| Row | Helper (verbatim) | Control | State |
| --- | --- | --- | --- |
| Activity in tabs | "A word on a tab whose programs need you or failed, and Done on one that finished while you were away." | Toggle | On |
| Desktop notifications | "For tabs you aren't looking at, through your desktop's notification service. Clicking one shows its tab." | Toggle | On |

Section header **NOTIFY WHEN A PROGRAM**:
| Row | Helper | Toggle |
| --- | --- | --- |
| Needs you | "Waiting for approval, an answer or a sign-in." | On |
| Fails | "Reports an error." | On |
| Finishes | "Stops working, or reports that it's done." | On |
| Starts working | (none) | Off |
| Messages from programs | "Notifications programs send themselves with OSC 9 or OSC 777." | On |

**Visual spec:** Toggles are right-aligned at about 72 × 36 px with a light-blue `#81C6FE` track when on and `#253C51` when off. The rows under the section header have dividers. Activity in tabs and Desktop notifications have a divider between them, but none before the section header.

**Behaviour implied:** Tab activity words (needs-you, failed, "Done") and desktop notifications, filtered per event type. OSC 9/777 passthrough.

**Polish notes:** "Starts working" has no helper, so its row is shorter and the label sits vertically centred, which breaks the rhythm. The sub-toggles are not visibly dependent on (indented under or disabled by) "Desktop notifications".

---

## 04 — Settings › Integrations, top (`04-settings-integrations.png`)

**Content (title: `Integrations`):**
Intro paragraph: "On this machine, Rex can teach coding agents to use the rex command (a skill) and, where the agent supports it, tell each agent session which Rex session and terminal it runs in (a hook). Every change is shown before it's made."

Section **AGENTS**, one row each, with an **Install…** secondary button on the right:
| Agent | Status line | Extra helper (verbatim) |
| --- | --- | --- |
| Claude Code | Not installed | none |
| Codex | Not installed | "Shares its skill at ~/.agents/skills/rex with OpenCode and Pi and OMP." |
| OpenCode | Not installed | "Shares its skill at ~/.agents/skills/rex with Codex and Pi and OMP." |
| Pi | Not installed | "Shares its skill at ~/.agents/skills/rex with Codex and OpenCode and OMP." |
| OMP | Not installed | "Shares its skill at ~/.agents/skills/rex with Codex and OpenCode and Pi." / "Shares Pi's skill directory: installing or removing it here does the same for Pi." |

Below: buttons **Check again** (secondary) and **Set up all detected…** (primary gradient, left-aligned). Then the row **Let agents name sessions** with its toggle, clipped at the bottom edge.

**Behaviour implied:** Per-agent install opens a preview (16). Check again re-detects agents. Set up all detected… opens a combined preview (17).

**Polish notes:** The primary/secondary action pair is left-aligned while every other action in Settings is right-aligned. The bottom row and toggle are clipped mid-control. The repeated "Shares its skill at…" sentences are verbose, and OMP's two lines are redundant with Pi's.

---

## 05 — Settings › Remote Hosts (`05-settings-remote-hosts.png`)

**Content (title: `Remote Hosts`):**
- **Client label**: value `shuvdev`, helper "Visible on every host; reopen the app after editing.", button **Edit…**.
- Host rows. Each has an icon, a name (semibold) over an endpoint (helper colour), and the buttons **Edit…** (secondary) and **Remove…** (destructive: red text `#FB8888` and a reddish outline):
  - monitor glyph, **shuvdev**, `unix:///run/user/1000/rex/server.sock · zsh`
  - server-stack glyph, **shuvbot-rex**, `https://shuvbot-rex.tail586a6d.ts.net`
  - server-stack glyph, **shuvmbp**, `ssh://shuv@shuvmbp.tail586a6d.ts.net`
- Under each host is a **Remote control** row with the helper "Let rex do and other clients on this host change the view in this window." and a toggle that is **Off** for all three.
- **Add host…** secondary button, left-aligned.
- Footer helper: "Removing a host only stops this app connecting to it; its sessions keep running."

**Behaviour implied:** Per-host consent for remote control (`rex do`). Add, edit and remove hosts. The monitor glyph marks the local socket host and the server glyph marks remote hosts (inferred).

**Polish notes:** "Remote control" rows sit flush-left at the same indent as the host name's icon column, not indented under the host, so it is unclear which host they belong to (the grouping relies only on order and dividers). `rex do` is not code-styled. The local host shows its shell (`· zsh`) while the remote hosts don't.

---

## 06 — Settings › Shortcuts, first page (`06-settings-shortcuts.png`)

**Content (title: `Shortcuts`):** A two-column list of key chips (left) and the action (right, white). It shows no edit controls here (the README says pages 25–29 cover the rest).

- **NAVIGATE**: `Ctrl` `PageDown` Next tab · `Ctrl` `PageUp` Previous tab · `Ctrl` `Tab` Next tab across hosts · `Ctrl` `Shift` `Tab` Previous tab across hosts · `Ctrl` `K` Switch session
- **CREATE AND CLOSE**: `Ctrl` `Shift` `W` Close focused pane or tab · `Ctrl` `Shift` `T` New tab · `Ctrl` `Shift` `N` New session
- **PANES**: `Ctrl` `Alt` `D` Split pane · down (list continues below the fold)

**Visual spec:** Each key is a separate small chip (`#152D42`, label `#8D9EB2`, radius about 4 px). The action column starts at about x = 1037 (display), so there is a large fixed gap. The row pitch is about 43 display px.

**Polish notes:** Key chips are low contrast and visually smaller than the action labels. There is no visible way to rebind on this page, even though AGENTS.md says every binding is removable or replaceable. The wide dead gap between the chips and the actions looks like a table without column headers.

---

## 07 — Settings › Updates (`07-settings-updates.png`)

| Row | Helper | Control / value |
| --- | --- | --- |
| Installed version | "Release installer (install.sh)" | **v0.6.0** (right-aligned text) |
| Automatic update checks | "Checks signed release metadata on GitHub at most once a day. Never downloads binaries or installs automatically." | Toggle **On** |
| Release status | "Rex Linux 0.6.0 is up to date." | **Check now** (secondary button) |

**Polish notes:** "v0.6.0" has a "v" prefix here, but About (08) shows "0.6.0". About 60% of the panel is empty.

---

## 08 — Settings › About (`08-settings-about.png`)

| Row | Helper | Right side |
| --- | --- | --- |
| Rex Linux | Native Linux client for Rex | `0.6.0` |
| Commit | (none) | `ad196ff` |
| Build date | (none) | `2026-11-10 PST` |
| Server API revision | Active host · negotiated protocol | `13` |
| Repository | github.com/shuv1337/rex-rune | **Open repository** |
| Release notes | Changes in this version | **Open release notes** |
| License | MIT | **Open license** |
| Bug reports | Version details only; no host addresses or credentials. | **Copy version info** |

Dividers sit only after "Server API revision" and after "License".

**Polish notes:** Commit and Build date rows lack helpers, so their rows look taller and emptier than the rest (inconsistent vertical rhythm). The commit hash is not monospace. The buttons have different widths with no shared column width.

---

## 09 — Interface theme picker (`09-interface-theme-picker.png`)

**What it is:** A popup opened from Settings › Interface › Theme › **Change…**. It **replaces** the Settings panel (Settings is not visible behind it). It is anchored top-centre under the toolbar, about 597 px wide, with a `#0C1524` surface and radius about 14 px.

**Contents:** Caption **Theme** (small, blue-grey), then the options **✓ Midnight** (selected: `#222A38` fill, check glyph, semibold), **Rosé Pine**, **Rosé Pine Moon** and **Gruvbox Dark**. It has no search field and no previews or swatches.

**Behaviour implied:** Choosing an item sets the chrome theme. With terminal theme "Follow chrome", terminals follow too.

**Polish notes:** The popup position (top-centre of the window) is detached from the Change… button that opened it. There are no colour swatches, so you can't preview. The list has 4 items but the popup is as wide as the searchable 450-theme picker.

---

## 10 — Terminal fonts form (`10-font-family-and-fallbacks.png`)

**What it is:** A modal dialog from Settings › Terminal › Font family › **Change…**. It is about 675 px wide, centred, with a dimmed backdrop. The Settings panel is hidden.

**Contents:**
- Title **Terminal fonts**
- Label **Font family** above a text field with the value `JetBrainsMono Nerd Font` (focused, caret at the end)
- Label **Fallback families (comma-separated)** above a text field with the placeholder `Automatic system fallbacks when empty`
- Helper: "Applied when you reopen the client. --font and --font-fallback override saved choices for that launch. Server sessions keep running."
- Buttons, right-aligned: **Cancel** (secondary) and **Save** (primary gradient)

**Visual spec:** Text fields are full-width, about 41 px tall, radius about 8 px, with a 1 px subtle border and the same fill as the modal. Field text is larger than the labels. The placeholder is the same size in a muted colour.

**Polish notes:** The fields are free text with no font list or validation. The focused field shows no obvious focus ring beyond the caret. The placeholder looks only a little dimmer than the real value.

---

## 11 — Terminal theme picker (`11-terminal-theme-picker.png`)

**What it is:** A searchable popup from Settings › Terminal › Theme › **Change…**. It is anchored top-centre, about 597 × 1260 px, and extends nearly to the window bottom.

**Contents:**
- Search field: magnifier glyph, placeholder **Filter themes...** (focused), and the hint **esc** on the right.
- **SERVER**: **✓ Follow server**, with the right meta **Declares nothing** (selected row) · **Rex default colours**
- **BUILT IN**: **Follow chrome**, with the right meta **Midnight** · Midnight · Rosé Pine · Rosé Pine Moon · Gruvbox Dark
- **DARK**: 0x96f · 12-bit Rainbow · 3024 Night · Aardvark Blue · Aardvark Ink · Abernathy · Acid Lime · Adventure · Adventure Time · Adwaita Dark · Afterglow · Aizen Dark · Alien Blood · Amber CRT Retro · Andromeda · Apple Classic · Apple System Colors · Arcoiris · Ardoise · Argonaut · Arthur …
- Footer caption **More results below**
- A thin scrollbar thumb at the top right

**Visual spec:** Rows are about 38 px tall. Section caps match Settings section headers. The selected row has a `#222A38` fill with a check.

**Behaviour implied:** Filters by name. Groups are Server / Built in / Dark / Light (Light seen in 13). Choosing a theme declares it to Rex through `terminal.set_theme` (inferred from the Theme helper text).

**Polish notes:** It has no colour swatches or preview for any theme. "More results below" is a static caption rather than a scroll fade. The popup is very tall and narrow.

---

## 12 — Default shell form (`12-default-shell.png`)

**What it is:** A modal from Settings › Terminal › DEFAULT SHELL › shuvdev › **Change…**.

**Contents:** Title **Default shell on shuvdev**, label **Shell**, a text field with the value `zsh` (focused), the helper "Used for new sessions, tabs and splits on this host. Running terminals are unchanged.", and the buttons **Cancel** and **Save**.

**Polish notes:** It's a free-text field with no list of shells detected on the host and no "use server default" reset option visible.

---

## 13 — Terminal theme search (`13-theme-search.png`)

**What it is:** Picker 11 with the query `rose`.

**Results:** **DARK**: **Rose Pine** (highlighted) · Rose Pine Moon. **LIGHT**: Rose Pine Dawn.

**Polish notes:** The **built-in "Rosé Pine" / "Rosé Pine Moon" entries do not match the query "rose"**, so the search is not accent-insensitive. The query also hides the Server and Built in groups. There are two near-duplicate theme families (built-in "Rosé Pine" and catalogue "Rose Pine"). Match text is not highlighted.

---

## 14 — Settings › Terminal, scrolled to the bottom (`14-settings-terminal-lower.png`)

**What it is:** Screenshot 02 scrolled down. Font family has scrolled off the top, so the section starts at Font size.

**New content:** The shuvmbp row is fully visible, and a footer reads "**Saved in /home/shuv/.config/rex-linux/workspace.json**" (helper style).

**Polish notes:** The panel title "Terminal" stays pinned while rows scroll under it with no top fade or divider. Content scrolls inside a fixed-height panel even though the screen has a lot of free vertical space.

---

## 15 — Settings › Integrations, scrolled down (`15-settings-integrations-lower.png`)

**What it is:** Screenshot 04 scrolled down, with the intro paragraph out of view.

**New content:** **Let agents name sessions**, with the helper "Agents rename the Rex session after their task. Applies the next time Rex sets up your agents." and a toggle that is **Off**.

---

## 16 — Single-agent setup preview (`16-agent-integration-preview.png`)

**What it is:** A confirmation modal from Integrations › Claude Code › **Install…**. It is about 675 px wide.

**Contents:** Title **Set up Rex for Claude Code?**. List lines:
- "Create skill ~/.claude/skills/rex · Claude Code"
- "Create hook ~/.claude/settings.json · Claude Code"

Buttons: **Keep as is** (secondary) and **Set up** (primary).

**Behaviour implied:** It shows every file change before writing it, and nothing is written until Set up.

**Polish notes:** Paths are not monospace or distinguished from the agent name. "Keep as is" is an unusual label for cancel. Nothing distinguishes "create" from "modify existing settings.json".

---

## 17 — All-agents setup preview (`17-all-agents-setup-preview.png`)

**What it is:** A modal from Integrations › **Set up all detected…**.

**Contents:** Title **Set up Rex for Claude Code, Codex, OpenCode, Pi, OMP?**. List lines:
- "Create skill ~/.claude/skills/rex · Claude Code"
- "Create skill ~/.agents/skills/rex · Codex, OpenCode, Pi, OMP"
- "Create hook ~/.claude/settings.json · Claude Code"
- "Create hook ~/.codex/hooks.json · Codex"

Helper: "A shared skill changes for every agent that uses it. OMP and Pi share one skill, so both get it." Buttons: **Keep as is** and **Set up**.

**Polish notes:** It has no per-item checkboxes, so it is all or nothing. Note that "Set up all **detected**" lists all five agents, which were all "Not installed" in 04, so "detected" may mean the agent binary was found rather than the integration (inferred).

---

## Cross-cutting polish observations

1. Divider use is inconsistent across sections. Interface is missing one divider, Default shell has none, About has two, and Notifications has none above its section header.
2. Action alignment varies. Most actions are right-aligned, but "Check again / Set up all detected…" and "Add host…" are left-aligned.
3. Rows have no consistent minimum height. Rows without helpers (Commit, Build date, Starts working) change the rhythm.
4. Pickers and forms **replace** Settings instead of stacking over it. Picker popups anchor at the window's top-centre, not at the invoking control.
5. Fixed panel height leaves big empty areas (Updates, Interface) yet clips scrolled content (Terminal, Integrations) with no scroll affordance.
6. There are no theme previews or swatches anywhere.
7. Low contrast: key chips (`#8D9EB2` on `#152D42`) and helper text (`#92A2B7` on `#0C1523`). Helper text passes, but the chips are borderline.
8. Version formatting is inconsistent ("v0.6.0" vs "0.6.0"), and code-like strings (paths, `rex do`, the commit) use the proportional font.

## Feature inventory

- [workspace.toolbar.sidebar-toggle] Toolbar button to show or hide the session sidebar
- [workspace.toolbar.session-pill] Session pill shows the session name and host, with a chevron menu
- [workspace.toolbar.tab-pill] Toolbar tab pills labelled with the terminal title and a prompt glyph
- [workspace.toolbar.commands] ⌘ toolbar button opens Commands
- [workspace.toolbar.new] + toolbar button for a new tab or session
- [workspace.toolbar.settings] Sliders toolbar button opens Settings
- [workspace.toolbar.close] × closes the window
- [workspace.terminal] Full-bleed server-owned terminal pane rendered with Ghostty (Nerd Font glyphs, colours, right prompt)
- [settings.nav] Settings modal with 8 sections: Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts, Updates, About
- [settings.interface.theme] Interface theme: Midnight | Rosé Pine | Rosé Pine Moon | Gruvbox Dark (default Midnight)
- [settings.interface.zoom] Interface zoom stepper 70–150% with Reset, also Ctrl + / Ctrl − (value 130%)
- [settings.interface.pane-density] Pane density: Compact | Comfortable (current Comfortable)
- [settings.interface.session-sidebar] Session sidebar toggle, Ctrl Shift B (current off)
- [settings.interface.tab-position] Tab position: Top | Sidebar (current Top)
- [settings.interface.notices] Notices position: Top right | Bottom right (current Top right)
- [settings.terminal.font-family] Terminal font family, free text, applies on reopen (current JetBrainsMono Nerd Font)
- [settings.terminal.font-fallbacks] Fallback font families, comma-separated, empty means Automatic, applies on reopen
- [settings.terminal.font-cli-override] --font / --font-fallback CLI flags override saved fonts for one launch
- [settings.terminal.font-size] Font size stepper 8–48 pt (current 15 pt)
- [settings.terminal.pane-padding] Pane padding stepper 0–32 pt, applies immediately (current 8 pt)
- [settings.terminal.theme] Terminal theme declared to Rex: Follow server | Rex default colours | Follow chrome | built-ins | large Dark/Light catalogue (current Follow server)
- [settings.terminal.theme-search] Searchable terminal theme picker with Server / Built in / Dark / Light groups and an esc hint
- [settings.terminal.default-shell] Per-host default shell for new sessions, tabs and splits (shuvdev: zsh; others: server default)
- [settings.terminal.config-path] Footer shows the settings file path (~/.config/rex-linux/workspace.json)
- [settings.notifications.tab-activity] Activity words on tabs (needs you, failed, Done) (on)
- [settings.notifications.desktop] Desktop notifications through the system service; clicking one shows its tab (on)
- [settings.notifications.needs-you] Notify when a program needs you (on)
- [settings.notifications.fails] Notify when a program fails (on)
- [settings.notifications.finishes] Notify when a program finishes (on)
- [settings.notifications.starts-working] Notify when a program starts working (off)
- [settings.notifications.osc] Pass through program notifications from OSC 9 / OSC 777 (on)
- [settings.integrations.agents] Per-agent skill/hook install for Claude Code, Codex, OpenCode, Pi and OMP with status (all Not installed)
- [settings.integrations.shared-skill] Shared ~/.agents/skills/rex skill for Codex, OpenCode, Pi and OMP; OMP shares Pi's directory
- [settings.integrations.check-again] Re-detect installed agents
- [settings.integrations.setup-all] Set up all detected agents in one previewed change
- [settings.integrations.preview] Confirmation preview listing every file created (skill/hook paths) with Keep as is | Set up
- [settings.integrations.agents-name-sessions] Let agents rename the Rex session after their task (off)
- [settings.remote-hosts.client-label] Client label shown on every host, edit and reopen (current shuvdev)
- [settings.remote-hosts.list] Saved hosts with endpoint (unix socket, https, ssh) and Edit… / Remove…
- [settings.remote-hosts.remote-control] Per-host Remote control consent toggle for rex do / other clients (all off)
- [settings.remote-hosts.add] Add host… (tailnet discovery, manual, SSH; see the other inventory)
- [settings.shortcuts.list] Read-only grouped shortcut list (Navigate, Create and close, Panes, …)
- [settings.updates.installed-version] Installed version and install channel (v0.6.0, Release installer install.sh)
- [settings.updates.auto-check] Automatic daily signed-metadata update check, no auto-install (on)
- [settings.updates.check-now] Release status with a Check now button
- [settings.about.version] About: version, commit, build date, server API revision
- [settings.about.links] Open repository / Open release notes / Open license
- [settings.about.copy-version] Copy version info for bug reports (no hosts or credentials)
