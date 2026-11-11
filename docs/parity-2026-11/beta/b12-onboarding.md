# B12. Rex Beta onboarding: welcome, appearance, permissions, system permissions, migration and finish

Source: Rex Beta 0.1.0 (1932), Bleeding Edge, macOS, with Rex Server 0.1.0-preview.15. Captured on November 10, 2026 (PST). Screenshots are `rex-beta-ui-2026-11-10/screenshots/132` to `137`, and the accessibility trees are in `rex-beta-ui-2026-11-10/evidence/132` to `137` `.json`.

COVERAGE.md says the capture used a temporary onboarding session that was closed afterwards. It also says no permissions were granted and no themes were saved, so every value below is the untouched default.

## Common frame (applies to all six)

**Geometry and scale.** In the AX tree the window is 1302 x 760 pt at screen (368,168). Screenshots 132, 133, 134, 136 and 137 are 1414 x 872 px, 1 px per pt. They include the macOS window shadow, and the window sits at image offset (56,37). To map AX points to image pixels, use `img = (x - 312, y - 131)`. Screenshot 135 is cropped to the window only (1302 x 760), so AX points map with `img = (x - 368, y - 168)`. The window has about 10 pt corner radius and a large, soft macOS drop shadow.

**Onboarding is an ordinary Rex session and tab, not a separate window or modal.** Window title (AX): `Welcome, Kyle - Onboarding`.

- **Title / tab bar** (AX group `Tabs`, 1302 x 46 pt, bg `#0B0F13`, no separate titlebar):
  - Traffic lights at the far left: Close, Minimize and Full Screen, each 16 pt (system red, yellow and green).
  - **Session button** (AX `Change Session`, button, 126 x 34): a stacked-box/archive outline glyph, then **`Onboarding`** in semibold, grey-white (`#A2A9AF`). It shows the current session name and opens the session switcher (inferred from the label).
  - **Tab** (AX radiobutton `Welcome, Kyle`, state selected, 242 x 32): a rounded pill with bg `#2A2C30` and a hairline lighter border, radius about 10. It holds a small gradient Rex app icon (pink, orange and blue, about 20 pt) and the label `Welcome, Kyle` in regular weight, text `#E1EAF1`. This is the only tab.
  - Right side: an eye glyph (AX `Activity`, value `Off`, 36 x 32), a ⌘ glyph (AX `Command Palette`, 36 x 30) and `+` (AX `New Tab`, 36 x 32). The glyphs are light grey and have no background.
- **Pane** (AX group `Split layout` > group `Get Started with Rex`, which has state `focused` in every capture except 135): 1286 x 706 pt, inset 8 pt from the window sides. It has bg `#0F1217`, a 1 px border of about `#22252A` at the top edge, and about 10 pt corner radius.
  - **Pane header row** (about 22 pt tall, no separate bg): at the left, a small grey Rex dino-head glyph and the title **`Get Started with Rex`** in about 13 pt medium, `#E1EAF1`. AX exposes it as button `Move Get Started with Rex pane`, a drag handle 155 x 16. At the right are five 22 x 22 icon buttons, drawn as outline glyphs:
    - `Split Right` (rectangle divided vertically, enabled, mid grey)
    - `Split Down` (rectangle divided horizontally, enabled)
    - `Float Pane` (two overlapping rectangles, **disabled**, very faint)
    - `Zoom Pane` (diagonal double arrow, **disabled**, faint)
    - `Close Pane` (×, **disabled**, faint)

    Because it is the only pane, float, zoom and close are disabled. Split is still allowed.
- **Content** is horizontally centred in the pane, and vertically centred as a block, so the block moves up or down with its height.
- **Navigation footer** (screens 2 to 4): a circular **Previous** button, 30 x 30 pt, with a blue ← glyph on a dark navy disc (`#0C2442` / `#0D213A`). Next to it, after a 10 pt gap, is the **Continue →** pill, 105 x 30 pt, fill `#006FFE` / `#0073FF`. Its label is semibold, about 15 pt, in **dark text (`#0F1217`) on the blue**, not white, followed by a → arrow in the same dark colour.
- **Typography:** proportional SF-style system font everywhere. No monospace appears except the `~/` prompt chips on the theme cards.
- **Palette summary** (sampled):

  | Role | Hex |
  | --- | --- |
  | Window / tab bar bg | `#0B0F13` |
  | Pane bg | `#0F1217` |
  | Inset card / group bg | `#15181C` |
  | Dividers inside cards | `#282B2E` / `#272A2E` |
  | Primary text | `#E1EAF1` |
  | Secondary / helper text | `#787F84` / `#767C81` |
  | Section-heading text in cards | `#373B3F` to `#6A6F74` |
  | Accent / selection ring | `#0073FF` (button fill `#006FFE`) |
  | Accent tinted chip bg | `#11253F` |
  | Success chip bg / text | `#11301C` / `#00BD19` |
  | Neutral chip / keycap bg | `#24272D` / `#3E4146` |

---

## 132. Onboarding step 1: Welcome ("Get Started with Rex")

**What it is.** This is the first onboarding page, shown in a tab titled `Welcome, Kyle` in a session named `Onboarding`. The user's first name is interpolated into the tab title. How it is reached: the first run, or re-running onboarding. The capture used a temporary onboarding session (COVERAGE.md). The trigger itself is not shown.

**Controls and labels, in visual order:**

1. Hero illustration: a large Rex dinosaur-head mascot, about 160 x 165 pt, with no rounded-square container. It has a scalloped back crest and a gradient running magenta and purple → pink → orange → yellow at the top right, with cyan and blue at the bottom. It has a dark navy oval eye and an open smiling jaw.
2. Heading (static text): **`Get Started with Rex`**, about 32 pt bold, `#E1EAF1`, centred. AX bounds are 325 x 40.
3. Body (static text, two centred lines, about 15 pt regular, `#787F84`): `Take a moment to customize Rex's appearance, set up permissions, and optionally migrate your own configuration.` The line breaks after "set up".
4. **`Continue`** button with a → arrow: a primary blue pill, 119 x 36 pt (larger than the Continue button on later steps), fill `#006FFE` and dark label `#0F1217`, semibold, about 17 pt.
5. **`Skip for now`**: a text-link button, blue (`#0073FF`), about 13 pt regular, centred about 22 pt below Continue. AX bounds are 76 x 16.

**Layout.** The vertical stack is: mascot → 40 pt → heading → 12 pt → body → 25 pt → Continue → 22 pt → Skip link. It is all centred, with the block centred in the pane. There is no step indicator or progress dots on any onboarding page.

**Behaviour implied.** `Continue` moves on to Appearance (133). `Skip for now` skips the whole onboarding flow (inferred from the label). The text promises three optional parts: appearance, permissions, and migrating your own configuration.

---

## 133. Onboarding step 2: Appearance ("Customize the look and feel of Rex")

**What it is.** The appearance step, reached with Continue from 132.

**Controls and labels, in visual order:**

1. Hero icon, about 104 x 60 pt. It is a dark rounded-square "window" icon with red, yellow and green traffic-light dots and a grid of dark tiles. Overlapping it at the right is a tilted gradient card with a painter's palette (coloured dots).
2. Heading: **`Customize the look and feel of Rex`**, about 26 pt bold, `#E1EAF1`. AX bounds are 415 x 30.
3. Subtitle: `Choose a color scheme, style, preferred tab layout and theme.`, about 15 pt, grey `#787F84`. Note that the copy says "tab layout" but the section is labelled "Toolbar layout".
4. **Settings card**: one inset rounded rectangle, bg `#15181C`, radius about 12, 478 x 426 pt. It holds three bands separated by 1 px horizontal dividers (`#282B2E`), inset 18 pt from the card edges.

   **Band 1** has two groups side by side, separated by a vertical 1 px divider (`#272A2E`):
   - **`Color scheme`** (AX heading, about 12 pt medium, grey). It is a single-choice tile picker with 3 tiles. Each tile is about 80 x 68 pt, radius about 12, with a blue "macOS wallpaper" style background (navy `#0F2E55` / `#002755` / `#003977` with lighter diagonal swooshes) and a miniature window mock in the centre:
     - **`Auto`**: **selected**. The mock window is split diagonally, white on the left and black on the right. It has a 2 to 3 px blue `#0073FF` selection ring with a small gap outside the tile, and its label is bold white (`#E1EAF1`).
     - `Light`: a white mock window. The label is in regular weight, grey.
     - `Dark`: a black mock window with traffic lights. Grey label.

     AX: buttons named `Auto`, `Light` and `Dark`, with values `Selected` / `Not selected`. Each button is 74 x 88, which includes the label.
   - **`Interface style`** (heading), 2 tiles:
     - **`Classic`**: **selected** (blue ring, bold label). The tile shows a solid black circle (`#0B0F13`) on the blue wallpaper.
     - `Glass`: a translucent, lighter circle (`#1A1F29`) with a faint rim, suggesting a frosted-glass material. Grey label.

   **Band 2: `Toolbar layout`** has 3 wide tiles, about 139 x 68 pt each (AX buttons 139 x 88). The same wallpaper background has a small dark app-window mock in the centre:
   - **`Above`**: **selected**. The mock has a horizontal tab strip across the top of the window, above a 1 + 2 pane grid.
   - `Left Side`: the mock has a vertical tab/session sidebar column at the left, with tiny list rows, and panes to the right.
   - `Right Side`: the mock has the sidebar column at the right.

   **Band 3: Theme**. This is a horizontally scrolling strip (AX scrollarea `Theme`, 479 x 123), clipped at the card's right edge, with no visible scrollbar. It has two groups split by a short vertical divider:
   - **`Default theme`** (heading): one card, **`Merino Dark`**, **selected** (blue ring, bold white label).
   - **`Built-in themes`** (heading): 26 cards. They are listed here in order, from the AX tree. The first three are visible and the fourth is clipped as "Cathode Aml…"; the rest are offscreen.
     `Abyssal Trench`, `Basalt Shore`, `Cathode Amber`, `Charcoal Atelier`, `Cinder Peak`, `Cobalt Foundry`, `Ember Hollow`, `Heliotrope Haze`, `Ink Meridian`, `Juniper Smoke`, `Midnight Tundra`, `Misty Forest`, `Moss Cathedral`, `Neon Arcade`, `Nocturne Rose`, `Oxblood Library`, `Petrol Lagoon`, `Phosphor Archive`, `Polar Aurora`, `Roasted Umber`, `Saffron Nightfall`, `Silver Point Dark`, `Sonoran Dusk`, `Static Noir`, `Velvet Dusk`, `Whitby Bay`.
     The AX names are `<Name> theme`, as buttons with values `Selected` / `Not selected`. Each card is 112 pt wide with a 120 pt pitch (8 pt gap). The visual card is about 112 x 70 pt, radius about 10, filled with that theme's own background colour: Merino Dark `#0F1217`, Abyssal Trench `#08111A`, Cathode Amber `#16120B`. It has a 1 px subtle border, and the theme name sits below it in about 11 pt grey (bold white when selected).
   - **Theme card anatomy** (each card is a live preview in the theme's colours):
     - Top left: **`Aa`** in about 18 pt, in the theme's foreground colour (Cathode Amber's is amber).
     - Top right: a small rounded chip showing **`~/`** in monospace, as a prompt sample. Merino Dark draws it light on a pale chip; other themes draw it on a darker tinted chip.
     - Next to the chip: a tall rounded **cursor block** swatch in the theme's cursor colour. Merino Dark's is blue `#006EFE` with a small underline mark; Abyssal Trench's is green `#00EBB6`; Basalt Shore's is teal.
     - Bottom row: **8 overlapping circles** (each about 14 pt, overlapping by about 30%), showing a palette strip. Merino Dark's, sampled left to right: `#3D464F` (dark grey), `#FF5557` (red), `#00BD1A` (green), `#D68D00` (amber), blue, purple, `#00BAC7` (cyan) and `#A7B1BC` (light grey). These look like ANSI black, red, green, yellow, blue, magenta, cyan and white (inferred).
5. Footer: **Previous** (← disc, 30 x 30) and **`Continue` →** (105 x 30), centred 30 pt below the card.

**Visual spec notes.** Tile labels sit about 8 pt below the tiles. The selected label is semibold `#E1EAF1` and unselected labels are regular grey (`#6A6F74`-ish). The selection ring is `#0073FF`, about 2.5 pt, separated from the tile by about 2 pt of the card bg, so it reads as an outer focus ring. Unselected tiles have no ring and a faint 1 px border. Group headings are about 12 pt medium grey, placed 18 pt from the card's left edge and about 20 pt above their tiles.

**Behaviour implied.** These are single-select pickers.
- Color scheme (Auto / Light / Dark) sets the app's light or dark mode. Auto follows the system (inferred).
- Interface style (Classic / Glass) chooses opaque or translucent chrome (inferred from the tile art).
- Toolbar layout puts the tab/session toolbar above the panes, in a left sidebar or in a right sidebar.
- The theme choice sets the terminal and app theme. Merino Dark is the default.

Changes probably preview live in the window (inferred, not captured). COVERAGE.md notes that no themes were saved.

---

## 134. Onboarding step 3: Integrations & Permissions

**What it is.** The permissions step, reached with Continue from 133.

**Controls and labels, in visual order:**

1. Hero icon, 60 x 60 pt: a dark rounded square (radius about 14, bg about `#1B1F24`) with a gradient jigsaw-puzzle piece (pink → purple → orange) in it.
2. Heading: **`Integrations & Permissions`**, about 26 pt bold.
3. Subtitle: `Install the Rex skill and grant permission to let agents use Rex.`, grey.
4. **Two-column card**: bg `#15181C`, radius about 12, about 480 x 230 pt, split by a vertical 1 px divider (`#282B2E`). Each column is centred:
   - **Left column: Rex Agent Skill**
     - Icon (AX image `OnboardingSkill`, 48 x 48): a dark rounded tile holding a 2 x 2 mosaic of agent logos. They look like a pink/orange starburst, a pixel "P" glyph in orange and yellow, a blue outlined square and a pink knot (inferred to be the Claude, Pi, OpenCode and OpenAI marks). A green circular **checkmark badge** sits at the bottom right.
     - Title **`Rex Agent Skill`**, about 15 pt semibold, `#E1EAF1`.
     - Badge **`Recommended`**: a filled grey pill (bg about `#6A6F74`), about 10 pt semibold dark text.
     - Description: `Install to give your agents the ability to fully drive Rex.`, about 12 pt grey `#767C81`, on 2 lines.
     - Status chip **`Installed`**, preceded by a ✓-in-circle glyph. It is a pill about 88 x 24 with bg `#11301C` and green text/glyph `#00BD19`. AX: image `Selected` (state selected) plus static text `Installed`. It is not a button in this state.
   - **Right column: System Permissions**
     - Icon (AX image `OnboardingPermissions`, 48 x 48): a blue rounded tile (system-blue gradient) with a white raised **hand** (stop/privacy) glyph.
     - Title **`System Permissions`**.
     - Badge **`None granted yet`**: an *outlined* pill with a 1 px grey border, transparent fill and grey text `#767C81`.
     - Description: `Grant system permissions for a seamless Rex experience.`
     - **`Grant…`** button: a pill 65 x 24 with tinted accent bg `#11253F` and blue text `#0073FF`. The label ends in a real ellipsis (`…`).
5. Footer: Previous ← and **`Continue` →**.

**Behaviour implied.**
- The Rex Agent Skill was already installed on this machine, so the card shows a passive success state. When it is not installed, presumably an Install button appears (inferred, not captured).
- `Grant…` opens the Manage Permissions sheet (135).
- The `None granted yet` badge summarises the permission count or status and would change once permissions are granted (inferred).

---

## 135. System permissions sheet ("Manage Permissions")

**What it is.** A modal sheet or popover over the onboarding pane, opened by **`Grant…`** on the System Permissions card in 134. This screenshot is cropped to the window, 1302 x 760. **The AX tree in `135-system-permissions.json` does not include the sheet.** It only lists the underlying onboarding controls, and the pane has lost its `focused` state. Everything below is therefore read from pixels only.

**Placement and frame.** The sheet is about 500 x 442 pt. It is anchored right of centre at window (592,119) to (1091,560), overlapping the right half of the permissions card. That suggests it is a panel positioned near the Grant button, not centred (inferred). Its bg is `#0B0F13`, darker than the pane. It has a 1 px border `#23272B`, about 12 pt corner radius and a soft dark drop shadow. The content behind it is **dimmed and desaturated**: pane bg changes from `#0F1217` to about `#15181B`, and the Continue pill becomes grey with grey text, as if disabled.

**Controls and labels, in visual order:**

1. **Close button** at the top left (centre at about (609,137)): a 20 pt grey circle (about `#3A3D42`) with a white **×**.
2. App icon, about 78 x 78 pt: the **Rex Beta app icon**. It is a squircle with a magenta → orange → yellow gradient, a pale pink dino-head in profile, and a white pill reading **`BETA`** in gradient (blue → orange) letters.
3. Heading **`Manage Permissions`**, about 26 pt bold, `#E1EAF1`, centred.
4. **Grouped list**: an inset rounded rect, 460 x 222 pt, bg `#0F1217`, 1 px border about `#282B2E`, radius about 10. It has three rows, each about 75 pt tall, separated by inset 1 px dividers that start at the row content and stop before the edges. Each row contains:
   - A 36 x 36 rounded icon tile (bg `#1D2025`, radius about 8) with a grey outline glyph.
   - A title in about 13 pt semibold `#E1EAF1`, followed by a small **circled ↗ arrow** glyph. This is an "open in System Settings" link affordance (inferred).
   - A 2-line description in about 12 pt grey `#797F85`.
   - A trailing **`Grant…`** button: a neutral pill about 66 x 26, bg `#1F2227`, light text `#BBC2C9`. It is neutral, not the accent tint used on the onboarding card.

   Rows, verbatim:

   | Icon glyph | Title | Description | Button |
   | --- | --- | --- | --- |
   | Internal drive (flat rounded box with dots) | `Full Disk Access` ↗ | `Allows processes running in Rex to open files in protected folders and sandboxed containers.` | `Grant…` |
   | Display with a record dot in the corner | `Screen & System Audio Recording` ↗ | `Allows processes and agents running in Rex to capture the screen, windows, and system audio.` | `Grant…` |
   | Bell with a badge dot | `Notifications` ↗ | `Allow Rex to notify you when running processes and agents complete tasks or need your attention.` | `Grant…` |

   None of the three is granted, so all show `Grant…`. The granted state is not captured.

**Behaviour implied.**
- Each `Grant…` triggers that macOS permission: the TCC prompt, or opening the matching System Settings privacy pane (inferred).
- The ↗ glyph next to each title probably opens the relevant System Settings page or documentation (inferred).
- × dismisses the sheet and returns to 134.

For Linux this maps to portal or notification permissions rather than TCC.

---

## 136. Onboarding step 4: Migrate your config

**What it is.** The migration step, reached with Continue from 134 (after any permissions).

**Controls and labels, in visual order:**

1. Hero icon, 60 x 60 pt: a gradient rounded square (magenta → orange → yellow, with cyan at the bottom left) holding a white **dotted line leading into an arrow** (`··→`), a "move in" metaphor.
2. Heading **`Migrate your config to Rex`**, about 26 pt bold.
3. Subtitle `Choose your agent to find and migrate your config to Rex.`, grey.
4. **Card**: bg `#15181C`, radius about 12, about 543 x 203 pt.
   - **`Migrate with`** (AX group; there is no visible heading). It is a row of 5 single-select tiles, each about 87 x 68 pt visual (AX 87 to 88 x 88 including the label), radius about 12, with 12 pt gaps. The fourth and fifth tiles are separated by a short vertical divider:
     - `Claude Code`: orange-red tile (`#E97152`) with a white starburst mark. Not selected.
     - `Codex`: white tile (`#F9F9F9`) with the black OpenAI knot mark. Not selected.
     - `OpenCode`: dark grey gradient tile (`#525353`) with a white square-in-square mark. Not selected.
     - `Pi`: off-white tile (`#EDE9E6`) with a black pixel "P" mark. Not selected.
     - | divider |
     - **`Skip migration`**: **selected (the default)**. A card-coloured tile (`#15181C`) with a 1 px dark rim, a grey ⦸ "prohibited" circle-slash glyph and the blue `#0073FF` selection ring. Its label is bold white.

     Unselected labels are about 11 pt regular grey, centred under the tiles.
   - Helper text, 2 centred lines, about 12 pt: **`Using another agent?`** in semibold light text, then in grey: `Copy the prompt and run it any time, in a Rex terminal or any other. Press ⌘↩ to open one beside setup.` The shortcut is drawn inline as the glyphs `⌘↩`. AX gives it as "Press Command Return".
   - **`Copy Prompt`** button: a neutral pill, 117 x 24, bg `#24272D`, with a clipboard/document-copy glyph and light text.
5. Footer: Previous ← and **`Continue` →**.

**Behaviour implied.**
- Picking an agent tile presumably launches or prepares that agent to find the user's existing terminal config (Ghostty, iTerm, tmux and so on) and convert it to Rex config (inferred from the copy). Skip migration is the default.
- `Copy Prompt` copies the migration prompt to the clipboard so it can be run in any agent.
- **⌘↩ opens a terminal pane beside the setup pane.** This matches the pane's enabled `Split Right` button and the "Split pane ⌘↩" binding on the next page. Setup stays open while the user works in a real shell.

---

## 137. Onboarding step 5: Finish ("You're all set")

**What it is.** The final page, reached with Continue from 136.

**Controls and labels, in visual order:**

1. Hero: the **Rex Beta app icon** (about 104 x 104 pt, the same artwork as in 135, with the `BETA` pill), with a green circular **✓ badge** overlapping its bottom-right corner.
2. Heading **`You're all set`**, about 26 pt bold.
3. Subtitle `A few keys worth knowing, from your keymap.`, grey. The copy implies the shortcuts are read from the user's actual keymap, not hard-coded.
4. **Shortcut list card**: bg `#15181C`, radius about 12, 400 x 135 pt, three rows about 48 pt tall separated by inset 1 px dividers (`#282B2E`). Each row has a left-aligned label (about 13 pt medium `#E1EAF1`) and right-aligned **keycaps**. Each keycap is about 24 x 23 pt, radius about 5, bg `#3E4146`, with a dimmer grey glyph `#737A7E`, and keycaps are 6 pt apart:
   - `Command palette`: keycaps `⇧` `⌘` `P` (AX: "Shift Command P")
   - `Switch session`: keycaps `⌘` `K` (AX: "Command K")
   - `Split pane`: keycaps `⌘` `↩` (AX: "Command Return")
5. **`Start Using Rex`**: a primary blue pill, 149 x 36 pt (the same size class as Continue on page 1), with dark semibold label text.
6. **`← Return to Migration`**: a blue text link with a leading ← arrow, about 13 pt. This replaces the circular Previous button here.

**Behaviour implied.**
- `Start Using Rex` ends onboarding. It probably closes the onboarding tab or session and returns to the normal workspace (COVERAGE.md says the temporary onboarding session was closed afterwards).
- The keycap rows teach the three core shortcuts, generated from the live keymap.
- `Return to Migration` goes back one step.

---

## Flow summary

```
132 Welcome ──Continue──▶ 133 Appearance ──▶ 134 Integrations & Permissions ──▶ 136 Migration ──▶ 137 Finish ──Start Using Rex──▶ workspace
   └─ Skip for now (exit)        ◀ Previous           │ Grant… ▶ 135 Manage Permissions sheet (× close)     ◀ Return to Migration
```

- The flow has 5 pages plus 1 sheet, with no step counter or progress dots.
- The steps run in a normal tab and pane, so the pane chrome (split and so on) still works during onboarding. ⌘↩ during migration opens a terminal pane beside setup.

## Feature inventory

- [onboarding.host] Onboarding runs as a regular session (`Onboarding`) with a tab titled `Welcome, <FirstName>` and a pane titled `Get Started with Rex`. The pane chrome stays live (Split Right and Split Down enabled; Float, Zoom and Close disabled because it is the only pane).
- [onboarding.welcome] Welcome page with mascot, `Get Started with Rex`, explanatory copy, a primary `Continue` and a `Skip for now` link.
- [onboarding.skip] Skip the whole onboarding flow from the first page (`Skip for now`).
- [onboarding.nav] Per-step navigation: circular Previous (←) and `Continue →` pill. The final page uses a `← Return to Migration` link.
- [onboarding.appearance.color-scheme] Color scheme picker: Auto | Light | Dark (default Auto), shown as illustrated tiles.
- [onboarding.appearance.interface-style] Interface style picker: Classic | Glass (default Classic). Glass is a translucent/frosted chrome style.
- [onboarding.appearance.toolbar-layout] Toolbar layout picker: Above | Left Side | Right Side (default Above). The tab/session toolbar goes on top or in a left or right sidebar.
- [onboarding.appearance.theme] Theme picker in a horizontal scroller: Default theme `Merino Dark` (default, selected), plus 26 built-in themes (Abyssal Trench, Basalt Shore, Cathode Amber, Charcoal Atelier, Cinder Peak, Cobalt Foundry, Ember Hollow, Heliotrope Haze, Ink Meridian, Juniper Smoke, Midnight Tundra, Misty Forest, Moss Cathedral, Neon Arcade, Nocturne Rose, Oxblood Library, Petrol Lagoon, Phosphor Archive, Polar Aurora, Roasted Umber, Saffron Nightfall, Silver Point Dark, Sonoran Dusk, Static Noir, Velvet Dusk, Whitby Bay).
- [onboarding.appearance.theme-card] Theme preview card rendered in the theme's own colours: `Aa` foreground sample, monospace `~/` prompt chip, cursor-colour block and an 8-swatch palette strip (likely ANSI 0 to 7).
- [onboarding.integrations.agent-skill] Rex Agent Skill card (`Recommended` badge) to install a skill that lets coding agents drive Rex. Shows an `Installed` success chip when it is present.
- [onboarding.integrations.system-permissions] System Permissions card with a status badge (`None granted yet`) and a `Grant…` button that opens Manage Permissions.
- [permissions.sheet] Manage Permissions sheet: app icon, title, close (×) button, and a grouped list of permissions, each with an icon, a title with a ↗ link glyph, a description and a `Grant…` button.
- [permissions.full-disk-access] Full Disk Access: lets processes in Rex open files in protected folders and sandboxed containers.
- [permissions.screen-recording] Screen & System Audio Recording: lets processes and agents in Rex capture the screen, windows and system audio.
- [permissions.notifications] Notifications: notifies the user when processes or agents complete tasks or need attention.
- [onboarding.migration.agent-picker] Migrate config with a chosen agent: Claude Code | Codex | OpenCode | Pi | Skip migration (default Skip migration).
- [onboarding.migration.copy-prompt] `Copy Prompt` copies the migration prompt to run in any agent or terminal.
- [onboarding.migration.open-terminal] ⌘↩ during setup opens a terminal pane beside the setup pane.
- [onboarding.finish.shortcuts] Finish page lists key shortcuts read from the user's keymap: Command palette ⇧⌘P, Switch session ⌘K, Split pane ⌘↩, drawn as keycaps.
- [onboarding.finish.start] `Start Using Rex` ends onboarding.
- [keymap.defaults] Default bindings shown: ⇧⌘P command palette, ⌘K switch session, ⌘↩ split pane.
- [chrome.tabbar] Tab bar elements seen here: a session switcher button (`Change Session`) showing the session name, tab pills with an icon and title, Activity (eye) toggle (Off), Command Palette (⌘) and New Tab (+).
- [chrome.pane-header] Pane header: drag handle with icon and title, plus Split Right, Split Down, Float Pane, Zoom Pane and Close Pane buttons.
