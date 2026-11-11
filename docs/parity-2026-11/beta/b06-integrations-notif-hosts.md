# B06: Settings > Integrations, Notifications, Remote Hosts, Add Remote Host, Pair Device (Rex Beta 0.1.0 (1932), macOS)

Sources: `rex-beta-ui-2026-11-10/screenshots/052, 053, 054, 061–071-*.png`, the accessibility trees `evidence/<same name>.json`, and the full-display captures `evidence/062/067/068/069-*-display.png`. Per `COVERAGE.md`, the set was captured on November 10, 2026 (Pacific Time) with Rex Beta 0.1.0 (1932), Bleeding Edge, against Rex Server 0.1.0-preview.15. COVERAGE.md lists no known gaps for these pages. It warns that the pairing image may expose local machine details. Here, the capture machine's hostname, tailnet ID and account are replaced with `<host>`, `<tailnet>` and `<account>`.

## Conventions

- **Units.** All measurements are in points. Window screenshots are 1x: 928×962 px including the macOS window shadow. The window itself is 816×850 pt, drawn at image offset (56, 38).
- **Coordinates.** Tree coordinates are screen coordinates, with the window origin at (1104, 100). "win(x,y)" means window-relative points. Image px = win + (56, 38).
- **Colours.** Colours were sampled from the 1x PNGs. Text colours are the most frequent bright pixel in each anti-aliased glyph run, so true values may be slightly brighter.
- **Shared chrome.** The traffic lights, toolbar, nav capsule, sidebar, search field and category list are identical to `b07-developer.md` § "Shared chrome" and are only summarised here.

---

## Shared chrome and content grid (all window screenshots in this file)

- **Window.** "Settings - <Page>" (role window), 816×850, dark appearance. The outer window edge has a 1 px light hairline (#272A2E at the left edge, #23272B at the top). The window has a large soft macOS drop shadow.
- **Traffic lights.** Close is red. Minimize is grey and **disabled**. Zoom is green (#19BB00 sampled when the window is not key). When a sheet is up (061, 071), Close is also grey: its tree state is `disabled` in 071.
- **Toolbar.** 52 pt tall, unified with the content background (#0B0F13), with no divider.
  - **Nav capsule:** Back `<` / Forward `>`, a 76×38 pill with fill #292E33/#292E34 and a thin vertical separator. Back is white and enabled on every subpage. Forward is dim and **disabled**.
  - **Page title:** "Settings page" statictext, about 15 pt semibold, #DADADB, at win(300,15).
- **Sidebar.** 215 pt wide, fill #0F1217. The splitter value is `214` and the splitter is disabled, so the sidebar cannot be resized.
  - **Search field:** "Search Settings" capsule, fill #202328, magnifier glyph, placeholder #767C81.
  - **Category list (outline "Settings categories"):**
    - "Server" with the subtitle "Running" (40 pt row; Rex mascot avatar)
    - gap
    - "Interface" (dark tile, three coloured dots) and "Themes" (rainbow palette tile)
    - gap
    - "Terminal" (green `>_` tile), "Shortcuts" (grey ⌘ tile), "Integrations" (purple/pink puzzle-piece tile), "Notifications" (red-orange bell tile) and "Remote Hosts" (purple globe tile)
    - gap
    - "Developer" (blue-grey hammer tile)
  - **Category rows:** 32 pt cells with 20×20 rounded-square colour icons. Labels are about 13 pt, #E1EAF1.
  - **Selected row:** a rounded rect about 195×32, fill #191D22, label semibold, value "Selected".
- **Content pane.** Background #0B0F13. Cards are fill #12161A, with no border or shadow and a corner radius of about 12 pt (the anti-alias ramp spans about 12 px).
  - **Cards:** 561 pt wide at win(235), with 10 pt inner padding. Text starts at win(245) and trailing controls end at win(786).
  - **Section heading:** about 13 pt semibold, #E1EAF1, placed about 10 pt above its card.
  - **Row text:** title about 13 pt regular, #E1EAF1. Subtitle about 11 pt, #767C81.
  - **Footnote below a card:** about 10.5 pt, #777D83, about 10 pt below the card.
  - **Dividers:** 1 pt, #26292D, inset 10 pt on both sides.
  - **Spacing:** sibling cards sit about 10–11 pt apart. A single-line toggle row has a 39 pt pitch. A row with a subtitle is about 45 pt.
- **Switch** (role switch, tree 36×16, drawn as a pill about 36×20):
  - ON: track #0676FD with a white knob at the right.
  - OFF: track #282C2F/#292D30 with a white knob at the left.
  - **Disabled ON:** track #0D5BBA (a desaturated, darker blue) with a white knob.
- **Popup button** (role combobox, readonly):
  - Borderless. The value is right-aligned, about 13 pt, #DBDBDC.
  - It is followed by a circular badge about 22 pt wide, fill #24282C, with up/down chevrons in #E1EAF1.
  - Menus open as native macOS pop-up menus, with the current item drawn over the button.
- **Overlay scrollbar.** 6 pt handle, about #595B5E. It is hidden at rest (053 tree: `hidden`) and visible after scrolling (070).

---

## 052: Settings > Integrations

**What it is:** The Integrations category page. You reach it by clicking "Integrations" in the settings sidebar, which then shows the selected highlight. Window title: "Settings - Integrations". Toolbar title: "Integrations".

**Controls, in visual order:**

1. Section heading **"Coding Agents"** (role heading), at win(245,64).
2. Card at win(235,90), 561×72:
   - **Agent icon row.** At win(245,~100) there are four monochrome glyphs, about 14 pt each, ink #767C81, spaced about 24 pt apart, left to right:
     1. A starburst/asterisk with many thin rays (Claude-style mark).
     2. An interlocking knot/rosette (OpenAI/Codex-style mark).
     3. A hollow upright rectangle/bracket (OpenCode-style mark).
     4. A blocky pixel "P"-like glyph (Pi-style mark).

     The brand mapping is inferred from the footnote; the glyphs carry no labels in the tree.
   - Row title **"Integrate with available agents"**.
   - Subtitle **"Installs a Rex skill and session hooks for supported agents."**
   - **Switch** "Integrate with available agents": **ON** (checked, value 1). It is vertically centred on the text block at win(750,132).
3. Footnote under the card: **"Teach Claude Code, Codex, OpenCode and Pi how to use Rex."**
4. Section heading **"Agent Behavior"**, at win(245,215).
5. Card, 561×45:
   - Row title **"Let agents name sessions"**.
   - Subtitle **"Agents label the session after the task they're working on."**
   - **Switch** "Let agents name sessions": **ON**.
6. Footnote: **"Changes apply to new agent sessions."**

The rest of the pane is empty, and the scrollbar is not shown.

**Layout and visual:**
- The first card is taller (72 pt) because the icon strip sits above the title.
- There are about 30 pt from the footnote to the next heading.
- No dividers: each card holds one row.
- Colours:
  - Card: #12161A
  - Pane: #0B0F13
  - Sidebar: #0F1217
  - Heading and title: #E1EAF1
  - Subtitle: #6E7378 to #767C81
  - Footnote: #777D83
  - Switch track ON: #0676FD

**Behaviour implied:**
- The first toggle installs or removes a Rex "skill" plus session hooks in the supported coding agents: Claude Code, Codex, OpenCode and Pi. Detection appears to be automatic ("available agents").
- The second toggle lets an agent rename the Rex session after its current task. It only affects new agent sessions.

---

## 053: Settings > Notifications (top of page)

**What it is:** The Notifications category page, scrolled to the top. You reach it from the sidebar "Notifications" item. Window title: "Settings - Notifications". Toolbar title: "Notifications".

**Controls, in visual order:**

1. Heading **"In-App Notifications"**, at win(245,64).
2. Card, 561×82:
   - Row title **"Notification position"** and subtitle **"Displayed bottom right"**. The subtitle reflects the current choice.
   - **Position picker** (group "Notification position", 112×64 at win(674,99)): a rounded-rect mini "screen" with fill #1E2225 and a #25292C hairline border.
   - The picker contains six pill-shaped buttons, each about 34×27 (hit area), in two rows of three:
     - Top row: **Top left**, **Top center**, **Top right**. All have the value "Not selected".
     - Bottom row: **Bottom left**, **Bottom center**, **Bottom right**. Bottom right is **selected** (value "Selected").
   - Unselected pills are fill #25292C on #1E2225.
   - The selected pill is accent blue #0073FF with a dark checkmark ✓.
   - The middle band of the "screen" is empty, so the picker represents a display.
3. Heading **"Program Activity"**, with the subheading **"How status from processes and agents is handled"** (#767C81, about 11 pt, directly under the heading).
4. Card (5 rows, dividers between them, 39 pt pitch):
   - **"Show activity in tabs"**: switch ON
   - **"Show system notifications"**: switch ON
   - **"Badge Dock icon"**: switch ON
   - **"Play sounds"**: switch ON
   - **"Sound volume"**: a **slider**
     - Layout, left to right: a small speaker glyph button "Volume Lower" (8×11), the slider track (158×18), and a speaker-with-waves glyph button "Volume Higher" (17×13).
     - Value **6**. The track has 9 tick marks under it, so the range is probably 0–8 in integer steps (inferred from the tick count). The knob sits on the 7th tick.
     - The filled track is accent #176EDA. The unfilled track is #5A5E63. The knob is a white rounded vertical capsule, about 10×18.
5. Card of four status rows. Each row is a disclosure button (541×32 hit area) on a 45 pt pitch, with dividers between rows:
   - **Status icon:** a 24 pt circle with a gradient fill and a white glyph.
   - **Title:** about 13 pt.
   - **Subtitle:** lists the channels currently enabled for that status.
   - **Chevron:** a `>` at the far right, #45494E.

   | Row | Icon | Title | Subtitle (verbatim) | Tree name |
   |---|---|---|---|---|
   | 1 | Blue circle (#3B9BFE→#0075FE gradient) with three white dots "•••" | **Working** | "Tabs" | "Working, Tabs" |
   | 2 | Orange circle (#FFC57A→#FF950F→#FF7010) with a white "no entry" ⃠ (circle with slash) | **Blocked** | "Tabs, Notifications, Dock Badge, Sound" | "Blocked, Tabs, Notifications, Dock Badge, Sound" |
   | 3 | Red-orange circle (#FF5E40→#FF4025) with a white outlined warning triangle ⚠ | **Error** | "Tabs, Notifications, Dock Badge, Sound" | "Error, Tabs, Notifications, Dock Badge, Sound" |
   | 4 | Green circle (#6DBE38→#34A700) with a white outlined circled checkmark ✓ | **Done** | "Tabs, Notifications, Dock Badge, Sound" | "Done, Tabs, Notifications, Dock Badge, Sound" |
6. Card (2 rows):
   - **"Include idle processes in activity"**: switch ON
   - **"Show each process in tabs"**: switch ON
7. Heading **"System Notifications"**.
8. Card:
   - Title **"Terminal notifications"**.
   - Subtitle **"Programs can send notifications with OSC 9 or OSC 777."**
   - Switch ON.
9. A separate card follows, partly cut off at the bottom edge. Its rows are described under 070:
   - **"Tailscale sign-in"**: switch ON, **disabled**
   - Offscreen in the tree: **"Tailscale device approval"** (ON, disabled) and **"Tailscale failures"** (ON, disabled)

**Layout and visual:**
- The page is taller than the 806 pt content viewport. The scrollbar is hidden, with value 0.
- The card for "Show activity in tabs" … "Sound volume" starts at win(235,244). There is a 10 pt gap to the status-row card.
- Status-row text is indented to win(283) (title x = 1349+40 in the tree; image x 339), clearing the 24 pt icon at win(245).
- The volume slider's tick marks are tiny 1 px dots under the track.

**Behaviour implied:**
- The six-position picker chooses where in-app toasts appear.
- The global Program Activity toggles are master switches for activity in tabs, system notifications, Dock badge and sounds. The master volume applies to all sounds. The speaker glyphs step the volume down or up.
- Each status row opens a per-status subpage (063–066). The subtitle summarises which channels are enabled for that status:
  - Working: Tabs only.
  - The others: Tabs, Notifications, Dock Badge and Sound.
- "Include idle processes in activity" counts idle processes toward activity. "Show each process in tabs" shows each process separately in tabs. The exact semantics are not stated.
- "Terminal notifications" lets programs post notifications via OSC 9 and OSC 777 escape sequences.

---

## 070: Settings > Notifications (scrolled lower)

**What it is:** The same page as 053, scrolled down by 55 pt (all tree y values are 55 less). The overlay scrollbar is now **visible** (value 0.5) at the right edge: a #595B5E handle, about 6 pt wide.

**New content visible versus 053:**
- "In-App Notifications" heading is offscreen. The position picker card is partly clipped at the top.
- The System Notifications area now shows two cards:
  1. "Terminal notifications" / "Programs can send notifications with OSC 9 or OSC 777.": switch ON (#0676FD).
  2. A second card with dividers:
     - **"Tailscale sign-in"**: switch ON, **disabled**, drawn with the dimmer track #0D5BBA.
     - **"Tailscale device approval"**: switch ON, **disabled**, also #0D5BBA.
     - **"Tailscale failures"**: switch ON, disabled. It is still below the visible area (offscreen in the tree, at win(245,858)).
- None of the rows has a helper text explaining why the Tailscale switches are disabled.

Everything else (volume value 6, all switches ON, the four status rows and their subtitles) is unchanged from 053.

**Behaviour implied:** The three Tailscale notification categories are shown as always on and not user-editable in this state. Why they are disabled (for example because Tailscale is not managed here) is not stated (inferred).

---

## 063: Notifications > Working

**What it is:** The subpage reached by clicking the "Working" row in 053. The toolbar title is "Working" and Back is enabled. The sidebar keeps "Notifications" selected.

**Controls:**

1. Card 1 (561×45 at win(235,64)):
   - Blue "•••" status icon (24 pt, as in 053) at win(247,85).
   - Title **"Enable status"**, indented to win(285).
   - Subtitle **"Shown when a program is busy or reports progress."**
   - Switch "Enable status": **ON**.
2. Card 2 (3 rows, dividers):
   - **"Show activity in tabs"**: ON
   - **"Show system notifications"**: **OFF** (value 0; dark track #282C2F, knob at left)
   - **"Badge Dock icon"**: **OFF**
3. Card 3 (1 row):
   - **"Play sound"**: **OFF**
   - No Sound picker is shown while this is off.

**Layout:** The cards are stacked with about 10 pt gaps. There are no section headings on this subpage. The rest of the pane is empty.

**Behaviour implied:**
- "Enable status" turns the Working status on or off as a whole.
- The other switches choose this status's channels, which the parent row summarises as "Tabs".
- The sound picker appears only when "Play sound" is ON (compare 064–066).

---

## 064: Notifications > Blocked

**What it is:** The subpage for the "Blocked" row. The toolbar title is "Blocked".

**Controls:**

1. Card 1:
   - Orange "no-entry" icon.
   - **"Enable status"**, with the subtitle **"Shown when a program needs you to respond."**
   - Switch ON.
2. Card 2:
   - "Show activity in tabs": ON
   - "Show system notifications": ON
   - "Badge Dock icon": ON
3. Card 3 (2 rows, divider):
   - **"Play sound"**: ON
   - **"Sound"**:
     - A popup button (combobox, readonly, 103×24) with value **"Attention"**. It is drawn as the text "Attention", #DBDBDC, followed by a circular ⌃⌄ chevron badge with fill #24282C.
     - To its right, a **"Play"** button: a small white ▶ triangle (10×11, #E1EAF1).

**Behaviour implied:**
- The sound picker chooses this status's sound.
- ▶ previews it.
- The popup's width hugs its value: "Attention" is 103 wide, "Error" 77 and "Success" 98. Because the popup is right-aligned, its left edge moves.

---

## 065: Notifications > Error

**What it is:** The subpage for the "Error" row. The toolbar title is "Error".

**Controls:**
- **Card 1:** red warning-triangle icon, **"Enable status"** with the subtitle **"Shown when a program reports that something went wrong."**, switch ON.
- **Card 2:** "Show activity in tabs", "Show system notifications" and "Badge Dock icon", all ON.
- **Card 3:**
  - "Play sound": ON.
  - "Sound": popup with value **"Error"** (77×24), plus the ▶ "Play" button.

---

## 066: Notifications > Done

**What it is:** The subpage for the "Done" row. The toolbar title is "Done".

**Controls:**
- **Card 1:** green circled-check icon, **"Enable status"** with the subtitle **"Shown when a program finishes a task."**, switch ON.
- **Card 2:** "Show activity in tabs", "Show system notifications" and "Badge Dock icon", all ON.
- **Card 3:**
  - "Play sound": ON.
  - "Sound": popup with value **"Success"** (98×24), plus the ▶ "Play" button.

---

## 067: Done > Sound menu (open)

**What it is:** A crop (117×194 px) of the native pop-up menu opened from the "Sound" popup on the Done subpage. The full display is in `evidence/067-menu-done-sound-display.png`.

**Menu items, top to bottom:** 7 menuitems, each 101×24 pt.
1. Attention
2. Warning
3. Error
4. **✓ Success** (selected)
5. Ready
6. Loading
7. Tap

**Visual:**
- **Menu panel:** macOS dark menu, translucent dark grey (about #292E35 over the content), rounded about 6–8 pt, with a thin light border and a shadow.
- **Selected or hovered item:** a full-width rounded accent-blue highlight (#0069FF) with white text and a leading ✓.
- **Other items:** white/light text with no check.
- **Placement:** the menu is positioned so the selected item (Success, y=390) sits exactly over the popup button (tree y=390). This is standard NSPopUpButton behaviour.
- **Ordering:** no separators, shortcuts or icons.

---

## 068: Error > Sound menu (open)

**What it is:** The same pop-up opened on the Error subpage (crop 117×194).

**Items:** Attention, Warning, **✓ Error** (selected, blue highlight), Success, Ready, Loading, Tap. Each is 101×24, and the selected item is again aligned to y=390. A sliver of the ▶ Play glyph is visible at the right edge of the crop.

---

## 069: Blocked > Sound menu (open)

**What it is:** The same pop-up opened on the Blocked subpage (crop 120×194).

**Items:** **✓ Attention** (selected, top, blue highlight), Warning, Error, Success, Ready, Loading, Tap. Each is 104×24. Because Attention is first, the menu extends downward from y=390.

**Sound set (identical in all three menus):** Attention, Warning, Error, Success, Ready, Loading, Tap. These are 7 named sounds in a fixed order.

---

## 054: Settings > Remote Hosts (empty state)

**What it is:** The Remote Hosts category page with no hosts configured. You reach it from the sidebar "Remote Hosts" item. Window title: "Settings - Remote Hosts". Toolbar title: "Remote Hosts".

**Controls, in visual order:**

1. An empty-state card, 561×188 at win(235,64), fill #12161A, with no heading above it. Its contents are centred:
   - A **globe glyph** (outlined meridian/parallel globe), about 36 pt, #5B5F64, centred at win(516,105).
   - Title **"No Remote Hosts"**: about 14 pt semibold, #E1EAF1, centred.
   - Body **"Add a Rex HTTP or HTTPS address to use its sessions in every window."**: about 14 pt regular, #E1EAF1. It wraps to two centred lines: "Add a Rex HTTP or HTTPS address to use its" / "sessions in every window."
2. Below the card, right-aligned with the card's trailing edge minus 10 pt: the button **"Add Remote Host…"** (with an ellipsis).
   - Size: 137×24.
   - Shape: a capsule with fill #1B1F24 and label #E1EAF1, about 13 pt.

**Behaviour implied:**
- "Add Remote Host…" opens the Add Remote Host sheet (061).
- Remote hosts are Rex servers reached over HTTP or HTTPS, and their sessions become available in every window.
- No list or table is visible here, because the list is empty. The populated-list state was not captured.

---

## 061: Add Remote Host sheet

**What it is:** A modal sheet opened by "Add Remote Host…" on the Remote Hosts page (054).
- **Sheet:** role sheet, 480×348 at win(168,251), centred horizontally on the window and attached from the vertical middle (not a top-anchored sheet).
- **Window behind:** dimmed by a dark scrim. The sidebar and content text drop to about 50% brightness.
- **Traffic lights:** Close and Minimize are grey, and Zoom is still green.

**Controls, in visual order:**

1. **Header** (about 64 pt tall):
   - A white outlined **globe icon**, about 28 pt, at the left (win(188,269)).
   - Title **"Add Remote Host"** (role heading, about 17 pt bold, #E1EAF1), at win(236,263).
   - Subtitle **"Rex connects to the address and saves it on this Mac."** (about 13 pt, #777D83).
2. A full-width 1 pt divider, #22262A, under the header.
3. Scroll area (480×220) containing:
   - Heading **"Host Details"** (about 13 pt semibold).
   - A grouped card (440×118, fill #12161A, inset 20 pt from the sheet edges, radius about 10 pt) with 3 rows separated by #26292D dividers:
     1. **URL**:
        - Text field "Host URL", **focused**.
        - Placeholder **"https://machine.tailnet.ts.net"** (#5B5F64).
        - Field fill #1C1D20, rounded about 6 pt, with a **blue focus ring** about 3 pt (#164F95/#1A5399). The field spans win(275..617).
     2. **Name**:
        - Text field "Host name".
        - Placeholder **"Host Name"**.
        - Same fill, with a 1 px #2B2D30 border and no focus ring.
     3. **Icon**:
        - Popup button "Host icon" (combobox, readonly, 101×24) with value **"Globe"**, drawn as a small globe glyph, then "Globe", then a ⌃⌄ circular chevron badge.
        - Options: see 062.
   - Footnote under the card (about 10.5 pt, #777D83, two lines): **"Rex does not provide transport authentication. Use a trusted tunnel such as Tailscale or an SSH-forwarded socket."**
4. A full-width divider (#22262A) above the footer.
5. Footer, right-aligned buttons:
   - **"Cancel"**: 71×30 capsule with a dark tinted-blue fill (#091E37/#054697 at the edges) and **accent-blue text #0073FF**.
   - **"Add"**: 53×30 capsule, **filled accent #006FFE**, with **dark text** (#0F1217). This is the default/primary button.
   - The two buttons are 8 pt apart, with a 16 pt right margin.

**Visual:**
- **Sheet:** background #0B0F13, corner radius about 16 pt, a 1 px light hairline border (#23272B), and a heavy shadow (a #010101 ring just outside the border).
- **Label column:** labels ("URL", "Name", "Icon", about 13 pt, #E1EAF1) sit at win(198). Fields start at win(275), giving a fixed label column of about 77 pt.
- **Row pitch:** 39 pt.

**Behaviour implied:**
- You enter a Rex server URL (Tailscale MagicDNS-style is suggested), an optional display name and an icon. "Add" saves the host locally ("on this Mac") and connects.
- The URL field gets keyboard focus when the sheet opens.
- No validation or error state was captured.
- Whether "Add" is disabled while the URL is empty is not visible: it is drawn enabled, and the tree has no disabled state.

---

## 062: Add Remote Host > Icon menu (open)

**What it is:** A crop (154×194 px) of the native pop-up menu opened from the "Icon" popup in the Add Remote Host sheet. The full-display capture shows the menu overlapping the Icon row, with the selected "Globe" item over the popup button at screen y=549.

**Menu items, top to bottom:** 7 menuitems, each 138×24 pt. Each has a leading monochrome SF-Symbol-style glyph.

| # | Label | Glyph (as seen) | State |
|---|---|---|---|
| 1 | **Globe** | globe | ✓ selected (blue #0069FF highlight, white text) |
| 2 | Server | stacked server/rack (rectangle with horizontal bars) | — |
| 3 | Cloud | outlined cloud | — |
| 4 | MacBook | open laptop | — |
| 5 | Mac mini | flat wide low box | — |
| 6 | Mac Studio | squat box with front slot/ports | — |
| 7 | Desktop | monitor on a stand (iMac/display) | — |

**Visual:** A dark translucent macOS menu (#292E35 area), with a rounded highlight on the selected row, a check column at the left, the icon column, then the label. Blue background bleed from the sheet's buttons shows through the translucency at the bottom right (vibrancy).

**Behaviour implied:** The chosen glyph represents the host wherever hosts are listed (inferred: the sidebar or host switcher).

---

## 071: Pair Device sheet

**What it is:** A modal sheet over **Settings > Server**: the toolbar title is "Server" and "Server, Running" is selected in the sidebar. It is opened by the **"Pair"** button (tree name "Show pairing QR code", 68×24, with a QR glyph) on the Server page's "Remote Address" row. That row also has a "Copy Remote Address" button. The Server page itself is covered in another file. The tree for the dimmed page behind the sheet shows:
- "Rex Server" / "Running for 16m" / "Stop Server…"
- "Remote Address" with `https://<host>-rex.<tailnet>.ts.net`
- A Tailscale section: "Signed in to Tailscale", "Sign Out", "Serve over Tailscale" ON, "Limit access to your Tailscale account" ON (subtitle "Rex will verify the caller is signed in as <account>."), "Tags", "Control Server", and "Preferred Hostname" with the field "Tailscale hostname"
- "Client Info", with "Label" and "Remote Control" ON

**Sheet:** role sheet, focused, 480×551 at win(168,149). Behind it, the Back and Forward buttons and the toolbar title are **disabled**, and Close and Minimize are disabled.

**Controls, in visual order:**

1. **Header:**
   - Icon: a white outlined **display plus laptop** pair glyph, about 32 pt.
   - Title **"Pair Device"** (about 17 pt bold, #E1EAF1).
   - Subtitle **"Tap "Pair via QR Code" in Rex on your other device."** (about 13 pt, #777D83; the straight double quotes are verbatim).
2. A full-width divider (#22262A).
3. **QR code card:**
   - Size and shape: a white (#FFFFFF) rounded square, 272×272 pt, corner radius about 12 pt.
   - Position: horizontally centred at win(272,250).
   - Contents: a black QR code with about 20 pt quiet-zone padding. The code is about 224 pt square, with three finder squares.
   - The QR payload is not decoded here.
4. A centred host label: a small **laptop glyph** (#E1EAF1), then the machine name **"<host>"** (about 13 pt bold, #E1EAF1).
5. A centred address line, **"<host>-rex.<tailnet>.ts.net"** (about 10.5 pt, #777D83, selectable statictext).
6. Centred helper text on two lines: **"Your device must be able to reach this machine, such as on the same Tailscale network."** (about 12 pt, #777D83).
7. A full-width divider (#22262A).
8. Footer: a **"Done"** button, 61×30, right-aligned. It is an accent capsule (#006FFE) with dark text, the default button.

**Visual:**
- **Sheet:** background #0B0F13, rounded about 16 pt, a 1 px hairline border and a shadow. The header layout matches 061.
- **Spacing:**
  - QR card to host label: about 15 pt.
  - Address line to helper text: about 20 pt.
  - Helper text to divider: about 24 pt.

**Behaviour implied:**
- The sheet shows a QR code that encodes this machine's Rex remote address (inferred) for another Rex client (for example a phone) to scan via "Pair via QR Code".
- Pairing requires network reachability, such as the same tailnet.
- "Done" dismisses the sheet.

---

## Feature inventory

- [settings.integrations.agents.install] Integrate with available agents: switch (default seen ON). Installs a Rex skill and session hooks for Claude Code, Codex, OpenCode and Pi. A row of agent brand glyphs is shown.
- [settings.integrations.agents.footnote] Footnote listing the supported agents: "Teach Claude Code, Codex, OpenCode and Pi how to use Rex."
- [settings.integrations.agent-behavior.name-sessions] Let agents name sessions: switch (ON). Agents label the session after their task. Applies to new agent sessions only.
- [settings.notifications.in-app.position] Notification position: 6-way visual picker of Top left | Top center | Top right | Bottom left | Bottom center | Bottom right (default Bottom right). The subtitle echoes the choice ("Displayed bottom right").
- [settings.notifications.activity.tabs] Show activity in tabs: global switch (ON)
- [settings.notifications.activity.system-notifications] Show system notifications: global switch (ON)
- [settings.notifications.activity.dock-badge] Badge Dock icon: global switch (ON)
- [settings.notifications.activity.play-sounds] Play sounds: global switch (ON)
- [settings.notifications.activity.volume] Sound volume: slider with 9 ticks (value 6; range probably 0–8, inferred), with Volume Lower and Volume Higher speaker buttons
- [settings.notifications.status-list] Per-status rows (Working, Blocked, Error, Done). Each has a coloured status icon, a subtitle summarising its enabled channels, and a chevron that opens a subpage.
- [settings.notifications.status.icons] Status icons: Working = blue circle "•••"; Blocked = orange circle with a no-entry sign; Error = red circle with a warning triangle; Done = green circle with a check
- [settings.notifications.status.enable] Per-status "Enable status" switch, with descriptions:
  - Working: "Shown when a program is busy or reports progress."
  - Blocked: "Shown when a program needs you to respond."
  - Error: "Shown when a program reports that something went wrong."
  - Done: "Shown when a program finishes a task."
- [settings.notifications.status.tabs] Per-status "Show activity in tabs" switch (ON for all four)
- [settings.notifications.status.system-notifications] Per-status "Show system notifications" switch (Working OFF; Blocked, Error and Done ON)
- [settings.notifications.status.dock-badge] Per-status "Badge Dock icon" switch (Working OFF; the others ON)
- [settings.notifications.status.play-sound] Per-status "Play sound" switch (Working OFF; the others ON). The Sound row appears only when it is ON.
- [settings.notifications.status.sound] Per-status Sound popup: Attention | Warning | Error | Success | Ready | Loading | Tap. Defaults: Blocked = Attention, Error = Error, Done = Success.
- [settings.notifications.status.sound-preview] Per-status ▶ Play button that previews the selected sound
- [settings.notifications.activity.include-idle] Include idle processes in activity: switch (ON)
- [settings.notifications.activity.each-process] Show each process in tabs: switch (ON)
- [settings.notifications.system.terminal-osc] Terminal notifications: switch (ON). Programs can notify via OSC 9 or OSC 777.
- [settings.notifications.system.tailscale-sign-in] Tailscale sign-in notifications: switch (ON, disabled)
- [settings.notifications.system.tailscale-device-approval] Tailscale device approval notifications: switch (ON, disabled)
- [settings.notifications.system.tailscale-failures] Tailscale failures notifications: switch (ON, disabled)
- [settings.remote-hosts.empty-state] Remote Hosts empty state: globe glyph, "No Remote Hosts", and "Add a Rex HTTP or HTTPS address to use its sessions in every window."
- [settings.remote-hosts.add] "Add Remote Host…" button that opens the Add Remote Host sheet
- [settings.remote-hosts.add.url] Add Remote Host: URL text field (autofocused, placeholder "https://machine.tailnet.ts.net")
- [settings.remote-hosts.add.name] Add Remote Host: Name text field (placeholder "Host Name")
- [settings.remote-hosts.add.icon] Add Remote Host: Icon popup of Globe | Server | Cloud | MacBook | Mac mini | Mac Studio | Desktop, each with a glyph (default Globe)
- [settings.remote-hosts.add.security-note] Add Remote Host footnote: "Rex does not provide transport authentication. Use a trusted tunnel such as Tailscale or an SSH-forwarded socket."
- [settings.remote-hosts.add.actions] Add Remote Host footer: Cancel (tinted) and Add (filled primary). Hosts are saved locally ("saves it on this Mac").
- [settings.server.pair-device] Pair Device sheet, opened from the "Pair" (QR) button on Settings > Server > Remote Address. It shows a QR code, the machine name, the tailnet address and a reachability hint, plus a Done button.
- [settings.server.pair-device.instructions] Pair instructions: "Tap "Pair via QR Code" in Rex on your other device." and "Your device must be able to reach this machine, such as on the same Tailscale network."
- [settings.chrome.subpage-navigation] Settings subpages (for example Notifications > Working) push onto a Back/Forward toolbar history. The toolbar title shows the subpage name, and the sidebar keeps the parent category selected.
- [settings.chrome.modal-sheet] Modal sheets dim the settings window and disable its navigation and Close/Minimize. They use a header with icon, title and subtitle, and a divider-separated footer with right-aligned actions.
