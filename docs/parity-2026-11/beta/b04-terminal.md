# Rex Beta (macOS) — Settings > Terminal: fonts, advanced font, cursor, option key, selection, links

Reference: Rex Beta 0.1.0 (1932), Bleeding Edge, Rex Server 0.1.0-preview.15, captured November 10, 2026 (PST) with agent-desktop. Source: `rex-beta-ui-2026-11-10/screenshots/022`–`037`, accessibility trees in `rex-beta-ui-2026-11-10/evidence/*.json`, full-display captures `*-display.png` where present.

## Conventions used in this file

- **Scale.** Screenshots are 1x (1 px = 1 pt). Window screenshots are 928x962 px and include a ~56/38 px shadow margin; the window itself is **816 x 850 pt** at screen (1104,100). "Window-relative" coordinates below are `(screen x − 1104, screen y − 100)` = `(image x − 56, image y − 38)`.
- **Colours** are sampled sRGB hex from the PNGs (dark appearance). Text colours are the most extreme ink pixel against the local background, so they are near the true fill but anti-aliasing may shade them slightly.
- **(inferred)** marks anything not directly read from a picture or the accessibility (AX) tree.
- The font picker popovers (023, 024, 028) are **not in the AX trees** (separate panel). Their contents come from the pictures only.

## Shared chrome (applies to every full-window screenshot: 022, 023, 024, 025, 028, 033, 037)

**Window.** AX window title `Settings - <page title>`: `Settings - Terminal` on the Terminal page, `Settings - Advanced Font Settings` on the sub-page (025, 028). Borderless-titlebar style: traffic lights sit inside the sidebar top. Large continuous corner radius (~16 pt, inferred from the image), soft black drop shadow (`#000000` at ~23% alpha just outside the edge, fading out over ~40 pt). Close (red), Minimize (grey, AX `disabled`), Zoom (green) at window-relative x 18/41/64, y 18, 16x16 each.

**Sidebar** (window-relative x 0–214; AX `outline` "Settings categories" 215 pt wide; AX `splitter` value `214`, disabled = not user-resizable).
- Background `#0F1217`. Separator to content: hard edge to content `#0B0F13` at x=215 (no visible line, just the tone change).
- **Search field** "Search Settings" (placeholder, `#767C81`), magnifier glyph at left, bg `#202328`, capsule/rounded ~10 pt, about 195 x 28 pt at window-relative (10,64).
- Rows (AX buttons inside cells; cell 195 x 32 pt, button 187 x 24 pt; 32 pt row pitch). Order and grouping, with group gaps of ~13 pt between groups:
  1. **Server** / sub-label **Running** (two-line row, 40 pt cell; AX name `Server, Running`). Icon: round multicolour gradient (orange/pink/blue) disc with a white creature/mascot glyph.
  2. **Interface** (icon: dark rounded square with red/yellow/green dots, like a window) · **Themes** (icon: multicolour gradient rounded square with a palette glyph).
  3. **Terminal** (icon: dark green-black square with a green `>_` prompt glyph) · **Shortcuts** (grey square, white `⌘`) · **Integrations** (purple/pink square, puzzle-piece glyph) · **Notifications** (red-orange square, white bell) · **Remote Hosts** (purple/magenta square, white globe).
  4. **Developer** (blue-grey square, hammer glyph).
- Label text `#E1EAF1`, ~13 pt regular, proportional (SF). Sub-label "Running" `#767C81` ~10 pt.
- **Selected row** (Terminal): rounded-rect fill `#191D22` (~8 pt radius) spanning the 195 pt cell; label switches to semibold. AX value `Selected`.

**Content header** (window-relative y 0–52): a liquid-glass capsule containing **Back** (`‹`, enabled) and **Forward** (`›`, AX disabled, dimmer) buttons, 38 x 36 pt each, separated by a thin vertical divider; capsule ~76 x 36 at window-relative (223,9). Page title to its right, bold ~15 pt `#DADADB` (AX statictext with description `Settings page`): **Terminal** or **Advanced Font Settings**. Content scrolls under the header with a fade/blur scroll-edge effect (in 033/034 the "Cursor style" row under the header renders at ~60% contrast: label `#98A0A6`, value `#949698`).

**Content area.** Background `#0B0F13`. Content column: cards at window-relative x 235–795 (561 pt wide, AX groups 561 wide), i.e. 20 pt left margin from the sidebar edge and 21 pt right margin (scrollbar gutter). Section headings at x 245 (10 pt inset inside the card edge).
- **Section heading**: bold ~13 pt `#E1EAF1`, 16 pt line box, sits 30 pt below the previous card and 10 pt above its card (card-bottom → next-card-top = 56 pt).
- **Card**: fill `#12161A`, no border, continuous corner radius ~12 pt. Rows inset 10 pt left/right (labels at x+10, trailing controls end 10 pt before the card edge).
- **Row heights**: single-line row 38–39 pt; row with helper line 45–46 pt. Inter-row divider: 1 px `#26292D`, inset 10 pt both sides.
- **Row label**: ~13 pt regular `#E1EAF1`. **Helper text** under a label: ~10–11 pt `#7A8086`, 4 pt below the label. **Footnote** text below a card (e.g. under Option key): ~10–11 pt `#7A8086`, 10 pt below the card, wraps at ~516 pt.
- **Switch**: 36 x 16 pt (AX), drawn ~36 x 16; on = track `#0676FC` with white `#FFFFFF` knob at right; off = track `#282C2F` with white knob at left. Inactive window (popover open): on-track becomes grey `#323639`.
- **Popup button (trailing)**: plain value text `#DBDBDC` ~13 pt then a ~18 pt circular badge `#24282C` with a stacked up/down chevron glyph (`⌃⌄`), right-aligned 10 pt from card edge; AX role `combobox`, readonly, 24 pt tall. Inactive window: value `#63686D`, badge `#606469`.
- **Push button** (e.g. Choose..., Add...): capsule, fill `#22262A`, no border, label `#E1EAF1` ~13 pt, 24 pt tall.
- **Preview well** (terminal sample inside a card): inset 6 pt from the card edges, 1 px border `#24282D`, fill `#0F1217` (terminal bg sampled `#0F1216` behind glyphs), corner radius ~8 pt; monospaced text (the chosen terminal font, Menlo 12) in `#E2EAF1`, 10 pt left padding.
- **Scroll bar**: overlay style, AX scrollbar 19 pt gutter, handle 6 x 340 pt; hidden at rest in 022 (value 0), visible in 033 (value 0.5) and 037 (value 1) as a thin grey rounded bar at the right edge.

The Terminal page is a single long scroll view with sections, top to bottom: **Typography, Colors, Cursor, Keyboard, Scrolling, Resizing, Selection, Links, Integrated Path Browser, Titles, Shell**. (Scrolling/Resizing/Titles/Shell/Path Browser are documented here because they appear in 033/037, even though the topic focus is fonts, cursor, option key, selection and links.)

---

## 022 — `022-settings-terminal.png` — Settings > Terminal (top of page)

**What it is.** The Terminal settings page scrolled to the top. Reached by clicking **Terminal** in the Settings sidebar (Settings window opened from the app; how the window was opened is not shown). AX evidence: `evidence/022-settings-terminal.json` (it also contains the off-screen rest of the page, used below to cross-check 033/037).

### Controls, in visual order

**Typography** (heading)

Card 1 (window-relative y 90–258, 169 pt tall):
1. Row **Terminal font** (38 pt): label at left; at right the current font name **`Menlo`** rendered *in the font itself* (monospaced, `#E1EAF1`, AX statictext value `Menlo`), then push button **Choose...** (78 x 24 pt capsule `#22262A`; AX button name `Terminal font`, description `Choose...`). Divider below.
2. A **four-column control strip** (column headers ~11 pt `#767C81`, at y+11 from divider; controls 24 pt below headers). Columns separated by 1 px vertical dividers `#26292D` (at window-relative x ≈ 437, 562, 668), spanning from just under the horizontal divider to the bottom of the controls:
   - **Font style** — full-width popup field (~180 x 23 pt, fill `#22262A`, radius ~6 pt) showing **Regular** with a `⌃⌄` chevron glyph at its right end. AX `menubutton` name `Font style` value `Regular`. (Its menu is shown in 029.)
   - **Font size** — bordered stepper field (~82 x 23 pt, fill `#1C1D20`, 1 px border `#26292D`/`#2B2D30`, radius ~6 pt): leading small-caps glyph **`AA`** (small A + large A, dim grey), value **12** (AX textfield `Font size` = `12`), up/down stepper arrows (AX incrementor), then unit label **pt** (`#7A8086`) outside the field.
   - **Weight** — disabled field (fill `#1C2125`, no visible border), leading dim "≡"-style weight glyph, value **400** in `#5B5F64`. AX textfield name **`Font weight, fixed by the font`**, states `disabled, readonly`. No stepper.
   - **Line height** — bordered stepper field: leading line-spacing glyph (vertical double arrow beside horizontal lines), value **0** (AX textfield `Line height adjustment in percent` = `0`), stepper arrows, unit **%** outside.
3. **Typography preview** well (AX group `Typography preview: regular, bold, and italic terminal text.`; 549 x 58 pt). Three lines in Menlo 12:
   - `~/rex ᚴ main $ swift build && swift test` — the character after `~/rex` is a git-branch glyph (Powerline/Nerd-font branch symbol, U+E0A0 likely — inferred), regular weight.
   - `main.swift:4: warning: 'run() -> Bool' is deprecated` — regular.
   - **`Build complete!`** (bold) followed by *`(1.24 sec)`* (italic).
   All text one colour `#E2EAF1` on `#0F1216`.

Card 2 (38 pt): **Advanced font settings** — full-width navigation row (AX button, 541 pt wide), label left, a dim disclosure chevron `›` (`#45494E`) at the right edge. Opens the Advanced Font Settings sub-page (025). 10 pt gap from card 1.

**Colors** (heading)

Card (3 rows, each 46 pt with helper text, dividers between):
1. **Improve text contrast** — helper *Adjust unreadable text against its background.* — switch **On**.
2. **Force theme palette for text** — helper *Map text colors to the nearest theme color.* — switch **Off**.
3. **Use Display P3 color space** — helper *Read theme and program colors as wide-gamut Display P3.* — switch **Off**.

**Cursor** (heading)

Card (window-relative y 556–715):
1. **Cursor preview** well (AX group `Cursor preview: the selected cursor at a terminal prompt.`, 549 x 30 pt): text `~/rex $ ` in Menlo. In 022 no cursor is drawn (blink "off" phase); in 023/024/031 a **blue block cursor** (`#0071F6`, one cell wide, full line height) follows `$ ` — i.e. the preview blinks live (inferred from the two captures differing).
2. **Cursor style** — popup **Block** (AX combobox, 81 x 24). Options in 030.
3. **Blink speed** — popup **Normal** (AX combobox, 91 x 24). Options in 031.
   - AX tree also has a hidden, `disabled` statictext **`Blink interval`** occupying a 541 x 45 pt row slot between Blink speed and Force style; it is not drawn. Likely the custom-interval row shown only when Blink speed = Custom (inferred).
4. **Force style** — helper *Ignore application requests to change cursor shape.* — switch **Off**.

**Keyboard** (heading)

Card:
1. **Show key sequence hints** — switch **On** (no helper).
2. **Option key** — popup **Automatic (by keyboard layout)** (AX combobox, 237 x 24). Options in 032.

Below the card (off-screen in 022, visible in 033): footnote *Automatic sends Alt on U.S. layouts and types special keys elsewhere. Applies to all terminals on this Mac, including remote hosts.*

### Layout and visual spec (measured)

- Typography card: x 235–795, y 90–258 (window-rel). Terminal font row 38 pt; divider at +38; column-header baseline region 16 pt; control row 23–24 pt; preview well top at +106, height 58, bottom 6 pt above card bottom.
- Advanced row card: 38 pt tall, 10 pt below card 1.
- Colors card: 137 pt (3 x 46 rows minus dividers). Cursor card: preview 30 pt (6 pt inset) + rows 39 / 39 / 45 pt. Keyboard card rows 39 pt.
- Typography: page title ~15 pt bold; section headings ~13 pt bold; row labels ~13 pt regular; column headers ~11 pt regular grey; helper text ~10.5 pt grey; all chrome text proportional (SF Pro); font name and previews monospaced.
- Colours: content bg `#0B0F13`; sidebar `#0F1217`; card `#12161A`; dividers `#26292D`; preview border `#24282D`, fill `#0F1217`; filled controls `#22262A`; stepper field `#1C1D20`; disabled field `#1C2125`; primary text `#E1EAF1`; secondary `#767C81`–`#7A8086`; disabled text `#5B5F64`; accent/switch on `#0676FC`; switch off track `#282C2F`; chevron badge `#24282C`; disclosure chevron `#45494E`.
- No hover state captured.

### Behaviour implied

- Choose... opens a searchable font popover (023/024). Font name shown in its own face.
- Weight is locked ("fixed by the font") for Menlo — presumably editable for variable fonts (inferred).
- Line height is a percent adjustment (0 = font default); font size in points.
- The preview reflects font, style, size, line height live (inferred), and demonstrates regular/bold/italic.
- Colour switches change how terminal text colours are rendered (contrast fix, quantize to theme palette, interpret colours as Display P3).
- Cursor preview shows the chosen style and blink.

---

## 023 — `023-terminal-font-picker.png` — Terminal font picker, Monospaced tab

**What it is.** The popover that opens from **Choose...** on the Terminal font row. Not in the AX tree (the tree still shows only the Settings window, unchanged). The Settings window renders as **inactive** while it is open (switches grey `#323639`, popup values `#63686D`, chevron badges `#606469`), so the picker is a separate key panel/window (inferred).

### Controls

Popover anchored under/left of the Choose... button, overlapping the Terminal font row. Top to bottom:
1. **Search field** — placeholder **Search fonts** (`#767C81`), leading magnifier glyph, blinking blue text caret (`#0073FF`) at the start = field has keyboard focus on open. Fill `#272A2E`, ~278 x 30 pt, fully rounded (capsule).
2. **Segmented control**, two segments: **Monospaced** (selected) | **All Fonts**. Track `#212429`, ~278 x 24 pt capsule; selected thumb `#3E4248` with a 1 px dark outline (`#0D0E10`) and subtle shadow, covering the left half. Labels ~13 pt, selected `#E2E2E3`, unselected `#DDDEDE`.
3. Hairline divider `#202327` across the popover (6 pt below the segmented control).
4. **Font list** (scrolling), rows 28 pt pitch, names in the system UI font (not previewed in their own face), `#E1EAF1` ~13 pt, left inset ~15 pt. Visible rows, in order:
   - Andale Mono
   - Courier New
   - Hack Nerd Font Mono
   - **Menlo** — selected: rounded-rect highlight `#0055BF` (28 pt tall, ~8 pt radius, inset ~4 pt from popover edges), white text, a light check mark `✓` (`#7BA7DE`-ish on blue) right-aligned.
   - MesloLGL Nerd Font Mono
   - MesloLGLDZ Nerd Font Mono
   - MesloLGM Nerd Font Mono
   - MesloLGMDZ Nerd Font Mono
   - MesloLGS Nerd Font Mono
   - MesloLGSDZ Nerd Font Mono
   - Monaco (partially clipped at the bottom — list scrolls)

### Layout and visual spec

- Popover frame: window-relative x 415–702, y 97–468 (~287 x 371 pt). Background `#0F1217` (opaque-looking dark), 1 px outer dark stroke (`#020304`/`#010202`) plus 1 px inner light rim `#272A2E`; corner radius ~14 pt; drop shadow.
- Search field inset ~5 pt from the popover edges; 6 pt gap to the segmented control.
- No scrollbar visible in Monospaced view (overlay scroller, hidden at rest).

### Behaviour implied

- Filter by typing into Search fonts; toggle between monospaced-only and all installed families.
- Current font is selected and marked with a check; clicking a row presumably sets Terminal font (inferred).
- Monospaced list includes user-installed Nerd Fonts, so it is a live system font enumeration filtered by fixed pitch.

---

## 024 — `024-terminal-font-picker-all.png` — Terminal font picker, All Fonts tab

**What it is.** Same popover as 023 after clicking **All Fonts**. The list auto-scrolls so the current font (Menlo) is visible.

### Controls

- Search field **Search fonts** (focused, empty).
- Segmented: Monospaced | **All Fonts** (selected thumb now on the right half).
- List (visible portion, in order): Kokonor, Krungthep, KufiStandardGK, Lao MN, Lao Sangam MN, Lucida Grande, Luminari, Malayalam MN, Malayalam Sangam MN, Marker Felt, **Menlo** (selected, blue `#0055BF` highlight, white text, ✓ at right; it is the bottom visible row).
- **Scroll bar** visible: thin grey rounded thumb at the popover's right edge, about mid-height (window-relative ~(695,340)), i.e. the list is long and scrolled about halfway.

### Layout/visual

Identical metrics and colours to 023. Settings window again rendered inactive.

### Behaviour implied

All Fonts lists every installed family alphabetically, including proportional and non-Latin families (e.g. Kokonor, Lao MN). The picker scrolls the current selection into view on mode switch.

---

## 025 — `025-settings-advanced-font.png` — Advanced Font Settings sub-page

**What it is.** Reached via the **Advanced font settings** row (022). Page title **Advanced Font Settings**; the Back button returns to Terminal (Back enabled, Forward disabled). Sidebar still highlights **Terminal**. AX: `evidence/025-settings-advanced-font.json`.

### Controls, in visual order (no section headings on this page)

Card 1 (window-relative y 64–244, ~181 pt):
1. **Typography preview** well (same three Menlo lines as 022: `~/rex ᚴ main $ swift build && swift test`, `main.swift:4: warning: 'run() -> Bool' is deprecated`, **`Build complete!`** *`(1.24 sec)`*). AX name `Typography preview: regular, bold, and italic terminal text.` 549 x 58 pt.
2. **Character spacing** — trailing: a bordered stepper field (~85 x 20 pt, fill gradient `#2B2D30`–`#313438`, border `#202428`) with leading dim glyph **`Abc`** with a horizontal double-arrow under it (letter-spacing icon), value **0** (AX textfield `Character spacing adjustment in pixels` = `0`), up/down stepper; then a unit popup **px** with chevron badge (AX combobox `Character spacing unit` = `px`, 54 x 24). Options in 026.
3. **Ligatures** — switch **On**.
4. **Render bold and italic** — popup **Both** (AX combobox, 76 x 24). Options in 027.

Card 2 (38 pt): **Font smoothing** — switch **Off**.

Card 3 (45 pt): **Fallback fonts** — helper *Used in order for characters the terminal font doesn't have.* — push button **Add...** (AX name `Add…`, 55 x 24, capsule `#22262A`). No list of fallback fonts is shown (none configured).

### Layout and visual spec

- Card 1 starts 12 pt below the header (y 64). Preview inset 6 pt. Rows: Character spacing 39 pt, Ligatures 39 pt, Render 38 pt, dividers `#26292D`.
- Gaps between the three cards: 10 pt (no headings). Below the last card the page is empty `#0B0F13` to the bottom (no scroll).
- Same colours/typography as the Terminal page.

### Behaviour implied

- Character spacing adds/removes horizontal space per cell, in **px** or **%**.
- Ligatures enable font ligatures in the terminal.
- Render bold and italic decides whether bold and/or italic SGR attributes are drawn with bold/italic faces.
- Font smoothing toggles macOS-style font smoothing (heavier antialiasing) (inferred from name).
- Fallback fonts: an ordered list (first match wins) used when the primary font lacks a glyph; Add... opens a font picker (028).

---

## 026 — `026-menu-character-spacing-unit.png` — Character spacing unit menu

**What it is.** The popup menu of the **px** unit control on Advanced Font Settings (crop 76 x 74 px; full display in `evidence/026-menu-character-spacing-unit-display.png`).

### Options (AX menuitems)

- **px** — selected (✓, blue highlight). Item at screen y 236, the same y as the control, i.e. standard macOS popup that overlays the selected item on the button.
- **%**

### Visual

Native macOS (Tahoe "glass") popup menu: translucent dark material (~`#2E3338`–`#3C4146`, varies with what's behind), ~10 pt outer radius, items 24 pt tall, 60 pt wide; highlighted item fill `#0069FF` with white text and white ✓; unhighlighted text `#E0E0E0`-ish; check column at left.

### Behaviour

Switches the character spacing adjustment unit between pixels and percent.

---

## 027 — `027-menu-font-rendering.png` — Render bold and italic menu

**What it is.** Popup menu of **Render bold and italic** (file name says "font-rendering"). Crop 119 x 144 px; display capture available.

### Options (AX menuitems, in order)

- **Both** (✓ selected, highlighted)
- — separator — (AX disabled menuitem, 11 pt)
- **Bold only**
- **Italic only**
- — separator —
- **Neither**

Menu 103 pt wide, items 24 pt. Same native glass styling as 026.

### Behaviour

Choose which of bold and italic faces are used for styled terminal text; "Neither" renders everything in the regular face (inferred).

---

## 028 — `028-fallback-font-picker.png` — Fallback font picker (Add...)

**What it is.** The font popover opened by **Add...** on the Fallback fonts row of Advanced Font Settings. Not in the AX tree. Settings window rendered inactive (Ligatures switch grey, "Both"/px badges `#606469`).

### Controls

Same component as the terminal font picker (023), anchored under the Add... button and extending down over the empty page (window-relative x ~439–725, y ~314–684):
1. Search field **Search fonts** (focused, caret visible).
2. Segmented **Monospaced | All Fonts** — **All Fonts selected by default** here (differs from the terminal font picker, which opened on Monospaced).
3. Divider, then list:
   - **Academy Engraved LET** — first row highlighted with the same blue `#0055BF` rounded highlight but **no check mark** (keyboard/default highlight, not a current selection).
   - Al Bayan
   - Al Nile
   - Al Tarikh
   - American Typewriter
   - Andale Mono
   - Apple Braille
   - Apple Chancery
   - Apple Color Emoji
   - Apple SD Gothic Neo
   - Apple Symbols (clipped)

### Behaviour implied

Pick any installed family to append to the fallback list; all fonts shown by default because fallbacks are typically symbol/CJK/emoji faces (inferred). Return/click presumably adds the highlighted family (inferred).

---

## 029 — `029-menu-font-style.png` — Font style menu

**What it is.** Menu opened from the **Font style** popup in the Typography card. Crop 192 x 122 px; display capture shows it drops **below** the field (top at screen y 292, field at 265), left-aligned with the field, overlapping the preview well — a pull-down rather than a selected-item-over-button popup.

### Options (AX menuitems)

- **Regular** (✓ — checked but **not** highlighted; no hover)
- **Italic**
- **Bold**
- **Bold Italic**

AX also lists two empty disabled menuitems (an invisible header slot at the Regular position and a 0x0 item), not drawn. Menu ~176 pt wide, items 24 pt, glass material (`#31363B`–`#3C4146` sampled), ~12 pt corner radius.

### Behaviour

Selects the face of the chosen family used for normal text (lists the family's available styles; Menlo has these four — inferred that the list depends on the font).

---

## 030 — `030-menu-cursor-style.png` — Cursor style menu

**What it is.** Popup from **Cursor style** (Cursor card). Crop 120 x 98 px; display capture available.

### Options

- **Block** (✓ selected, highlighted `#0069FF`; overlaid on the button at screen y 700)
- **Bar**
- **Underline**

Menu 104 pt wide, items 24 pt.

### Behaviour

Sets the terminal cursor shape; reflected in the cursor preview well.

---

## 031 — `031-menu-cursor-blink.png` — Blink speed menu

**What it is.** Popup from **Blink speed**. Crop 108 x 168 px. The display capture shows the menu over the Cursor card with the blue block cursor visible in the preview (`~/rex $ █`).

### Options (in order)

- **Off**
- — separator —
- **Slow**
- **Normal** (✓ selected, highlighted; aligned with the button at screen y 739, so the menu extends upward over the Cursor style row)
- **Fast**
- — separator —
- **Custom**

Menu 92 pt wide, items 24 pt, separators 11 pt.

### Behaviour

Off disables blinking; three presets; Custom presumably reveals the hidden **Blink interval** row seen in the AX tree of 022 (inferred).

---

## 032 — `032-menu-option-key.png` — Option key menu

**What it is.** Popup from **Option key** in the Keyboard card. Crop 254 x 146 px; display capture available. Menu opens upward (selected item over the button area; first item at screen y 793 while the button sat at 918 before scrolling — page was scrolled when captured).

### Options (in order)

- **Automatic (by keyboard layout)** (✓ selected, highlighted)
- **Composes characters**
- **Sends Alt**
- **Left sends Alt**
- **Right sends Alt**

Menu 238 pt wide, items 24 pt, no separators.

### Behaviour

Controls whether macOS Option acts as Meta/Alt (escape-prefixed) or composes characters, with per-side variants. Footnote: *Automatic sends Alt on U.S. layouts and types special keys elsewhere. Applies to all terminals on this Mac, including remote hosts.* — this is a per-Mac client setting, not per-host.

---

## 033 — `033-settings-terminal-middle.png` — Settings > Terminal, scrolled to middle

**What it is.** The Terminal page scrolled about halfway (AX scrollbar value 0.5, handle visible at right).

### Controls, in visual order

(Top, faded under the header by the scroll-edge effect) **Cursor style** — **Block**.
- **Blink speed** — **Normal**
- **Force style** — *Ignore application requests to change cursor shape.* — Off

**Keyboard**
- **Show key sequence hints** — On
- **Option key** — **Automatic (by keyboard layout)**
- Footnote: *Automatic sends Alt on U.S. layouts and types special keys elsewhere. Applies to all terminals on this Mac, including remote hosts.*

**Scrolling**
- **Smooth scrolling** — switch Off (single-line row, no inline helper)
- Footnote: *Move through terminal history smoothly with a trackpad or Magic Mouse. Programs that handle scrolling themselves, and scrolling while Find is open, still move by whole lines.*

**Resizing**
- **Smooth terminal resizing** — helper *Reflow text on the frame a pane changes size, without waiting for the server.* — switch On

**Selection**
Card:
1. **Selection preview** well (AX `Selection preview: highlighted terminal text.`, 549 x 30): Menlo text `On branch feature/tabs` where **`feature/tabs`** is drawn selected: highlight fill `#E2EAF1` with text `#0F1216` — an exact fg/bg inversion of the terminal colours (consistent with Foreground/Background = Theme), with small rounded corners on the highlight (Rounded corners = On).
2. **Copy on selection** — Off
3. **Play animation on copy** — On
4. **Foreground** — popup **Theme** (options in 034)
5. **Background** — popup **Theme** (options in 035)
6. **Rounded corners** — On

### Layout/visual

As shared spec. Footnotes sit outside cards (10 pt below), helpers inside rows. Scroll handle visible (grey, ~6 pt wide).

### Behaviour implied

- Copy on selection copies selected text to the clipboard automatically; "Play animation on copy" flashes the selection when copied (inferred from name).
- Selection colours can follow the theme or use the cell's own foreground/background.
- Rounded corners rounds the selection highlight shape.
- Smooth scrolling = pixel scrolling of scrollback; Smooth terminal resizing = client-side reflow before the server replies.

---

## 034 — `034-menu-selection-foreground.png` — Selection Foreground menu

**What it is.** Popup from **Selection > Foreground**. Crop 159 x 98 px; display capture shows the page in the 033 scroll position with the menu over the Foreground row.

### Options

- **Theme** (✓ selected, highlighted)
- **Cell Foreground**
- **Cell Background**

Menu 145 pt wide, items 24 pt.

### Behaviour

Selected-text colour: the theme's selection foreground, or the cell's own foreground or background colour.

---

## 035 — `035-menu-selection-background.png` — Selection Background menu

**What it is.** Popup from **Selection > Background**. Crop 159 x 98 px.

### Options

- **Theme** (✓ selected, highlighted)
- **Cell Foreground**
- **Cell Background**

Same menu geometry as 034 (145 pt wide), one row lower.

### Behaviour

Selection highlight fill: theme selection background, or the cell's foreground or background colour (Cell Foreground on background gives the classic inverse-video selection — inferred).

---

## 036 — `036-menu-link-handling.png` — Links that open apps menu

**What it is.** Popup from **Links > Links that open apps**. Crop 140 x 98 px; display capture available.

### Options

- **Ask First** (✓ selected, highlighted)
- **Always Open**
- **Never Open**

Menu 124 pt wide, items 24 pt.

### Behaviour

Governs clicking non-web links (custom URL schemes like `ssh://`) printed by terminal programs: prompt first, always launch the registered app, or never. Web, mail and file links are not affected (see 037 footnote).

---

## 037 — `037-settings-terminal-lower.png` — Settings > Terminal, scrolled to bottom

**What it is.** The Terminal page scrolled to the end (AX scrollbar value 1).

### Controls, in visual order

(The **Selection** heading is scrolled under the header and nearly invisible.)

**Selection** card: preview `On branch feature/tabs` (selected `feature/tabs`), **Copy on selection** Off, **Play animation on copy** On, **Foreground** Theme, **Background** Theme, **Rounded corners** On.

**Links**
- **Links that open apps** — popup **Ask First**
- Footnote: *Web, mail, and file links open directly. Other links, such as ssh://, launch the app registered for them, and any program in a terminal can print one.*

**Integrated Path Browser** — heading followed by a **shortcut badge** **`⌃⌘I`** (AX role unknown, 32 x 18 pt): rounded-rect keycap outline in grey (`#71777D`-ish), small grey glyphs, 8 pt right of the heading text.
- **Show recently used paths** — On
- Footnote: *Insert Path types the path of a file or folder at the cursor. Recently used paths are shared by every session on the same host.*

**Titles**
- **Prefer generated titles** — On
- Footnote: *Show the running program and its directory instead of titles that shells set. Titles that programs set for themselves, and names you give tabs, still appear.*

**Shell**
- **Startup command** — single-line text field (AX textfield, 386 pt wide) filling the rest of the row; placeholder **`Default login shell`** in monospaced `#5B5F64`; field fill `#1C1D20`, 1 px border, radius ~5 pt, ~22 pt tall.
- Footnote: *Executable and arguments. Use quotes to group arguments. Applies to new terminals on this Mac. Remote hosts use their own default shell. New tabs and splits use the source terminal’s working directory when available. Otherwise, they use the server’s default directory.*

### Layout/visual

Every single-setting section uses the same pattern: heading → one-row card → grey footnote. Bottom padding below the last footnote ~20 pt. Scroll handle at bottom of gutter.

### Behaviour implied

- Path browser: ⌃⌘I inserts a file/folder path at the cursor; recents are per host and shared across sessions.
- Titles: generated titles (program + cwd) override shell-set titles.
- Startup command overrides the local login shell for new local terminals only.

---

## Feature inventory

- [settings.nav.terminal] Settings sidebar category "Terminal" (icon `>_`), one long scrolling page with sections Typography, Colors, Cursor, Keyboard, Scrolling, Resizing, Selection, Links, Integrated Path Browser, Titles, Shell
- [settings.nav.back-forward] Back/Forward history buttons in the settings header; page title shows current page (Terminal / Advanced Font Settings)
- [settings.search] "Search Settings" field in the settings sidebar
- [settings.terminal.font] Terminal font: current family shown in its own face (default Menlo) + "Choose..." button
- [settings.terminal.font-picker] Font picker popover: "Search fonts" field (auto-focused), Monospaced | All Fonts segmented filter (opens on Monospaced for the terminal font), alphabetical family list, current font highlighted with check mark, scrolls selection into view; includes user-installed Nerd Fonts
- [settings.terminal.font-style] Font style popup: Regular | Italic | Bold | Bold Italic (per-family faces; default Regular)
- [settings.terminal.font-size] Font size stepper field in pt (default 12)
- [settings.terminal.font-weight] Weight field (400), disabled with label "Font weight, fixed by the font" when the font has no variable weight
- [settings.terminal.line-height] Line height adjustment stepper in % (default 0)
- [settings.terminal.typography-preview] Live typography preview (regular, bold, italic sample lines with a git-branch glyph)
- [settings.terminal.advanced-font] "Advanced font settings" navigation row to a sub-page
- [settings.terminal.char-spacing] Character spacing adjustment stepper (default 0) with unit popup px | % (default px)
- [settings.terminal.ligatures] Ligatures switch (default On)
- [settings.terminal.render-bold-italic] Render bold and italic: Both | Bold only | Italic only | Neither (default Both)
- [settings.terminal.font-smoothing] Font smoothing switch (default Off)
- [settings.terminal.fallback-fonts] Fallback fonts: ordered list "Used in order for characters the terminal font doesn't have." with "Add..." button opening the font picker (defaults to All Fonts, no checkmark)
- [settings.terminal.contrast] Improve text contrast switch — "Adjust unreadable text against its background." (default On)
- [settings.terminal.force-palette] Force theme palette for text — "Map text colors to the nearest theme color." (default Off)
- [settings.terminal.display-p3] Use Display P3 color space — "Read theme and program colors as wide-gamut Display P3." (default Off)
- [settings.terminal.cursor-preview] Live cursor preview at a `~/rex $` prompt, blinking per setting
- [settings.terminal.cursor-style] Cursor style: Block | Bar | Underline (default Block)
- [settings.terminal.cursor-blink] Blink speed: Off | Slow | Normal | Fast | Custom (default Normal)
- [settings.terminal.cursor-blink-interval] Custom blink interval row (present but hidden/disabled in AX; shown for Custom — inferred)
- [settings.terminal.cursor-force-style] Force style — "Ignore application requests to change cursor shape." (default Off)
- [settings.terminal.key-sequence-hints] Show key sequence hints switch (default On)
- [settings.terminal.option-key] Option key: Automatic (by keyboard layout) | Composes characters | Sends Alt | Left sends Alt | Right sends Alt (default Automatic); applies to all terminals on this Mac including remote hosts
- [settings.terminal.smooth-scrolling] Smooth scrolling switch for trackpad/Magic Mouse scrollback (default Off); apps that handle scrolling and Find still scroll by lines
- [settings.terminal.smooth-resizing] Smooth terminal resizing — client-side reflow on resize frames without waiting for the server (default On)
- [settings.terminal.selection-preview] Selection preview showing highlighted text with current colours/corners
- [settings.terminal.copy-on-select] Copy on selection switch (default Off)
- [settings.terminal.copy-animation] Play animation on copy switch (default On)
- [settings.terminal.selection-fg] Selection foreground: Theme | Cell Foreground | Cell Background (default Theme)
- [settings.terminal.selection-bg] Selection background: Theme | Cell Foreground | Cell Background (default Theme)
- [settings.terminal.selection-rounded] Rounded corners for selection highlight (default On)
- [settings.terminal.links-open-apps] Links that open apps (non-web schemes such as ssh://): Ask First | Always Open | Never Open (default Ask First); web, mail and file links open directly
- [settings.terminal.path-browser] Integrated Path Browser (shortcut ⌃⌘I, shown as keycap badge): Insert Path types a file/folder path at the cursor
- [settings.terminal.path-recents] Show recently used paths (default On); recents shared by all sessions on the same host
- [settings.terminal.generated-titles] Prefer generated titles (program + directory over shell-set titles; program-set titles and user tab names still win) (default On)
- [settings.terminal.startup-command] Shell startup command text field (placeholder "Default login shell"); local terminals only, quotes group arguments; new tabs/splits inherit source cwd else server default directory
- [settings.ui.inactive-dimming] Settings window draws inactive state (grey switches/badges) while a font picker popover is key
- [settings.ui.scroll-edge] Content scrolls under a glass header with a fade/blur scroll-edge effect; overlay scrollbar
