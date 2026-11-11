# B03: Themes (Settings > Themes, light and dark theme editors, extended palette, theme switcher)

Reference product: Rex Beta 0.1.0 (1932) for macOS, Bleeding Edge channel, Rex Server 0.1.0-preview.15. Captured on November 10 and 11, 2026 (PST).

Sources: `rex-beta-ui-2026-11-10/screenshots/{018,019,020,021,129}-*.png`, the accessibility trees in `rex-beta-ui-2026-11-10/evidence/` with the same names, and `COVERAGE.md`. The `evidence/138-theme-extended-color-cube.json` tree exists with no PNG. It was checked and adds nothing beyond 020; it has no swatch nodes for the colour cube.

Conventions:
- Sizes are in points from the accessibility tree. The screenshots are 1x (1 px = 1 pt). Each one has a 56 pt shadow margin left and right, a 38 pt margin at the top and a 74 pt margin at the bottom. Settings window origin: image (56,38) = screen (1104,100). Main window origin in 129: image (56,38) = screen (368,168).
- Colours are sampled sRGB hex from the PNGs. Values in the colour wells come from the tree. Swatch pixels can differ slightly from the tree values (colour management, possibly Display P3).
- Corner radii are estimated from anti-aliasing and are approximate.
- (inferred) marks anything not directly visible or in the tree.

---

## 018-settings-themes.png: Settings > Themes page

### What it is
This is the Themes category of the Settings window, with window title "Settings - Themes". Click **Themes** in the settings sidebar to reach it. It is the second item of the second sidebar group, under Interface. The window is in the dark appearance. Dark Mode is the active appearance.

A transient hover tooltip covers part of the "Light Mode" heading. It is a dark rounded bubble with an arrow pointing down. It reads two key/value rows:
- `Server Version` `0.1.0-preview.15`
- `API Version` `Revision 13`

The tooltip belongs to some hovered element above it (inferred: the toolbar or the Server item). It is not part of the Themes page.

### Window chrome and sidebar (shared settings shell)
- Window: 816 × 850 pt, dark, rounded corners of about 10 pt (inferred from the macOS default) with the standard large window shadow. Traffic lights at top left: Close (red), Minimize (grey, **disabled**), Zoom (green). Each is 16 pt at y 118.
- Toolbar: a **Back** / **Forward** capsule (38 × 36 pt each). Forward is **disabled**. Chevrons `<` and `>` sit in one dark capsule (#292E33 to #393F44) with a thin vertical separator between them. The title **Themes** follows (statictext, description "Settings page") in semibold at about 15 pt, colour #D1D1D2.
- Sidebar: 215 pt wide (splitter value "214"), background #0F1217. The content background is #0B0F13. The boundary between them is a hard colour change with no visible line.
  - Search field "Search Settings": a capsule about 28 pt tall, background #202328, magnifier glyph #8C9298, placeholder #767C81.
  - **Server** with subtitle **Running**: an app-icon-style rounded avatar with a multicolour gradient (orange/pink/blue) and a white creature glyph. The row is 40 pt tall. The title is #E1EAF1 and the subtitle is small grey #767C81.
  - Group 2: **Interface** (dark tile with three coloured dots, like traffic lights) and **Themes** (rainbow gradient tile with a white artist-palette glyph). **Themes is selected**: a full-width rounded highlight (#191D22, radius about 6 pt) and a bold label.
  - Group 3: **Terminal** (dark tile with a green `>_` prompt), **Shortcuts** (grey tile with ⌘), **Integrations** (purple/pink tile with a puzzle-piece glyph), **Notifications** (red tile with a bell glyph), **Remote Hosts** (purple/magenta tile with a globe glyph).
  - Group 4: **Developer** (light blue-grey tile with a hammer glyph).
  - Rows are 32 pt pitch. Buttons are 24 pt tall. Icon tiles are about 20 pt rounded squares. Labels are about 13 pt regular, #E1EAF1. Groups are separated by about 13 pt extra vertical space, with no divider lines.

### Content, in visual order
The content column starts at x 1349, which is 30 pt from the sidebar edge. Group cards span x 1339 to 1900, a width of 561 pt, with a 20 pt right margin.

1. Section heading **Light Mode** (bold about 13 pt, #E1EAF1). At the far right is the **New** button, a link-style button with a ⊕ (circled plus) glyph followed by "New", in accent blue #006FF8. Its accessibility name is "New Light Theme".
2. Grouped card (background #12161A on page #0B0F13, radius about 10 pt, no border) containing:
   - A horizontally scrolling strip (scrollarea "Light theme", 561 × 123 pt) with two labelled sub-columns:
     - Sub-heading **Default** (small, grey #767C81) above one card: **Merino Light**. It is **selected**: label bold and white (#E1EAF1), with a blue focus ring around the card.
     - A short vertical hairline separator (#393D40), about 40 pt tall.
     - Sub-heading **Built-in** above cards (each "Not selected"; the label is regular weight, grey #767B80): **Alpine Milk**, **Buttercream Diner**, **Coastal Linen**, **Glacier Mint** (clipped at the right edge as "Glaci…"), then offscreen **Ivory Gallery**, **Lavender Postcard**, **Newsprint Fog**, **Orchard Breeze**, **Peach Veranda**, **Porcelain Morning**, **Silver Point Light**, **Sunlit Parchment**, **Terracotta Noon**. That makes 13 built-in light themes plus the 1 default.
     - No scroll indicator is visible. The strip clips the fifth card mid-way, which signals horizontal scrolling.
   - A full-width hairline divider (#26292D) inside the card.
   - Row **Use extended theme palette** with a **toggle**, currently **on** (blue #0675FC, white knob, 36 × 16 pt). Label #E1EAF1, regular about 13 pt.
3. Section heading **Dark Mode** followed by the dim suffix **— Active** (#777D83, regular). The accessibility name is "Dark Mode, — Active". At right is **⊕ New** (accessibility name "New Dark Theme").
4. Grouped card with the same structure:
   - Scrollarea "Dark theme":
     - **Default**: **Merino Dark** (**selected**, blue ring, bold white label).
     - Separator.
     - **Built-in**: **Abyssal Trench**, **Basalt Shore**, **Cathode Amber**, **Charcoal Atelier** (clipped as "Charco…"), then offscreen **Cinder Peak**, **Cobalt Foundry**, **Ember Hollow**, **Heliotrope Haze**, **Ink Meridian**, **Juniper Smoke**, **Midnight Tundra**, **Misty Forest**, **Moss Cathedral**, **Neon Arcade**, **Nocturne Rose**, **Oxblood Library**, **Petrol Lagoon**, **Phosphor Archive**, **Polar Aurora**, **Roasted Umber**, **Saffron Nightfall**, **Silver Point Dark**, **Sonoran Dusk**, **Static Noir**, **Velvet Dusk**, **Whitby Bay**. That makes 29 built-in dark themes plus the 1 default.
   - Divider, then **Use extended theme palette**, a **toggle**, **on**. This is a separate toggle per appearance.
5. Section heading **Import Theme**.
6. Grouped card with one row:
   - Title **Import a Ghostty or iTerm theme file** (#E1EAF1).
   - Helper text below in small grey (#767C81, about 11 pt): **Themes are matched to an appearance based on their background color.**
   - At right, the push button **Import** (60 × 24 pt, capsule, background #2C3034, label #E1EAF1).

### Theme card (thumbnail) spec
- The card is 112 × 88 pt. The theme name sits below the card, centred, at about 11 pt. Card pitch is 120 pt (8 pt gap). The Default column is 137 pt from the first Built-in card, because the separator sits between them.
- Card background = the theme's terminal background. Merino Light is #FFFFFF; other light cards are #F9F9F9 or tinted (for example Buttercream is cream, Coastal Linen is warm off-white). Merino Dark is #0F1117 and Abyssal Trench is #08121A. Dark cards have a faint 1 px border slightly lighter than their background. Light cards have a soft light edge (#E7E8E8). Radius is about 12 pt.
- Top half: there is a faintly raised inner panel (a slightly different shade from the card background). It holds:
  - **"Aa"** in the theme's text colour, proportional, about 20 pt light weight.
  - A **"~/"** chip: a rounded rectangle (radius about 4 pt) filled with the theme's selection colour, with "~/" in the selected-text colour. Merino Light shows a dark chip (#1C2023) with white text. Merino Dark shows a light chip (#E2E9F1) with dark text. This matches the editor's Selection and Selected Text defaults.
  - A **vertical pill** (about 12 × 28 pt, radius about 5 pt) filled with the cursor colour, with a short underline glyph "_" near its bottom in the cursor-text colour. Merino Light is blue #0050F4 and Merino Dark is #006FFE.
- Bottom row: **8 overlapping circular dots**, each about 14 pt in diameter at a 12 pt pitch. Each dot overlaps the previous one with a thin background-coloured stroke, which gives a coin-stack look. They show ANSI 0 to 7 (inferred from the colour match). Merino Light's dots sample as #1F252A, #D60022, #00600E, #9C4D00, #004FF4, #7237E1, #00757E and #626C76. These match Black, Red, Green, Yellow, Blue, Magenta, Cyan and White in the editor.
- Selection treatment: a 2 to 3 px accent ring (#004FF4 core with a #0E2450 anti-aliased edge), offset about 4 pt outside the card with a gap that shows the group background. The selected name is bold and white. Unselected names are grey, regular weight.
- There is no hover state in these captures.

### Colours (dark appearance)
| Element | Hex |
| --- | --- |
| Window/content background | #0B0F13 |
| Sidebar background | #0F1217 |
| Sidebar selected row | #191D22 |
| Group card | #12161A |
| In-card divider | #26292D |
| Default/Built-in separator | #393D40 |
| Primary text | #E1EAF1 |
| Secondary text (sub-headings, helper, unselected names, "— Active") | #767C81 to #777D83 |
| Accent (New, toggle, ring) | #006FF8 / #0675FC / #004FF4 |
| Search field | #202328 |
| Import button | #2C3034 |
| Tooltip bubble | #2F3439, key text #8A9096, value text #E1EAF1 |

### Behaviour implied
- Choosing a card selects the theme used when the system is in that appearance. Light and dark have independent selections. "— Active" marks which appearance currently applies.
- **New** (one per appearance) opens the theme editor sheet for a new light or dark theme (see 019 and 021).
- **Use extended theme palette** turns the generated 256-colour extended palette on or off per appearance (inferred from the editor's Extended Palette section).
- **Import** opens a file picker for Ghostty or iTerm theme files (inferred). The imported theme is filed under Light or Dark according to its background colour.
- The cards are buttons with the value "Selected" or "Not selected", which implies single selection per strip.
- No custom (user) theme column appears because none had been saved. COVERAGE.md says "No ... themes saved". So the UI for a user theme section is not captured.

---

## 019-new-light-theme-editor.png: New light theme editor sheet

### What it is
A modal **sheet** attached to the Settings window (role sheet, 614 × 704 pt). It opens from **Light Mode > ⊕ New** on the Themes page. The sheet renders in the **light** appearance even though the window behind is dark (the sheet follows the appearance of the theme being edited). While the sheet is up, the Settings window behind shows its inactive state: accents turn grey (the "New" link turns #2F2F2F) and colours are slightly desaturated.

### Controls, in visual order
**Header** (about 64 pt tall, separated from the body by a full-width hairline #E5E6E6):
- Left: an icon about 34 pt wide. It is an outlined artist's palette with a filled black circled "+" badge at the top right, drawn in near-black.
- **Text field** (accessibility name "New Theme Name"), value **Untitled Light Theme**, **focused**. The text is large and bold (about 17 pt semibold, #1C2024) and the caret is accent blue. The field is 250 × 20 pt of text with a rounded focus ring (#81A7F6, about 3 pt) on a white field.
- Right: a non-interactive **badge** (statictext) **Light Theme** with a small sun glyph ☼. It is a capsule (background #E6E7E7, text #636669, about 11 pt), 95 × 23 pt.

**Body** (scrolls vertically; background #F8F8F8):
1. Row: heading **Colors** (bold, about 13 pt) on the left. At right, the label **Preview** with a **toggle** that is **on** (#004CEA).
2. Grouped card (background #F1F1F1, radius about 10 pt, inset 20 pt from the sheet edges) containing three sub-sections:
   - Sub-heading **Terminal** (semibold, about 12 pt), then six **colour wells** in three visual pairs. The pairs are separated by a 20 pt gap; wells within a pair are 6 pt apart. Each well is about 83 pt wide. A small grey column label (#86888A, about 11 pt) sits above each well:
     | Label | Value (tree) | Hex field |
     | --- | --- | --- |
     | Background | rgb 1 1 1 | `#FFFFFF` |
     | Text | rgb 0.1216 0.1373 0.1569 | `#1F2328` |
     | Cursor | rgb 0.0353 0.4118 0.8549 | `#0969DA` |
     | Cursor Text | rgb 0.7137 0.8471 1 | `#B6D8FF` |
     | Selection | rgb 0.1216 0.1373 0.1569 | `#1F2328` |
     | Selected Text | rgb 1 1 1 | `#FFFFFF` |
   - Sub-heading **ANSI Standard**. Column labels: **Black**, **Red**, **Green**, **Yellow**, **Blue**, **Magenta**, **Cyan**, **White**. Below are 8 wells (64 pt wide, 70 pt pitch):
     | Well (a11y name) | Hex |
     | --- | --- |
     | Black #0 | `#24292F` |
     | Red #1 | `#CF222E` |
     | Green #2 | `#116329` |
     | Yellow #3 | `#9D5C00` |
     | Blue #4 | `#0969DA` |
     | Magenta #5 | `#8250DF` |
     | Cyan #6 | `#1B7C83` |
     | White #7 | `#6E7781` |
   - Sub-heading **ANSI Bright**. Same column labels, then 8 wells:
     | Well | Hex |
     | --- | --- |
     | Bright Black #8 | `#57606A` |
     | Bright Red #9 | `#A40E26` |
     | Bright Green #10 | `#1A7F37` |
     | Bright Yellow #11 | `#C47A00` |
     | Bright Blue #12 | `#218BFF` |
     | Bright Magenta #13 | `#A475F9` |
     | Bright Cyan #14 | `#3192AA` |
     | Bright White #15 | `#8C959F` |
3. Row: heading **Extended Palette** (bold) with a **toggle** at right, **on**.
4. Grouped card containing:
   - **Automatically regenerate palette** (regular, about 13 pt), with helper **Changes to the base theme colors and the ANSI Standard palette update the extended palette.** (grey, about 10 to 11 pt). **Toggle**, **on**.
   - Hairline divider (#E2E2E3).
   - **Reverse palette**, with helper **Orders generated colors from the background to the text, rather than dark to light.** **Toggle**, **on**.
   - Hairline divider.
   - A full-width centred button row **Show extended palette** (semibold, near-black, 554 × 35 pt). Its accessibility value is "Collapsed", so it works as a disclosure.

**Footer** (separated by a hairline #E5E6E6, background #F8F8F8):
- Left: **Revert…**, **disabled**. It is a capsule 80 × 30 pt, with a pale blue tint background (#DFE7F8) and washed-out blue text (#6F9BF6).
- Right: **Cancel**, a capsule 71 × 30 pt with a light blue tint background (#D3DFF7 to #E1E9F8) and accent blue text (#004FF4).
- **Save & Apply**: a primary capsule 110 × 30 pt, solid accent #004FF4, with white semibold text. This is the default button (inferred).

### Colour well spec
- Each well is one rounded rectangle (radius about 5 to 6 pt) with a 1 px border (#CBCBCB in light). It is split horizontally: the top part (about 22 pt) is the solid swatch, and the bottom part (about 20 pt) is a white **hex text field**. A hairline (#EBEBEB) separates the two.
- The hex is uppercase with a leading `#`, set in a **monospaced** font at about 11 pt and centred (#1C2024). Each hex field is its own accessible text field ("Hex value for Background", and so on), so hex is directly editable. Clicking the swatch presumably opens the macOS colour panel (inferred; role colorwell).

### Layout and visual spec
- Sheet: 614 × 704 pt, radius about 16 pt, white-ish (#F8F8F8) with a drop shadow over the dimmed window. It is centred horizontally on the Settings window, with its top about 73 pt below the window top.
- Body horizontal inset: headings at sheet x + 30. Cards at about sheet x + 20, width about 574 pt. Card inner padding is about 10 pt.
- Vertical rhythm: section heading about 16 pt, then 20 pt to the card. Sub-heading, column labels and well rows are spaced by about 104 pt per ANSI block (label row at +23, wells at +45).
- Typography: proportional system font (SF Pro) everywhere except the hex values, which are monospaced.
- Colours (light): sheet #F8F8F8, card #F1F1F1, dividers #E2E2E3 to #E5E6E6, primary text #1C2024, secondary text #86888A, accent #004FF4, toggle #004CEA.

### Behaviour implied
- The new theme is seeded with the default palette. Its values look like the Merino Light / GitHub-light family (inferred from the match with Merino Light's card dots).
- **Preview** on: edits apply live as a preview (inferred: to terminals, and possibly to the sheet itself, which renders in the theme's appearance).
- Each base colour can be edited through the colour picker or the hex field.
- **Extended Palette** on: the theme carries a generated extended palette (the 256-colour cube and grey ramp, see 020).
- **Automatically regenerate palette**: edits to the base Terminal colours and ANSI Standard (not ANSI Bright, per the helper text) regenerate the extended palette.
- **Reverse palette**: generated ramps run from background to text instead of dark to light. Since this is a light theme, that flips the ordering (inferred).
- **Revert…** is disabled until there are changes. The ellipsis implies a confirmation (inferred).
- **Cancel** dismisses without saving. **Save & Apply** saves the theme and makes it the selected theme for its appearance (inferred from the label).
- The appearance (Light or Dark) is fixed by the New button you used, and is shown as a badge rather than as a choice.

---

## 020-theme-extended-palette.png: Light editor with the extended palette expanded

### What it is
The same sheet as 019, after clicking **Show extended palette**. All values are unchanged.

### Differences from 019
- The disclosure button now reads **Hide extended palette** (accessibility value "Expanded").
- Below it are a new divider and the sub-heading **Color Cube** (semibold, about 12 pt). They sit inside the same grey card, at the bottom edge of the visible body. The content below is clipped by the sheet footer, so the body scrolls.
- From the tree only (offscreen, not visible):
  - **Color Cube** section: heading at y 794, about 347 pt tall before the next heading. It probably holds a 6×6×6 grid of 216 swatches (palette indices 16 to 231) (inferred). The tree exposes no child nodes for the swatches.
  - **Grayscale Ramp** heading at y 1141 (offscreen). Inferred to hold indices 232 to 255.
  - **Reset Overrides** button, 134 × 24 pt, **disabled** and offscreen, at y 1200. This implies that individual generated colours can be overridden by hand and later reset (inferred).
- COVERAGE.md confirms the gap: "the extended theme color cube and grayscale ramp are clipped below the viewport in 020". The swatch grid itself was never captured.
- The footer is unchanged: **Revert…** (disabled), **Cancel**, **Save & Apply**.

### Behaviour implied
- The extended palette is shown and hidden on demand inside the editor and is collapsed by default.
- Swatches for generated colours are shown (and are probably overridable). **Reset Overrides** returns them to the generated values.

---

## 021-new-dark-theme-editor.png: New dark theme editor sheet

### What it is
The same editor, opened from **Dark Mode > ⊕ New**. The sheet renders in the **dark** appearance. It is 614 × 658 pt, which is 46 pt shorter than the light sheet because the Reverse palette row is absent. Its top sits about 96 pt below the window top, so it remains centred.

### Controls (differences from the light editor)
- Header: the text field "New Theme Name" holds **Untitled Dark Theme** (focused). The focus ring is dark blue (#063F86 to #164F96). The field background is #1B1E23 and the text is #E1EAF1.
- Badge: **Dark Theme** with a crescent moon glyph. Capsule background #1C2125, text #767C84, 92 × 23 pt.
- **Colors** / **Preview** toggle **on** (#0676FD).
- **Terminal** defaults:
  | Label | Hex |
  | --- | --- |
  | Background | `#0D1117` |
  | Text | `#E6EDF3` |
  | Cursor | `#2F81F7` |
  | Cursor Text | `#EFF8FF` |
  | Selection | `#E6EDF3` |
  | Selected Text | `#0D1117` |
- **ANSI Standard**: Black #0 `#484F58`, Red #1 `#FF7B72`, Green #2 `#3FB950`, Yellow #3 `#D29922`, Blue #4 `#58A6FF`, Magenta #5 `#BC8CFF`, Cyan #6 `#39C5CF`, White #7 `#B1BAC4`.
- **ANSI Bright**: Bright Black #8 `#6E7681`, Bright Red #9 `#FFA198`, Bright Green #10 `#56D364`, Bright Yellow #11 `#E3B341`, Bright Blue #12 `#79C0FF`, Bright Magenta #13 `#D2A8FF`, Bright Cyan #14 `#56D4DD`, Bright White #15 `#FFFFFF`.
- **Extended Palette** toggle **on**.
- Card: **Automatically regenerate palette** (same helper text), toggle **on**. **There is no "Reverse palette" row in the dark editor.** The tree confirms it is absent.
- **Hide extended palette** (value "Expanded"). The expanded state carried over from 020. The **Color Cube** sub-heading is visible at the bottom. The tree also has the **Grayscale Ramp** heading, offscreen at y 1118, and **Reset Overrides**, offscreen and disabled, at y 1177. So the dark editor has the same extended-palette sections as the light one.
- Footer: **Revert…** (disabled; background #0A192B, text #054697), **Cancel** (background #0A1829, text #0073FF), **Save & Apply** (solid #006FFE, white text).

### Visual spec (dark)
| Element | Hex |
| --- | --- |
| Sheet background | #0B0F13 |
| Header and footer dividers | #22262B |
| Card | #12161A |
| Card row divider | #252A2F |
| Well border | #3A3D40 / #2D3036 |
| Hex field background | #1B1E23 |
| Hex text and primary text | #E1EAF1 |
| Column labels and helper text | #7A8086 |

The sheet has a subtle light 1 px outer edge against the dimmed window.

### Behaviour implied
- The new dark theme is seeded with GitHub-dark-like defaults (inferred). The Selection default is the inverse pair (light selection, dark selected text).
- The reverse-ordering option exists only for light themes (inferred, because the row is missing here). By default, dark themes already run from background (dark) to text (light).

---

## 129-theme-switcher.png: Theme switcher popover (Change Theme…)

### What it is
A searchable theme picker overlaid at the top centre of the main terminal window. The window is titled "shuv@shuvbot:~ - lunar-basin" and is 1302 × 760 pt, with one tab, `shuv@shuvbot:~`. It is reached from **View > Customize View > Change Theme…** (that menu item exists in the 074 View menu tree). The Shortcuts settings also list **Change Theme…** with no default shortcut ("Record a shortcut for Change Theme…"). Which of these two routes opened it for this capture is not recorded (inferred). The popover is **not** in the accessibility tree. The tree only shows the window, tabs, pane header and terminal.

### Controls, in visual order
- Search field, placeholder **Select Theme…**, with a magnifier glyph (#8D8D8D). It is a rounded rectangle (radius about 8 pt) filling the popover width minus about 6 pt insets, about 30 pt tall, background #333437, placeholder #8F8F91.
- Below is a vertical list of theme names, one per row, about 28 pt row height, proportional about 13 pt, text #F3F3F3:
  1. **Merino Dark**: **selected/highlighted**. It has a full-row accent background (#0055BF, radius about 6 pt, inset about 5 pt from the sides), white text and a **✓** checkmark at the right edge (the current theme).
  2. Abyssal Trench
  3. Basalt Shore
  4. Cathode Amber
  5. Charcoal Atelier
  6. Cinder Peak
  7. Cobalt Foundry
  8. Ember Hollow
  9. Heliotrope Haze
  10. Ink Meridian
  11. Juniper Smoke
  12. Midnight Tundra
  13. Misty Forest
  14. Moss Cathedral
  15. Neon Arcade
  16. Nocturne Rose (the last visible row; more continue below)
- A vertical overlay scrollbar thumb at the right edge (6 pt wide, #99999A, rounded) spans about the top 60% of the list track. That fits the 30 dark themes in 018 (inferred). The visible names follow the 018 dark order exactly: Default first, then Built-in alphabetically.
- Only dark themes are listed, which matches the active Dark appearance (inferred). Light themes are not shown in the visible portion.

### Layout and visual spec
- The popover is about 331 × 500 pt, centred horizontally over the window content, with its top just below the tab bar (about 77 pt below the window top).
- Background #1C1D20. A double edge: a 1 px outer near-black line (#020203) and a 1 px inner light line (#333436, then #212224). Radius about 12 pt. A soft shadow (inferred; it sits over a dark terminal).
- No section headers, previews or swatches. It is a plain text list.
- Context visible behind it: a tab bar with the session chip "lunar-basin" (folder/stack glyph), the active tab `shuv@shuvbot:~` (a pill-shaped tab, #2A2C30, with a terminal `>_` glyph), and right-side toolbar glyphs: an eye (Activity), ⌘ (Command Palette) and + (New Tab). The pane header shows `>_ shuv@shuvbot:~` with dim split, float, zoom and close glyphs. The terminal background is #0F1216 and shows a login banner and prompt.

### Behaviour implied
- Type to filter themes by name (inferred from the "Select Theme…" search field). Arrow keys move the highlight and Return applies it (inferred, standard picker). The checkmark marks the currently applied theme.
- This is a quick switcher for the theme of the current appearance, without opening Settings.

---

## Feature inventory
- [settings.themes.page] Settings > Themes category (sidebar item with a rainbow palette icon), with the page title "Themes" and Back/Forward navigation
- [settings.themes.light.section] "Light Mode" section listing light themes as a horizontally scrolling strip of preview cards
- [settings.themes.dark.section] "Dark Mode" section listing dark themes the same way
- [settings.themes.active-indicator] "— Active" suffix on the heading of the appearance currently in effect (Dark in the capture)
- [settings.themes.light.select] Select the light-appearance theme: single selection, default Merino Light
- [settings.themes.dark.select] Select the dark-appearance theme: single selection, default Merino Dark
- [settings.themes.groups] Cards grouped under "Default" and "Built-in" sub-headings with a vertical separator
- [settings.themes.builtin.light] 13 built-in light themes: Alpine Milk, Buttercream Diner, Coastal Linen, Glacier Mint, Ivory Gallery, Lavender Postcard, Newsprint Fog, Orchard Breeze, Peach Veranda, Porcelain Morning, Silver Point Light, Sunlit Parchment, Terracotta Noon (plus default Merino Light)
- [settings.themes.builtin.dark] 29 built-in dark themes: Abyssal Trench, Basalt Shore, Cathode Amber, Charcoal Atelier, Cinder Peak, Cobalt Foundry, Ember Hollow, Heliotrope Haze, Ink Meridian, Juniper Smoke, Midnight Tundra, Misty Forest, Moss Cathedral, Neon Arcade, Nocturne Rose, Oxblood Library, Petrol Lagoon, Phosphor Archive, Polar Aurora, Roasted Umber, Saffron Nightfall, Silver Point Dark, Sonoran Dusk, Static Noir, Velvet Dusk, Whitby Bay (plus default Merino Dark)
- [settings.themes.card-preview] Theme card preview: background fill, "Aa" in text colour, "~/" chip in selection/selected-text colours, cursor pill with cursor-text underscore, and 8 overlapping ANSI 0 to 7 dots. The name appears below; the selected card gets an accent ring and a bold name
- [settings.themes.light.extended-palette] "Use extended theme palette" toggle for light mode (default on)
- [settings.themes.dark.extended-palette] "Use extended theme palette" toggle for dark mode (default on)
- [settings.themes.new-light] "⊕ New" button in Light Mode opens the new light theme editor
- [settings.themes.new-dark] "⊕ New" button in Dark Mode opens the new dark theme editor
- [settings.themes.import] "Import Theme": import a Ghostty or iTerm theme file via the "Import" button. The theme is assigned to light or dark by its background colour
- [theme-editor.sheet] Theme editor as a modal sheet over Settings, rendered in the edited theme's appearance (light sheet for light themes, dark sheet for dark themes)
- [theme-editor.name] Editable theme name field, focused on open (default "Untitled Light Theme" / "Untitled Dark Theme")
- [theme-editor.appearance-badge] Read-only appearance badge: "☼ Light Theme" or "☾ Dark Theme"
- [theme-editor.preview] "Preview" toggle (default on), presumably live preview of edits
- [theme-editor.terminal-colors] Terminal colours: Background, Text, Cursor, Cursor Text, Selection, Selected Text. Each is a colour well plus an editable monospaced hex field
- [theme-editor.ansi-standard] ANSI Standard colours 0 to 7 (Black, Red, Green, Yellow, Blue, Magenta, Cyan, White), each a well plus a hex field
- [theme-editor.ansi-bright] ANSI Bright colours 8 to 15, each a well plus a hex field
- [theme-editor.defaults.light] New light theme defaults: bg #FFFFFF, text #1F2328, cursor #0969DA, cursor text #B6D8FF, selection #1F2328, selected text #FFFFFF; ANSI #24292F #CF222E #116329 #9D5C00 #0969DA #8250DF #1B7C83 #6E7781; bright #57606A #A40E26 #1A7F37 #C47A00 #218BFF #A475F9 #3192AA #8C959F
- [theme-editor.defaults.dark] New dark theme defaults: bg #0D1117, text #E6EDF3, cursor #2F81F7, cursor text #EFF8FF, selection #E6EDF3, selected text #0D1117; ANSI #484F58 #FF7B72 #3FB950 #D29922 #58A6FF #BC8CFF #39C5CF #B1BAC4; bright #6E7681 #FFA198 #56D364 #E3B341 #79C0FF #D2A8FF #56D4DD #FFFFFF
- [theme-editor.extended-palette] "Extended Palette" per-theme toggle (default on)
- [theme-editor.extended.auto-regenerate] "Automatically regenerate palette" toggle (default on): base and ANSI Standard edits regenerate the extended palette
- [theme-editor.extended.reverse] "Reverse palette" toggle (default on, light editor only): order generated colours from background to text instead of dark to light
- [theme-editor.extended.disclosure] "Show extended palette" / "Hide extended palette" disclosure row (collapsed by default)
- [theme-editor.extended.color-cube] "Color Cube" section of generated colours (probably the 216-colour cube; contents never captured)
- [theme-editor.extended.grayscale-ramp] "Grayscale Ramp" section (contents never captured)
- [theme-editor.extended.reset-overrides] "Reset Overrides" button (disabled when nothing is overridden), which implies hand overrides of generated colours
- [theme-editor.revert] "Revert…" button (disabled until edited)
- [theme-editor.cancel] "Cancel" button closes without saving
- [theme-editor.save-apply] "Save & Apply" primary button saves the theme and applies it
- [view.change-theme] "Change Theme…" command (View > Customize View; rebindable in Shortcuts, no default key) opens the theme switcher
- [theme-switcher.popover] Searchable theme switcher popover over the terminal window: "Select Theme…" search field and a scrolling name list
- [theme-switcher.current-check] The current theme is highlighted with an accent row background and a ✓
- [theme-switcher.appearance-scope] The list shows the themes of the active appearance (dark list when in Dark mode; inferred), in Default-then-Built-in alphabetical order
