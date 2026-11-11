# B02: Rex Beta Settings > Interface (with Tabs and Panes sub-pages and their menus)

Reference: Rex Beta 0.1.0 (1932), Bleeding Edge, macOS, connected to Rex Server 0.1.0-preview.15 (API Revision 13). The screenshots were captured on November 10, 2026 (Pacific Time) with agent-desktop. Source: `rex-beta-ui-2026-11-10/screenshots/007`–`017` and the matching accessibility trees in `rex-beta-ui-2026-11-10/evidence/`.

Units: the window screenshots are at 1x, so 1 image px = 1 pt. Each one is 928x962 px: the 816x850 pt window plus about 56/38 px of drop shadow on the left and top edges. Image coordinate = screen pt − (1048, 62). Accessibility bounds below are screen points.

Colours were sampled from the PNGs with ImageMagick. Text colours are the brightest pixel in the glyph area, so anti-aliasing can make them slightly lighter than the true fill.

## Shared chrome (applies to 007, 008, 011, 016)

**Window.** "Settings - Interface", "Settings - Tabs" or "Settings - Panes" (AX window title), 816x850 pt, dark mode. The outer corners have a large radius (about 16–20 pt, by eye) and a soft drop shadow.
- Traffic lights at the top left: Close (red), Minimize (grey, **disabled**) and Zoom (green), each 16 pt, at y≈118. Minimize is disabled in every capture.
- There is no separate titlebar strip. The sidebar runs to the top of the window (full-height sidebar, unified title area).

**Sidebar.** 214 pt wide. The AX splitter has value `214` and is disabled, so its width is fixed. Background `#0F1217`, slightly lighter than the content area. A 1 pt splitter sits at x=1318.
- **Search field:** placeholder `Search Settings` with a magnifying-glass glyph. It is a capsule (radius about 14) with fill `#202328` and placeholder text `#767C81`, at sidebar x+10, y≈102–130 (image), and about 194 pt wide.
- **Category list** (AX `outline` "Settings categories"). Each row is 32 pt tall and holds a 24 pt button. Groups are separated by about 13 pt gaps. Each row has a 20 pt rounded-square colour icon followed by a label in `#E1EAF1` (regular weight, about 13 pt).
  1. **Server**, with the subtitle **Running** in smaller grey (`#767C81`, about 10 pt). The icon is a round, multicolour gradient app-icon glyph. This row is 40 pt tall with a 32 pt button. AX name: "Server, Running".
  2. (group) **Interface**: a dark rounded-square icon with red/yellow/green dots, like window traffic lights. **Selected**: its row has a rounded (about 8 pt) highlight `#222324` across the sidebar width minus about 10 pt insets, and its label is semibold.
     **Themes**: a rainbow/gradient painter's-palette icon.
  3. (group) **Terminal**: a black icon with a green `>_` prompt. **Shortcuts**: a grey icon with the `⌘` glyph. **Integrations**: a pink/orange/purple puzzle-piece icon. **Notifications**: a red/orange icon with a white bell. **Remote Hosts**: a purple/magenta icon with a white globe.
  4. (group) **Developer**: a grey-blue icon with a white hammer.

**Content header.** At the top left of the content pane there is a capsule nav control, 76x36 pt, fill `#292E34`. It holds Back (`<`, enabled) and Forward (`>`, **disabled**, dimmed), separated by a thin vertical divider. To its right is the page title (AX statictext, description "Settings page") in bold, about 17 pt: "Interface", "Tabs" or "Panes".

**Persistent tooltip artefact.** Every capture shows a dark, translucent callout (fill about `#2D3237`, rounded about 10 pt, with a downward tail). It covers the page title and the first section heading. It reads:
- `Server Version` (grey, right-aligned label) `0.1.0-preview.15` (white)
- `API Version` (grey) `Revision 13` (white)

The callout is not in the AX tree. It looks like a hover tooltip left over from the capture pointer (inferred); its anchor is not identifiable. It hides the headings "Appearance", "Tab behavior", "Padding" and "Toolbar" partly; the AX tree supplies the full text.

**Content area.** Background `#0B0F13`.
- Grouped "inset cards": fill `#12161A`, corner radius 12 pt (measured from the corner curve). They are 561 pt wide, with a 20 pt margin from the content's left edge (x=1339) and 20 pt to the right edge, beside the scrollbar gutter.
- Section headings sit outside the cards, about 10 pt in from the card edge, in semibold `#E1EAF1` at about 13 pt.
- Row labels are regular, about 13 pt, `#E1EAF1`. Helper text is about 11 pt, `#767C81`, directly under the label.
- Row dividers are 1 pt `#26292D`, inset 10 pt from both card edges.
- A row with a plain control is 39 pt high (38 + 1 divider). A row with a segmented control is 44 pt. A single-row card is 38–44 pt; label + helper rows are 44–45 pt. Card-to-card spacing across a section heading is 56 pt: 38 pt from the card's bottom to the heading text, then the heading, then the card.
- The scrollbar is an overlay: a thin grey (`#7F8183`) capsule thumb about 6 pt wide, shown only while scrolling (visible in 016, hidden in 007).

**Shared control styles.**
- **Toggle (switch):** 36x16 pt track hit area. On: track `#0676FE`, knob white. Off: track `#292D30`, knob white on the left. The knob is a horizontal pill about 21 pt wide (macOS 26 style). The control is right-aligned, ending 10 pt from the card edge.
- **Segmented control (AX tablist of radiobuttons):** a capsule 30 pt tall, track `#24282C`. The selected segment is a capsule 28 pt tall with fill `#41464B` and white text. Unselected text is `#E1EAF1` on the track. Segments can have a leading monochrome glyph.
- **Popup button (AX combobox, readonly):** the current value in `#DBDBDC`, right-aligned and borderless. After it is a 20 pt circular button (`#24282C`) holding an up/down chevron (`⌃⌄`). It opens a native macOS pop-up menu.
- **Disclosure/navigation row (AX button spanning the row):** the label on the left and a small grey `>` chevron (`#45494E`) at the right edge.
- **Native pop-up menus (009–017):** fill `#292E33` with a translucent/vibrant blur, about 8 pt corner radius, a dark 1 px outer border (`#101418`) and a soft shadow. Items are 24 pt tall. The highlighted item is a full-width rounded rectangle in `#0069FF` with white text. The current value shows a `✓` checkmark at the left, and every item is indented to leave room for it. When the menu opens, the checked item sits directly over the control (standard NSPopUpButton positioning, confirmed by the AX bounds: checked item y = combobox y).

---

## 007-settings-interface.png: Settings > Interface (top of page)

**What it is.** The main Interface page of Settings, scrolled to the top. Reached by choosing **Interface** in the Settings sidebar. Back is enabled, which suggests the user navigated here from another page (inferred).

**Controls in visual order.**

1. Section heading **Appearance** (partly hidden by the tooltip). The heading text comes from AX.
   - One card, about 176 pt tall, with two side-by-side groups separated by a vertical 1 pt divider:
     - **Color scheme** group. Its grey caption `Color scheme` (`#767C81`, about 12 pt) sits inside the card. It holds three picture tiles, each 92x92 pt including its caption. A tile is a rounded rectangle (about 12 pt radius) about 92x70 pt, filled with a blue abstract macOS-wallpaper image (`#003977` range). On it sits a miniature window thumbnail with traffic-light dots and a two-pane layout:
       - **Auto**: **selected**. The thumbnail is split diagonally, white on the left and black on the right. The caption is bold `#E1EAF1`.
       - **Light**: a white thumbnail. Caption grey `#767C81`. AX value "Not selected".
       - **Dark**: a black thumbnail. Caption grey.
     - **Interface style** group, with caption `Interface style`:
       - **Classic**: **selected**. The tile shows a solid dark (near-black) circle on the wallpaper. The caption is bold white.
       - **Glass**: the tile shows a dark circle with a lighter, translucent-looking fill and a subtle rim (a glass material). The caption is grey.
     - Selection treatment: a 2–3 pt `#0073FF` ring around the tile, with a dark gap of about 2 pt between the ring and the tile.
   - A divider, then the row **Tint with theme** with a toggle that is **on**.
2. Section heading **Toolbar**. Card with 3 rows:
   - **Toolbar layout**, a segmented control (294x30 pt) with 3 segments, each with a glyph:
     - `Above`: glyph is a window outline with a top bar. **Selected**.
     - `Left Side`: glyph is a window with a left sidebar strip.
     - `Right Side`: glyph is a window with a right sidebar strip.
   - **Toolbar behavior**, a popup with value `Always show`. Options are in 014.
   - **Customize tabs**, a disclosure row with `>`. It navigates to the Tabs sub-page (008).
3. Section heading **Panes**. Card with 3 rows:
   - **Pane style**, a segmented control (224x30):
     - `Comfortable`: glyph is a 2x2 grid of separated, rounded squares. **Selected**.
     - `Compact`: glyph is a 2x2 grid with shared borders (a single box quartered).
   - **Pane header behavior**, a popup with value `Always show`. Options are in 015.
   - **Customize panes**, a disclosure row `>`. It navigates to the Panes sub-page (011).
4. Section heading **Gestures**. Card with 1 row:
   - **Use trackpad gestures**, with helper text `Pinch to zoom panes and swipe to toggle the sidebar.` The toggle is **on**.
5. Section heading **Animation**. Card with 1 row:
   - **Animation style**, a segmented control (175x30) with text only: `Snappy` (**selected**), `Natural`, `Off`.
6. (Offscreen in this capture; the AX tree confirms it; shown in 016) the **App Icon** section and the **Show window on launch** card.

**Layout and visual spec.**
- The first card spans y 128–304 (image).
- The Toolbar card spans 360–482: a 44 pt segmented row, then two 39 pt rows.
- The Panes card spans 538–660, built the same way.
- The Gestures card is 716–760, 44 pt tall.
- The Animation card is 816–860.
- Segmented controls and popups are right-aligned to x=1890 (10 pt inside the card).

**Behaviour implied.**
- Color scheme picks Auto (follow the system), Light or Dark for the app chrome.
- Interface style picks the Classic (opaque) or Glass (translucent "Liquid Glass") material.
- Tint with theme tints the chrome with the active terminal theme's colours (inferred from the label).
- Toolbar layout places the tab/toolbar strip above the content or as a vertical sidebar on the left or right.
- Pane style switches pane spacing and density.
- Trackpad gestures: pinch zooms a pane, and swipe toggles the sidebar.
- Animation style controls the chrome motion curve, or turns animation off.

---

## 008-settings-customize-tabs.png: Settings > Interface > Customize tabs ("Tabs" page)

**What it is.** A sub-page pushed in place of the Interface page. The title is `Tabs` and the AX window title is "Settings - Tabs". Back is enabled and Forward is disabled. Reached through **Customize tabs >** on the Interface page. The sidebar still highlights **Interface**.

**Controls in visual order.**

1. Section heading **Tab behavior** (partly hidden by the tooltip; AX text). Single-row card:
   - **Tabs fill available space**: a toggle that is **off**. The track is `#292D30` with the knob on the left.
2. Section heading **Tab icons**. Card containing:
   - **Tab icons preview** (AX group, 549x54 pt). This is a live preview box inset inside the card. It is a rounded rectangle (about 10 pt) with a 1 pt border slightly lighter than the card, filled with the content background `#0B0F13`. It shows two sample tabs as capsules:
     - **Feature work**: the selected/active sample, 144x30 pt. Fill `#2A2C30`, with a faint lighter border and white text.
     - **Code review**: inactive, 121x24 pt. Fill `#1C1D20`, with grey text.
     - Each tab shows two leading icons. The first is a small dark rounded square with a green `>_` terminal prompt (the shell icon). The second is a multicolour (pink/orange/blue) stacked-layers glyph, possibly the app/program icon (inferred).
   - **Show tab icons**: a slider (AX slider, value `3`), 160 pt wide. The filled part is blue `#176EDA`, the rest of the track grey `#5B5F64`. The thumb is a white vertical capsule (10x18). There are **6 tick marks** below the track, so the range is probably 0–5 (inferred from the tick count and the thumb at 60%). The current value `3` is printed in grey to the right of the slider.
   - **Use color for tab icons**: a popup with value `Always`. Options are in 009.
   - **Shell icon theme**: a popup with value `Default`. Options are in 010.

**Layout.**
- Tab behavior card: image y≈128–166, 38 pt tall.
- Tab icons heading at y≈204.
- Tab icons card: y≈222–398. Its parts are the preview inset (with about 6 pt card padding around it), then three rows of about 39 pt with dividers.
- The rest of the page is empty `#0B0F13`.

**Behaviour implied.**
- With "Tabs fill available space" on, tabs stretch to share the tab strip's width; off, they size to their content (inferred).
- "Show tab icons" sets how many icons a tab can show. The value is a count (3), and the preview shows up to 2 icons per tab (inferred: probably shell, program and other status icons).
- "Use color for tab icons" controls when the icons are drawn in colour rather than monochrome.
- "Shell icon theme" chooses how the shell icon is coloured.
- The preview box updates live (inferred).

---

## 009-menu-tab-icon-color.png: "Use color for tab icons" pop-up menu

**What it is.** A 159x122 px crop of the open native pop-up menu for **Use color for tab icons**, on the Tabs page (008). The full display is at `evidence/009-menu-tab-icon-color-display.png`.

**Items (in order).** Each is 145x24 pt.
1. `✓ Always`: **checked**, with a blue `#0069FF` highlight.
2. `For active panes`
3. `For active tabs`
4. `Never`

There are no shortcuts, separators or disabled items.

**Behaviour.**
- Always: icons are always in colour.
- For active panes / For active tabs: colour only for the active pane's or tab's icons, and monochrome elsewhere (inferred).
- Never: always monochrome.

---

## 010-menu-shell-icon-theme.png: "Shell icon theme" pop-up menu

**What it is.** A 169x98 px crop of the open pop-up for **Shell icon theme** on the Tabs page.

**Items.** Each is 155x24 pt.
1. `✓ Default`: **checked** and highlighted blue.
2. `Use theme`
3. `Use theme accent`

**Behaviour.**
- Default: the built-in shell icon colours (the green `>_` on dark).
- Use theme: colours taken from the terminal theme.
- Use theme accent: the theme's accent colour (inferred from the labels).

---

## 011-settings-customize-panes.png: Settings > Interface > Customize panes ("Panes" page)

**What it is.** A sub-page with title `Panes` (AX window "Settings - Panes"). Reached through **Customize panes >** on the Interface page. The sidebar still highlights Interface. Back is enabled and Forward is disabled.

**Controls in visual order.**

1. Section heading **Padding** (partly hidden by the tooltip). Card with 3 rows:
   - **Minimum size**: two compact stepper fields side by side, right-aligned. Each is a rounded rectangle (about 6 pt radius) with a subtle border and a dark fill, about 66x24 pt:
     - The first has a leading grey `↔` (horizontal double arrow) glyph, the value `10`, and a stacked up/down stepper (`⌃⌄`). AX: textfield "Horizontal padding in points" = `10`, plus incrementor.
     - The second has a leading grey `↕` glyph, the value `6`, and a stepper. AX: textfield "Vertical padding in points" = `6`, plus incrementor.
   - **Fill with program colors**, with helper text `Extends background colors into the padding area.` It is a popup with value `Automatic`. Options are in 012.
   - **Include pane header**, with helper text `Extends the background color into the pane headers.` The toggle is **on**.
2. Section heading **Active Pane**. Two separate cards:
   - Card A: **Highlight active pane edges**, a toggle that is **off**.
   - Card B, which has two rows:
     - **Fade inactive panes**: a popup with value `Surface & Content`. Options are in 013.
     - **Fade panes for inactive windows**: a toggle that is **on**.
3. Section heading **Splits & Focus**. Card:
   - **Autobalance splits**: a toggle that is **on**.
   - **Focus follows mouse**: a toggle that is **off**.

**Layout.**
- Padding card: image y≈128–258, with rows of about 39, 45 and 45 pt (the rows with helper text are taller).
- Active Pane heading at y≈296.
- Card A: 314–352.
- Card B starts 10 pt below Card A: 362–438.
- Splits & Focus heading at 477.
- Its card: 495–572.

All values sit right-aligned at x=1890 (screen).

**Behaviour implied.**
- Minimum size sets the minimum pane padding in points, horizontal (10) and vertical (6).
- Fill with program colors paints the padding with the program's background colour (Off / Automatic / Always). Include pane header carries that colour into the pane header.
- Highlight active pane edges draws an accent edge on the focused pane.
- Fade inactive panes dims unfocused panes: the surface, the content, or both. Fade panes for inactive windows dims panes when the app window loses focus.
- Autobalance splits evens out split sizes automatically. Focus follows mouse focuses a pane on hover.

---

## 012-menu-pane-fill.png: "Fill with program colors" pop-up menu

**What it is.** A 126x98 px crop of the pop-up for **Fill with program colors** (Panes page). The display capture shows the menu opening over the row, with **Automatic** aligned to the control.

**Items.** Each is 110x24 pt.
1. `Off`
2. `✓ Automatic`: **checked** and highlighted.
3. `Always`

**Behaviour.**
- Off: the padding uses the default background.
- Automatic: fill when the program sets a background (inferred, possibly with a heuristic).
- Always: always fill.

---

## 013-menu-pane-fade.png: "Fade inactive panes" pop-up menu

**What it is.** A 176x122 px crop of the pop-up for **Fade inactive panes** (Panes page).

**Items.** Each is 160x24 pt.
1. `Off`
2. `Surface`
3. `Content`
4. `✓ Surface & Content`: **checked** and highlighted. It is the last item, because the menu opens upward so the checked item lands on the control.

**Behaviour.** Which layers of an inactive pane are dimmed: none, the pane background/chrome, the terminal text, or both.

---

## 014-menu-toolbar-behavior.png: "Toolbar behavior" pop-up menu

**What it is.** A 151x98 px crop of the pop-up for **Toolbar behavior** (Interface page, Toolbar card).

**Items.** Each is 135x24 pt.
1. `✓ Always show`: **checked** and highlighted.
2. `Show on hover`
3. `Hidden`

**Behaviour.** The toolbar (tab strip) is always visible, revealed when the pointer hovers its edge (inferred), or hidden.

---

## 015-menu-pane-header-behavior.png: "Pane header behavior" pop-up menu

**What it is.** A 151x98 px crop of the pop-up for **Pane header behavior** (Interface page, Panes card). It looks the same as 014.

**Items.** Each is 135x24 pt.
1. `✓ Always show`: **checked** and highlighted.
2. `Show on hover`
3. `Hidden`

**Behaviour.** The per-pane header bar is always shown, shown on hover, or hidden.

---

## 016-settings-interface-lower.png: Settings > Interface (scrolled to bottom)

**What it is.** The same Interface page scrolled down by about 227 pt; the AX scrollbar value is 0.90. The Toolbar, Panes, Gestures and Animation sections from 007 are visible again with identical values. The overlay scrollbar thumb is visible at the right edge, about 6 pt wide, `#7F8183`.

**New controls (below Animation).**

1. Section heading **App Icon**. Card (AX group, about 561x156):
   - **App icon** picker (AX group "App icon", 531x91). It holds three wallpaper tiles in the same style as the Appearance tiles: a blue abstract background with about 12 pt radius.
     - **Default — Dynamic**: **selected** (blue `#0073FF` ring). This is a wide tile, 254x91 including its caption. It shows three app icon variants side by side:
       1. A vivid pink/purple/orange gradient icon.
       2. A dark/black icon with an orange-red creature.
       3. A translucent/frosted "clear" icon.

       Each is a rounded-square icon of a stylised creature head with a spiky crest (the Rex mascot), with a small `BETA` pill at the bottom. The caption has `Default` in bold white, then ` — Dynamic` in grey.
     - **Bright**: 126x91. One icon with a white background, an orange/pink creature and a `BETA` pill. Grey caption `Bright`.
     - **Twilight**: 127x91. One icon with a dark purple background, an orange/purple creature and a `BETA` pill. Grey caption `Twilight`.
   - Divider, then **Show app icon**: a popup with value `Dock and menu bar`. Options are in 017.
2. A separate single-row card: **Show window on launch**, with a toggle that is **on**. This card has no section heading.

**Layout.**
- App Icon heading at image y≈671.
- Card: 689–852. The tile row is 704–795 including captions, and the Show app icon row is centred at 826.
- The Show window on launch card starts at about 856 and is clipped by the window bottom.

**Behaviour implied.**
- App icon chooses the Dock/app icon variant. "Default — Dynamic" appears to follow the system appearance (light, dark or clear) and switch between the three icons shown (inferred from "Dynamic" and the three variants).
- Show app icon sets where the app appears (Dock, menu bar extra, or both).
- Show window on launch opens a window at launch. If it is off, the app probably starts as a background/menu-bar-only app (inferred).

---

## 017-menu-show-app-icon.png: "Show app icon" pop-up menu

**What it is.** A 182x98 px crop of the pop-up for **Show app icon** on the Interface page, with the page scrolled to the very bottom (see `evidence/017-menu-show-app-icon-display.png`). In the display capture, the Rex Beta app icon (pink gradient with `BETA`) is visible in the macOS Dock, and the Rex Beta menu bar is active.

**Items.** Each is 166x24 pt.
1. `✓ Dock and menu bar`: **checked** and highlighted.
2. `Dock only`
3. `Menu bar only`: AX state "offscreen", because the menu extends past the window or screen edge region. It is still visible and enabled in the image.

**Behaviour.**
- Dock and menu bar: a regular app with a Dock icon, plus a menu bar extra (a status item).
- Dock only: no menu bar extra.
- Menu bar only: an accessory app with no Dock icon (inferred from standard macOS semantics).

---

## Notes and uncertainties

- The tooltip callout (Server Version / API Version) hides the first section heading on every page. The heading text comes from AX: "Appearance", "Tab behavior", "Padding" and "Toolbar" (016).
- The slider range for Show tab icons (0–5) is inferred from its 6 tick marks; the AX tree gives only the value `3`.
- The meaning of the two icons in the tab preview (terminal prompt and multicolour stacked glyph) is inferred.
- Font sizes are estimated from text heights; all text is the proportional system font (SF Pro). No monospace appears on these pages.

## Feature inventory

- [settings.nav.sidebar] Settings sidebar categories: Server (status subtitle "Running"), Interface, Themes, Terminal, Shortcuts, Integrations, Notifications, Remote Hosts, Developer. Each has a coloured icon; the list is grouped with gaps.
- [settings.nav.search] Settings search field "Search Settings" in the sidebar.
- [settings.nav.back-forward] Back and Forward capsule buttons with in-place navigation to sub-pages (Customize tabs, Customize panes); the page title updates.
- [settings.nav.server-version-tooltip] A tooltip/callout showing Server Version (0.1.0-preview.15) and API Version (Revision 13).
- [settings.interface.color-scheme] Color scheme: Auto | Light | Dark, as picture tiles (default Auto).
- [settings.interface.interface-style] Interface style: Classic | Glass, as picture tiles (default Classic).
- [settings.interface.tint-with-theme] Tint with theme: toggle (on).
- [settings.interface.toolbar-layout] Toolbar layout: Above | Left Side | Right Side, as a segmented control with glyphs (Above).
- [settings.interface.toolbar-behavior] Toolbar behavior: Always show | Show on hover | Hidden (Always show).
- [settings.interface.customize-tabs] Customize tabs: a disclosure row that opens the Tabs sub-page.
- [settings.interface.pane-style] Pane style: Comfortable | Compact, as a segmented control with glyphs (Comfortable).
- [settings.interface.pane-header-behavior] Pane header behavior: Always show | Show on hover | Hidden (Always show).
- [settings.interface.customize-panes] Customize panes: a disclosure row that opens the Panes sub-page.
- [settings.interface.trackpad-gestures] Use trackpad gestures: toggle (on). Helper text: "Pinch to zoom panes and swipe to toggle the sidebar."
- [settings.interface.animation-style] Animation style: Snappy | Natural | Off (Snappy).
- [settings.interface.app-icon] App icon: Default — Dynamic (three variants that follow the appearance) | Bright | Twilight, as picture tiles (Default).
- [settings.interface.show-app-icon] Show app icon: Dock and menu bar | Dock only | Menu bar only (Dock and menu bar).
- [settings.interface.show-window-on-launch] Show window on launch: toggle (on).
- [settings.tabs.fill-available-space] Tabs fill available space: toggle (off).
- [settings.tabs.icons-preview] Tab icons preview: two sample tabs ("Feature work" active, "Code review" inactive) that reflect the tab-icon settings.
- [settings.tabs.show-tab-icons] Show tab icons: slider with 6 ticks (range probably 0–5) and a numeric readout (3).
- [settings.tabs.icon-color] Use color for tab icons: Always | For active panes | For active tabs | Never (Always).
- [settings.tabs.shell-icon-theme] Shell icon theme: Default | Use theme | Use theme accent (Default).
- [settings.panes.padding-min-size] Minimum size: horizontal padding stepper (10 pt) and vertical padding stepper (6 pt), as numeric fields with up/down steppers.
- [settings.panes.fill-program-colors] Fill with program colors: Off | Automatic | Always (Automatic). Helper text: "Extends background colors into the padding area."
- [settings.panes.include-pane-header] Include pane header: toggle (on). Helper text: "Extends the background color into the pane headers."
- [settings.panes.highlight-active-edges] Highlight active pane edges: toggle (off).
- [settings.panes.fade-inactive] Fade inactive panes: Off | Surface | Content | Surface & Content (Surface & Content).
- [settings.panes.fade-inactive-windows] Fade panes for inactive windows: toggle (on).
- [settings.panes.autobalance-splits] Autobalance splits: toggle (on).
- [settings.panes.focus-follows-mouse] Focus follows mouse: toggle (off).
- [ui.settings.card-style] Grouped inset cards: `#12161A` on `#0B0F13`, 12 pt radius, 1 pt `#26292D` dividers inset 10 pt, rows 39–45 pt.
- [ui.controls.popup] Pop-up buttons: borderless value plus a circular up/down chevron button. Native menus put a checkmark on the current item and open with it over the control.
- [ui.controls.segmented] Capsule segmented controls: track `#24282C`, selected `#41464B`, 30 pt tall, optional leading glyphs.
- [ui.controls.toggle] Switches: on `#0676FE`, off `#292D30`, white pill knob, 36x16 pt.
