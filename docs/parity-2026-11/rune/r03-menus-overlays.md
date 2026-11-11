# Rex Linux 0.6.0: sidebar, menus, overlays (screenshots 30 to 49)

Source: `rex-rune-2026-11-11/screenshots/30..49`, captured on 2026-11-11 (PST) on Omarchy/Hyprland. Build: About shows commit `ad196ff`, build date 2026-11-10, server API revision 13. Each original is 2560 x 1410 at 2x scale, so **logical px = image px / 2**. Every size below is in logical px (approximate, ±1 to 2 px). Colours were sampled from the pixels. A sample that falls on antialiased text is marked "approx". There is no accessibility tree, so every label was read from the pixels.

## Common chrome, seen in all 20 captures

- **Window**: borderless, 1280 x 705 logical. The window background is the terminal colour `#001528`, and the toolbar has no separate fill.
- **Toolbar** (about 26 px tall, y ≈ 6 to 32):
  - Far left: a sidebar-toggle icon (a rectangle with a left column).
  - Session chip (about 136 x 26, fill `#0E2234`, about 8 px radius). It holds a server-stack icon (two stacked rounded rectangles), the session name **`still-atoll`** in semibold on line 1, the host **`shuvbot-rex`** in smaller muted text on line 2, and a small `⌄` chevron at the right.
  - Tab chip (about 100 x 26) with a `>_` glyph and the label **`shuv@shuvbot:~`**.
  - Right cluster: `⌘` (Commands), `+` (new tab, inferred), a sliders icon (Settings, inferred), a thin vertical divider, then `×` (close).
- **Terminal**: a Powerline-style prompt. On the left are an Apple glyph, `>`, a house icon with `~`, and a block cursor. On the right are `✔ < shuv@shuvbot < 00:21:37` and a clock icon. This content belongs to the server and is not part of the client chrome.

---

## 30: Workspace with the session sidebar expanded

**What it is:** the main workspace with the session sidebar open. The sidebar toggle at the far left of the toolbar opens it, as does Ctrl Shift B. Commands lists this action as "Hide sidebar Ctrl Shift B".

**Controls and labels, top to bottom:**
1. Header **`Sessions`**: semibold, about 13 px, white. A `+` button sits at the right (new session, inferred).
2. Search field. It shows a magnifier icon, the placeholder **`Find a session`** in a muted colour, and the hint **`^K`** at the right.
3. Host group **`SHUVDEV`**:
   - Header row: a `⌄` disclosure chevron, a monitor icon, the name in all caps with small bold letter-spaced text in a muted colour, and a `+` at the right.
   - Session row: the `>_` glyph, **`cosmic-mesa`**, and the count badge **`1`** at the right (a plain numeral, no pill).
4. Host group **`SHUVBOT-REX`** (server-stack icon). Its header text is brighter than the other hosts' (current host), and it has a `+`.
   - Row: `lunar-basin` with count `1`.
   - Row: **`still-atoll`** with count `1`. This row is **selected**: filled background, semibold white text, rounded rectangle about 6 px radius.
5. Host group **`SHUVMBP`** (server-stack icon, `+`):
   - Row: `brave-isle` with count `1`.
6. Footer, pinned to the bottom: **`+ Add host`** on the left and **`4 sessions`** on the right, both small and muted.

**Layout and visual spec:**
- The sidebar is an inset floating card at x ≈ 5 to 121, about 116 px wide and almost the full window height. It has about 6 px corner radius and fill `#061B2D`, which is slightly lighter than the window `#001528`. No visible border or shadow.
- Inner horizontal padding is about 4 to 7 px.
- The search field is about 108 x 16 with about 6 px radius and fill `#142739`.
- Session rows are about 17 px tall and indented about 14 px under the host header. The gap between host groups is about 13 px.
- The selected row fill is about `#1D2A36`, which is a neutral grey-blue rather than an accent colour.
- Session names are about 7 px proportional (sans) in a muted light grey. The count badge is right-aligned at about x ≈ 111.
- All typography is proportional sans. Only the terminal is monospace.

**Behaviour implied:**
- Clicking a row switches session.
- The `+` on a host creates a session on that host.
- The disclosure chevron collapses a host group.
- `^K` focuses session search.
- The footer opens Add host and shows the total session count.

**Polish notes:**
- The count badges (`1`) carry no meaning in this state and add noise, because every row shows `1`.
- The host header icon differs: a monitor for SHUVDEV and a server stack for the others. The UI does not explain why (presumably local versus remote, inferred).
- There is a large empty area below the list.

---

## 31: Host context menu

**What it is:** right-click on the host header `SHUVBOT-REX`. The menu opens anchored at the pointer, over the sidebar.

**Items:**
- Header (muted, small, not selectable): **`shuvbot-rex`**
- **`New session`**: highlighted (hover or keyboard focus)
- **`Default shell...`** `›`
- **`Edit host...`** `›`
- **`Remove host...`** `›`

**Visual spec:**
- The menu is about 131 x 77 at x ≈ 53 to 184. Fill is `#0C1524`, radius about 6 px, with a subtle 1 px lighter border and a soft shadow.
- Item rows are about 15 px tall. The highlight is a rounded fill `#222A38` inset about 3 px from the menu edge.
- Labels are regular weight, about 7 px, near-white. Items that open a further surface show a trailing `›` chevron and an ellipsis.
- No separators and no shortcut hints.

**Behaviour implied:** create a session on this host, choose the host's default shell, edit the host, or remove the host (with a confirmation, per screenshot 20 elsewhere).

**Polish notes:**
- The menu covers the host header text (`SHUVBOT-REX` is cut to `SHUVBOT-` with `REX` hidden), and it truncates the session names under it (`lunar-ba`, `still-atol`). This is expected for a popup.
- `Remove host...` is destructive but has no red or destructive styling, and it sits directly under Edit host with no separator.
- The menu has no shortcut hints, although the same action in Commands shows Ctrl Shift N.

---

## 32: Session context menu

**What it is:** right-click on the `still-atoll` row. The row gets a 1 px focus ring in a light-blue accent (about `#3D607D` blended).

**Items:**
- Header: **`shuvbot-rex · still-atoll`** (muted)
- **`Rename session...`** `›`: highlighted
- **`Move up`**
- **`Move down`**: **disabled**, dimmed grey text (still-atoll is last in its host)
- **`Close session...`** `›`

**Visual spec:** the same menu style as 31, about 131 x 77, highlight `#222A38`. The disabled item is rendered at about 50 % text opacity.

**Behaviour implied:**
- Rename the session through a dialog (screenshot 33).
- Reorder the session within its host.
- Close the session, with a confirmation (screenshot 34).

**Polish notes:** `Close session...` is destructive but not styled as such and has no separator above it.

---

## 33: Rename session dialog

**What it is:** opened from Rename session... in the session menu or in Commands.

**Controls:**
- Title **`Rename session`**: semibold, about 9 px.
- Field label **`Session name`**: small, semibold, muted.
- Text field with the value **`still-atoll`**, the caret at the end, and no visible selection.
- Buttons, right-aligned: **`Cancel`** (neutral pill, fill `#19222F`) and **`Save`** (primary pill, light-blue fill about `#81BDFE` to `#8FAAFC`, which reads as a gradient, with dark text).

**Visual spec:**
- The modal is centred at about x 368 to 632 and y 230 to 321, so about 264 x 91. Fill `#0C1523`, radius about 10 px, 1 px lighter border, and a large soft drop shadow.
- The rest of the window is dimmed by a scrim: the terminal goes from `#001528` to about `#000B13`.
- The field fills the full width with about 10 px padding. It has a 1 px border and the same fill as the dialog.
- Buttons are about 13 px tall and fully rounded (pill).
- The sidebar row keeps its accent ring while the dialog is open.

**Behaviour implied:**
- Enter or Save renames the session.
- Esc or Cancel dismisses the dialog (inferred).

**Polish notes:**
- The field has the same fill as the dialog, so it reads mostly by its border and has weak contrast.
- Save, the primary button, is pale blue with dark text. That is a very different weight from the dark Cancel button, but it is consistent across the dialogs.

---

## 34: Close session confirmation

**What it is:** opened from Close session... (session menu, switcher `×`, or Commands, inferred).

**Content:**
- Title **`Close still-atoll?`**
- Body, regular, muted: **`Ends every program in its 1 tab on shuvbot-rex. Closing the app itself leaves programs running.`**
- Buttons: **`Keep running`** (neutral pill, dark) and **`Close and end programs`** (destructive pill: dark fill about `#24212E`, a red-tinted 1 px border, and salmon-red text about `#EF8283`).

**Visual spec:** about 264 x 78, the same modal style and scrim as 33. The body wraps to 2 lines.

**Behaviour implied:**
- The close is destructive on the server.
- The body text states that quitting the client does not end programs.

**Polish notes:**
- Nothing tells you which button is the default or Enter target.
- The destructive button is wider than the neutral one. That is fine.
- The singular wording "its 1 tab" reads slightly mechanical.

---

## 35: Tab context menu

**What it is:** right-click on the tab chip `shuv@shuvbot:~` in the toolbar. The menu drops below and right of the chip, at about x 174 to 305.

**Items:**
- Header: **`shuv@shuvbot:~`**
- **`Rename tab...`** `›`: highlighted
- **`Close tab...`** `›`
- **`New tab`**, with the shortcut hint **`Ctrl Shift T`** (right-aligned, muted)

**Visual spec:** the same menu style as 31. Shortcut hints are about 6 px and muted.

**Polish notes:**
- The menu overlaps the tab chip it came from, so the chip label is partly covered (`shuv@shuvbot:~` is half hidden).
- The menu also covers the first character columns of the terminal prompt.
- Destructive `Close tab...` sits between two neutral items.

---

## 36: Rename tab dialog

**What it is:** opened from Rename tab....

**Content:**
- Title **`Rename tab`**
- Field label **`Tab name`**
- Value **`shuv@shuvbot:~`**, with the caret at the end.
- Buttons **`Cancel`** and **`Save`**.

The layout and styling are identical to 33.

---

## 37: Close tab confirmation

**Content:**
- Title **`Close this tab?`**
- Body: **`Ends every program in this tab on the server. Closing the app itself leaves programs running.`**
- Buttons: **`Keep running`** and **`Close and end programs`** (destructive styling).

Styling is identical to 34.

**Polish notes:**
- The title does not name the tab, while the session and pane titles name their target.
- The body says "on the server" where the session dialog names the host (`shuvbot-rex`).

---

## 38: Pane context menu

**What it is:** right-click inside the terminal pane. The menu opens at the pointer, mid-pane, at about x 347 to 478 and y 139 to 245. While the menu is open, the pane gets a 1 px focus border in a light-blue accent (`#6FB2E8`) around the whole terminal area.

**Items** (header first, then actions with their shortcut hints):

| Item | Shortcut hint |
| --- | --- |
| Header: **`Shell · shuv@shuvbot:~`** (muted) | |
| **`Split right`** (highlighted) | `Ctrl Shift D` |
| **`Split down`** | `Ctrl Alt D` |
| **`Zoom pane`** | `Ctrl Shift Enter` |
| **`Rename pane...`** | `›` |
| **`Reconnect terminal`** | `Ctrl Shift R` |
| **`Close pane...`** | `›` |

**Visual spec:** the same menu style. Shortcut hints are right-aligned and muted. The trailing `›` sits further right than the shortcut column, so the two right-edge columns do not line up.

**Polish notes:**
- The shortcut column and the `›` column are misaligned. The hints end at about x 459 and the chevrons sit at about x 466.
- There is no separator between the split and zoom group and the rename and close group.
- `Close pane...` is not styled as destructive.
- `Split down` uses Ctrl **Alt** D, which is inconsistent with Ctrl **Shift** D for Split right.

---

## 39: Rename pane dialog

**Content:**
- Title **`Rename pane`**
- Field label **`Pane name`**
- Value **`shell`**
- Buttons **`Cancel`** and **`Save`**.

The style is identical to 33, and the pane focus border stays visible behind the scrim.

**Note:** the menu header calls the pane "Shell", but the field value is lower-case `shell`. The display name is capitalised somewhere (inferred).

---

## 40: Close pane confirmation

**Content:**
- Title **`Close shuv@shuvbot:~ in Shell?`**
- Body: **`Ends the program in this pane on shuvbot-rex. Closing the app itself keeps programs running.`**
- Buttons **`Keep running`** and **`Close and end programs`**.

**Polish notes:**
- The body says "keeps programs running", but the session and tab dialogs say "leaves programs running", so the wording is inconsistent.
- The button says "programs" (plural) where the body says "the program".
- The title "Close shuv@shuvbot:~ in Shell?" reads awkwardly: it combines the tab title and the pane name.

---

## 41: Commands

**What it is:** opened from the `⌘` button at the top right, which shows a pressed state with a filled circle `#172A3B`. A keyboard shortcut probably opens it too, but the capture does not show one. The panel is a centred overlay below the toolbar, at about x 384 to 616 (about 233 px wide) and y 39 to 481. There is no scrim, so the terminal is not dimmed.

**Search field:**
- Magnifier icon, placeholder **`Search commands, sessions, and tabs...`**, focused with the caret visible.
- **`esc`** hint at the right.
- Field fill is about `#3C4757` (approx, sampled near text). The field is a lighter inset bar.

**Section `WORKSPACE`** (small, bold, letter-spaced, muted caps):

| # | Command | Shortcut / affordance |
| --- | --- | --- |
| 1 | **Switch session...** (highlighted) | `Ctrl K` `›` |
| 2 | Next tab across hosts | `Ctrl Tab` |
| 3 | Previous tab across hosts | `Ctrl Shift Tab` |
| 4 | Back in history | `Mouse Back` |
| 5 | Forward in history | `Mouse Forward` |
| 6 | Search terminal... | `Ctrl Shift F` |
| 7 | New session on shuvbot-rex | `Ctrl Shift N` |
| 8 | New tab | `Ctrl Shift T` |
| 9 | Rename session... | `›` |
| 10 | Hide sidebar | `Ctrl Shift B` |
| 11 | Turn on remote control for shuvbot-rex | (none) |
| 12 | Default shell for shuvbot-rex... | `›` |
| 13 | Settings... | `Ctrl ,` `›` |
| 14 | Check for updates | (none) |
| 15 | Add host... | `›` |
| 16 | Edit host shuvbot-rex... | `›` |
| 17 | Reconnect host | (none) |
| 18 | Rename tab... | `›` |
| 19 | Close tab... | `›` |

**Section `PANE · SHUV@SHUVBOT:~`**:

| # | Command | Shortcut / affordance |
| --- | --- | --- |
| 20 | Split right | `Ctrl Shift D` |
| 21 | Split down | `Ctrl Alt D` |
| 22 | Zoom pane | `Ctrl Shift Enter` |
| 23 | Rename pane... | `›` |
| 24 | Reconnect terminal | `Ctrl Shift R` |
| 25 | Close pane... | `›` |

**Visual spec:**
- Panel fill `#0C1524`, radius about 8 px, 1 px border, and a shadow.
- Rows are about 16 px tall. The highlight fill is `#363F4C` (approx), with radius about 4 px.
- Labels are regular, about 7 px, near-white. Shortcuts are about 6 px and muted, right-aligned at x ≈ 596, with the `›` at x ≈ 604.
- The section headers are tiny caps.
- The list is not scrolled. All 25 commands fit, ending about 9 px above the panel bottom.

**Behaviour implied:**
- Fuzzy filtering over commands, sessions and tabs (the placeholder says so).
- Commands are contextual: they include the current host name (shuvbot-rex) and the current pane title.
- The `›` marks a command that opens a sub-surface.
- Mouse back and forward buttons navigate history.

**Polish notes:**
- The `WORKSPACE` section is long, 19 items with no subgrouping. It mixes navigation, session, host, app (Settings, Check for updates) and tab commands.
- Host-scoped commands are named inconsistently: "Edit host shuvbot-rex..." and "Default shell for shuvbot-rex..." include the host name, but "Reconnect host" does not.
- When a row has both a shortcut and `›` (Switch session, Settings), the shortcut moves left. That gives a ragged shortcut column: `Ctrl K` and `Ctrl ,` end at about x 594, versus about 596 for the others.
- Close tab... and Close pane... are not styled as destructive.
- With no scrim, the overlay competes with the terminal prompt on the right.

---

## 42: Commands, filtered

**What it is:** the same panel with the query **`rename`** typed. The panel shrinks to fit the results (about 233 x 95).

**Results:**
- `WORKSPACE`: **`Rename session...`** `›` (highlighted), then **`Rename tab...`** `›`
- `PANE · SHUV@SHUVBOT:~`: **`Rename pane...`** `›`

Section headers stay while they have matches. Match characters are not highlighted in the results.

**Polish notes:** the matched substring has no emphasis.

---

## 43: Session switcher

**What it is:** a popover dropped from the session chip at the top left (about x 25 to 197 and y 25 to 194, about 172 px wide). It is reached by clicking the chip or with Ctrl K. Commands item 1 is "Switch session... Ctrl K". The sidebar search shows `^K`, so its field and this popover share the shortcut (inferred). There is no scrim.

**Controls:**
- Search field: placeholder **`Filter sessions or hosts...`**, focused, with an **`esc`** hint.
- Host header **`SHUVDEV`** (monitor icon, caps, muted) with `+` at the right. Row: **`cosmic-mesa`**.
- Host header **`SHUVBOT-REX`** (server icon) with `+`. Rows: **`lunar-basin`**, then **`✓ still-atoll`**. The current session has a check mark and the highlight fill `#222A38`, and on hover or selection shows a **pencil** (rename) icon and an **`×`** (close) icon at the right.
- Host header **`SHUVMBP`** with `+`. Row: **`brave-isle`**.
- A thin horizontal divider.
- **`New session on shuvbot-rex`** with `Ctrl Shift N`.
- **`Create on another host...`** `›`
- **`Add host...`** `›`

**Visual spec:**
- Same panel style as the menus: `#0C1524`-family fill, about 6 px radius, border and shadow.
- Session rows are about 15 px tall and indented about 16 px. The check mark sits in the left gutter.
- No session count badges here, unlike the sidebar.

**Behaviour implied:**
- Type to filter, then Enter to switch.
- `+` on a host creates a session there.
- The pencil icon starts an inline rename (screenshot 49). The `×` closes the session (with a confirmation, inferred).

**Polish notes:**
- Only the selected row shows its rename and close actions, so they are not discoverable on the other rows.
- The pencil and `×` icons are very small, about 5 px.
- The popover covers the sidebar header (the `Sessions` label shows as "Ses").

---

## 44: Session switcher, filtered

**What it is:** the switcher with the query **`still`**.

**Results:**
- `SHUVBOT-REX` (with `+`): **`✓ still-atoll`**, highlighted, with pencil and `×`.
- A divider.
- **`New session "still" on shuvbot-rex`**. No shortcut hint in this state.
- **`Create "still" on another host...`** `›`
- **`Add host...`** `›`

**Behaviour implied:**
- The query doubles as a name for a new session.
- Hosts with no matches are hidden.

**Polish notes:** the `Ctrl Shift N` hint disappears once the label contains a query.

---

## 45: New-session host chooser

**What it is:** the submenu reached from **`Create "still" on another host...`**. It replaces the switcher contents in place, in a popover at about x 25 to 197 and y 25 to 86.

**Items:**
- Header (muted): **`Create "still" on...`**
- **`shuvdev`**: highlighted
- **`shuvbot-rex`**
- **`shuvmbp`**

**Visual spec:** the same menu style. Host names are lower-case (the sidebar headers are upper-case), and there are no host icons.

**Polish notes:**
- The list labelled "another host" still includes the current host `shuvbot-rex`.
- There is no back affordance to return to the switcher. The capture shows no back control.

---

## 46: Terminal search, empty

**What it is:** opened with Ctrl Shift F ("Search terminal..." in Commands). A compact floating search bar is anchored to the top right of the pane, at about x 851 to 993 (about 142 px wide) and y 35 to 50.

**Controls:**
- Magnifier icon, placeholder **`Search terminal`** (no ellipsis), and the caret.
- **`esc`** hint at the right.

**Visual spec:**
- Fill about `#0C1524` to `#232D3D`, radius about 6 px, 1 px border.
- The text is the proportional UI font.

**Polish notes:** the bar sits on top of the right-hand prompt segment (`shuv@shuvbot < 00:21:37`), which shows through above and around it as clipped glyphs. The bar overlaps live terminal text, including text it may be matching.

---

## 47: Terminal search with matches

**Content:**
- Query **`shuv`**
- Counter **`1/2`** at the right, replacing the `esc` hint.

**Matches in the terminal:** two highlight rectangles on the right prompt (`shuv@shuvbot`):
- Inactive match: dim blue `#1D3D58`.
- Active match: brighter blue `#406E94`.

Both matches are partly hidden behind the search bar itself.

**Behaviour implied:**
- Live incremental search.
- Enter and Shift Enter move through the matches (inferred, not shown).
- The counter shows the current and total match.

**Polish notes:**
- The matches here are hidden under the bar. The bar does not move away from them.
- There are no previous or next buttons, and no case or regex toggles.

---

## 48: Terminal search, no matches

**Content:**
- Query **`shuv-no-match`**
- Status **`No matches`** at the right, small and muted.

The field gets no red or error tint.

**Polish notes:** the "No matches" status uses the same muted style as the counter. That is calm but easy to miss.

---

## 49: Inline session rename in the switcher

**What it is:** the pencil icon in the session switcher on the `still-atoll` row. The row turns into an inline text field.

**Content:**
- `✓` stays in the gutter.
- Editable text **`still-atoll`** with a selection highlight behind the text and the caret at the end.
- **`enter`** hint at the right, replacing the pencil and `×`.
- The rest of the switcher is unchanged: the filter field still reads `Filter sessions or hosts...` with `esc`, followed by the host groups, the divider, New session on shuvbot-rex `Ctrl Shift N`, Create on another host... `›` and Add host... `›`.

**Visual spec:**
- The row becomes a field with fill `#18212F` and a 1 px light-blue accent border (about `#3D607D` to `#5A86B0`). Radius is about 5 px. It keeps the row height of about 15 px.

**Behaviour implied:**
- Enter commits the rename.
- Esc cancels (inferred).

**Polish notes:**
- The search field above keeps its `esc` hint while the inline field is focused, so it is unclear which surface Esc applies to.
- The selection highlight fill is very close to the field fill and has low contrast.

---

## Cross-cutting observations

- **One menu style** is shared by the host, session, tab and pane menus, the switcher and the host chooser: dark `#0C1524`, about 6 px radius, 1 px border, shadow, about 15 px rows, highlight `#222A38`, and a muted context header line.
- **One dialog style** is shared by the rename dialogs and the confirmations: centred, about 264 px wide, scrim, about 10 px radius, pill buttons. Primary actions are pale blue and destructive actions are red-outlined with red text.
- **Destructive menu items** (Remove host, Close session, tab and pane) are never styled as destructive in menus or Commands. Only the confirmation dialog shows red.
- **No separators** inside the context menus. Commands uses section headers instead.
- **Shortcut format:** `Ctrl Shift X` spelled out with spaces in menus and Commands, but `^K` in the sidebar search field. The two notations are inconsistent.
- **Focus accents:**
  - The session row gets a light-blue ring when its menu or dialog is open.
  - The pane gets a light-blue 1 px border when it is the target of a menu or dialog.
  - The inline rename field gets an accent border.
- **Unpolished:**
  - Terminal search covers the text it searches (46 to 48).
  - The shortcut and chevron columns are misaligned (38, 41).
  - "leaves" versus "keeps" programs running (34/37 versus 40).
  - The chooser offers the current host under "another host" (45).
  - The tab menu covers its own tab chip (35).
  - Sidebar count badges add noise.
  - Commands is a long, flat list.

---

## Feature inventory

- [sidebar.toggle] Show or hide the session sidebar: toolbar button and Ctrl Shift B ("Hide sidebar").
- [sidebar.header.new-session] `+` next to the "Sessions" header.
- [sidebar.search] "Find a session" field with the `^K` hint.
- [sidebar.host-groups] Sessions grouped under host headers (caps name, monitor or server icon) that collapse with a disclosure chevron.
- [sidebar.host.new-session] `+` on each host header creates a session on that host.
- [sidebar.session-row] Session row: `>_` glyph, name, count badge (shows `1`; tabs or panes, inferred). The current session is filled and semibold.
- [sidebar.footer.add-host] Footer "+ Add host".
- [sidebar.footer.count] Footer total "4 sessions".
- [menu.host] Host context menu: header with the host name; New session; Default shell... ›; Edit host... ›; Remove host... ›.
- [menu.session] Session context menu: header `host · session`; Rename session... ›; Move up; Move down (disabled at the edges); Close session... ›.
- [menu.tab] Tab context menu: header with the tab title; Rename tab... ›; Close tab... ›; New tab (Ctrl Shift T).
- [menu.pane] Pane context menu: header `Shell · <title>`; Split right (Ctrl Shift D); Split down (Ctrl Alt D); Zoom pane (Ctrl Shift Enter); Rename pane... ›; Reconnect terminal (Ctrl Shift R); Close pane... ›.
- [session.reorder] Move a session up or down within its host.
- [dialog.rename-session] Rename session dialog: "Session name" field; Cancel / Save.
- [dialog.rename-tab] Rename tab dialog: "Tab name" field; Cancel / Save.
- [dialog.rename-pane] Rename pane dialog: "Pane name" field; Cancel / Save.
- [dialog.close-session] "Close <session>?": body names the tab count and host, says that closing the app leaves programs running; buttons Keep running / Close and end programs.
- [dialog.close-tab] "Close this tab?": same pattern, "on the server".
- [dialog.close-pane] "Close <tab> in <pane>?": same pattern, "keeps programs running".
- [commands.open] Commands overlay: `⌘` toolbar button; search "Search commands, sessions, and tabs..."; `esc` to dismiss.
- [commands.filter] Commands filter by text and keep their section headers (WORKSPACE, PANE · <title>).
- [commands.switch-session] Switch session... (Ctrl K).
- [commands.next-tab] Next tab across hosts (Ctrl Tab).
- [commands.prev-tab] Previous tab across hosts (Ctrl Shift Tab).
- [commands.history-back] Back in history (Mouse Back).
- [commands.history-forward] Forward in history (Mouse Forward).
- [commands.search-terminal] Search terminal... (Ctrl Shift F).
- [commands.new-session] New session on <current host> (Ctrl Shift N).
- [commands.new-tab] New tab (Ctrl Shift T).
- [commands.rename-session] Rename session... ›
- [commands.hide-sidebar] Hide sidebar (Ctrl Shift B).
- [commands.remote-control] Turn on remote control for <host> (no shortcut).
- [commands.default-shell] Default shell for <host>... ›
- [commands.settings] Settings... (Ctrl ,) ›
- [commands.check-updates] Check for updates.
- [commands.add-host] Add host... ›
- [commands.edit-host] Edit host <host>... ›
- [commands.reconnect-host] Reconnect host.
- [commands.rename-tab] Rename tab... ›
- [commands.close-tab] Close tab... ›
- [commands.split-right] Split right (Ctrl Shift D).
- [commands.split-down] Split down (Ctrl Alt D).
- [commands.zoom-pane] Zoom pane (Ctrl Shift Enter).
- [commands.rename-pane] Rename pane... ›
- [commands.reconnect-terminal] Reconnect terminal (Ctrl Shift R).
- [commands.close-pane] Close pane... ›
- [switcher.open] Session switcher popover from the toolbar session chip (Ctrl K): "Filter sessions or hosts..." with `esc`.
- [switcher.list] Host-grouped session list. The current session has `✓`, and each host has a `+`.
- [switcher.row-actions] Pencil (inline rename) and `×` (close) on the highlighted session row.
- [switcher.new-session] "New session on <host>" (Ctrl Shift N). With a query: `New session "<query>" on <host>`.
- [switcher.create-elsewhere] "Create on another host..." ›. With a query: `Create "<query>" on another host...`.
- [switcher.add-host] "Add host..." ›
- [switcher.host-chooser] Host chooser `Create "<name>" on...` lists all saved hosts (shuvdev, shuvbot-rex, shuvmbp).
- [switcher.inline-rename] Inline rename field in the switcher row with the `enter` hint.
- [search.terminal] Terminal search bar at the top right of the pane: placeholder "Search terminal", `esc` hint.
- [search.terminal.counter] Match counter `n/total` (for example 1/2). Active match highlighted brighter than the others.
- [search.terminal.no-matches] "No matches" status.
