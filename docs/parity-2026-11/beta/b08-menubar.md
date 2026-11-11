# B08 — Rex Beta menu bar (File, Edit, View, Window, Help and all submenus)

Reference product: Rex Beta 0.1.0 (1932), Bleeding Edge, macOS, with Rex Server 0.1.0-preview.15. Captured 2026-11-11 about 00:01 PST with agent-desktop (from `rex-beta-ui-2026-11-10/captures.jsonl`).

Sources: `rex-beta-ui-2026-11-10/screenshots/072…088-*.png` (menu crops at 1x, 1 px = 1 pt), the matching `evidence/*.json` accessibility trees (labels, roles, disabled state, bounds in points, AppKit selectors in `native_id`), and `evidence/*-display.png` (full 1920x1080 pt display). Shortcuts, checkmarks, icons and section headers come only from the pixels, because the trees do not record key equivalents or check state. Anything not read directly is marked (inferred).

The Rex Beta app menu (the bold "Rex Beta" title) was **not** captured in this set. These images and trees do not cover it.

## Conventions used below

- `⌘` Command, `⇧` Shift, `⌥` Option, `^` Control, `fn` Globe/fn key. `◀ ▶ ▲ ▼` are the arrow-key glyphs exactly as drawn (solid triangles). `↩` Return, `⇥` Tab (drawn as a right arrow to a bar), `↑ ↓` thin arrows.
- `---` is a separator (an 11 pt tall disabled `menuitem` with no name in the tree).
- **[disabled]** means greyed out in the image and `disabled` in the tree.
- `✓` is a checkmark drawn in the left gutter.
- `›` is the submenu chevron at the right edge.
- Selector: the AppKit action from `native_id`. `menuAction:` is Rex's own generic action handler. Other selectors are standard AppKit or system items.

## Common visual spec (applies to every menu here)

- **Menu bar** (from `072-menu-file-display.png`, 1920x1080 pt): black bar, white text. Order: Apple logo, **Rex Beta** (bold app name), File, Edit, View, Window, Help. The open title gets a rounded grey pill highlight (about 34x22 pt behind "File" or "View"). Top-level x positions from tree bounds: File menu at x=114, Edit at x=156, View at x=199, Window at x=248, Help at x=316. Menus drop to y=31.
- **Menu panel**: the standard macOS 26-style dark menu. It is translucent, with blur and vibrancy: it shows a blue tint where it sits over the blue terminal and toolbar chrome (sampled `#2B4153` near the top) and neutral grey elsewhere (sampled `#323233` to `#363636`). The outer edge is a 1 px dark line (sampled about `#151415` just outside) plus a soft shadow. Corner radius is large, about 12 pt (inferred from the crop). There are about 5 pt of vertical inset above the first row and below the last (tree: menu y=31, first item y=36).
- **Rows**: 24 pt tall. Separators are 11 pt tall rows with a 1 px hairline inset about 16 pt from both sides. Labels are left-aligned about 16 pt from the panel edge (24 pt when the menu has an icon or check gutter, as in Window). Text is San Francisco, proportional, regular weight, about 13 pt.
- **Text colours** (sampled, approximate because of antialiasing): enabled label about `#E0E0E0` to `#FFFFFF`; disabled label about `#656565` to `#6C6C6E`; shortcut glyphs a muted grey, about `#686868`, right-aligned in a column, even on enabled rows.
- **Highlight**: a solid accent fill `#0069FF` with white text, drawn as a rounded rect (radius about 6–8 pt, inferred) inset about 5 pt from the panel sides. An open submenu's parent row keeps this highlight.
- **Submenus**: open to the right, with the first item aligned to the parent row. Each is the same style of panel with its own shadow.
- **Check gutter**: radio-like groups show `✓` to the left of the label. Labels in submenus with a check gutter start about 24 pt in.

---

## 072 — File menu (`072-menu-file.png`)

**What it is**: the File menu, reached by clicking **File** in the menu bar. Panel bounds 211x237 pt at (114,31).

**Items, in order:**

| Label | Shortcut | State | Selector |
|---|---|---|---|
| New Window | ⌘N | enabled | menuAction: |
| New Session | ⇧⌘N | enabled | menuAction: |
| Add Remote Host… | — | enabled | menuAction: |
| New Tab | ⌘T | enabled | menuAction: |
| Go to Directory… | ⇧⌘G | enabled | menuAction: |
| --- | | | |
| Close | ⌘W | enabled | menuAction: |
| Close Tab | ⌥⌘W | enabled | menuAction: |
| Close Session | — | enabled | menuAction: |
| Close Window | ⇧⌘W | enabled | menuAction: |

Tree note: at y=167 the tree has an extra unnamed disabled 24 pt `menuitem` that overlaps "Close". It is probably a hidden or alternate item (inferred), and no extra row is drawn.

**Layout**: 9 items plus 1 separator. There is no icon or check gutter, and labels start at about x+16 pt. Width is 211 pt.

**Behaviour implied**: create a window, a session (on the server) or a tab; add a remote host (opens a dialog, per the "…"); jump to a directory (dialog). Close is a context-sensitive close (⌘W). Close Tab is ⌥⌘W, Close Window is ⇧⌘W, and Close Session has no shortcut. Close Session closes the Rex session itself, which is an explicit, destructive action (inferred).

---

## 073 — Edit menu (`073-menu-edit.png`)

**What it is**: the Edit menu, reached from the menu bar while a terminal window ("shuv@shuvbot:~ - lunar-basin") is key. Panel 215x355 pt at (156,31).

| Label | Shortcut | State | Selector |
|---|---|---|---|
| Undo | ⌘Z | **[disabled]** | undo: |
| Redo | ⇧⌘Z | **[disabled]** | redo: |
| --- | | | |
| Cut | ⌘X | **[disabled]** | cut: |
| Copy | ⌘C | **[disabled]** | copy: |
| Paste | ⌘V | enabled | paste: |
| Delete | — | **[disabled]** | delete: |
| Select All | ⌘A | enabled | selectAll: |
| Insert Path… | ^⌘I | enabled | menuAction: |
| Find | › | enabled (submenu) | — |
| --- | | | |
| Writing Tools | › | enabled (submenu, system) | __NSTextViewContextSubmenuIdentifierWritingTools |
| --- | | | |
| AutoFill | › | enabled (submenu, system); icon: a small keyboard/field glyph with a pen | _NSMenuItemAutoFillIdentifier |
| Start Dictation | fn D | enabled; icon: microphone | startDictation: |
| Emoji & Symbols | fn E | enabled; icon: round globe/emoji badge | orderFrontCharacterPalette: |

**Layout**: the first three groups have no icons and labels at about x+16. The last group (AutoFill, Start Dictation, Emoji & Symbols) has small monochrome system icons at about x+16, with labels pushed to about x+40. The submenu chevrons `›` sit in the shortcut column.

**Behaviour implied**: Copy and Cut are disabled because there is no selection. Paste and Select All act on the terminal. **Insert Path…** (^⌘I) is a Rex-specific command that opens a picker to insert a file path into the terminal (inferred from the "…"). Writing Tools, AutoFill, Dictation and Emoji are standard macOS additions.

---

## 074 — View menu (`074-menu-view.png`)

**What it is**: the View menu. Panel 225x401 pt at (199,31).

| Label | Shortcut | State | Selector |
|---|---|---|---|
| Command Palette… | ⇧⌘P | enabled | menuAction: |
| Change Session… | ⌘K | enabled | menuAction: |
| Refresh Sessions | ⌘R | enabled | menuAction: |
| --- | | | |
| Pane | › | submenu (see 080) | — |
| --- | | | |
| Customize View | › | submenu (see 081) | — |
| --- | | | |
| Reset Font Size | ⌘0 | **[disabled]** (font already at default, inferred) | — |
| Increase Font Size | ⌘+ | enabled | menuAction: |
| Decrease Font Size | ⌘− | enabled | menuAction: |
| --- | | | |
| Toggle Sidebar | ⌘/ | **[disabled]** | — |
| Show Activity | ⌘. | enabled | menuAction: |
| Focus Tab Filter | — | enabled | menuAction: |
| --- | | | |
| Reveal Toolbar | ⌥⌘T | **[disabled]** | — |
| Reveal Pane Header | ⌥⇧⌘T | **[disabled]** | — |
| Enter Full Screen | fn F | enabled | toggleFullScreen: |

The shortcut for Reset Font Size is drawn as "⌘0" (the zero is narrow, matching the font size rows). Show Activity is drawn as "⌘ ." with a gap before the period.

**Behaviour implied**:
- The Command Palette and Change Session switcher are separate surfaces.
- Refresh Sessions re-queries the server's session list.
- Font size steps up and down, with a reset back to the default.
- Toggle Sidebar is disabled here, probably because of the current layout (inferred).
- Show Activity (⌘.) opens an activity view or panel.
- Focus Tab Filter moves focus into the tab filter field.
- Reveal Toolbar and Reveal Pane Header are disabled, which matches Toolbar "Always Show" and Pane Headers "Always Show" in 083 and 085. They only apply when those elements are hidden or shown on hover (inferred).

---

## 075 — Window menu (`075-menu-window.png`)

**What it is**: the Window menu. Panel 309x532 pt at (248,31). This menu has a left gutter for checkmarks and icons, so labels start at about x+28 pt.

| Label | Shortcut | State | Selector |
|---|---|---|---|
| Minimize | ⌘M | enabled | performMiniaturize: |
| Minimize All | — | enabled (drawn as its own row) | miniaturizeAll: |
| Zoom | — | enabled | performZoom: |
| (Zoom All) | — | hidden alternate (⌥) at the same bounds as Zoom | zoomAll: |
| Fill | ^fn F | enabled | _zoomFill: |
| Center | ^fn C | enabled | _zoomCenter: |
| --- | | | |
| Move & Resize | › | submenu; icon: a window with a small inner rect | — |
| Full Screen Tile | › | submenu; icon: a window split in two | — |
| --- | | | |
| Rex Beta Diagnostics | ⇧⌘, | enabled | menuAction: |
| Copy Version Info | — | enabled | menuAction: |
| Copy Debug Info | — | enabled | menuAction: |
| --- | | | |
| Show Previous Tab | ^⇧⇥ | **[disabled]** | — |
| Show Next Tab | ^⇥ | **[disabled]** | — |
| Move Tab Backward | ^⇧↑ | **[disabled]** | — |
| Move Tab Forward | ^⇧↓ | **[disabled]** | — |
| --- | | | |
| Previous Session | — | enabled | menuAction: |
| Next Session | — | enabled | menuAction: |
| --- | | | |
| Bring All to Front | — | enabled | arrangeInFront: |
| (Arrange in Front) | — | hidden alternate at the same bounds | alternateArrangeInFront: |
| Remove Window from Set | — | **[disabled]** | _removeWindowFromStageManagerSet: |
| --- | | | |
| ✓ shuv@shuvbot:~ - lunar-basin | — | enabled, **checked** (current window); icon: window outline | makeKeyAndOrderFront: |

Only "Zoom All" and "Arrange in Front" share bounds with another item, so those two are the hidden ⌥ alternates.

**Behaviour implied**:
- Window titles take the form `<terminal title> - <session name>`.
- Rex adds Diagnostics (⇧⌘,), Copy Version Info and Copy Debug Info (clipboard).
- Tab navigation and reordering (^⇥, ^⇧⇥, ^⇧↑, ^⇧↓) are disabled here, probably because the window has a single tab (inferred).
- Previous Session and Next Session have no default shortcuts.
- The rest are standard macOS window-management items.

---

## 076 — Help menu (`076-menu-help.png`)

**What it is**: the Help menu. Panel 360x104 pt at (316,31).

1. **Search field**: a rounded, capsule-shaped text field (342x26 pt) with a magnifying-glass icon and the placeholder "Search". It has focus (blue caret), and the search result table under it is empty. This is the standard macOS Help search.
2. **Send Rex Beta Feedback to Apple**: icon is a speech bubble with an exclamation mark; no shortcut; selector `_openFeedbackAssistant:`.
3. ---
4. **Rex Help**: ⌘?; selector `menuAction:` (opens Rex's own help, inferred).

---

## 077 — Edit › Find (`077-menu-edit-find.png`)

**What it is**: Edit menu with **Find** highlighted (`#0069FF`) and its submenu open to the right. The submenu is about 158 pt wide (measured from the crop).

| Label | Shortcut | State |
|---|---|---|
| Find… | ⌘F | enabled (highlighted in the capture) |
| --- | | |
| Find Next | ⌘G | **[disabled]** |
| Find Previous | — (no shortcut drawn) | **[disabled]** |

**Behaviour implied**: ⌘F opens terminal find. Next and Previous stay disabled until a search is active (inferred). Find Previous shows no ⇧⌘G hint. Note that ⇧⌘G is taken by File › Go to Directory….

---

## 078 — Edit › Writing Tools (`078-menu-edit-writing-tools.png`)

**What it is**: Edit menu with **Writing Tools** highlighted and the system Writing Tools submenu open, about 147 pt wide. Every item is **[disabled]** (selector `showWritingTools:`), but the first is still drawn with the blue highlight because of hover.

- Show Writing Tools
- ---
- Proofread
- Rewrite
- ---
- Make Friendly
- Make Professional
- Make Concise
- ---
- Summarize
- Create Key Points
- Make List
- Make Table
- ---
- Compose…

No shortcuts. This menu is injected by macOS, and Rex adds nothing here.

---

## 079 — Edit › AutoFill (`079-menu-edit-autofill.png`)

**What it is**: Edit menu with **AutoFill** highlighted and its system submenu open, about 130 pt wide. Every item is enabled and has an icon.

| Label | Icon | Selector |
|---|---|---|
| Contact… (highlighted) | person in a circle | _handleInsertFromContactsCommand: |
| Passwords… | key | _handleInsertFromPasswordsCommand: |
| Credit Card… | card | _handleInsertFromCreditCardsCommand: |

No shortcuts. These are system-provided items.

---

## 080 — View › Pane (`080-menu-view-pane.png`)

**What it is**: View menu with **Pane** highlighted and the Pane submenu open, about 271 pt wide and 24 rows tall. All shortcuts were read at 4x zoom.

| Label | Shortcut | State |
|---|---|---|
| Split Left | ⇧⌘◀ | enabled (highlighted) |
| Split Right | ⌘D | enabled |
| Split Up | ⇧⌘▲ | enabled |
| Split Down | ⇧⌘D | enabled |
| Split Best Fit | ⌘↩ | enabled |
| --- | | |
| Float Pane | ⌥⌘↩ | **[disabled]** |
| New Floating Pane | — | enabled |
| Minimize Floating Window | ⌥⌘M | **[disabled]** |
| Minimize Floating Windows | ⌥⇧⌘↩ | **[disabled]** |
| Zoom Pane | ⇧⌘↩ | enabled |
| --- | | |
| Focus Left | ⌥⌘◀ | enabled |
| Focus Right | ⌥⌘▶ | enabled |
| Focus Up | ⌥⌘▲ | enabled |
| Focus Down | ⌥⌘▼ | enabled |
| Focus Previous Pane | ⌘[ | enabled |
| Focus Next Pane | ⌘] | enabled |
| --- | | |
| Move Pane Left | ⌥⇧⌘◀ | **[disabled]** |
| Move Pane Right | ⌥⇧⌘▶ | **[disabled]** |
| Move Pane Up | ⌥⇧⌘▲ | **[disabled]** |
| Move Pane Down | ⌥⇧⌘▼ | **[disabled]** |

**Behaviour implied**:
- Four directional splits plus "Best Fit", which picks the direction from the pane's aspect ratio (inferred).
- Floating panes are supported: Float Pane converts the current pane, New Floating Pane creates one, and there are minimize actions for one or all floating windows.
- Zoom Pane toggles maximizing one pane.
- Focus moves by direction or by order.
- Move Pane swaps by direction.
- Float, Minimize Floating and Move Pane are disabled because the session has a single pane and no floating windows (inferred).

---

## 081 — View › Customize View (`081-menu-view-customize-view.png`)

**What it is**: View menu with **Customize View** highlighted and its submenu open, about 164 pt wide.

- Change Theme… (highlighted; no shortcut)
- ---
- Color Scheme ›
- Toolbar Layout ›
- Pane Style ›
- Interface Style ›

There are no checkmarks at this level and no disabled items. Change Theme… opens the theme picker (inferred from the "…").

---

## 082 — View › Customize View › Color Scheme (`082-menu-view-customize-view-color-scheme.png`)

The third-level submenu is about 74 pt wide and has a check gutter.

- ✓ **Auto** (checked, highlighted)
- ---
- Light
- Dark

**Behaviour implied**: an app-appearance radio group. Auto follows the macOS appearance; Light and Dark force one. The separator puts Auto apart from the explicit choices.

---

## 083 — View › Customize View › Toolbar Layout (`083-menu-view-customize-view-toolbar-layout.png`)

The submenu is about 135 pt wide and holds **two independent radio groups**, each with its own checkmark.

Position group:
- ✓ **Above** (checked, highlighted)
- Left Side
- Right Side

---

Visibility group:
- ✓ **Always Show** (checked)
- Show on Hover
- Hidden

**Behaviour implied**: the toolbar can sit above the content or as a vertical bar on the left or right. Its visibility can be permanent, revealed on hover, or hidden. View › Reveal Toolbar (⌥⌘T) applies when it is not "Always Show" (inferred).

---

## 084 — View › Customize View › Pane Style (`084-menu-view-customize-view-pane-style.png`)

The submenu is about 160 pt wide.

- ✓ **Comfortable** (checked, highlighted)
- Compact
- ---
- Pane Headers ›

**Behaviour implied**: a pane density radio group (Comfortable or Compact), plus a nested setting for pane header visibility.

---

## 085 — View › Customize View › Pane Style › Pane Headers (`085-menu-view-customize-view-pane-style-pane-headers.png`)

This is the fourth-level submenu, about 135 pt wide.

- ✓ **Always Show** (checked, highlighted)
- Show on Hover
- Hidden

The panel behind it shows a pane header pill: a terminal-prompt icon, "shuv@shuvbot:~" and a close ×.

**Behaviour implied**: a pane header visibility radio group. View › Reveal Pane Header (⌥⇧⌘T) applies when headers are hidden or shown on hover (inferred).

---

## 086 — View › Customize View › Interface Style (`086-menu-view-customize-view-interface-style.png`)

The submenu is about 137 pt wide.

- ✓ **Classic** (checked, highlighted)
- Glass
- ---
- ✓ **Tint with theme** (checked; an independent toggle)

**Behaviour implied**: the chrome material is either Classic (opaque) or Glass (translucent Liquid Glass, inferred). "Tint with theme" is a separate checkbox that tints the chrome with the terminal theme's colours, and it is on.

---

## 087 — Window › Move & Resize (`087-menu-window-move-and-resize.png`)

**What it is**: Window menu with **Move & Resize** highlighted and the system tiling submenu open, about 263 pt wide. Disabled grey **section headers** (smaller, semibold, tree `disabled`) group the items. Each item has a small monochrome tile-layout icon.

| Label | Shortcut | Selector |
|---|---|---|
| *Halves* (header) | | — |
| Left (highlighted) | ^fn◀ | _zoomLeft: |
| Right | ^fn▶ | _zoomRight: |
| Top | ^fn▲ | _zoomTop: |
| Bottom | ^fn▼ | _zoomBottom: |
| --- | | |
| *Quarters* (header) | | — |
| Top Left | — | _zoomTopLeft: |
| Top Right | — | _zoomTopRight: |
| Bottom Left | — | _zoomBottomLeft: |
| Bottom Right | — | _zoomBottomRight: |
| --- | | |
| *Arrange* (header) | | — |
| Left & Right | ^⇧fn◀ | _zoomLeftAndRight: |
| (Left & Quarters) | — | _zoomLeftThreeUp: (hidden alternate, in the tree but not drawn) |
| Right & Left | ^⇧fn▶ | _zoomRightAndLeft: |
| (Right & Quarters) | — | _zoomRightThreeUp: (hidden alternate) |
| Top & Bottom | ^⇧fn▲ | _zoomTopAndBottom: |
| (Top & Quarters) | — | _zoomTopThreeUp: (hidden alternate) |
| Bottom & Top | ^⇧fn▼ | _zoomBottomAndTop: |
| (Bottom & Quarters) | — | _zoomBottomThreeUp: (hidden alternate) |
| Quarters | — | _zoomQuarters: |
| --- | | |
| Return to Previous Size | ^fn R | **[disabled]** — _zoomUntile: |

This is all standard macOS 15+ window tiling, not Rex-specific.

---

## 088 — Window › Full Screen Tile (`088-menu-window-full-screen-tile.png`)

The submenu is about 144 pt wide and contains two items, each with an icon and no shortcut:

- Left of Screen (highlighted) — `_tileLeft:`
- Right of Screen — `_tileRight:`

This is a standard macOS item.

---

## Cross-menu observations for parity

- **Rex-specific commands** (selector `menuAction:`) are:
  - File: everything.
  - Edit: Insert Path…, Find….
  - View: everything except Enter Full Screen.
  - Window: Rex Beta Diagnostics, Copy Version Info, Copy Debug Info, Previous Session, Next Session.
  - Help: Rex Help.
- Items with no selector in the tree are disabled Rex items whose action is not active (Find Next/Previous, Float Pane, Move Pane *, Reset Font Size, Toggle Sidebar, Reveal *, the tab navigation items).
- All appearance settings that Settings also exposes (Color Scheme, Toolbar position and visibility, Pane density, Pane headers, Interface style, Tint) can be reached directly from View › Customize View as checkmark radio groups.
- Shortcut conflicts and choices to mirror: ⌘D splits right and ⇧⌘D splits down; ⌘K is Change Session (it is **not** clear-screen); ⌘R refreshes sessions; ⇧⌘G is Go to Directory; ⌘. is Show Activity; ⌘/ toggles the sidebar; ⇧⌘, opens Diagnostics.

## Feature inventory

- [menu.file.new-window] New Window (⌘N)
- [menu.file.new-session] New Session (⇧⌘N): creates a server session explicitly
- [menu.file.add-remote-host] Add Remote Host… (no shortcut, opens a dialog)
- [menu.file.new-tab] New Tab (⌘T)
- [menu.file.go-to-directory] Go to Directory… (⇧⌘G)
- [menu.file.close] Close, context-sensitive (⌘W)
- [menu.file.close-tab] Close Tab (⌥⌘W)
- [menu.file.close-session] Close Session (no shortcut)
- [menu.file.close-window] Close Window (⇧⌘W)
- [menu.edit.undo-redo] Undo ⌘Z / Redo ⇧⌘Z (disabled in terminal context)
- [menu.edit.clipboard] Cut ⌘X / Copy ⌘C (disabled without a selection) / Paste ⌘V / Delete (disabled) / Select All ⌘A
- [menu.edit.insert-path] Insert Path… (^⌘I)
- [menu.edit.find] Find submenu: Find… ⌘F, Find Next ⌘G (disabled until a search runs), Find Previous (no shortcut, disabled)
- [menu.edit.system] System Writing Tools submenu (all disabled), AutoFill submenu (Contact…, Passwords…, Credit Card…), Start Dictation (fn D), Emoji & Symbols (fn E)
- [menu.view.command-palette] Command Palette… (⇧⌘P)
- [menu.view.change-session] Change Session… switcher (⌘K)
- [menu.view.refresh-sessions] Refresh Sessions (⌘R)
- [menu.view.pane.split] Split Left ⇧⌘←, Split Right ⌘D, Split Up ⇧⌘↑, Split Down ⇧⌘D, Split Best Fit ⌘↩
- [menu.view.pane.floating] Float Pane ⌥⌘↩ (disabled), New Floating Pane (no shortcut), Minimize Floating Window ⌥⌘M (disabled), Minimize Floating Windows ⌥⇧⌘↩ (disabled)
- [menu.view.pane.zoom] Zoom Pane (⇧⌘↩)
- [menu.view.pane.focus] Focus Left/Right/Up/Down (⌥⌘ + arrow), Focus Previous Pane ⌘[, Focus Next Pane ⌘]
- [menu.view.pane.move] Move Pane Left/Right/Up/Down (⌥⇧⌘ + arrow; disabled with a single pane)
- [menu.view.customize.change-theme] Change Theme… opens the theme picker
- [menu.view.customize.color-scheme] Color Scheme: Auto | Light | Dark (default Auto, checked)
- [menu.view.customize.toolbar-position] Toolbar Layout position: Above | Left Side | Right Side (current Above)
- [menu.view.customize.toolbar-visibility] Toolbar visibility: Always Show | Show on Hover | Hidden (current Always Show)
- [menu.view.customize.pane-density] Pane Style: Comfortable | Compact (current Comfortable)
- [menu.view.customize.pane-headers] Pane Headers: Always Show | Show on Hover | Hidden (current Always Show)
- [menu.view.customize.interface-style] Interface Style: Classic | Glass (current Classic)
- [menu.view.customize.tint-with-theme] Tint with theme checkbox (current on)
- [menu.view.font-size] Reset Font Size ⌘0 (disabled at default), Increase Font Size ⌘+, Decrease Font Size ⌘−
- [menu.view.toggle-sidebar] Toggle Sidebar (⌘/; disabled in the capture)
- [menu.view.show-activity] Show Activity (⌘.)
- [menu.view.focus-tab-filter] Focus Tab Filter (no shortcut)
- [menu.view.reveal-toolbar] Reveal Toolbar (⌥⌘T; disabled while the toolbar is Always Show)
- [menu.view.reveal-pane-header] Reveal Pane Header (⌥⇧⌘T; disabled while headers are Always Show)
- [menu.view.full-screen] Enter Full Screen (fn F)
- [menu.window.system] Minimize ⌘M, Minimize All, Zoom (⌥ alternate Zoom All), Fill ^fn F, Center ^fn C
- [menu.window.move-resize] Move & Resize submenu: Halves (Left/Right/Top/Bottom ^fn+arrow), Quarters (4 corners), Arrange (Left & Right, Right & Left, Top & Bottom, Bottom & Top with ^⇧fn+arrow, ⌥ alternates "& Quarters", Quarters), Return to Previous Size ^fn R (disabled)
- [menu.window.full-screen-tile] Full Screen Tile: Left of Screen | Right of Screen
- [menu.window.diagnostics] Rex Beta Diagnostics window (⇧⌘,)
- [menu.window.copy-version-info] Copy Version Info to the clipboard
- [menu.window.copy-debug-info] Copy Debug Info to the clipboard
- [menu.window.tab-nav] Show Previous Tab ^⇧⇥, Show Next Tab ^⇥ (disabled with a single tab)
- [menu.window.tab-reorder] Move Tab Backward ^⇧↑, Move Tab Forward ^⇧↓ (disabled with a single tab)
- [menu.window.session-nav] Previous Session / Next Session (no default shortcuts)
- [menu.window.bring-all-to-front] Bring All to Front (⌥ alternate Arrange in Front); Remove Window from Set (Stage Manager, disabled)
- [menu.window.window-list] Window list with a ✓ on the current window, titled "<terminal title> - <session name>"
- [menu.help.search] Help menu search field
- [menu.help.feedback] Send Rex Beta Feedback to Apple
- [menu.help.rex-help] Rex Help (⌘?)
- [menu.app] Rex Beta app menu: not captured in this set
