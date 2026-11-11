# Rex Beta (macOS) UI reference, part 1: main window, app menu, Services, About, Settings, Server page

The source is Rex Beta 0.1.0 (1932), Bleeding Edge channel, running against Rex Server 0.1.0-preview.15. Screenshots were captured on 2026-11-10 (PST) with agent-desktop. They are in `rex-beta-ui-2026-11-10/screenshots/`, and the accessibility trees are in `rex-beta-ui-2026-11-10/evidence/`.

The screenshots are at **1x (points)**. On the 002 menu, the crop is 251x262 px and the AX bounds are 235x246 pt plus the shadow margin, so pixel measurements below are in points. Window crops include about 56 px of drop shadow on the left and about 38 px on top. Colours were sampled with ImageMagick and are approximate for antialiased text. Anything marked *(inferred)* was not read directly from pixels or the AX tree.

---

## 001-main-terminal.png: main terminal window

**What it is:** this is the primary Rex Beta window. It shows one workspace/session (`lunar-basin`) with one tab and one terminal pane: a local login shell on macOS. It appears at app launch. There is no AX JSON for this capture. The AX tree of 002 gives the window title as **`shuv@shuvbot:~ - lunar-basin`**, in the form `<focused pane/tab title> - <session name>`.

### Controls and labels, in visual order

Title bar / toolbar: one unified row with no separate macOS title bar.
1. **Traffic lights** (close red, minimize yellow, zoom green) at the far left, in their standard positions (centres at about x 23, 46, 69 from the window edge, y about 23).
2. **Session indicator:** a glyph of stacked windows (a rounded rectangle with two shorter lines stacked above it, like SF Symbol `square.stack`), followed by the label **`lunar-basin`**. Medium weight, grey `#A2A9AF`. It has no pill background. *(inferred)* This is the Rex session name.
3. **Tab pill (selected tab):** a rounded capsule of about 240 x 30 pt, fill `#2A2C30`, 1 px border `#3E4044`, radius about 15 (fully rounded ends). Its contents:
   - A small dark rounded-square badge (about 18 pt) holding a green `>_` terminal prompt glyph (`#58935F`).
   - The title **`shuv@shuvbot:~`**, near-white `#E1EAF1`, medium weight, proportional font.
   - A close **`×`** on the right, grey `#A6ABB1`.
   - Only one tab is open. No "new tab" button sits next to the tabs.
4. Flexible empty space (the window drag area).
5. Right-side toolbar icons, all line glyphs in grey `#A3A9AF` with about 34 pt between them:
   - **Eye** icon *(inferred: a toggle for an observe/visibility mode, see the menus in later files)*.
   - **`⌘`** (Command key glyph). *(inferred)* Opens the command palette.
   - **`+`**. *(inferred)* New tab or new terminal.

Pane: a single pane fills the window below the toolbar.
6. **Pane header row** (about 20 pt tall, at the top of the pane card):
   - Left: a `>_` prompt glyph (white) and the pane title **`shuv@shuvbot:~`**. The text is about 13 pt, regular/medium, proportional, colour `#BCC4CB` to `#E1EAF1`.
   - Right: five small icon buttons (about 14 pt glyphs, 24 pt apart):
     - **Split side-by-side** (rectangle with a vertical divider), visible grey `#4F545A`.
     - **Split stacked** (rectangle with a horizontal divider), visible grey.
     - Two overlapping squares (duplicate/float/pop-out?), **very dim**.
     - Diagonal two-way arrow (maximize/zoom pane), **very dim**.
     - **`×`** close pane, **very dim** (about `#1C1F24` on the `#0F1217` background).
   - The three dim icons look disabled or reveal on hover. *(inferred: zoom and close are dim because this is the only pane.)*
7. **Terminal content** (monospace, about 12 pt, cell height about 14 pt):
   - `Last login: Tue Nov 10 23:46:47 on ttys000`
   - `You have mail.`
   - The prompt line is a powerline-style left prompt: an Apple logo glyph (light grey `#D6DFE6`), a thin chevron `>`, a teal house glyph and `~` (teal `#397E90`).
   - The right prompt is right-aligned on the same row: a check `✔` (yellow-green), the chevron `<`, then **`shuv@shuvbot`** in bold amber `#F5A73B`, then `<` and `23:47:29` in muted blue-grey, then a clock glyph.
   - No cursor shows in 001. The full-display capture in 002/003 shows the same pane with a **solid blue block cursor `#0071F6`** after the prompt *(inferred: it is blinking and was caught in the off phase in 001)*.

### Layout and visual spec
- Window: about **1302 x 760 pt**. The outer corners are rounded (radius about 12) with a 1 px light edge `#23272B`, and a large soft macOS drop shadow sits behind the window.
- Window chrome background (toolbar strip and the margin around the pane): **`#0B0F13`**.
- Toolbar height: about 46 pt from the window top to the pane top.
- Pane card: inset about **8 pt** from the left, right and bottom window edges, about 1287 x 707 pt. Background **`#0F1216`**, 1 px border **`#222529`**, corner radius about 8 to 10. The pane reads as a slightly lighter card floating on the darker chrome.
- Terminal text starts about 10 pt from the pane's left edge, and the first output row is about 26 pt below the pane top, which leaves room for the header row.
- Text colours: primary near-white `#E2EAF1`. The pane title is proportional (SF), not mono. The terminal grid uses a monospaced font (it looks like SF Mono / Menlo-style).
- No status bar is visible at the bottom in this capture.

### Behaviour implied
- Each tab can hold split panes (the split buttons are in the pane header). The window title tracks the focused pane title plus the session name.
- The session name in the toolbar identifies which Rex session the window is attached to *(inferred: clicking it switches sessions, see later files)*.

---

## 002-menu-rex-beta.png: app menu (Rex Beta)

**What it is:** the macOS application menu, opened by clicking **Rex Beta** in the menu bar. The full display (`002-menu-rex-beta-display.png`) shows these menu-bar titles: **` (Apple)`, `Rex Beta` (bold, highlighted pill), `File`, `Edit`, `View`, `Window`, `Help`**.

### Items, verbatim and in order (AX bounds: menu 235 x 246 pt, each item 24 pt, each separator 11 pt)

| # | Item | Shortcut hint | Notes |
|---|------|---------------|-------|
| 1 | `About Rex Beta` | none | |
| 2 | `Check for Updates...` | none | Written with three ASCII dots in both the AX tree and the visual. |
| - | separator | | AX: a disabled, unnamed menuitem |
| 3 | `Settings…` | `⌘ ,` | Has a leading gear/aperture-style icon. The AX name uses a Unicode ellipsis. |
| - | separator | | |
| 4 | `Services` | `>` submenu chevron | |
| - | separator | | |
| 5 | `Hide Rex Beta` | `⌘H` | |
| 6 | `Hide Others` | `⌥⌘H` | |
| 7 | `Show All` | none | |
| - | separator | | |
| 8 | `Quit Rex Beta` | `⌘Q` | |

The AX tree also lists the (closed) Services submenu contents, which are the same as in 003.

### Visual spec
- This is a standard macOS Tahoe menu: a translucent, blurred vibrancy material that picks up the content behind it (bluish tint from the window below), with a large corner radius (about 12) and a thin light outer edge.
- Text is white (system menu font, 13 pt regular). Shortcut glyphs are right-aligned in dim grey (`#7C7E82`-ish).
- Separators are thin lines inset to the text column.
- Only the Settings item has an icon. The other items have no icon column.

### Behaviour implied
- About opens the About panel (004). Check for Updates triggers an updater; COVERAGE.md says it showed no visible panel. Settings opens the Settings window (005). Hide, Hide Others, Show All and Quit are standard.

---

## 003-menu-rex-services.png: app menu, Services submenu open

**What it is:** the same app menu with the pointer on **Services**, which opens the system Services submenu. The `Services` row is highlighted in **accent blue `#0069FF`** (rounded-rect selection), with white text and chevron.

### Services submenu items, verbatim and in order (AX: submenu 175 x 272 pt at x 268, items 24 pt)

| # | Item | Kind | Icon |
|---|------|------|------|
| 1 | `Development` | section header, grey, small, not selectable | none |
| 2 | `Activity Monitor` | item, currently **selected/hovered** (blue `#0069FF` pill highlight) | Instruments-style dark gauge icon |
| 3 | `Allocations & Leaks` | item | same Instruments icon |
| 4 | `File Activity` | item | same |
| 5 | `System Trace` | item | same |
| - | separator | | |
| 6 | `Shortcuts` | section header | none |
| 7 | `Lights 1%` | item | blue Shortcuts stacked-layers icon |
| 8 | `Lights 100%` | item | blue Shortcuts icon |
| 9 | `Turn Text Into Audio` | item | red/pink waveform icon |
| - | separator | | |
| 10 | `Services Settings…` | item | grey gear icon |

### Visual spec
- The same vibrancy material as the parent menu, with a radius of about 12. The submenu overlaps the parent menu's right edge by about 5 pt.
- The selection highlight is a rounded rectangle (radius about 5) inset from the menu edges and filled with the system accent blue.

### Behaviour implied
- These are **system-provided** macOS Services (Instruments, Shortcuts) and contain nothing Rex-specific *(inferred: standard NSApplication servicesMenu)*. Parity relevance: the macOS app exposes a standard Services menu. There is no Linux equivalent requirement.

---

## 004-about-rex-beta.png: About Rex Beta panel

**What it is:** a small, separate window titled **`About Rex Beta`** (AX window, 280 x 243 pt), opened from **Rex Beta > About Rex Beta**.

### Controls and labels, in visual order
1. Traffic lights: **Close** is enabled (red). **Minimize** and **Zoom** are present but **disabled** (grey) per the AX states.
2. **App icon image "Rexie"** (AX image named `Rexie`), 72 x 72 pt: a stylised dinosaur/axolotl head in profile facing right, with a frilled crest on the left. It has a gradient from orange/yellow at the top right through pink to purple/violet at the bottom left, a dark eye and a mouth line.
3. **`Rex Beta`**: the app name, bold, about 13 pt, `#DCDCDC`/white.
4. **`Version 0.1.0 (1932)`**: medium/regular, about 11 pt, `#DCDCDC`.
5. **`Bleeding Edge`**: the update channel, secondary grey `#989898`, about 11 pt.
6. **`Rex Server 0.1.0-preview.15`**: the server version, secondary grey `#989898`, about 11 pt.
7. Button **`Copy Version Info`** (134 x 22 pt) with a leading copy glyph (two stacked documents). Fill `#2D2D2D`, radius about 6, text `#DCDCDC`.

### Layout and visual spec
- Window background: **`#1B1B1B`**. This is the standard macOS dark window grey, *not* the app's blue-black `#0B0F13`. The window has rounded corners (about 12) and a strong drop shadow.
- All content is centre-aligned in one column. The icon top sits about 32 pt below the window top, and the name sits about 12 pt below the icon. The text lines are spaced 17 pt apart, and the button sits about 14 pt below the last line, with about 20 pt bottom padding.

### Behaviour implied
- Copy Version Info puts the app version, channel and server version on the clipboard *(inferred from the label)*.
- The About panel shows both the client build and the **connected server version**, plus the release channel.

---

## 005-settings-overview.png: Settings window, Server page (default)

**What it is:** the Settings window (AX window title **`Settings - Server`**, 816 x 850 pt), opened with **Rex Beta > Settings…** or `⌘,`. It opens on the **Server** category. The layout is a sidebar and a detail pane, styled like macOS System Settings but in the app's own dark palette.

### Sidebar (width 215 pt; AX splitter value `214`, disabled, so the width is fixed)
1. Traffic lights at the top left (close and zoom enabled, **minimize disabled**). There is no title text.
2. **Search field**, AX textfield `Search Settings`, with placeholder **`Search Settings`** and a leading magnifier glyph. About 195 x 28 pt, fill `#202328`, radius about 9, placeholder grey `#767C81`.
3. Outline **`Settings categories`**. The groups are separated by an extra gap of about 13 pt:
   - Group 1:
     - **Server** (two-line row: title **`Server`** semibold, subtitle **`Running`** in grey `#63686D`, smaller). The icon is the colourful Rex logo in a circle (about 28 pt). The row is **selected** (AX `Server, Running`, value `Selected`), 187 x 32 pt, with selection fill **`#191D22`** and radius about 8. This row is taller than the others.
   - Group 2:
     - **Interface**: icon is a dark tile showing red, yellow and green dots over two pane shapes.
     - **Themes**: icon is a rainbow-gradient tile with a palette glyph.
   - Group 3:
     - **Terminal**: icon is a dark green tile with a green `>_`.
     - **Shortcuts**: icon is a grey tile with `⌘`.
     - **Integrations**: icon is a dark tile with a pink/blue puzzle-piece glyph.
     - **Notifications**: icon is a red-orange tile with a white bell and a badge dot.
     - **Remote Hosts**: icon is a purple/magenta tile with a globe.
   - Group 4:
     - **Developer**: icon is a blue-grey tile with a hammer.
   - Single-line rows are 24 pt high, with each cell taking 32 pt (8 pt spacing). Icons are about 20 pt colourful rounded squares in the style of macOS System Settings. Labels are 13 pt regular, `#E1EAF1`.

### Detail pane toolbar
4. **Back / Forward**: a segmented capsule of two buttons (each 38 x 36 pt) with `<` and `>` chevrons. Fill `#292E34`, a thin vertical divider `#5D6268`, and both **disabled** (AX).
5. Page title **`Server`**: bold, about 15 to 16 pt (AX 21 pt line), left-aligned next to the nav capsule. AX description: `Settings page`.

### Detail content (cards start 30 pt right of the splitter and are about 560 pt wide)

**Card 1: server status** (about 55 pt tall)
- Rex logo icon (about 30 pt circle, gradient).
- **`Rex Server`**: 13 pt semibold, primary.
- **`Running for 10m`**: 11 pt, secondary grey. It is followed by an **ⓘ** button (AX `Server details`, 11 x 11 pt) that opens the popover in 006.
- Right side: button **`Stop Server…`** (101 x 24 pt capsule). Destructive styling: fill **`#3A2023`** (dark red tint), text **`#FF5658`** (red).

**Card 2: remote address** (about 44 pt)
- **`Remote Address`** (13 pt primary).
- **`https://shuvbot-rex.tail586a6d.ts.net`** (11 pt secondary, the value).
- Right side:
  - An icon-only button **Copy** (AX `Copy Remote Address`, 37 x 24 pt capsule, fill `#22262A`, copy-documents glyph `#C6CED4`).
  - Button **`Pair`** (AX `Show pairing QR code`, 68 x 24 pt capsule) with a QR-code glyph. **Accent fill `#006FFE`, and the label and glyph are dark (`#0F1217`), not white.**

**Section heading `Tailscale`** (AX heading; 13 pt bold, primary, about 10 pt left inset, about 18 pt above the card)

**Card 3: Tailscale** (rows separated by 1 px dividers `#26292D`, inset 10 pt on both sides)
- Row **Signed in** (about 56 pt):
  - A Tailscale logo (a 3 x 3 dot grid, white and grey dots) in a dark circular badge (about 32 pt).
  - **`Signed in to Tailscale`** (13 pt semibold).
  - Subtitle **`Signed in as shuv1337@github`** (11 pt secondary).
  - Right: button **`Sign Out...`**. The visual uses three dots; the AX name is `Sign Out`. It is an 83 x 24 pt capsule, fill `#22262A`, text `#E1EAF1`.
- Row **toggle**:
  - **`Serve over Tailscale`**, with helper text **`Connect to your local Rex server over your Tailnet.`**
  - Switch (AX `switch`, 36 x 16 pt): **ON**, track `#0676FD`, white knob on the right.
- Row **toggle**:
  - **`Limit access to your Tailscale account`**, with helper text **`Rex will verify the caller is signed in as shuv1337@github.`**
  - Switch: **ON**.
- Row **read-only value**:
  - **`Tags`**, on the right **`None`** (secondary grey `#767C81`) followed by a **ⓘ-style `?` circle help glyph**.
- Row **read-only value**:
  - **`Control Server`**, on the right **`Tailscale`** (grey) followed by a `?` help glyph.
- Row **text field**:
  - **`Preferred Hostname`**, with helper text **`Used to generate the Tailscale address.`**
  - Right: a text field (AX `Tailscale hostname`) with value **`shuvbot-rex`**. It is about 319 x 22 pt, fill `#1C1D20`, 1 px border `#2B2D30`, radius about 5, text `#E1EAF1`. It starts at about 50% of the card width.

**Section heading `Client Info`**

**Card 4: client info**
- Row **text field**:
  - **`Label`**, with a wide text field (AX `Client label`, about 468 pt wide) that starts right after the label column.
  - The field shows **`shuvbot`** in dim grey `#5B5F64`. The AX value is empty, so this is a **placeholder** (it defaults to the hostname). Fill `#1C1D20`.
- Row **toggle**:
  - **`Remote Control`**, with helper text **`Let Rex and other clients perform actions in this app.`**
  - Switch: **ON**.

The rest of the pane is empty, and nothing scrolls.

### Layout and visual spec
- Window: **816 x 850 pt**, outer radius about 12, 1 px light edge, big drop shadow.
- **Sidebar background `#0F1217`**. **Content background `#0B0F13`**. The split is a hard edge with no visible divider line (the splitter is 1 pt).
- **Card fill `#12161A`**, corner radius about 10 to 12, no border or shadow. Cards are spaced about 10 pt apart within a section, and about 30 pt separates a section from the next heading.
- Inner padding is 10 pt left and right. Rows with a subtitle are about 46 pt tall, and single-line rows about 39 pt.
- Typography: titles 13 pt (regular, or semibold for card headers), helper text 11 pt secondary. Everything is proportional system font, and nothing is mono.
- Colours:
  - Primary text `#E1EAF1`.
  - Secondary text about `#767C81`, with the sidebar subtitle `#63686D`.
  - Accent / switch on `#0676FD` to `#006FFE`.
  - Destructive text `#FF5658` on `#3A2023`.
  - Neutral button fill `#22262A`.
  - Input fill `#1C1D20`.
  - Dividers `#26292D`.
- Controls are right-aligned to the card's inner right edge (10 pt inset). Buttons are capsule shaped (radius = height / 2).
- Toggles are a compact macOS switch (36 x 16 pt AX bounds, about 36 x 20 visible).

### Behaviour implied
- **Stop Server…** stops the locally hosted Rex server. The ellipsis implies a confirmation step.
- **Copy** copies the remote URL. **Pair** shows a QR code that another device scans to pair (see later files).
- Tailscale integration: the app embeds a Tailscale node and can sign out. **Serve over Tailscale** exposes the local server on the tailnet at `https://<hostname>.<tailnet>.ts.net`. **Limit access** restricts callers to the same Tailscale identity. Tags and Control Server are informational, with help tooltips. **Preferred Hostname** sets the tailnet machine name.
- **Client Info > Label** is the name this client advertises to the server and other clients (placeholder = machine hostname). **Remote Control** lets the server or other clients drive actions in this app.
- Sidebar search filters the settings *(inferred)*. Back and Forward navigate the settings page history.

---

## 006-settings-server-details.png: Server page with the details popover

**What it is:** the same Settings > Server page as 005, after activating the **ⓘ** button (AX `Server details`) next to `Running for 10m`. A popover/tooltip bubble appears **above** the button and overlaps the toolbar title area.

### Popover contents (AX statictexts, 2 rows, 2 columns)

| Label (right-aligned, grey) | Value (left-aligned, white) |
|---|---|
| `Server Version` | `0.1.0-preview.15` |
| `API Version` | `Revision 13` |

### Visual spec
- The bubble is about 215 x 64 pt with a corner radius of about 12, a translucent dark grey fill (about `#2A2F34`, `#2E3338` near the edges) and a thin lighter edge. A small **downward-pointing arrow** at the bottom centre points at the ⓘ glyph.
- Text is 12 to 13 pt. Labels are secondary grey and values near-white, with about 8 pt between the columns and a row pitch of about 23 pt.
- Everything else on the page is identical to 005, with the same AX values. The popover partly hides the `Server` title and the `Rex Server` label.

### Behaviour implied
- The info button shows the connected server's version and **protocol/API revision** (Revision 13). It is a lightweight popover that does not need dismissal UI *(inferred: hover or click toggles it)*.

---

## Feature inventory

- [app.window.unified-toolbar] Main window has a unified dark toolbar (`#0B0F13`) with traffic lights, the session name, tab pills and right-side action icons. No separate title bar.
- [app.window.title] The window title is `<focused tab title> - <session name>` (for example `shuv@shuvbot:~ - lunar-basin`).
- [app.toolbar.session-name] Toolbar shows the current Rex session name (`lunar-basin`) with a stacked-windows glyph.
- [app.toolbar.tabs] Tabs are capsules (about 240 x 30 pt, fill `#2A2C30`, border `#3E4044`) holding a terminal icon badge, the tab title and a close `×`.
- [app.toolbar.eye] Toolbar eye icon button *(inferred: watch/observe toggle)*.
- [app.toolbar.command-palette] Toolbar `⌘` icon button *(inferred: command palette)*.
- [app.toolbar.new] Toolbar `+` button *(inferred: new tab/terminal)*.
- [app.pane.card] Each pane is a rounded card (`#0F1216`, border `#222529`) inset 8 pt in the window chrome.
- [app.pane.header] Pane header shows the prompt glyph and the pane title.
- [app.pane.split-horizontal] The pane header has a split side-by-side button.
- [app.pane.split-vertical] The pane header has a split stacked button.
- [app.pane.float-or-duplicate] The pane header has a two-overlapping-squares button (dimmed in a single-pane layout).
- [app.pane.zoom] The pane header has a maximize/zoom pane button (dimmed with a single pane).
- [app.pane.close] The pane header has a close pane `×` (dimmed with a single pane).
- [terminal.cursor] Default cursor is a solid blue block (`#0071F6`).
- [terminal.font] The terminal grid is monospaced, about 12 pt. Chrome text is a proportional system font.
- [menu.app] App menu items: About Rex Beta; Check for Updates...; Settings… (`⌘,`); Services >; Hide Rex Beta (`⌘H`); Hide Others (`⌥⌘H`); Show All; Quit Rex Beta (`⌘Q`).
- [menu.bar] Menu bar titles: Rex Beta, File, Edit, View, Window, Help.
- [menu.services] The standard macOS Services submenu (system items: Development / Shortcuts / Services Settings…). It is system-provided.
- [app.updates.check] Check for Updates... menu command (Bleeding Edge channel).
- [about.panel] About window (280 x 243 pt, `#1B1B1B`) shows the Rexie icon, the name, `Version 0.1.0 (1932)`, the channel `Bleeding Edge` and `Rex Server 0.1.0-preview.15`.
- [about.copy-version-info] The `Copy Version Info` button copies the client, channel and server versions.
- [settings.window] Settings window (816 x 850 pt) with a fixed 215 pt sidebar, a search field, a categories outline and Back/Forward navigation with a page title.
- [settings.search] `Search Settings` field in the sidebar.
- [settings.nav.back-forward] Back/Forward capsule for settings page history (disabled when there is no history).
- [settings.categories] Categories: Server (with a live status subtitle `Running`), Interface, Themes, Terminal, Shortcuts, Integrations, Notifications, Remote Hosts, Developer. Four groups, with coloured icon tiles.
- [settings.server.status] Rex Server card shows the running state and uptime (`Running for 10m`).
- [settings.server.details] ⓘ popover shows `Server Version` (0.1.0-preview.15) and `API Version` (Revision 13).
- [settings.server.stop] `Stop Server…` destructive button (red text on a dark red capsule).
- [settings.server.remote-address] Remote Address shows the tailnet HTTPS URL of the local server.
- [settings.server.copy-address] Copy Remote Address icon button.
- [settings.server.pair] `Pair` accent button shows the pairing QR code.
- [settings.server.tailscale.account] Tailscale account status (`Signed in to Tailscale` / `Signed in as <user>`) with a `Sign Out...` button.
- [settings.server.tailscale.serve] Toggle `Serve over Tailscale`: "Connect to your local Rex server over your Tailnet." (on).
- [settings.server.tailscale.limit-access] Toggle `Limit access to your Tailscale account`: "Rex will verify the caller is signed in as <user>." (on).
- [settings.server.tailscale.tags] Tags: read-only value (`None`) with a help `?`.
- [settings.server.tailscale.control-server] Control Server: read-only value (`Tailscale`) with a help `?`.
- [settings.server.tailscale.hostname] Preferred Hostname text field (`shuvbot-rex`): "Used to generate the Tailscale address."
- [settings.server.client.label] Client Info > Label text field (placeholder = hostname `shuvbot`).
- [settings.server.client.remote-control] Toggle `Remote Control`: "Let Rex and other clients perform actions in this app." (on).
- [visual.palette] Chrome background `#0B0F13`, sidebar/pane `#0F1217`, cards `#12161A`, selection `#191D22`, input `#1C1D20`, neutral button `#22262A`, divider `#26292D`, text `#E1EAF1`, secondary `#767C81`, accent `#006FFE`, destructive `#FF5658` on `#3A2023`.
