# B09: Rex Beta overlays, context menus, find and status bar menu

Reference product: Rex Beta 0.1.0 (1932), Bleeding Edge channel, Rex Server 0.1.0-preview.15, macOS. Captured 2026-11-10/11 (PST) with agent-desktop. Source: `rex-beta-ui-2026-11-10/screenshots/` and `rex-beta-ui-2026-11-10/evidence/`.

## Conventions used in this file

- The capture display is 1920x1080 at 1x, so **1 screenshot pixel = 1 point**. The window screenshots (1414x872) include the macOS drop shadow; the window itself is 1302x760 pt and sits at screenshot (56, 38). Screen points = screenshot px + (312, 130).
- Coordinates given as `screenshot (x, y)` are in the 1414x872 image; "window-relative" means relative to the window's top-left.
- Colours were sampled with ImageMagick from the PNGs and are sRGB hex. Anti-aliased edges are not reported.
- **Important accessibility caveat:** none of the in-app SwiftUI overlays (session switcher, command palette, activity popover, rename-tab field, find bar, go to directory, insert path, theme switcher) appear in the accessibility trees; their labels below are read from the pixels. Native NSMenus (tab menu, terminal menu, status bar menu) are present in the trees and their labels are ground truth from the tree.
- (inferred) marks anything not directly visible or in the tree.

### Shared window chrome visible behind every overlay (from the trees)

Window title `shuv@shuvbot:~ - lunar-basin`, 1302x760 pt. Toolbar group `Tabs` 46 pt high containing, left to right: traffic lights (`Close`, `Minimize`, `Full Screen`, 16x16), `Change Session` button (123x34, shows a stacked-box glyph + `lunar-basin` in semibold), tab radiobutton `shuv@shuvbot:~` (242x32, selected, green `>_` terminal icon in a dark rounded square at its left, `×` at its right), and on the right `Activity` (36x32, eye glyph, value `Off`/`On`), `Command Palette` (36x30, ⌘ glyph), `New Tab` (36x32, `+`). Below: `Split layout` group with one pane `shuv@shuvbot:~`, a pane header (`>_ shuv@shuvbot:~` on the left; right-side icon buttons `Split Right`, `Split Down`, `Float Pane` (disabled), `Zoom Pane` (disabled), `Close Pane` (disabled), each 22x22) and the read-only `Terminal` text area (1286x670). Terminal background #0F1216, window/toolbar background ~#0B0F13.

### Shared "floating panel" style (session switcher, palette, activity, rename, find, go to, insert path, theme)

All the custom overlays share one look:

- 1 px outer hairline #020203 (near-black), then a 1 px inner highlight #333436 (top and sides), then the panel body #1C1D20 (1 px #212224 transition). Corner radius ~10-12 pt on panels (search capsules and the find bar are fully rounded/pill ends ~19 pt). Soft dark drop shadow below. Opaque (no visible translucency of the terminal behind).
- Search field: inset rounded rectangle (radius ~8 pt), fill #333437, 30 pt high, magnifier glyph at left in grey (#8C8D8E-ish), placeholder text grey (#8F8F91), caret is a blue bar (#0069FF / system accent). Field inset ~5-6 pt from the panel edges.
- List rows: 28 pt pitch, label 13 pt-ish proportional (SF Pro) regular/medium in near-white (#ECECEC-#F3F3F3); right-aligned shortcut hints in grey (#707072) using macOS glyphs (⌘ ⇧ ⌥); optional trailing chevron `>` in grey.
- Selection: full-width pill (radius ~8 pt), fill **#0055BF**, inset ~5 pt from the panel sides; text on selection white; shortcut hint on selection turns light (#B3CCEC-ish).
- Section header: small (~11-12 pt) semibold grey (#8x8x8x) text, e.g. `Go to`, `Working`.
- Dividers: 1 px #2A2B2D, inset ~ 10-14 pt from panel edges.

---

## 089-session-switcher.png

**What it is.** The session switcher popover, a dropdown anchored under the `Change Session` toolbar button (the `lunar-basin` chip at the left of the toolbar). Reached by clicking that chip (the palette lists `Change Session…` with ⌘K, so ⌘K also opens session switching (inferred: same surface)). The chip shows a lighter rounded background (#3F4145 sampled inside it) while the popover is open; in other screenshots, when closed, it has no fill.

**Controls, verbatim, in visual order**

1. Search text field, placeholder `Filter or create…` (magnifier glyph at left). Focused (inferred from it being the popover's first control; no caret visible in this frame).
2. Row `lunar-basin`: leading checkmark `✓` (current session), shortcut `⌘1`. Selected/highlighted (blue pill).
3. Row `still-atoll`: no check, shortcut `⌘2`.
4. Divider.
5. Row `New Session`, leading icon (a stacked-box/session glyph with a small `+`-circle badge at its lower left), shortcut `⇧⌘N`.
6. Divider.
7. Row `Add Remote Host…`, leading icon (globe glyph with a small badge at its lower left), no shortcut.

**Layout and visual spec**

- Popover outer bounds: screenshot x 137-332, y 80-262 → ~196 x 183 pt. Window-relative origin ~(81, 42): left edge sits ~4 pt left of the chip, top overlaps the bottom of the 46 pt toolbar by ~4 pt. No arrow/beak.
- Search field: y 86-116 (30 pt), fill #333436/#333437, radius ~8.
- Session rows at y 126-154 and 154-182 (28 pt). Selected row fill #0055BF, text white, `⌘1` light.
- Divider at y ~186 (#2A2B2D), `New Session` row centred at y 206, divider at y ~224, `Add Remote Host…` row centred at y 243. Action rows are ~37 pt apart (slightly taller than session rows because of dividers).
- Label column starts at x ~176 (check / icon column ~16 pt wide at x ~152-168). Shortcut hints right-aligned at x ~316, grey #727274.
- Labels: SF Pro ~13 pt, medium weight, #ECECEC-ish; `still-atoll` and action labels white-ish on #1C1D20.

**Behaviour implied**

- Type to filter sessions; "or create" means typing a name that does not exist offers to create a session with that name.
- Click a session (or ⌘1…⌘9 (inferred for >2)) to switch; the current session is check-marked.
- `New Session` (⇧⌘N) creates a session; `Add Remote Host…` opens the add-host flow.

---

## 090-command-palette.png

**What it is.** The command palette, a centred floating panel over the pane. Reached via the toolbar `Command Palette` button (⌘ glyph, top right) (inferred keyboard shortcut not shown in this frame).

**Controls, verbatim, in visual order**

Search field placeholder: `Search commands, sessions, and tabs…` (magnifier, blue caret at start, focused).

Command rows (label — right-side accessories). These are the 15 rows visible; there is **no visible scrollbar**, so the list may or may not continue below (unknown; the panel height matches the 16-row theme list in 129, which does scroll, so more commands below `Close Session` are possible (inferred)).

| # | Label | Shortcut hint | Chevron `>` |
|---|---|---|---|
| 1 | `Change Session…` (selected) | `⌘K` | yes |
| 2 | `Rename Session` | — | yes |
| 3 | `Rename Tab` | — | yes |
| 4 | `Add Remote Host…` | — | yes |
| 5 | `New Session` | `⇧⌘N` | — |
| 6 | `New Tab` | `⌘T` | — |
| 7 | `Go to Directory…` | `⇧⌘G` | yes |
| 8 | `New Window` | `⌘N` | — |
| 9 | `Move Tab to New Window` | — | — |
| 10 | `Close Tab` | `⌥⌘W` | — |
| 11 | `Previous Session` | — | — |
| 12 | `Next Session` | — | — |
| 13 | `Refresh Sessions` | `⌘R` | — |
| 14 | `Collapse Other Sessions` | — | — |
| 15 | `Close Session` | — | — |

Footer bar (below a divider), right-aligned, grey small text with keycap chips:
`Change Keybinding` [`⌘↩`]   `Action` [`↩`]

**Layout and visual spec**

- Panel bounds: screenshot x 542-871, y 113-614 → ~330 x 501 pt. Horizontally centred in the window (panel centre 706.5 vs window centre 707). Top edge ~29 pt below the toolbar's bottom edge (window-relative y ~75).
- Search field y ~119-148 (~30 pt), fill #333437; placeholder #8C8D8E.
- First row (selected) y 159-187, fill #0055BF, pill radius ~8, inset ~5 pt from the panel sides (x 547-866).
- Row pitch 28 pt (row centres 173, 201, 229 … 565). Label left inset x 557 (~15 pt from the panel edge). Shortcut hints right-aligned at ~x 856 when no chevron, ~x 840 when a chevron follows; chevrons at x ~852. Hint colour #707072; on the selected row hint and chevron are white/light.
- Footer: divider at y ~579 (1 px), footer text baseline centred y ~597; text colour ~#61666B, ~11 pt. Keycaps are small rounded rectangles with a 1 px grey outline (no fill), containing `⌘↩` and `↩` glyphs.
- Labels SF Pro ~13 pt medium, #ECECEC.

**Behaviour implied**

- Fuzzy search across commands, sessions and tabs.
- Rows with `>` open a second step inside the palette (e.g. `Rename Tab` > opens the rename field seen in 093; `Change Session…` > presumably lists sessions; `Go to Directory…` > opens the directory browser in 127).
- `↩` runs the action; `⌘↩` on a row opens "Change Keybinding" for that command (palette doubles as a keybinding editor).

---

## 091-activity-popover.png

**What it is.** The Activity popover, anchored under the toolbar `Activity` button (eye glyph). Tree: `Activity` button state `pressed`, value `On` while open (value `Off` in all other captures). Reached by clicking the eye button.

**Controls, verbatim, in visual order**

1. Section header `Working` (small grey semibold).
2. Activity row (selected, blue): leading app/shell icon (a dark rounded-square tile with a white rounded-rectangle glyph inside), title `Shell` (white, semibold, ~13 pt), subtitle `Rex beta app screenshots` (white, ~11 pt), top-right `•••` (three small dots, likely a "working"/progress indicator (inferred), the leftmost dot dimmer), bottom-right elapsed time `4m`.
3. Filter control: a full-width rounded button/field with a circled-lines filter glyph (three horizontal lines of decreasing width inside a circle) and label `Filter` (grey). Fill #26272A.

**Layout and visual spec**

- Popover bounds: screenshot x ~988-1288, y ~81-198 → ~300 x 117 pt, right edge aligned ~3 pt left of the eye button's right edge; top just under the toolbar.
- Eye button while open: circular/rounded highlight #101418-#1C2125 behind the glyph; glyph brightens to #E3EAF1.
- Header `Working` at y ~98, left inset ~16 pt.
- Activity row y ~114-158 (two-line, ~44 pt), fill #0055BF, radius ~8, inset ~5 pt. Icon tile ~20 pt, at x ~1003-1022. Text column starts x ~1030. Time `4m` right-aligned at x ~1270 in light blue-white (#B3CCEC).
- Filter bar y ~164-193 (~29 pt), fill #26272A, radius ~8.
- Panel style as shared (#1C1D20, hairline, shadow).

**Behaviour implied**

- Shows running/attention activity across sessions grouped by state (`Working` here; other groups such as done/error/blocked exist in Settings notifications (inferred)). Each row names the process (`Shell`) and the session/tab context (`Rex beta app screenshots`) and how long it has been in that state (`4m`). Clicking a row jumps to that pane (inferred). `Filter` narrows the list (exact options not captured).

---

## 092-tab-context-menu.png

**What it is.** Native macOS context menu shown by right-clicking (secondary click) the tab `shuv@shuvbot:~`. Menu name in the tree: `shuv@shuvbot:~`. Menu bounds 187 x 152 pt at screen (706, 185), i.e. opening at the pointer over the right half of the tab.

**Items, verbatim (tree + pixels)**

| Item | State | Shortcut |
|---|---|---|
| `Rename Tab…` | enabled | — |
| `Mute Tab` | enabled | — |
| separator | | |
| `Move Tab Left` | **disabled** | `⌃⇧↑` |
| `Move Tab Right` | **disabled** | `⌃⇧↓` |
| separator | | |
| `Close Tab` | enabled | `⌥⌘W` |

(Move left/right are disabled because there is only one tab.) The tree also contains an unnamed disabled 24-pt menu item at the menu's top, which is not drawn (AppKit artefact).

**Layout and visual spec**

- Standard macOS dark menu (Sonoma/Sequoia style): translucent vibrant material, sampled body ~#2B2E33 (top edge lighter #34373C), corner radius ~10 pt, 1 px light rim, shadow. Items 24 pt high, separators 11 pt. Text ~13 pt SF Pro, enabled #DFDFDF-ish, disabled ~#6E6E70. Shortcut glyphs right-aligned in grey.
- Note shortcut arrows are up/down (not left/right) for Move Tab Left/Right.

**Behaviour implied**

- `Rename Tab…` opens the rename field (093). `Mute Tab` silences activity notifications/sounds for that tab (inferred from name). `Move Tab Left/Right` reorder tabs. `Close Tab` closes it (⌥⌘W).

---

## 093-rename-tab.png

**What it is.** The rename-tab overlay, using the palette panel style, centred under the toolbar. Reached from tab context menu `Rename Tab…` (092) or palette `Rename Tab >` (090).

**Controls, verbatim**

1. Text field with magnifier glyph, showing `shuv@shuvbot:~` in grey placeholder colour (#8F8F91) with the blue caret at the start: the current/default title is shown as placeholder; the field is empty and focused.
2. Single row (selected, blue): `Restore Default Title`, trailing `↩` return glyph (light #B3CCEC).

**Layout and visual spec**

- Panel bounds: screenshot x 542-871, y 113-193 → ~330 x 80 pt (same width and top as the palette; height shrinks to content).
- Field y ~119-148; row y 159-187 #0055BF.

**Behaviour implied**

- Type a new name and press ↩ to rename; with the field empty, ↩ triggers `Restore Default Title`, which clears the custom title so the tab follows the terminal/process title again. Esc cancels (inferred).

---

## 094-terminal-context-menu.png

**What it is.** Native macOS context menu from right-clicking inside the terminal area. Bounds 132 pt wide (items 132 x 24 at screen x 651, y 359…). The display capture shows it opened at the pointer in the middle of the pane.

**Items, verbatim (tree)**

| Item | State |
|---|---|
| `Copy` | **disabled** (no selection) |
| `Paste` | enabled |
| separator | |
| `Split Right` | enabled |
| `Split Left` | enabled |
| `Split Down` | enabled |
| `Split Up` | enabled |
| `Float Pane` | enabled |
| `Zoom Pane` | enabled |
| separator | |
| `Close Pane` | enabled |
| `Reset Terminal` | enabled |
| separator | |
| `AutoFill` > (submenu; system-provided, keyboard/pen glyph at left) | enabled |

No shortcut hints are shown on any item. (Contrast: the pane header buttons `Float Pane`, `Zoom Pane`, `Close Pane` are disabled for a single pane, but here the menu items are enabled.) Two additional unnamed disabled items exist in the tree at the top (not drawn).

**Layout and visual spec**

- Native dark menu, body ~#2B2E33/#2B2F33, items 24 pt, separators 11 pt, radius ~10 pt, translucent, shadow. Disabled `Copy` text ~#6E6E70; enabled ~#DFDFDF. The menu is narrow (132 pt): width is set by the longest label `Reset Terminal`.

**Behaviour implied**

- Copy/Paste via the clipboard; splits in four directions; float a pane; zoom (maximise) a pane; close the pane; `Reset Terminal` resets terminal state (RIS) (inferred). AutoFill is the standard macOS text-input services submenu.

---

## 095-terminal-context-autofill.png

**What it is.** Same terminal context menu (094) with the pointer on `AutoFill`, showing the system submenu.

**Items, verbatim (tree)**

- `AutoFill` row highlighted (state `selected`): native blue highlight #0069FF (sampled #0069FF / #0D71FF), white text and chevron.
- Submenu `AutoFill` (132 x 82 pt, to the right, top aligned to the parent row):
  - `Contact…` (selected/highlighted; person-in-circle glyph)
  - `Passwords…` (key glyph)
  - `Credit Card…` (card glyph)

**Layout and visual spec**

- Native submenu, same material, 24 pt items; highlight is the macOS system accent blue (#0069FF), different from the app's own selection blue #0055BF used in the custom overlays.

**Behaviour implied**

- System AutoFill into the focused text input (the terminal). Not Rex-specific; it appears because the terminal view accepts text input.

---

## 125-terminal-find.png

**What it is.** The terminal find bar, a floating pill in the pane's top-right corner over the terminal content. Reached via Edit > Find (⌘F (inferred)).

**Controls, verbatim, left to right**

1. Text field, placeholder `Find` (grey #99999A), blue caret before it, focused. No magnifier icon.
2. (Empty match-count area.)
3. Vertical divider (1 px #2B2D30).
4. Previous match button: up chevron `^`, **disabled** (very dim).
5. Next match button: down chevron `v`, **disabled** (very dim).
6. Close button `×` (enabled, grey #64696E).

**Layout and visual spec**

- Bar bounds: screenshot x ~1039-1340, y ~125-164 → ~300 x 39 pt; fully rounded ends (radius ~19-20 pt). Same layered border as the overlays (#020203 outer, #333436 inner highlight) and body #1C1D20. Right edge ~10 pt inside the pane's right edge; top ~5 pt below the top of the terminal area (just below the pane header).
- Field text starts at x ~1051 (~12 pt inset). Divider at x ~1249. Chevron buttons centred at x ~1270 and ~1296; close at x ~1321. Icons ~10-12 pt, thin strokes.
- No dimming of the terminal while the query is empty (terminal stays #0F1216).

**Behaviour implied**

- Incremental search of the terminal screen and scrollback; navigation disabled until there are matches.

---

## 126-terminal-find-results.png

**What it is.** The find bar after typing `login`.

**Controls, verbatim**

1. Field value `login` (white), caret after it.
2. Match counter `1/1` (grey, right-aligned in the field, ~x 1230).
3. Divider.
4. `^` previous and `v` next buttons now **enabled** (brighter grey).
5. `×` close.

**Visual changes on the terminal**

- The whole terminal area dims while searching with results: background goes from #0F1216 to **#080A0C**, and all text dims (e.g. `Last`, prompt glyphs become greyer; the prompt's blue cursor dims). The pane header (#0F1217) and toolbar are unchanged.
- The match `login` in `Last login: Tue Nov 10 …` is highlighted with a **yellow rounded-rectangle** background **#F4D722** (radius ~3 pt, slightly padded around the glyphs) and dark text (sampled ~#280A90-ish dark ink on yellow; reads as near-black/navy).
- With one match, the current match uses this yellow; a distinct colour for non-current matches is not visible in this capture.

**Behaviour implied**

- Live match counting (`current/total`), next/previous navigation (likely ↩ / ⇧↩ and ⌘G / ⇧⌘G (inferred)), dims non-matching content to make matches pop, close with × or Esc (inferred).

---

## 127-go-to-directory.png

**What it is.** The `Go to Directory…` overlay (palette `Go to Directory…` ⇧⌘G, or File menu). A path-completion browser using the palette panel style.

**Controls, verbatim, in visual order**

1. Path text field (magnifier glyph): value `/Users/shuv/` (white), blue caret at end, and a circular **clear button** (white `×` in a grey circle, #FCFCFC glyph) at the right end of the field.
2. Section header `Go to` (small grey semibold).
3. Directory rows, each with a blue macOS folder icon and the folder name (16 visible):
   - `shuv` (selected, blue; its folder icon carries a small house glyph: the home folder itself, i.e. "go to the typed directory")
   - `agent-tools`
   - `agent-ws`
   - `Applications`
   - `audits`
   - `backups`
   - `bin`
   - `ci`
   - `clawd`
   - `codex-a2a-workspace`
   - `Compositor-linux-reference`
   - `cua-dual-display-validation`
   - `Desktop`
   - `digup-test`
   - `Documents`
   - (list continues; no scrollbar visible in this frame; sorted case-insensitively alphabetically)

**Layout and visual spec**

- Panel bounds same as the palette: screenshot x 542-871, y 113-614 (~330 x 501 pt), centred.
- Field y ~119-148; header `Go to` at y ~173 (left x 559); first row y 187-215 (#0055BF). Rows 28 pt pitch. Folder icons ~16 x 13 pt (#4FBCED light blue, macOS Finder folder look) at x ~557-573; label x 580.
- No shortcut hints, no chevrons.

**Behaviour implied**

- Type or edit a path; the list completes child directories of the typed path; ↩ on the first row (`shuv`, the typed directory itself) or on a child changes the pane's working directory (sends `cd` to the shell (inferred)) . Clear button empties the path.

---

## 128-insert-path.png

**What it is.** The Insert Path overlay (caption "Insert Path"; reached from a menu/command named Insert Path (inferred; the command itself is not in the visible palette rows)). Visually almost identical to 127 (pixel diff ~210 px, all in the field).

**Controls, verbatim**

1. Path field showing `~/` in grey placeholder colour with the blue caret at the start (empty field), magnifier glyph, **no clear button** (field is empty).
2. Section header `Go to` (same verbatim header as 127; not "Insert").
3. Identical directory list: `shuv` (selected, home-badged folder), `agent-tools`, `agent-ws`, `Applications`, `audits`, `backups`, `bin`, `ci`, `clawd`, `codex-a2a-workspace`, `Compositor-linux-reference`, `cua-dual-display-validation`, `Desktop`, `digup-test`, `Documents`, …

**Layout and visual spec**

- Same as 127.

**Behaviour implied**

- Browse to a path and insert it as text at the terminal prompt instead of changing directory (inferred from the name). The default root is `~/`.

---

## 129-theme-switcher.png

**What it is.** Theme switcher overlay (palette panel style), reached from View > Customize View > Color Scheme or a theme command (inferred). Lists built-in terminal themes.

**Controls, verbatim, in visual order**

1. Search field, placeholder `Select Theme…` (magnifier).
2. Theme rows (16 visible, alphabetical after the current one):
   - `Merino Dark` — selected (blue) with trailing `✓` (current theme)
   - `Abyssal Trench`
   - `Basalt Shore`
   - `Cathode Amber`
   - `Charcoal Atelier`
   - `Cinder Peak`
   - `Cobalt Foundry`
   - `Ember Hollow`
   - `Heliotrope Haze`
   - `Ink Meridian`
   - `Juniper Smoke`
   - `Midnight Tundra`
   - `Misty Forest`
   - `Moss Cathedral`
   - `Neon Arcade`
   - `Nocturne Rose`
   - (more below: a scrollbar is visible)
3. Overlay scrollbar at the right edge: thumb #99999A, ~5 pt wide at x ~864-869, from y ~157 to ~428 (~271 of ~455 pt track), so roughly 60% of the list is visible: about 26-27 themes total (inferred from thumb ratio).

**Layout and visual spec**

- Panel bounds same as the palette (x 542-871, y 113-614). The selected row pill stops at the scrollbar (x 547-866). Row pitch 28 pt. Checkmark right-aligned at x ~851, white.
- No icons/swatches per theme (names only).

**Behaviour implied**

- Filter by typing; selecting a row applies the theme; arrow keys likely preview live (inferred; not shown). The current theme is checkmarked and listed first.

---

## 130-status-bar-menu.png

**What it is.** The menu bar extra (status item) menu. The Rex status item is a white Rex-mascot (stylised creature head) glyph in the macOS menu bar, tree name `Rex server running` (menuitem, state `selected` while open; 24 x 24 at screen (1270, 3)). Clicking it opens this native menu (288 pt wide, screen x 1258, y ~31-227).

**Items, verbatim (tree + pixels), top to bottom**

1. Custom header row (48 pt high, has a submenu `>`): large circular app icon (rainbow gradient: magenta/orange/blue disc with the white mascot), title `Rex Server` (semibold ~13 pt, white), status line with a **green dot** (#28CF4D) and `Running` (grey). Trailing chevron `>`.
2. Separator.
3. Version info block (38 pt high custom view, tree text `Rex Beta, 0.1.0 (1932) · Bleeding Edge, Rex Server, 0.1.0-preview.15`), two-column grid:
   - `Rex Beta` (grey label) — `0.1.0 (1932) · Bleeding Edge` (lighter value)
   - `Rex Server` (grey label) — `0.1.0-preview.15`
   - Trailing button `Copy Version Info` (12 x 15, a "two overlapping documents" copy glyph, grey).
4. Separator.
5. `Check for Updates…` (no shortcut).
6. `Settings…` — `⌘,`
7. Separator.
8. `Quit Rex Beta` — `⌘Q`

**Layout and visual spec**

- Native menu material: dark translucent with a blue-tinted vibrancy from the desktop behind (sampled #1F303B-#303E49 in the header area, #2A2D31 lower down); corner radius ~10 pt; standard 24 pt items; separators 11 pt. Labels ~13 pt SF Pro. The header icon is ~32 pt diameter. Shortcut hints grey, right-aligned.
- The status item itself shows a rounded highlight pill behind the mascot while the menu is open.
- `evidence/130-update-no-dialog.png`: after `Check for Updates…` no panel or alert appeared in the window (the window looks unchanged). COVERAGE confirms "Check for Updates produced no visible panel".

**Behaviour implied**

- At-a-glance server status (running/stopped with coloured dot), version and channel info with one-click copy, update check, open Settings, quit the app.

---

## 131-status-bar-server-submenu.png

**What it is.** Same status menu with the pointer on the `Rex Server` header row, which highlights it and opens its submenu. COVERAGE.md says this submenu "did not open", but the full-display evidence `evidence/131-status-bar-server-submenu-display.png` **does show it open**; the cropped 131 screenshot simply cuts it off.

**Items, verbatim**

- Header row highlighted with the native accent blue **#0069FF** (radius ~8, inset), title `Rex Server`, `Running` with green dot, chevron `>` white.
- Submenu (to the right of the menu, top aligned with the header row), two rows, each with a leading copy glyph (two overlapping documents):
  - `unix:///Users/shu…/rex/server.sock`
  - `https://shuvbot-…ail586a6d.ts.net`
  (labels are middle-truncated with `…`, matching the tree names exactly.)

**Layout and visual spec**

- Submenu ~240 pt wide (inferred from the display crop), 2 x 24 pt rows, same translucent material, same 13 pt text.

**Behaviour implied**

- Lists the server's listening endpoints (local Unix socket and the tailnet HTTPS URL); clicking an entry copies the full address to the clipboard (inferred from the copy glyph).

---

## Feature inventory

- [overlay.style] Shared floating panel style: #1C1D20 body, #020203 outer hairline + #333436 inner highlight, ~10-12 pt radius, shadow; 30 pt search field #333437; 28 pt rows; selection pill #0055BF; grey shortcut hints #707072
- [session-switcher] Session switcher popover from the toolbar session chip: filter field `Filter or create…`, session list with ✓ on current and `⌘1`, `⌘2`… shortcuts
- [session-switcher.create] Type a non-existent name in the session filter to create a session
- [session-switcher.new-session] `New Session` row (⇧⌘N) in the switcher
- [session-switcher.add-remote-host] `Add Remote Host…` row in the switcher
- [palette] Command palette, centred 330 pt panel, search `Search commands, sessions, and tabs…` (searches commands, sessions and tabs)
- [palette.cmd.change-session] `Change Session…` ⌘K, opens a sub-step (>)
- [palette.cmd.rename-session] `Rename Session` (>)
- [palette.cmd.rename-tab] `Rename Tab` (>)
- [palette.cmd.add-remote-host] `Add Remote Host…` (>)
- [palette.cmd.new-session] `New Session` ⇧⌘N
- [palette.cmd.new-tab] `New Tab` ⌘T
- [palette.cmd.go-to-directory] `Go to Directory…` ⇧⌘G (>)
- [palette.cmd.new-window] `New Window` ⌘N
- [palette.cmd.move-tab-new-window] `Move Tab to New Window`
- [palette.cmd.close-tab] `Close Tab` ⌥⌘W
- [palette.cmd.previous-session] `Previous Session`
- [palette.cmd.next-session] `Next Session`
- [palette.cmd.refresh-sessions] `Refresh Sessions` ⌘R
- [palette.cmd.collapse-other-sessions] `Collapse Other Sessions`
- [palette.cmd.close-session] `Close Session`
- [palette.list-may-scroll] Further palette commands below `Close Session` possible but not captured (unknown)
- [palette.footer] Footer hints `Change Keybinding ⌘↩` and `Action ↩`
- [palette.rebind] ⌘↩ on a palette row edits that command's keybinding
- [palette.subpages] Rows with `>` push a second step inside the palette
- [activity] Activity popover from the toolbar eye button (button value On/Off), grouped by state (`Working`)
- [activity.row] Activity row: process icon, title (`Shell`), session/tab context subtitle, progress dots, elapsed time (`4m`)
- [activity.filter] `Filter` control at the bottom of the activity popover
- [tab-menu] Tab right-click menu: `Rename Tab…`, `Mute Tab`, `Move Tab Left` ⌃⇧↑, `Move Tab Right` ⌃⇧↓ (disabled at the ends / single tab), `Close Tab` ⌥⌘W
- [tab.mute] Per-tab mute
- [tab.reorder] Move tab left/right by keyboard (⌃⇧↑ / ⌃⇧↓)
- [rename-tab] Rename-tab overlay: field with the current title as placeholder, ↩ to apply
- [rename-tab.restore-default] `Restore Default Title` row (↩ when the field is empty)
- [terminal-menu] Terminal right-click menu: `Copy` (disabled without selection), `Paste`, `Split Right`, `Split Left`, `Split Down`, `Split Up`, `Float Pane`, `Zoom Pane`, `Close Pane`, `Reset Terminal`, `AutoFill >`
- [terminal-menu.split-four-directions] Split in all four directions (right/left/down/up) from the terminal menu
- [terminal.reset] `Reset Terminal` command
- [terminal.autofill] System AutoFill submenu: `Contact…`, `Passwords…`, `Credit Card…` (macOS-provided)
- [find] Terminal find bar: pill at the top-right of the pane, placeholder `Find`, previous (^) / next (v) buttons, close (×)
- [find.count] Match counter `current/total` (e.g. `1/1`)
- [find.nav-disabled-empty] Previous/next disabled until there are matches
- [find.dim] Terminal dims (#0F1216 → #080A0C, text dimmed) while results are shown
- [find.highlight] Match highlight: yellow #F4D722 rounded background with dark text
- [go-to-directory] `Go to Directory…` overlay: path field (`/Users/shuv/`), clear (×) button, `Go to` header, folder rows with Finder folder icons, typed directory first (home-badged), children alphabetical
- [insert-path] Insert Path overlay: same directory browser, placeholder `~/`, inserts a path (inferred)
- [theme-switcher] Theme switcher overlay: search `Select Theme…`, current theme first with ✓, scrollable list
- [theme.builtins] Built-in themes visible: Merino Dark (current), Abyssal Trench, Basalt Shore, Cathode Amber, Charcoal Atelier, Cinder Peak, Cobalt Foundry, Ember Hollow, Heliotrope Haze, Ink Meridian, Juniper Smoke, Midnight Tundra, Misty Forest, Moss Cathedral, Neon Arcade, Nocturne Rose, … (~26-27 total, inferred)
- [status-item] Menu bar extra (Rex mascot glyph), accessible name `Rex server running`
- [status-item.server-row] Header row: app icon, `Rex Server`, green dot + `Running`, submenu >
- [status-item.endpoints] Server submenu lists endpoints with copy glyphs: `unix:///…/rex/server.sock`, `https://…ts.net` (copy on click, inferred)
- [status-item.version] Version block: `Rex Beta 0.1.0 (1932) · Bleeding Edge`, `Rex Server 0.1.0-preview.15`
- [status-item.copy-version] `Copy Version Info` button
- [status-item.check-updates] `Check for Updates…` (no visible result panel in this capture)
- [status-item.settings] `Settings…` ⌘,
- [status-item.quit] `Quit Rex Beta` ⌘Q
