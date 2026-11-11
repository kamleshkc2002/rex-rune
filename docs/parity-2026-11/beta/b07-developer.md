# B07: Settings > Developer (Rex Beta 0.1.0 (1932), macOS)

Sources: `rex-beta-ui-2026-11-10/screenshots/055-…060-*.png`, accessibility trees `evidence/055…060-*.json`, full-display captures `evidence/056/057/058-*-display.png`. Captured Tue Nov 10, 11:59 PM (menu-bar clock, PST). `COVERAGE.md` says the set covers all nine settings categories and that "Check for Updates produced no visible panel"; it does not list a known gap for the Developer page.

All measurements are in points. The window screenshots are 1x (928×962 px including the macOS window shadow; the window itself is 816×850 pt, drawn at image offset (56, 38)). Tree coordinates are screen coordinates with the window origin at (1104, 100). Below, "win(x,y)" means window-relative points (screen minus 1104,100). Colours were sampled from the 1x PNGs. "ink" means the most saturated or contrasting pixel in an anti-aliased glyph, so true text colours may be slightly brighter.

---

## Shared chrome (all Developer screenshots)

The settings window (`Settings - Developer`, role window, 816×850) is a two-column split with a unified toolbar.

- **Traffic lights**: Close (red), Minimize (grey, **disabled** in the tree), Zoom (green), 16×16 each, at win(18,18), win(41,18), win(64,18). Minimize renders as a dimmed grey dot.
- **Toolbar**: 52 pt tall, spanning the full window width. There is no visible divider under it.
  - **Back / Forward**: a pill-shaped, segmented capsule (76×38 group at win(222,7)). Each half is a 38×36 button. A thin vertical divider (grey, ~#5D6267) separates them. Back `<` is bright white (#FFFFFF) and enabled. Forward `>` is dim (#5D6267) and **disabled**. The capsule fill is #292E33, with fully rounded ends.
  - **Page title**: "Developer" (statictext, description "Settings page"), about 15–16 pt semibold, #DADADB, at win(300,15), left-aligned just right of the nav capsule.
- **Sidebar**: 215 pt wide (the splitter value is `214`; the splitter is disabled, so it is not resizable). Background #0F1217. A 1 pt splitter line at win(214) separates it from the content pane (no visible contrast at that column).
  - **Search field** "Search Settings" (textfield): rounded capsule about 195×28 with fill #202329 and a magnifier glyph at left. Placeholder #767C81.
  - **Categories outline** ("Settings categories"). Rows are 32 pt cells; each button is 187×24, with 4 pt inner inset. Icons are 20×20 coloured rounded-square app-style glyphs. Labels are about 13 pt regular, #E1EAF1. Groups are separated by about 13 pt gaps:
    1. "Server", with subtitle "Running" (#767C81, ~10–11 pt). The tree name is "Server, Running" and the row is 40 pt. The icon is a multicoloured round Rex mascot avatar.
    2. "Interface" (dark tile with three coloured dots), "Themes" (multicolour palette tile).
    3. "Terminal" (green `>_` prompt tile), "Shortcuts" (grey ⌘ tile), "Integrations" (pink/purple puzzle-like tile), "Notifications" (red/orange bell tile), "Remote Hosts" (purple globe tile).
    4. "Developer" (light blue-grey hammer tile). It is **selected** (value "Selected"): a rounded-rect highlight about 195×32, fill #191D22, label semibold.
- **Content pane**: win(215..816), background #0B0F13. It is a scroll area starting below the toolbar (y=44 window). There is an overlay scrollbar at the right edge (19 pt hit area, 6 pt handle, colour about #757779). The scrollbar is hidden at rest and appears while scrolling (it is visible in 059).

### Content grid (pane-wide constants)

- The card column x starts at win(235): 20 pt from the pane edge. Card width is 561 pt, with a 20 pt right margin.
- Card content inset: 10 pt left and right. Text starts at win(245); trailing controls end at win(786).
- Card fill is #12161A. There is no visible border or shadow. Corner radius is about 10–12 pt (inferred from the anti-aliased corner ramp).
- Section headings are about 13 pt semibold, #E1EAF1, 16 pt line, placed 10 pt above their card. Card bottom to the next heading top is 30 pt.
- Row title: about 13 pt regular, #E1EAF1, 16 pt line. Row subtitle: about 11 pt regular, #767C81, 14 pt line, with a 3 pt gap under the title.
- Single-line-subtitle rows are 45–46 pt. Rows with a two-line subtitle are about 60 pt.
- Intra-card dividers are 1 pt, #26292D, inset 10 pt on both sides (from the text column to the trailing control edge).
- **Switch**: macOS mini-ish switch (tree 36×16; drawn about 36×20 pill). ON uses an accent track #0676FD with a white knob at the right. OFF uses a dark track #292D30 with a white knob at the left.
- **Popup button** (tree role combobox, readonly): borderless, transparent. The value text is right-aligned, about 13 pt, #DBDBDC. It is followed by a 22 pt circular chevron-badge (up/down chevrons ⌃⌄, fill #24282C, glyph #DEDEDF). The button height is 24 and its width hugs the value.
- **Copy button**: 37×24 capsule (fill about #22262A) with a "two overlapping documents" copy glyph in #E1EAF1.
- **Link-style button**: text-only, system accent blue #0073FF, about 13 pt.

---

## 055 – settings-developer.png

**What it is.** The Developer settings page, scrolled to the top. Reach it from Settings (sidebar "Developer", the last item, in its own group). The window sits at the right of a 1920×1080 display.

**Every control and label, top to bottom:**

1. Section heading **"Developer"**
   - Card (561×45):
     - Title "Developer Mode"
     - Subtitle "Expose settings for development and debugging."
     - Switch "Developer Mode": **ON** (checked, value 1).
2. Section heading **"App Version"**
   - Card (561×44):
     - Title "Update Channel"
     - Subtitle "Every commit that passes CI. Expect some instability." (This subtitle describes the selected channel; see 056.)
     - Popup "Update Channel", value **"Bleeding Edge"** (combobox, readonly, 135×24), with a chevron badge.
3. Section heading **"Rex Installation"**
   - Card (561×83), two rows separated by a divider:
     - Row 1: title "Location", subtitle "Uses the Rex helper included with this app." Popup "Location", value **"Included with Rex.app"** (181×24).
     - Row 2 (status row, ~38 pt): green filled circle with a white check mark (#00BE19 to #02AC19, about 16 pt), then "Included Rex ready" (13 pt, #E1EAF1; the label is indented 26 pt from the text column to clear the icon). At the trailing edge is the link button **"Show Details"** (accent #0073FF, 79×16).
4. Section heading **"Local API"**
   - Card (561×44):
     - Title "Local API URL"
     - Subtitle (selectable static text, small, #767C81): `unix:///Users/shuv/Library/Application%20Support/rex/server.sock`. This is proportional, not monospace, and percent-encoded.
     - Copy button, accessibility name **"Copy local Rex endpoint"** (37×24).
5. Section heading **"Web Client Origins"**
   - Card (561×77):
     - Row 1 (~37 pt): "https://rex.superlogical.com" (13 pt, #E1EAF1). At the trailing edge is the badge-like label **"Default"** (about 10–11 pt, #767C81; accessibility value "Default origin, always allowed"). There is no remove button on this row.
     - Divider.
     - Row 2: a text field (accessibility "Allowed origin", visual about 447×24) with a rounded 1 pt border #26292D, fill #1C1D20, and placeholder "https://example.com or https://*.example.com" (#5B5F64). Then the button **"Add Origin"** (85×24, capsule fill #1C2125, label #7F868C). It is **disabled** because the field is empty.
   - Footnote below the card (outside it, ~11 pt, #777D83, two lines, 526 pt wide): "Browser origins allowed to open Rex WebSocket streams. The hosted Rex client is always allowed. Wildcards match subdomains, e.g. https://*.vercel.app. Changes apply after Rex restarts."
6. Section heading **"Experimental"**
   - Card (561×317, six rows with dividers; only the first three are visible here):
     - "Download new updates when available": "Downloads updates in the background and asks before restarting." Switch **ON**. The tree marks the switch **and** both labels as **disabled** (checked, disabled). Visually it still renders as a bright blue ON switch with full-contrast text. Only a very subtle difference is visible: the track samples #0E58B2 to #0D5BBA versus #0676FD for the enabled switches, so it is slightly darker or dimmer.
     - "Install updates automatically": "Relaunches into the new version while Rex is not visible. Sessions keep going." Switch **ON** (enabled).
     - "Terminal anchoring": "Pin terminal text to the top of each pane, or to the bottom so the last row meets the pane's bottom edge." (wraps to 2 lines, 421 pt wide). Popup "Terminal anchoring", value **"Top"** (70×24).
     - Further down and offscreen (see 059): "New quick switcher", "Quick Look links", "Cursor shaders".

**Layout and visual spec (055-specific).** Heading y positions (window): Developer 64, App Version 165, Rex Installation 265, Local API 404, Web Client Origins 504, Experimental 673. All trailing controls right-align at win(786). Switches sit vertically centred on the title+subtitle block. Popups sit centred on the row; for the 2-line Terminal anchoring row, the popup is centred on the full 3-line block. Nothing changes on hover in these static captures.

**Behaviour implied.**
- Developer Mode exposes development and debugging settings. That this page shows the App Version, Rex Installation and similar sections *because* the toggle is on is (inferred).
- The Update Channel picks which build stream the app updates from.
- Location chooses which Rex helper/server binary is used. Show Details expands build provenance (see 060).
- Copy puts the local Unix-socket endpoint on the pasteboard.
- Web Client Origins is an allow-list for browser origins opening Rex WebSocket streams. The hosted origin is permanent ("Default"). User origins are added via the field and the button, wildcards are supported, and changes require a Rex restart.
- The Download/Install update toggles control background updating. "Download" being disabled while ON suggests it is forced on by "Install updates automatically" being ON (inferred).

---

## 056 – menu-update-channel.png

**What it is.** The open popup menu of the **Update Channel** popup (App Version card). You reach it by clicking the "Bleeding Edge" popup. The crop is 152×74 px. The full display (`056-…-display.png`) shows the menu overlaid on the row, positioned so that the checked item sits on top of the popup's own position (standard macOS pop-up-button behaviour).

**Items (menuitem roles, 136×24 each, in order):**
1. "Preview": unselected.
2. "✓ Bleeding Edge": **selected** (checkmark at left; highlighted row).

There are no shortcuts, separators or disabled items.

**Visual spec.**
- Menu: a dark translucent vibrancy panel (fill about #2B3035), about 6–8 pt corner radius, 1 pt lighter rim, and a drop shadow.
- Menu width: 136 pt.
- The highlighted item is an accent-blue rounded rect (#0465EF to #0676FD) with an inset of about 5 pt. Its text and checkmark are white (#D8E8FF to #FFFFFF).
- Unselected text: light grey, about 13 pt.
- The checkmark column is about 14 pt wide. Unchecked items are indented to align with the checked item's text.

**Behaviour implied.** Two update channels exist: Preview and Bleeding Edge. The Bleeding Edge subtitle reads "Every commit that passes CI. Expect some instability." The Preview subtitle was not captured (not observed).

---

## 057 – menu-rex-installation.png

**What it is.** The open popup for **Rex Installation > Location**. Reached by clicking "Included with Rex.app". The crop is 198×74 px. In the display capture, the menu overlaps the "Show Details" link beneath it; the link is partly visible below the menu as "Show Details".

**Items (182×24 each):**
1. "✓ Included with Rex.app": **selected** (blue highlight, white text).
2. "Source Checkout": unselected.

**Visual spec.** Same menu style as 056; the width matches the popup (182).

**Behaviour implied.** The Rex helper can be the copy bundled in the app, or a source checkout (presumably a developer's local build of Rex, which would change the status row and the details; inferred). The "Source Checkout" state (and any path picker it might reveal) was not captured.

---

## 058 – menu-terminal-anchoring.png

**What it is.** The open popup for **Experimental > Terminal anchoring**. The crop is 106×74 px; the display capture shows it at the bottom of the visible card.

**Items (90×24 each):**
1. "✓ Top": **selected**.
2. "Bottom".

**Visual spec.** Same menu style; this is a narrow menu (90 pt).

**Behaviour implied.** "Top" pins terminal content to the top of each pane, as is usual for terminals. "Bottom" anchors the grid so the last row sits flush with the pane's bottom edge, putting any leftover sub-cell slack at the top. The helper text states this directly.

---

## 059 – settings-developer-lower.png

**What it is.** The same Developer page scrolled to the bottom (scrollbar value 0.90, so the scrollbar is visible: a 6 pt grey handle at the right edge, about #757779, on a transparent track). The App Version card is clipped under the toolbar; its ghosted value "Bleeding Edge" shows through faintly as content scrolls beneath the translucent toolbar.

**Visible controls (everything from 055 shifted up 167 pt, plus):**

- Rex Installation, Local API, Web Client Origins and their footnote: identical to 055.
- **Experimental** card, all six rows:
  1. "Download new updates when available": "Downloads updates in the background and asks before restarting." Switch ON (**disabled**).
  2. "Install updates automatically": "Relaunches into the new version while Rex is not visible. Sessions keep going." Switch ON.
  3. "Terminal anchoring": "Pin terminal text to the top of each pane, or to the bottom so the last row meets the pane's bottom edge." Popup **"Top"**.
  4. "New quick switcher": "Go to Anything, Go to Session, and View Activity open one list of every session, block, and reporting process, in place of the session switcher and the tab bar's popovers." (2 lines.) Switch **OFF** (value 0; dark track #292D30, white knob left).
  5. "Quick Look links": "The Quick Look shortcut, ⌘Y by default, previews the files and web links on screen. Shift-Command-click or the context menu previews one link." (2 lines.) Switch **ON**.
  6. "Cursor shaders": "Terminal cursors draw a Ghostty-style shader effect that follows focus between panes." Switch **OFF**.
- The card ends about 3 pt above the window's bottom edge. There is no further content below Experimental: this is the end of the page.

**Layout.** Experimental row tops (screen y): 805, 851, 897, 957, 1017, 1077 (from 055 coordinates). Row pitch is 46 for 1-line subtitles and 60 for 2-line subtitles.

**Behaviour implied.**
- New quick switcher: replaces the session switcher and the tab-bar popovers with one unified list (sessions, blocks, reporting processes) for the "Go to Anything", "Go to Session" and "View Activity" commands.
- Quick Look links: ⌘Y (rebindable, "by default") previews all on-screen files and links. Shift-⌘-click or the context menu previews a single link.
- Cursor shaders: Ghostty-style shader cursor animation that follows focus between panes.

---

## 060 – developer-installation-details.png

**What it is.** The same page, scrolled partway (scroll value 0.557), after clicking **"Show Details"** in the Rex Installation card. The link becomes **"Hide Details"** (74×16, accent #0073FF), and three detail rows are inserted inline. The card grows from 83 to 197 pt. The content below shifts down by 114 pt; there is no animation evidence in a still.

**Rex Installation card (expanded), in order:**
1. "Location": "Uses the Rex helper included with this app." Popup "Included with Rex.app".
2. Divider.
3. Green check icon, "Included Rex ready", then the trailing link **"Hide Details"**.
4. Detail rows. There are no dividers between them; the pitch is 38 pt and each has a 24 pt copy button at the trailing edge. The label is left (13 pt, #E1EAF1) and the value is right-aligned before the copy button, in grey #7A8086:
   - "Version": `0.1.0-preview.15` (**proportional** font). Copy button **"Copy Version"**.
   - "Commit": `99caa8e07107e0ec57eeaa868f9f589ccd8bd7cb` (**monospaced**, full 40-hex shown). Copy button **"Copy Commit"**.
   - "Final SHA-256": displayed as `f001dcb6be8e62aeccc0a…df99ec4abc9fb106bda0c` (**monospaced**, **middle-truncated** with an ellipsis). The full value in the tree is `f001dcb6be8e62aeccc0a3ccf0ddb3b656f28410337df99ec4abc9fb106bda0c`. Copy button **"Copy Final SHA-256"** (copies the full value; inferred).

The rest of the page (Local API, Web Client Origins, Experimental) is unchanged. In this scroll position the visible part ends at "New quick switcher".

**Behaviour implied.** It is a provenance and verification panel for the Rex helper in use: semantic version, git commit and the SHA-256 of the final binary. Each value can be copied individually. Show Details and Hide Details toggle the disclosure in place. The status line ("Included Rex ready") reports that the selected helper is present and usable. Other states (missing or failed helper) were not captured.

---

## Feature inventory

- [settings.developer.nav] Settings sidebar "Developer" category (hammer icon, last group), page title "Developer", with Back/Forward history capsule in the toolbar
- [settings.developer.mode] Developer Mode toggle: "Expose settings for development and debugging." (shown ON)
- [settings.developer.update-channel] Update Channel popup: Preview | Bleeding Edge (shown Bleeding Edge; subtitle describes the selected channel, "Every commit that passes CI. Expect some instability.")
- [settings.developer.rex-location] Rex Installation > Location popup: Included with Rex.app | Source Checkout (shown Included with Rex.app; subtitle "Uses the Rex helper included with this app.")
- [settings.developer.rex-status] Rex helper status row with a green check badge: "Included Rex ready"
- [settings.developer.rex-details] Show Details / Hide Details disclosure revealing the helper Version, Commit and Final SHA-256
- [settings.developer.rex-details.copy] Per-field copy buttons: Copy Version, Copy Commit, Copy Final SHA-256 (hashes monospaced; SHA-256 middle-truncated)
- [settings.developer.local-api] Local API URL display (unix socket URL, percent-encoded) with "Copy local Rex endpoint" button
- [settings.developer.web-origins] Web Client Origins allow-list; built-in "https://rex.superlogical.com" marked "Default" (always allowed, not removable)
- [settings.developer.web-origins.add] Add origin text field (placeholder "https://example.com or https://*.example.com") plus "Add Origin" button (disabled while empty); wildcard subdomains supported; changes apply after a Rex restart
- [settings.developer.experimental.auto-download] "Download new updates when available": background download, asks before restarting (shown ON, disabled)
- [settings.developer.experimental.auto-install] "Install updates automatically": relaunches into the new version while Rex is not visible; sessions keep going (shown ON)
- [settings.developer.experimental.terminal-anchoring] Terminal anchoring popup: Top | Bottom (default/shown Top); Bottom aligns the last row to the pane's bottom edge
- [settings.developer.experimental.quick-switcher] "New quick switcher" toggle: unified Go to Anything / Go to Session / View Activity list of sessions, blocks and reporting processes (shown OFF)
- [settings.developer.experimental.quick-look-links] "Quick Look links" toggle: ⌘Y previews on-screen files and web links; Shift-⌘-click or the context menu previews one link (shown ON)
- [settings.developer.experimental.cursor-shaders] "Cursor shaders" toggle: Ghostty-style cursor shader effect that follows focus between panes (shown OFF)
- [settings.ui.popup-style] Settings popups render as right-aligned value text plus a circular up/down chevron badge; menus are check-marked with the selected item placed over the control
- [settings.ui.card-style] Grouped inset cards (#12161A cards on a #0B0F13 pane, ~10–12 pt radius, inset 1 pt dividers #26292D) with a heading above and optional footnote below
