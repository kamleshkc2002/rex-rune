# B05 — Rex Beta 0.1.0 (1932) macOS: Settings > Shortcuts

Source: `rex-beta-ui-2026-11-10/screenshots/038-…051-*.png`, accessibility trees in `rex-beta-ui-2026-11-10/evidence/NNN-*.json`, captured Tue Nov 10 2026, 11:57:45 PM – 11:59:04 PM PST (window `w-669`, title "Settings - Shortcuts"). Rex Beta 0.1.0 (1932), Bleeding Edge; Rex Server 0.1.0-preview.15.

Conventions in this file:
- Coordinates are window-relative points. The PNGs are 928×962 at 1×; the 816×850 pt window sits at image offset (56, 38) with a drop-shadow margin around it. Screen-absolute AX x = window x + 1104, y = window y + 100.
- Binding notation: macOS glyphs as drawn (⌘ ⇧ ⌥ ⌃ ↩ ⇥ ⇞ ⇟ ← → ↑ ↓ ⌫), followed by the AX value verbatim in quotes, for example `⇧⌘W` ("Shift Command W").
- Row control kinds, taken from the AX button name prefix:
  - **Bound** — AX button "Change the shortcut for <Action>", value = binding. Drawn as a filled grey keycap pill.
  - **Unbound** — AX button "Record a shortcut for <Action>", no value. Drawn as a dim "Record Shortcut" pill.
  - **Not recordable** — AX button "Why this shortcut can't be recorded" (24×24). Drawn as an ⓘ icon at the right edge, with no pill.
  - **Parameterized** — a parent label with indented monospace sub-rows (`index: N`, `key: …`). Each sub-row has a read-only keycap-chip pair (AX role `unknown`, name = binding).

The list is a lazily laid out scroll view. The AX tree holds every row, including off-screen ones (`offscreen` state), so the complete row list below comes from the trees, cross-checked against the 14 screenshots. The final page (051) has scrollbar value `1` (end of list), and the last row is **Reload Config**.

---

## Shared chrome (identical in all 13 window captures 038, 040–051)

**Window.** "Settings - Shortcuts", 816×850 pt, dark appearance, rounded window corners (about 10–12 px, inferred from the image), large soft macOS shadow. The traffic lights are at the top-left (x≈18–80, y 18). AX: Close; Minimize **disabled** (drawn grey `#3x3x3x`); Zoom (green).

**Toolbar row (y 0–52), unified with the title bar:**
- Back/Forward segmented capsule at x 223–297, y 9–45 (two 38×36 buttons; capsule fill about `#292E33`). Glyphs are `‹` and `›` chevrons, with a thin vertical divider between them. **Back** is enabled (white chevron). **Forward** is **disabled** (dim chevron).
- Page title "Shortcuts" at x 300, semibold, about 15 pt, white. AX: statictext, description "Settings page".
- **Configuration** menu button at the top-right (AX `menubutton` "Configuration", 40×52 at x 772). Drawn as a 36 pt circle (fill about `#292E32`) with a white `•••` ellipsis glyph.

**Sidebar (x 0–214, AX outline "Settings categories", splitter value `214`, splitter disabled, so the width is fixed):**
- Background about `#0F1217`, slightly lighter than the content `#0B0F13`.
- "Search Settings" search field at the top (x 10–205, y 63–91), a rounded pill (radius about 10) with a magnifier glyph and grey placeholder.
- Group 1: **Server** — round, multicoloured app-icon avatar (about 28 pt), "Server" (13 pt, white) over "Running" (11 pt, grey). Row height 40.
- Group 2: **Interface** (icon: dark tile with three coloured dots), **Themes** (icon: rainbow palette circle).
- Group 3: **Terminal** (green `>_` tile), **Shortcuts** (grey ⌘ tile, **selected**: AX value "Selected"; the row has a rounded-rect highlight about `#25292E` at x 10–205, radius about 6, with bold label), **Integrations** (pink/purple tile), **Notifications** (red/orange bell tile), **Remote Hosts** (purple globe tile).
- Group 4: **Developer** (blue/grey hammer tile).
- Item rows are 32 pt pitch with 24 pt buttons. There is about a 13 pt gap between groups. Icons are about 20 pt squircle tiles, labels 13 pt regular, white.

**Content pane (x 215–816):**
- **Filter Shortcuts** search field (AX textfield "Filter Shortcuts"), x 235–795, y 63–91, full-width pill. Fill `#1C1D20`, hairline border lighter grey, radius about 14 (capsule). Magnifier glyph, placeholder "Filter Shortcuts…". It stays pinned while the list scrolls under it (in 041–051 rows are clipped under the field).
- **Helper paragraph** (top of the list, scrolls away) at x 245, 11 pt, grey (about `#8A9096`, inferred), two lines, verbatim:
  > A shortcut recorded here takes priority over file-based configuration. Search for a key, such as cmd+k, to see everything bound to it. The first result is the one the key performs.
- **Section headings** ("Global", "Window", …): 13 pt bold, white, x 245. There are 37 pt from heading baseline area to the first row, and about 57 pt from the last row of a card to the next heading.
- **Cards** (inset grouped): x 235–795 (560 wide), fill `#12161A` on page `#0B0F13`, corner radius about 8–10. Rows are separated by 1 px dividers `#26292D`, inset from the left edge by 10 pt (they start at the label x) and running to the right edge less 10 pt.
- **Row**: 39 pt pitch. Label 13 pt regular, `#E1EAF1`-ish white, x 245. Control pill 160×24 at x 626–786 (right-aligned with a 10 pt inset).
  - **Bound pill**: fill `#404549`, radius about 5, centred white glyph text (bold-ish SF symbols, e.g. "⇧⌘W"), text about `#E1EAF1`.
  - **Unbound pill**: fill `#1E2327` (barely above card), centred "Record Shortcut" text in dim grey `#696F74`, 12 pt.
  - **Multiple bindings**: extra 160×24 pills stacked below the first at +28 pt (the row grows to about 66 pt), e.g. Show Next Tab "⇧⌘]" and "⌃⇥".
  - **Not recordable**: ⓘ circle-i glyph (about 14 pt, grey `#6C7176`) at x about 774, the hit target 24×24 at x 762.
  - **Parameterized sub-rows**: label in **monospace** 11 pt grey (`#61676C`), e.g. `index: 1`. 30 pt pitch, no dividers between sub-rows. Right side has two small separate keycap chips (about 24×24 each, fill `#43484D`, radius about 5, glyph `#6A7075` grey, so dimmer than editable pills). Examples: `⌘` + `1`, or `⌘` + `⌫`. The chips are not buttons (AX role `unknown`) and look read-only.
- **Scrollbar**: an overlay scroller at the right edge (x 798–817), auto-hidden (AX `hidden` in 038). When visible it is a 6 pt thin rounded grey thumb (about 106–109 pt long) on a dark track. AX scrollbar value 0 → 1 across pages.

---

## 038 — `038-settings-shortcuts.png`: Shortcuts page, top

**What it is.** Settings window, **Shortcuts** category, scrolled to the top. Reached with Settings… (⌘,) → sidebar "Shortcuts".

**Visible controls, in order:**
1. Filter Shortcuts field (empty, placeholder "Filter Shortcuts…").
2. Helper paragraph (quoted above).
3. **Global** heading → card with one row:
   - Toggle Visibility — "Record Shortcut" (unbound).
   - Helper text *below the card* (outside it), 11 pt grey: "Works from any app: shows Rex, or hides it when it's in front."
4. **Window** heading → card:
   - New Window — ⌘N ("Command N")
   - Close Window — ⇧⌘W ("Shift Command W")
   - Toggle Full Screen — ⌃⌘F ("Control Command F")
   - Close — ⌘W ("Command W")
5. **App** heading → card (continues below the fold):
   - Command Palette… — ⇧⌘P
   - Settings… — ⌘,
   - Rex Diagnostics — ⇧⌘,
   - Open Onboarding Session… — Record Shortcut
   - Grant System Permissions… — Record Shortcut
   - Copy Version Info — Record Shortcut
   - Copy Debug Info — Record Shortcut
   - Pair Device with QR Code… — Record Shortcut
   - Install Update — ⇧⌘U (partially clipped at the window bottom)

**Layout.** Help text y 108–134. Heading "Global" y 164. Global card y 128–166 (one row). Note at y 236. "Window" heading y 279. Card rows at 316/355/394/433. "App" heading y 490. Rows start y 527. All values are window-relative.

**Colours sampled.** Page `#0B0F13`; card `#12161A`; divider `#26292D`; bound pill `#404549`; unbound pill `#1E2327`; unbound text `#696F74`; bound glyph text `#E1EAF1`; heading text white; sidebar `#0F1217`; filter field `#1C1D20`.

**Behaviour implied.**
- Filtering searches by action name *and* by key ("Search for a key, such as cmd+k, to see everything bound to it"). When several actions share a key, they are listed in priority order and "The first result is the one the key performs".
- Shortcuts recorded in the GUI override the file-based (Lua) configuration.
- Toggle Visibility is a system-wide hotkey ("Works from any app").

---

## 039 — `039-menu-shortcuts-configuration.png`: Configuration (•••) menu

**What it is.** The menu that opens from the circular `•••` **Configuration** button at the top-right of the Shortcuts page. The crop is 219×144. The full-display evidence (`039-…-display.png`) shows it anchored under the button, right-aligned to the window edge, overlapping the Filter field. The window was at screen (1104, 100) on a 1920×1080 display.

**Menu items (AX `menu` "Configuration", 205×128, item height 24, separators 11):**
1. **No GUI Overrides Yet** — *disabled* (grey). Acts as a status line saying there are no recorded overrides.
2. — separator —
3. **Copy Configuration File Path** — enabled (white).
4. **Copy Shortcuts as Lua** — *disabled* (grey; inferred to be disabled because there are no overrides to export).
5. — separator —
6. **Reset All…** — *disabled* (grey).

**Visual.** Standard macOS dark menu with translucent material, fill about `#2B3035`, corner radius about 8 and a soft shadow. Enabled text `#D9D9DA`, 13 pt; disabled text about 45% grey. The separators are 1 px hairlines inset about 10 pt. There are no shortcut hints and no icons.

**Behaviour implied.** The menu shows whether GUI overrides exist. It copies the config file path, exports GUI-recorded shortcuts as a Lua snippet (for pasting into the file-based config), and resets all overrides (with confirmation, implied by the "…").

---

## 040 — `040-shortcut-recorder.png`: inline shortcut recorder

**What it is.** The same page as 038 after the user clicks the **New Window** ⌘N pill. The pill turns into an inline recorder field in place.

**Recorder control** (replaces the 160×24 pill at x 626–786, y 312–336):
- Text field with placeholder **"Type Shortcut…"** in grey and field fill `#1C1D20`.
- **Focus ring**: a 2–3 px blue system-accent ring (`#144D93` outer / `#094389`), radius about 6, drawn around the field.
- Two trailing circular icon buttons (about 14 pt):
  - **Orange circle with a white counter-clockwise "undo/revert" arrow** (`#EE7E2D`). AX button **"Cancel Recording"** (17×24 at x 747).
  - **Grey circle with a dark ✕** (about `#5A5F64`). AX button **"Remove Command N from New Window"** (17×24 at x 764).
- In the AX tree the New Window row has no "Change the shortcut…" button while recording. It has only these two buttons.

The rest of the page is unchanged.

**Behaviour implied.** Clicking a bound pill enters recording mode. The next key combination typed becomes the binding. The orange button cancels (restoring the previous binding) and ✕ removes the existing binding, leaving the action unbound. Clicking "Record Shortcut" on an unbound row presumably opens the same recorder (inferred).

---

## 041 — `041-shortcuts-page-02.png`: scrolled, App section (lower)

Scroll position: the scrollbar thumb is at about 17%. Rows are clipped under the filter field at the top ("Settings…" ⌘, partially hidden).

Visible rows (App card, continued):
- Rex Diagnostics — ⇧⌘,
- Open Onboarding Session… — Record Shortcut
- Grant System Permissions… — Record Shortcut
- Copy Version Info — Record Shortcut
- Copy Debug Info — Record Shortcut
- Pair Device with QR Code… — Record Shortcut
- Install Update — ⇧⌘U
- Relaunch when Idle — ⌥⇧⌘U ("Option Shift Command U")
- Check for Updates in Background — Record Shortcut
- Report Update Status — Record Shortcut
- Install Update and Relaunch — Record Shortcut
- Set Update Automation — ⓘ (not recordable)
- Set Update Channel — ⓘ (not recordable)
- Dismiss Notice — ⇧⌘. ("Shift Command .")
- Change Shortcut… — Record Shortcut
- Install Rex for Agents — Record Shortcut
- Restart Server… — Record Shortcut

Then the **Session** heading, followed by New Session — ⇧⌘N (clipped at the bottom).

---

## 042 — `042-shortcuts-page-03.png`: Session section

Visible: Set Update Channel ⓘ (top, partially clipped), Dismiss Notice ⇧⌘., Change Shortcut… Record, Install Rex for Agents Record, Restart Server… Record (end of App card).

**Session** card:
- New Session — ⇧⌘N ("Shift Command N")
- Change Session… — ⌘K ("Command K")
- Go to Session — ⓘ (not recordable)
- Next Session — Record Shortcut
- Previous Session — Record Shortcut
- Rename Session — Record Shortcut
- Close Session — Record Shortcut
- Refresh Sessions — ⌘R ("Command R")
- Collapse Session — Record Shortcut
- Expand Session — Record Shortcut
- Collapse Other Sessions — Record Shortcut

**Host** heading, then Switch Host — Record Shortcut (clipped at the bottom).

---

## 043 — `043-shortcuts-page-04.png`: Host and Tab sections

Visible: Close Session Record (top), Refresh Sessions ⌘R, Collapse Session, Expand Session, Collapse Other Sessions (Record).

**Host** card:
- Switch Host — Record Shortcut
- Add Remote Host… — Record Shortcut
- Remove Host… — Record Shortcut

**Tab** card:
- New Tab — ⌘T ("Command T")
- Close Tab — ⌥⌘W ("Option Command W")
- Close Other Tabs — Record Shortcut
- Rename Tab — Record Shortcut
- Show Next Tab — **two pills**: ⇧⌘] ("Shift Command ]") and ⌃⇥ ("Control Tab")
- Show Previous Tab — **two pills**: ⇧⌘[ ("Shift Command [") and ⌃⇧⇥ ("Control Shift Tab")
- Go to Tab — parameterized parent (no control on the parent row); the first sub-row `index: 1` has chips ⌘ 1 (clipped).

This is the first screenshot showing multi-binding rows: the label stays top-aligned with the first pill, and the second pill sits 28 pt below.

---

## 044 — `044-shortcuts-page-05.png`: Tab section (Go to Tab, Move Tab)

Visible: Close Other Tabs, Rename Tab (Record), Show Next Tab (⇧⌘] / ⌃⇥), Show Previous Tab (⇧⌘[ / ⌃⇧⇥).

**Go to Tab** (parameterized; sub-rows in monospace grey, chips read-only):
| sub-row label (verbatim) | chips | AX name |
|---|---|---|
| `index: 1` | ⌘ 1 | Command 1 |
| `index: 2` | ⌘ 2 | Command 2 |
| `index: 3` | ⌘ 3 | Command 3 |
| `index: 4` | ⌘ 4 | Command 4 |
| `index: 5` | ⌘ 5 | Command 5 |
| `index: 6` | ⌘ 6 | Command 6 |
| `index: 7` | ⌘ 7 | Command 7 |
| `index: 8` | ⌘ 8 | Command 8 |
| `index: -1` | ⌘ 9 | Command 9 |

(`index: -1` means the last tab, inferred.)

Then:
- Move Tab — ⓘ (not recordable)
- Move Tab Backward — ⌃⇧⇞ ("Control Shift Page Up")
- Move Tab Forward — ⌃⇧⇟ ("Control Shift Page Down")
- Move Tab to New Window — Record Shortcut

**Pane** heading, then Split Pane ⓘ (clipped).

The chip visual: two separate rounded squares about 24 pt, fill `#43484D`, glyph `#6A7075` (dim), 4 pt gap, right-aligned at x 786. Sub-row labels are `#61676C` monospace about 11 pt.

---

## 045 — `045-shortcuts-page-06.png`: Pane section, part 1

Visible top: `index: -1` ⌘ 9; Move Tab ⓘ; Move Tab Backward ⌃⇧⇞; Move Tab Forward ⌃⇧⇟; Move Tab to New Window Record.

**Pane** card:
- Split Pane — ⓘ (not recordable)
- Split Pane Left — ⇧⌘← ("Shift Command Left Arrow")
- Split Pane Right — **two pills**: ⇧⌘→ ("Shift Command Right Arrow") and ⌘D ("Command D")
- Split Pane Up — ⇧⌘↑ ("Shift Command Up Arrow")
- Split Pane Down — **two pills**: ⇧⌘↓ ("Shift Command Down Arrow") and ⇧⌘D ("Shift Command D")
- Split Pane Best Fit — ⌘↩ ("Command Return")
- Focus Pane — ⓘ (not recordable)
- Focus Pane Left — ⌥⌘← ("Option Command Left Arrow")
- Focus Pane Right — ⌥⌘→
- Focus Pane Up — ⌥⌘↑
- Focus Pane Down — ⌥⌘↓
- Focus Next Pane — ⌘] (clipped at the bottom)

---

## 046 — `046-shortcuts-page-07.png`: Pane section, part 2

Visible:
- Split Pane Best Fit — ⌘↩
- Focus Pane — ⓘ
- Focus Pane Left — ⌥⌘←
- Focus Pane Right — ⌥⌘→
- Focus Pane Up — ⌥⌘↑
- Focus Pane Down — ⌥⌘↓
- Focus Next Pane — ⌘] ("Command ]")
- Focus Previous Pane — ⌘[ ("Command [")
- Move Pane — ⓘ (not recordable)
- Move Pane Left — ⌥⇧⌘← ("Option Shift Command Left Arrow")
- Move Pane Right — ⌥⇧⌘→
- Move Pane Up — ⌥⇧⌘↑
- Move Pane Down — ⌥⇧⌘↓
- Float Pane — ⌥⌘↩ ("Option Command Return")
- New Floating Pane — Record Shortcut
- Minimize Floating Window — ⌥⌘M ("Option Command M")
- Minimize Floating Windows — ⌥⇧⌘↩ ("Option Shift Command Return")
- Zoom Pane — ⇧⌘↩ ("Shift Command Return")
- Close Pane — ⌘W ("Command W")

Note: **⌘W is bound to both "Close" (Window group) and "Close Pane"**. This is the kind of conflict the helper text addresses: searching "cmd+w" would list both, with the first one winning.

---

## 047 — `047-shortcuts-page-08.png`: Pane section, part 3

Visible:
- Move Pane Down — ⌥⇧⌘↓ (top)
- Float Pane — ⌥⌘↩
- New Floating Pane — Record Shortcut
- Minimize Floating Window — ⌥⌘M
- Minimize Floating Windows — ⌥⇧⌘↩
- Zoom Pane — ⇧⌘↩
- Close Pane — ⌘W
- Close Other Panes — Record Shortcut
- Balance Splits — Record Shortcut
- Resize Pane — ⓘ (not recordable)
- Move Pane to New Tab — Record Shortcut
- Move Pane to Tab… — Record Shortcut
- Go to Directory… — ⇧⌘G ("Shift Command G")
- Go to Anything… — ⌘P ("Command P")
- View Activity… — Record Shortcut
- Insert Path… — ⌃⌘I ("Control Command I")
- Copy Pane ID — Record Shortcut
- Rename Pane — Record Shortcut
- Paste into Pane — Record Shortcut
- Copy from Pane — Record Shortcut (clipped)

Note: ⇧⌘G is bound to both **Go to Directory…** (Pane) and **Find Previous** (Find).

---

## 048 — `048-shortcuts-page-09.png`: Pane section, part 4 (Send Key)

Visible:
- Go to Anything… — ⌘P
- View Activity… — Record Shortcut
- Insert Path… — ⌃⌘I
- Copy Pane ID — Record Shortcut
- Rename Pane — Record Shortcut
- Paste into Pane — Record Shortcut
- Copy from Pane — Record Shortcut
- Select All in Pane — Record Shortcut
- **Send Key** — parameterized parent. Its sub-rows are read-only chip pairs, mapping a Mac key to a terminal key sent to the PTY:

| sub-row label (verbatim, mono) | chips | AX name |
|---|---|---|
| `key: ctrl+u` | ⌘ ⌫ | Command Delete |
| `key: ctrl+a` | ⌘ ← | Command Left Arrow |
| `key: ctrl+e` | ⌘ → | Command Right Arrow |
| `key: alt+b` | ⌥ ← | Option Left Arrow |
| `key: alt+f` | ⌥ → | Option Right Arrow |

- Send Key to Server — Record Shortcut
- Jump to Previous Prompt — ⌘↑ ("Command Up Arrow")
- Jump to Next Prompt — ⌘↓ ("Command Down Arrow")
- Scroll Page Up — Record Shortcut
- Scroll Page Down — Record Shortcut
- Quick Look — ⌘Y ("Command Y")
- Clear Terminal — Record Shortcut (clipped)

---

## 049 — `049-shortcuts-page-10.png`: Keys and Find sections

Visible top: `key: alt+f` ⌥→ (clipped). Then:
- Send Key to Server — Record Shortcut
- Jump to Previous Prompt — ⌘↑
- Jump to Next Prompt — ⌘↓
- Scroll Page Up — Record Shortcut
- Scroll Page Down — Record Shortcut
- Quick Look — ⌘Y
- Clear Terminal — Record Shortcut
- Reset Terminal — Record Shortcut (end of the Pane card)

**Keys** card:
- Enter Mode — ⓘ (not recordable)
- Exit Mode — Record Shortcut
- Exit All Modes — Record Shortcut

**Find** card:
- Find… — ⌘F ("Command F")
- Find Next — ⌘G ("Command G")
- Find Previous — ⇧⌘G ("Shift Command G")

**View** heading, then Increase Font Size ⌘= (clipped).

---

## 050 — `050-shortcuts-page-11.png`: View section, part 1

**Find** card (full): Find… ⌘F; Find Next ⌘G; Find Previous ⇧⌘G.

**View** card:
- Increase Font Size — **two pills**: ⌘= ("Command =") and ⌘+ ("Command +")
- Decrease Font Size — ⌘- ("Command -")
- Reset Font Size — ⌘0 ("Command 0")
- Toggle Sidebar — ⌘/ ("Command /")
- Toggle Activity — ⌘. ("Command .")
- Show Activity by Session — Record Shortcut
- Show Activity by Priority — Record Shortcut
- Focus Tab Filter — Record Shortcut
- Toggle Toolbar Layout — ⇧⌘S ("Shift Command S")
- Reveal Toolbar — ⌥⌘T ("Option Command T")
- Reveal Pane Header — ⌥⇧⌘T ("Option Shift Command T")
- Change Theme… — Record Shortcut
- Import Theme File — ⓘ (not recordable; clipped)

---

## 051 — `051-shortcuts-page-12.png`: View section, end of list

Visible: Find Previous ⇧⌘G (top), the **View** card in full as in 050, plus:
- Import Theme File — ⓘ (not recordable)
- Switch Appearance Mode — ⓘ (not recordable)
- Copy Config File Path — Record Shortcut
- Reload Config — Record Shortcut

This is the end of the list: the card's bottom edge is visible at y about 828, with about 20 pt of bottom padding. The scrollbar is at the bottom (AX value 1).

---

## Complete shortcut table (all 12 groups, every row)

Kind: **B** = bound (editable), **U** = unbound ("Record Shortcut"), **ⓘ** = not recordable ("Why this shortcut can't be recorded"), **P** = parameterized sub-row (read-only chips).

### Global
| Action | Binding | Kind |
|---|---|---|
| Toggle Visibility | — | U (note: "Works from any app: shows Rex, or hides it when it's in front.") |

### Window
| Action | Binding | Kind |
|---|---|---|
| New Window | ⌘N | B |
| Close Window | ⇧⌘W | B |
| Toggle Full Screen | ⌃⌘F | B |
| Close | ⌘W | B |

### App
| Action | Binding | Kind |
|---|---|---|
| Command Palette… | ⇧⌘P | B |
| Settings… | ⌘, | B |
| Rex Diagnostics | ⇧⌘, | B |
| Open Onboarding Session… | — | U |
| Grant System Permissions… | — | U |
| Copy Version Info | — | U |
| Copy Debug Info | — | U |
| Pair Device with QR Code… | — | U |
| Install Update | ⇧⌘U | B |
| Relaunch when Idle | ⌥⇧⌘U | B |
| Check for Updates in Background | — | U |
| Report Update Status | — | U |
| Install Update and Relaunch | — | U |
| Set Update Automation | — | ⓘ |
| Set Update Channel | — | ⓘ |
| Dismiss Notice | ⇧⌘. | B |
| Change Shortcut… | — | U |
| Install Rex for Agents | — | U |
| Restart Server… | — | U |

### Session
| Action | Binding | Kind |
|---|---|---|
| New Session | ⇧⌘N | B |
| Change Session… | ⌘K | B |
| Go to Session | — | ⓘ |
| Next Session | — | U |
| Previous Session | — | U |
| Rename Session | — | U |
| Close Session | — | U |
| Refresh Sessions | ⌘R | B |
| Collapse Session | — | U |
| Expand Session | — | U |
| Collapse Other Sessions | — | U |

### Host
| Action | Binding | Kind |
|---|---|---|
| Switch Host | — | U |
| Add Remote Host… | — | U |
| Remove Host… | — | U |

### Tab
| Action | Binding | Kind |
|---|---|---|
| New Tab | ⌘T | B |
| Close Tab | ⌥⌘W | B |
| Close Other Tabs | — | U |
| Rename Tab | — | U |
| Show Next Tab | ⇧⌘] ; ⌃⇥ | B (2) |
| Show Previous Tab | ⇧⌘[ ; ⌃⇧⇥ | B (2) |
| Go to Tab › `index: 1` … `index: 8` | ⌘1 … ⌘8 | P |
| Go to Tab › `index: -1` | ⌘9 | P |
| Move Tab | — | ⓘ |
| Move Tab Backward | ⌃⇧⇞ (Control Shift Page Up) | B |
| Move Tab Forward | ⌃⇧⇟ (Control Shift Page Down) | B |
| Move Tab to New Window | — | U |

### Pane
| Action | Binding | Kind |
|---|---|---|
| Split Pane | — | ⓘ |
| Split Pane Left | ⇧⌘← | B |
| Split Pane Right | ⇧⌘→ ; ⌘D | B (2) |
| Split Pane Up | ⇧⌘↑ | B |
| Split Pane Down | ⇧⌘↓ ; ⇧⌘D | B (2) |
| Split Pane Best Fit | ⌘↩ | B |
| Focus Pane | — | ⓘ |
| Focus Pane Left | ⌥⌘← | B |
| Focus Pane Right | ⌥⌘→ | B |
| Focus Pane Up | ⌥⌘↑ | B |
| Focus Pane Down | ⌥⌘↓ | B |
| Focus Next Pane | ⌘] | B |
| Focus Previous Pane | ⌘[ | B |
| Move Pane | — | ⓘ |
| Move Pane Left | ⌥⇧⌘← | B |
| Move Pane Right | ⌥⇧⌘→ | B |
| Move Pane Up | ⌥⇧⌘↑ | B |
| Move Pane Down | ⌥⇧⌘↓ | B |
| Float Pane | ⌥⌘↩ | B |
| New Floating Pane | — | U |
| Minimize Floating Window | ⌥⌘M | B |
| Minimize Floating Windows | ⌥⇧⌘↩ | B |
| Zoom Pane | ⇧⌘↩ | B |
| Close Pane | ⌘W | B |
| Close Other Panes | — | U |
| Balance Splits | — | U |
| Resize Pane | — | ⓘ |
| Move Pane to New Tab | — | U |
| Move Pane to Tab… | — | U |
| Go to Directory… | ⇧⌘G | B |
| Go to Anything… | ⌘P | B |
| View Activity… | — | U |
| Insert Path… | ⌃⌘I | B |
| Copy Pane ID | — | U |
| Rename Pane | — | U |
| Paste into Pane | — | U |
| Copy from Pane | — | U |
| Select All in Pane | — | U |
| Send Key › `key: ctrl+u` | ⌘⌫ | P |
| Send Key › `key: ctrl+a` | ⌘← | P |
| Send Key › `key: ctrl+e` | ⌘→ | P |
| Send Key › `key: alt+b` | ⌥← | P |
| Send Key › `key: alt+f` | ⌥→ | P |
| Send Key to Server | — | U |
| Jump to Previous Prompt | ⌘↑ | B |
| Jump to Next Prompt | ⌘↓ | B |
| Scroll Page Up | — | U |
| Scroll Page Down | — | U |
| Quick Look | ⌘Y | B |
| Clear Terminal | — | U |
| Reset Terminal | — | U |

### Keys
| Action | Binding | Kind |
|---|---|---|
| Enter Mode | — | ⓘ |
| Exit Mode | — | U |
| Exit All Modes | — | U |

### Find
| Action | Binding | Kind |
|---|---|---|
| Find… | ⌘F | B |
| Find Next | ⌘G | B |
| Find Previous | ⇧⌘G | B |

### View
| Action | Binding | Kind |
|---|---|---|
| Increase Font Size | ⌘= ; ⌘+ | B (2) |
| Decrease Font Size | ⌘- | B |
| Reset Font Size | ⌘0 | B |
| Toggle Sidebar | ⌘/ | B |
| Toggle Activity | ⌘. | B |
| Show Activity by Session | — | U |
| Show Activity by Priority | — | U |
| Focus Tab Filter | — | U |
| Toggle Toolbar Layout | ⇧⌘S | B |
| Reveal Toolbar | ⌥⌘T | B |
| Reveal Pane Header | ⌥⇧⌘T | B |
| Change Theme… | — | U |
| Import Theme File | — | ⓘ |
| Switch Appearance Mode | — | ⓘ |
| Copy Config File Path | — | U |
| Reload Config | — | U |

Totals (counted from the AX trees and the tables above): 10 groups (Global, Window, App, Session, Host, Tab, Pane, Keys, Find, View) and 118 top-level rows:
- 56 bound actions (61 binding pills, because 5 actions have a second binding)
- 49 unbound
- 11 not recordable
- 2 parameterized parents: Go to Tab with 9 sub-rows and Send Key with 5

By group, in bound/unbound/ⓘ order: Global 0/1/0, Window 4/0/0, App 6/11/2, Session 3/7/1, Host 0/3/0, Tab 6/3/1 (+Go to Tab), Pane 26/16/4 (+Send Key), Keys 0/2/1, Find 3/0/0, View 8/6/2.

The capture's title "12 pages" refers to 12 scroll positions of one list (038 plus 041–051), not 12 groups.

Duplicate bindings in the defaults: ⌘W (Close, Close Pane) and ⇧⌘G (Go to Directory…, Find Previous). ⌘. (Toggle Activity) and ⇧⌘. (Dismiss Notice) are distinct.

The 11 "Why this shortcut can't be recorded" rows are: Set Update Automation, Set Update Channel, Go to Session, Move Tab, Split Pane, Focus Pane, Move Pane, Resize Pane, Enter Mode, Import Theme File, Switch Appearance Mode. They look like actions that need an argument (channel, mode, index, direction, file) (inferred). The ⓘ popover was not opened in any capture, so its text is unknown.

---

## Feature inventory

- [settings.shortcuts.page] Settings category "Shortcuts" (⌘ sidebar icon) listing every app action, grouped under Global, Window, App, Session, Host, Tab, Pane, Keys, Find, View
- [settings.shortcuts.filter] "Filter Shortcuts…" search field pinned above the list; matches action names and keys (e.g. "cmd+k" lists everything bound to it, first result = the one the key performs)
- [settings.shortcuts.priority-note] Helper text: GUI-recorded shortcuts take priority over file-based configuration
- [settings.shortcuts.row.bound] Bound action shows a keycap pill with macOS glyphs; click it to re-record
- [settings.shortcuts.row.unbound] Unbound action shows a dim "Record Shortcut" pill; click it to record
- [settings.shortcuts.row.multi] Actions can carry several bindings, shown as stacked pills (Show Next/Previous Tab, Split Pane Right/Down, Increase Font Size)
- [settings.shortcuts.row.not-recordable] Parameterized actions show an ⓘ "Why this shortcut can't be recorded" button instead of a recorder
- [settings.shortcuts.row.parameterized] Parameterized bindings (Go to Tab index, Send Key key) are shown as read-only monospace sub-rows with keycap chips
- [settings.shortcuts.recorder] Inline recorder: the pill becomes a "Type Shortcut…" field with an accent focus ring, an orange Cancel Recording (revert) button and a grey ✕ "Remove <binding> from <action>" button
- [settings.shortcuts.global-hotkey] Global "Toggle Visibility" system-wide hotkey (unbound by default) with note "Works from any app: shows Rex, or hides it when it's in front."
- [settings.shortcuts.config-menu] ••• Configuration menu: "No GUI Overrides Yet" status (disabled), Copy Configuration File Path, Copy Shortcuts as Lua (disabled with no overrides), Reset All… (disabled with no overrides)
- [shortcuts.window] New Window ⌘N; Close Window ⇧⌘W; Toggle Full Screen ⌃⌘F; Close ⌘W
- [shortcuts.app.palette] Command Palette… ⇧⌘P
- [shortcuts.app.settings] Settings… ⌘,
- [shortcuts.app.diagnostics] Rex Diagnostics ⇧⌘,
- [shortcuts.app.onboarding] Open Onboarding Session… (unbound)
- [shortcuts.app.permissions] Grant System Permissions… (unbound)
- [shortcuts.app.copy-info] Copy Version Info; Copy Debug Info (unbound)
- [shortcuts.app.pair-qr] Pair Device with QR Code… (unbound)
- [shortcuts.app.updates] Install Update ⇧⌘U; Relaunch when Idle ⌥⇧⌘U; Check for Updates in Background, Report Update Status, Install Update and Relaunch (unbound); Set Update Automation, Set Update Channel (not recordable)
- [shortcuts.app.dismiss-notice] Dismiss Notice ⇧⌘.
- [shortcuts.app.change-shortcut] Change Shortcut… (unbound; a command-palette entry for rebinding, inferred)
- [shortcuts.app.install-agents] Install Rex for Agents (unbound)
- [shortcuts.app.restart-server] Restart Server… (unbound)
- [shortcuts.session.new] New Session ⇧⌘N
- [shortcuts.session.change] Change Session… ⌘K (session switcher)
- [shortcuts.session.goto] Go to Session (not recordable, parameterized)
- [shortcuts.session.cycle] Next Session; Previous Session (unbound)
- [shortcuts.session.manage] Rename Session; Close Session (unbound); Refresh Sessions ⌘R
- [shortcuts.session.collapse] Collapse Session; Expand Session; Collapse Other Sessions (unbound)
- [shortcuts.host] Switch Host; Add Remote Host…; Remove Host… (unbound)
- [shortcuts.tab.new-close] New Tab ⌘T; Close Tab ⌥⌘W; Close Other Tabs (unbound); Rename Tab (unbound)
- [shortcuts.tab.cycle] Show Next Tab ⇧⌘] and ⌃⇥; Show Previous Tab ⇧⌘[ and ⌃⇧⇥
- [shortcuts.tab.goto-index] Go to Tab index 1–8 = ⌘1–⌘8, index -1 (last) = ⌘9
- [shortcuts.tab.move] Move Tab (not recordable); Move Tab Backward ⌃⇧PageUp; Move Tab Forward ⌃⇧PageDown; Move Tab to New Window (unbound)
- [shortcuts.pane.split] Split Pane (not recordable); Split Pane Left ⇧⌘←; Right ⇧⌘→ and ⌘D; Up ⇧⌘↑; Down ⇧⌘↓ and ⇧⌘D; Split Pane Best Fit ⌘↩
- [shortcuts.pane.focus] Focus Pane (not recordable); Focus Pane Left/Right/Up/Down ⌥⌘←/→/↑/↓; Focus Next Pane ⌘]; Focus Previous Pane ⌘[
- [shortcuts.pane.move] Move Pane (not recordable); Move Pane Left/Right/Up/Down ⌥⇧⌘←/→/↑/↓
- [shortcuts.pane.floating] Float Pane ⌥⌘↩; New Floating Pane (unbound); Minimize Floating Window ⌥⌘M; Minimize Floating Windows ⌥⇧⌘↩
- [shortcuts.pane.zoom] Zoom Pane ⇧⌘↩
- [shortcuts.pane.close] Close Pane ⌘W; Close Other Panes (unbound)
- [shortcuts.pane.balance-resize] Balance Splits (unbound); Resize Pane (not recordable)
- [shortcuts.pane.move-to-tab] Move Pane to New Tab; Move Pane to Tab… (unbound)
- [shortcuts.pane.goto-directory] Go to Directory… ⇧⌘G
- [shortcuts.pane.goto-anything] Go to Anything… ⌘P
- [shortcuts.pane.view-activity] View Activity… (unbound)
- [shortcuts.pane.insert-path] Insert Path… ⌃⌘I
- [shortcuts.pane.ids-rename] Copy Pane ID; Rename Pane (unbound)
- [shortcuts.pane.clipboard] Paste into Pane; Copy from Pane; Select All in Pane (unbound)
- [shortcuts.pane.send-key] Send Key mappings: ⌘⌫→ctrl+u, ⌘←→ctrl+a, ⌘→→ctrl+e, ⌥←→alt+b, ⌥→→alt+f
- [shortcuts.pane.send-key-server] Send Key to Server (unbound)
- [shortcuts.pane.prompt-jump] Jump to Previous Prompt ⌘↑; Jump to Next Prompt ⌘↓
- [shortcuts.pane.scroll] Scroll Page Up; Scroll Page Down (unbound)
- [shortcuts.pane.quick-look] Quick Look ⌘Y
- [shortcuts.pane.clear-reset] Clear Terminal; Reset Terminal (unbound)
- [shortcuts.keys.modes] Enter Mode (not recordable); Exit Mode; Exit All Modes (unbound): modal key tables
- [shortcuts.find] Find… ⌘F; Find Next ⌘G; Find Previous ⇧⌘G
- [shortcuts.view.font-size] Increase Font Size ⌘= and ⌘+; Decrease Font Size ⌘-; Reset Font Size ⌘0
- [shortcuts.view.sidebar] Toggle Sidebar ⌘/
- [shortcuts.view.activity] Toggle Activity ⌘.; Show Activity by Session; Show Activity by Priority (unbound)
- [shortcuts.view.tab-filter] Focus Tab Filter (unbound)
- [shortcuts.view.toolbar] Toggle Toolbar Layout ⇧⌘S; Reveal Toolbar ⌥⌘T; Reveal Pane Header ⌥⇧⌘T
- [shortcuts.view.theme] Change Theme… (unbound); Import Theme File, Switch Appearance Mode (not recordable)
- [shortcuts.view.config] Copy Config File Path; Reload Config (unbound)
