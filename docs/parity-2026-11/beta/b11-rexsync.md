# B11: Diagnostics > RexSync, pages 2 to 11 (Rex Beta 0.1.0 (1932), macOS)

Source: `rex-beta-ui-2026-11-10/screenshots/115-rexsync-page-02.png` to `124-rexsync-page-11.png`, with the accessibility trees in `rex-beta-ui-2026-11-10/evidence/115-…json` to `124-…json`. None of these captures has a `-display.png`. Captured 2026-11-11 00:05:19 to 00:05:3x PST (from `captures.jsonl`). The window title in every capture is `RexSync` (window id w-919, the same Diagnostics window used for 113/114).

Page 1 of this pane, scrolled to the top, is `099-diagnostics-rexsync.png` and is outside this file's scope. Pages 2 to 11 are the same RexSync diagnostics pane scrolled down about 138 pt per capture. The vertical scrollbar value goes from 0.1 (page 2) to 1.0 (page 11) in steps of 0.1. Together they show the whole pane from the end of "Terminal Policy" down to the last row.

Coordinates below are in points. Each screenshot is 1203 x 767 px at 1 px = 1 pt and includes the window drop-shadow margin. The window occupies image x 56 to 1147 and y 37 to 692. In the accessibility tree the window is at screen (480, 211), size 1091 x 655.

---

## Common chrome (identical in all ten screenshots)

### What it is
This is the **Diagnostics** window with the **RexSync** category selected in its sidebar. It is a read-only inspector for the client's sync engine (RexSync) state, followed by one section per known host and the host's sessions and blocks. It is reached from the Diagnostics window's sidebar by clicking "RexSync". The Diagnostics window itself is covered in 096 to 114 (inferred: 113 shows that its host menu is reached from the same window).

### Window and title area
- Standard macOS traffic lights: `Close` (16x16 at 498,229), `Minimize` (521,229) and `Zoom` (544,229). They are red, yellow and green.
- `Hide Sidebar` button (44x52 hit area at 572,211). It is drawn as a circular, darker glass disc (about 36 pt) containing the sidebar-left glyph: a rounded rectangle with a vertical divider and small lines in the left pane.
- Title block left-aligned at x 628, just right of the sidebar. Two static texts share a single 854x52 frame:
  - `RexSync`: bold, about 13 pt, near-white (#CACBCC sampled).
  - `Diagnostics`: subtitle, smaller (about 11 pt), regular weight, grey (#8C8E90 sampled).
- `Show Inspector` button at the top right (41x52 hit area at 1526,211). It is a translucent rounded-capsule "glass" button about 54x36 px (fill about #2D3237 to #33383D), with an "i in a circle" glyph (info.circle) in #C1C9CF.
- **The title bar is transparent.** There is no opaque toolbar background or divider. Scrolled content passes under the title block and the inspector button, and in every page the row text visibly collides with "RexSync / Diagnostics". Examples: 116 "Event Sessions" under the title; 118 the host-section heading `unix:///Users/…` overprinting "RexSync"; 121 "Host ID" under "Diagnostics"; 124 "Window Count" under "RexSync". Near the inspector button the row values and copy icons show through or under the glass capsule (120: "…(NIOCore.IOError erro…" clipped under it). There is no scroll-edge blur or fade visible beyond the glass capsule.
- The window has large, continuous rounded corners (macOS 26 style, about 16 to 20 pt radius, inferred from the image) and a soft drop shadow (black at about 49% alpha just below the window, fading to 0).

### Sidebar
- Accessibility role `outline` named `Sidebar`, at (480,263) and 144 wide. A splitter at x 623 (value `143`, state disabled, so it cannot be resized).
- Background #1C2024. It is slightly lighter than the content area and runs the full height under the traffic lights.
- Six rows. Each row cell is 124x32 at x 490 (10 pt inset) and contains a 112x30 button at x 496. The rows are at a 32 pt pitch, with the first at y 263 (52 pt below the window top).
  1. `Rex Binary`: glyph is a terminal window (rounded rectangle with `>_`).
  2. `Rex Server`: glyph is a server rack (rounded rectangle with three horizontal bays and dots).
  3. `Rex API`: glyph is a globe with meridians.
  4. `RexSync` (**selected**): glyph is two circular arrows (arrow.triangle.2.circlepath style).
  5. `Network`: glyph is three nodes joined by lines in a triangle (network topology).
  6. `GPU`: glyph is a gauge or speedometer dial in a circle.
- Text is about 13 pt regular, light grey (#BFC7CD); icons are outlined, monochrome and the same colour.
- Selected treatment: a solid accent-blue rounded-rectangle pill (#006FFE; anti-aliased edge #046CF2) about 124x32 with about 7 pt corner radius. The glyph and label are white. There is no separate focus ring. The accessibility tree reports `selected` on the button and `focused` on every cell.

### Content area
- Background #0B0F13 (near-black, slightly blue). It spans x 624 to 1570 (946 pt), with a second disabled splitter at x 1570 (value `1090`) marking the window edge or the hidden-inspector boundary.
- A vertical scroll view. Its scrollbar is at x 1553, 19 wide and 597 tall. The thumb is 6 pt wide, about 189 pt tall and #4A4C4F, overlay style with no visible track. The scrollbar `value` is shown per page below. Its two scroll-arrow buttons are 0x0 and offscreen, so there are no stepper arrows.
- **Inset grouped cards ("Form/grouped" look):**
  - Card fill #12161A on the #0B0F13 background. The card spans screen x 640 to 1555 (915 pt), leaving 16 pt from the sidebar edge and about 15 pt to the window edge, where the scrollbar overlays.
  - Corner radius about 10 pt (from a 4x crop of the bottom-left corner in 124). No border and no shadow.
  - Bottom padding: about 18 pt below the last row's text to the card edge, where the last row has no divider.
- **Section headers** sit outside the card, just above it:
  - Accessibility role `heading`, 14 pt tall, starting at x 650, so they are indented about 10 pt from the card edge and 5 pt more than the row labels.
  - About 11 pt regular proportional text in light grey #BCC3CA, brighter than the row labels and not uppercase.
  - Spacing: the last row's text of the previous card is 44 pt above the header, and the first row's text is 33 pt below it. The header sits about 10 pt above the top of its card.
- **Rows** (label/value pairs; all read-only):
  - Pitch is **39 pt** for single-line rows. Two-line wrapped values make the row **48 pt**, or 34 pt from a single-line row up to the next.
  - Label on the left at x 645, only about 5 pt inside the card edge. It is about 13 pt regular proportional text in a muted grey (strongest pixel about #767C81).
  - Value right-aligned so that it ends at x 1528. It is about 13 pt regular proportional text (not monospace, even for IDs) in a light grey to near-white (#B8C0C6 to #E1E6EA anti-aliased). Long values wrap onto two right-aligned lines with a maximum value width of about 400 pt. In a wrapped row the label stays aligned to the first line (top-aligned), and the copy button is vertically centred on the value block.
  - **Copy button** at x 1537 (12x15), 9 pt right of the value and 6 pt from the card edge. Its glyph is two overlapping documents (doc.on.doc) in #C1C9CF. Accessibility role `button`, with no visible label and a `description` of `Copy <row label in lower case>` (for example `Copy scheme`, `Copy hosts migrated`, `Copy block block:01a1c993-…`). The accessibility `name` of every value and copy button is the section title (`Terminal Policy` or the host URL).
  - **Dividers:** 1 px hairlines in #26292D between rows. They are inset about 6 pt from both card edges (image x 222 to 1124), so they run under both the label and the copy button. There is no divider after a card's last row.
  - There are no hover, pressed or selected states for rows in these captures. Rows are not selectable cells; only the copy buttons are interactive.

### Full pane content (assembled from all ten trees, top to bottom)
This is in order. The **On screen in** column lists the pages where each row is visible on screen, meaning it is not marked `offscreen` in the tree.

**(page 1 only)** In-content large title `RexSync` (static text, 69x21, about 17 pt bold, inferred from the size).

**Section header: `Terminal Policy`**

| # | Label | Value (verbatim) | Copy button description | On screen in |
|---|---|---|---|---|
| 1 | Scheme | dark | Copy scheme | 02 (partly under title) |
| 2 | Cursor Style | block | Copy cursor style | 02 (under title) |
| 3 | Cursor Blink | true | Copy cursor blink | 02 |
| 4 | Option Key | actsAsAlt | Copy option key | 02 |
| 5 | Event Sessions | 2 | Copy event sessions | 02, 03 (under title) |
| 6 | Hosts Migrated | Yes (rex.sync.hosts-migrated.v1) | Copy hosts migrated | 02, 03 |
| 7 | Pending Mutations | 0 | Copy pending mutations | 02, 03 |
| 8 | Pending Pane Commands | 0 | Copy pending pane commands | 02, 03, 04 (clipped at top) |
| 9 | Launch to First Drawable | 777 ms | Copy launch to first drawable | 02, 03, 04 |
| 10 | Launch Prewarm | Finished (presented) | Copy launch prewarm | 02 to 04 |
| 11 | Last Host Reset | None | Copy last host reset | 02 to 04 |

Despite the header name, this card mixes terminal policy (scheme, cursor, option key) with sync-engine counters and launch timing.

**Section header: `unix:///Users/shuv/Library/Application%20Support/rex/server.sock`** (one card per host. The header text is the host's endpoint URL. Only one host exists in this capture.)

| # | Label | Value (verbatim) | Lines | Copy description |
|---|---|---|---|---|
| 12 | ID | A4D68E7A-8A6E-4C39-9D9D-CFFEA559A42C | 1 | Copy id |
| 13 | Kind | transient | 1 | Copy kind |
| 14 | Name | unix:///Users/shuv/Library/Application%20Support/rex/server.sock | 1 | Copy name |
| 15 | Icon | None | 1 | Copy icon |
| 16 | Connection | ready | 1 | Copy connection |
| 17 | Resumes | 0 control, 0 blocks | 1 | Copy resumes |
| 18 | Last Resume Failure | control attempt 1: The Rex control connection closed: The operation couldn’t be completed. (NIOCore.IOError error 1.) | 2 (wraps after "The operation") | Copy last resume failure |
| 19 | Resolved Endpoint | unix:///Users/shuv/Library/Application%20Support/rex/server.sock | 1 | Copy resolved endpoint |
| 20 | Instance ID | 2e3a66ea-d1ed-48fc-81b4-3b7a5543d8e8 | 1 | Copy instance id |
| 21 | Instance Generation | 0 | 1 | Copy instance generation |
| 22 | Host ID | HostID(rawValue: "c0332abd-8b89-4270-aca2-d1a7a3745230") | 1 | Copy host id |
| 23 | Client ID | ClientID(rawValue: "client:01a1c98f-73c0-7192-b125-a22ccf68bb1f") | 1 | Copy client id |
| 24 | Tailscale | state=running dns=shuvbot-rex.tail586a6d.ts.net tailnet=shuv1337.github | 1 | Copy tailscale |
| 25 | Last Failure | None | 1 | Copy last failure |

The same card continues with no sub-header and no extra spacing. It has one group of rows per session. The group for session 1 (`lunar-basin`):

| # | Label | Value (verbatim) | Lines | Copy description |
|---|---|---|---|---|
| 26 | Session ID | SessionID(rawValue: "session:01a1c993-7fd0-78ed-b86f-78ede44c4365") | 2 (wraps after "78ed-") | Copy session id |
| 27 | Session Label | lunar-basin | 1 | Copy session label |
| 28 | Session Status | live | 1 | Copy session status |
| 29 | Structure Status | current | 1 | Copy structure status |
| 30 | Demands | 6 structure, 6 attachment, 2 content | 1 | Copy demands |
| 31 | Attachment Status | attached(RexAPI.ClientID(rawValue: "client:01a1c98f-73c0-7192-b125-a22ccf68bb1f")) | 2 (wraps after "b125-") | Copy attachment status |
| 32 | Window Count | 1 | 1 | Copy window count |
| 33 | Presented Window | WindowID(rawValue: "window:01a1c993-7fef-7e21-8f1f-a900ed0732bb") | 1 | Copy presented window |
| 34 | Reporting Presentation | 7B8F77C7-12BA-4659-8F0A-32ED08CB3284 | 1 | Copy reporting presentation |
| 35 | Reporting Generation | 1 | 1 | Copy reporting generation |
| 36 | Block block:01a1c993-7fd0-7af8-aca8-35abeb7960f4 | open | 1 | Copy block block:01a1c993-7fd0-7af8-aca8-35abeb7960f4 |

The group for session 2 (`still-atoll`):

| # | Label | Value (verbatim) | Lines | Copy description |
|---|---|---|---|---|
| 37 | Session ID | SessionID(rawValue: "session:01a1c993-f469-7042-bb22-aadd2dd2da1f") | 2 (wraps after "bb22-") | Copy session id |
| 38 | Session Label | still-atoll | 1 | Copy session label |
| 39 | Session Status | live | 1 | Copy session status |
| 40 | Structure Status | current | 1 | Copy structure status |
| 41 | Demands | 3 structure, 4 attachment, 0 content | 1 | Copy demands |
| 42 | Attachment Status | attached(RexAPI.ClientID(rawValue: "client:01a1c98f-73c0-7192-b125-a22ccf68bb1f")) | 2 | Copy attachment status |
| 43 | Window Count | 1 | 1 | Copy window count |
| 44 | Presented Window | WindowID(rawValue: "window:01a1c993-f488-7340-a053-bbd45fe71955") | 2 (wraps after "a053-") | Copy presented window |
| 45 | Reporting Presentation | None | 1 | Copy reporting presentation |
| 46 | Reporting Generation | 0 | 1 | Copy reporting generation |
| 47 | Block block:01a1c993-f469-7255-a1ba-982ffc8a62aa | cold | 1 | Copy block block:01a1c993-f469-7255-a1ba-982ffc8a62aa |

This is the end of the pane (124 shows the card's rounded bottom about 15 pt above the window bottom).

Notes on the content:
- Session groups are not visually separated. "Session ID" simply follows the previous host or session's last row, divided by the same hairline. A session's block rows are labelled `Block <block id>`, with the block state (`open`, `cold`) as the value. There is one block row per block (one each here).
- Values are raw debug strings, including Swift `rawValue:` wrappers (for example `HostID(rawValue: "…")`, `attached(RexAPI.ClientID(rawValue: …))`) and enum case names (`actsAsAlt`, `transient`, `ready`, `live`, `current`).
- The Event Sessions count (`2`) matches the two session groups listed under the host.

---

## 115-rexsync-page-02.png

**What it is:** The RexSync pane scrolled one step (scrollbar value 0.1, thumb near the top at y 306 to 495). Reached from Diagnostics > sidebar `RexSync`, then by scrolling down once.

**Visible, in visual order (image y):**
- y 43 to 81 under the title block: `Scheme` / `dark` (y 43, partly hidden behind "RexSync") and `Cursor Style` / `block` (y 81, overprinted by "Diagnostics"). Their copy icons sit partly under the inspector glass capsule.
- `Cursor Blink` / `true` · `Option Key` / `actsAsAlt` · `Event Sessions` / `2` · `Hosts Migrated` / `Yes (rex.sync.hosts-migrated.v1)` · `Pending Mutations` / `0` · `Pending Pane Commands` / `0` · `Launch to First Drawable` / `777 ms` · `Launch Prewarm` / `Finished (presented)` · `Last Host Reset` / `None`. The Terminal Policy card's rounded bottom is at about image y 455.
- Section header `unix:///Users/shuv/Library/Application%20Support/rex/server.sock` (y 475).
- New card: `ID` / `A4D68E7A-8A6E-4C39-9D9D-CFFEA559A42C` · `Kind` / `transient` · `Name` / `unix:///Users/shuv/Library/Application%20Support/rex/server.sock` · `Icon` / `None` · `Connection` / `ready`. The next row's divider is at the window bottom.
- Every row has a copy icon at the right.

**Layout and visual:** This page shows the card break between two groups: about 18 pt of card bottom padding, the 20 pt gap to the header (header text sits in the #0B0F13 gap), then the new card's rounded top. Card top-corner radius matches the bottom, at about 10 pt. Everything else is as in Common chrome.

**Behaviour implied:** Read-only status. Each copy button copies that row's value (inferred from `Copy <label>`) to the clipboard.

## 116-rexsync-page-03.png

**What it is:** Scrollbar value 0.2. The Terminal Policy tail and the top of the host card.

**Visible, in order:** `Event Sessions` (clipped under the title; value hidden) · `Hosts Migrated` / `Yes (rex.sync.hosts-migrated.v1)` · `Pending Mutations` / `0` · `Pending Pane Commands` / `0` · `Launch to First Drawable` / `777 ms` · `Launch Prewarm` / `Finished (presented)` · `Last Host Reset` / `None` · [card end] · header `unix:///Users/shuv/Library/Application%20Support/rex/server.sock` · `ID` / `A4D68E7A-8A6E-4C39-9D9D-CFFEA559A42C` · `Kind` / `transient` · `Name` / `unix:///…/rex/server.sock` · `Icon` / `None` · `Connection` / `ready` · `Resumes` / `0 control, 0 blocks` · `Last Resume Failure` / `control attempt 1: The Rex control connection closed: The operation` + newline + `couldn’t be completed. (NIOCore.IOError error 1.)` · `Resolved Endpoint` / `unix:///Users/shuv/Library/Application%20Support/rex/server.sock` · `Instance ID` / `2e3a66ea-d1ed-48fc-81b4-3b7a5543d8e8` (cut by the window bottom).

**Layout and visual:** This is the first appearance of a wrapped two-line value. The label "Last Resume Failure" is top-aligned with the value's first line (both at screen y 764). The copy icon is centred on the two-line block, and the row is 48 pt tall instead of 39. The value wraps right-aligned with a ragged left edge.

**Behaviour implied:** The host card reports connection health, the resume counters and the last error string verbatim from the network layer.

## 117-rexsync-page-04.png

**What it is:** Scrollbar value 0.3.

**Visible, in order:** `Pending Pane Commands` / `0` (clipped under title) · `Launch to First Drawable` / `777 ms` (value partly behind the inspector capsule) · `Launch Prewarm` / `Finished (presented)` · `Last Host Reset` / `None` · [card end] · header `unix:///Users/shuv/Library/Application%20Support/rex/server.sock` · `ID` · `Kind` · `Name` · `Icon` · `Connection` · `Resumes` · `Last Resume Failure` (2 lines) · `Resolved Endpoint` · `Instance ID` / `2e3a66ea-d1ed-48fc-81b4-3b7a5543d8e8` · `Instance Generation` / `0` · `Host ID` / `HostID(rawValue: "c0332abd-8b89-4270-aca2-d1a7a3745230")` · `Client ID` / `ClientID(rawValue: "client:01a1c98f-73c0-7192-b125-a22ccf68bb1f")`. Values are as in the full content table.

**Layout and visual:** As in Common chrome. The thumb is a little lower.

## 118-rexsync-page-05.png

**What it is:** Scrollbar value 0.4. The host section header has scrolled up under the title area.

**Visible, in order:** header `unix:///Users/shuv/Library/Application%20Support/rex/server.sock` drawn over "RexSync" at the very top (image y about 60). This is not a sticky header: the tree places it at screen y 226, which is simply where it has scrolled to. Then `ID` · `Kind` · `Name` · `Icon` · `Connection` · `Resumes` · `Last Resume Failure` (2 lines) · `Resolved Endpoint` · `Instance ID` · `Instance Generation` / `0` · `Host ID` · `Client ID` · `Tailscale` / `state=running dns=shuvbot-rex.tail586a6d.ts.net tailnet=shuv1337.github` · `Last Failure` / `None` · `Session ID` / `SessionID(rawValue: "session:01a1c993-7fd0-78ed-` + newline + `b86f-78ede44c4365")` · `Session Label` / `lunar-basin` (cut at the bottom).

**Layout and visual:** The overlap of the header text with the window title shows clearly that the title area has no backdrop. The first session's rows start directly after "Last Failure" with an ordinary divider and no sub-heading.

**Behaviour implied:** The Tailscale row shows the tailnet state, the MagicDNS name and the tailnet name of the host connection as `key=value` pairs.

## 119-rexsync-page-06.png

**What it is:** Scrollbar value 0.5 (thumb at mid-track).

**Visible, in order:** `Name` value (clipped under title) · `Icon` / `None` (label under "Diagnostics"; value partly behind the capsule) · `Connection` / `ready` · `Resumes` / `0 control, 0 blocks` · `Last Resume Failure` (2 lines) · `Resolved Endpoint` · `Instance ID` · `Instance Generation` / `0` · `Host ID` · `Client ID` · `Tailscale` · `Last Failure` / `None` · `Session ID` (2 lines, `…7fd0-78ed-` / `b86f-78ede44c4365")`) · `Session Label` / `lunar-basin` · `Session Status` / `live` · `Structure Status` / `current` · `Demands` / `6 structure, 6 attachment, 2 content`.

**Layout and visual:** As in Common chrome.

## 120-rexsync-page-07.png

**What it is:** Scrollbar value 0.6.

**Visible, in order:** `Last Resume Failure` (at the very top. The value's right end is hidden under the inspector capsule, so it reads `…The operation` / `couldn’t be completed. (NIOCore.IOError erro…`.) · `Resolved Endpoint` · `Instance ID` · `Instance Generation` · `Host ID` · `Client ID` · `Tailscale` · `Last Failure` · `Session ID` (2 lines) · `Session Label` / `lunar-basin` · `Session Status` / `live` · `Structure Status` / `current` · `Demands` / `6 structure, 6 attachment, 2 content` · `Attachment Status` / `attached(RexAPI.ClientID(rawValue: "client:01a1c98f-73c0-7192-b125-` + newline + `a22ccf68bb1f"))` · `Window Count` / `1` · `Presented Window` / `WindowID(rawValue: "window:01a1c993-7fef-7e21-8f1f-a900ed0732bb")` · `Reporting Presentation` / `7B8F77C7-12BA-4659-8F0A-32ED08CB3284` (cut at the bottom).

**Layout and visual:** The inspector glass capsule sits over content and obscures the value text, so there is no content inset for the toolbar area.

## 121-rexsync-page-08.png

**What it is:** Scrollbar value 0.7.

**Visible, in order:** `Instance Generation` (clipped) · `Host ID` (label under the title; value `HostID(rawValue: "c0332abd-8b89-4270-aca2-d1a7a374523…` cut by the capsule) · `Client ID` · `Tailscale` · `Last Failure` / `None` · `Session ID` (2 lines) · `Session Label` / `lunar-basin` · `Session Status` / `live` · `Structure Status` / `current` · `Demands` / `6 structure, 6 attachment, 2 content` · `Attachment Status` (2 lines) · `Window Count` / `1` · `Presented Window` / `WindowID(rawValue: "window:01a1c993-7fef-7e21-8f1f-a900ed0732bb")` · `Reporting Presentation` / `7B8F77C7-12BA-4659-8F0A-32ED08CB3284` · `Reporting Generation` / `1` · `Block block:01a1c993-7fd0-7af8-aca8-35abeb7960f4` / `open` · `Session ID` / `SessionID(rawValue: "session:01a1c993-f469-7042-bb22-` + newline + `aadd2dd2da1f")` (second session begins).

**Layout and visual:** The block row's label is a long identifier and is still drawn in the label style (grey, proportional). The value `open` is right-aligned as usual.

## 122-rexsync-page-09.png

**What it is:** Scrollbar value 0.8.

**Visible, in order:** `Last Failure` / `No…` (clipped under the title and capsule) · `Session ID` (lunar-basin, 2 lines) · `Session Label` / `lunar-basin` · `Session Status` / `live` · `Structure Status` / `current` · `Demands` / `6 structure, 6 attachment, 2 content` · `Attachment Status` (2 lines) · `Window Count` / `1` · `Presented Window` / `WindowID(rawValue: "window:01a1c993-7fef-7e21-8f1f-a900ed0732bb")` · `Reporting Presentation` / `7B8F77C7-12BA-4659-8F0A-32ED08CB3284` · `Reporting Generation` / `1` · `Block block:01a1c993-7fd0-7af8-aca8-35abeb7960f4` / `open` · `Session ID` / `SessionID(rawValue: "session:01a1c993-f469-7042-bb22-` / `aadd2dd2da1f")` · `Session Label` / `still-atoll` · `Session Status` / `live` · `Structure Status` / `current` · `Demands` / `3 structure, 4 attachment, 0 content` (cut at the bottom).

**Layout and visual:** Both session groups are visible. There is no gap, header or background change between the session groups.

## 123-rexsync-page-10.png

**What it is:** Scrollbar value 0.9.

**Visible, in order:** `Session Status` (clipped) · `Structure Status` / `current` (under the title) · `Demands` / `6 structure, 6 attachment, 2 content` · `Attachment Status` (2 lines) · `Window Count` / `1` · `Presented Window` / `WindowID(rawValue: "window:01a1c993-7fef-7e21-8f1f-a900ed0732bb")` · `Reporting Presentation` / `7B8F77C7-12BA-4659-8F0A-32ED08CB3284` · `Reporting Generation` / `1` · `Block block:01a1c993-7fd0-7af8-aca8-35abeb7960f4` / `open` · `Session ID` (still-atoll, 2 lines) · `Session Label` / `still-atoll` · `Session Status` / `live` · `Structure Status` / `current` · `Demands` / `3 structure, 4 attachment, 0 content` · `Attachment Status` / `attached(RexAPI.ClientID(rawValue: "client:01a1c98f-73c0-7192-b125-` / `a22ccf68bb1f"))` · `Window Count` / `1` · `Presented Window` / `WindowID(rawValue: "window:01a1c993-f488-7340-a053-` / `bbd45fe71955")`.

**Layout and visual:** The `Presented Window` value for still-atoll wraps (334 pt wide, 2 lines). The lunar-basin value of the same row stays on one line (416 pt). Wrapping happens per value at a maximum width of about 400 pt.

## 124-rexsync-page-11.png

**What it is:** Scrollbar value 1.0. This is the bottom of the pane, and the thumb is at the bottom of the track.

**Visible, in order:** `Window Count` (label under "RexSync"; value hidden) · `Presented Window` / `WindowID(rawValue: "window:01a1c993-7fef-7e21-8f1f-a900ed0732bb")` · `Reporting Presentation` / `7B8F77C7-12BA-4659-8F0A-32ED08CB3284` · `Reporting Generation` / `1` · `Block block:01a1c993-7fd0-7af8-aca8-35abeb7960f4` / `open` · `Session ID` / `SessionID(rawValue: "session:01a1c993-f469-7042-bb22-` / `aadd2dd2da1f")` · `Session Label` / `still-atoll` · `Session Status` / `live` · `Structure Status` / `current` · `Demands` / `3 structure, 4 attachment, 0 content` · `Attachment Status` (2 lines) · `Window Count` / `1` · `Presented Window` / `WindowID(rawValue: "window:01a1c993-f488-7340-a053-` / `bbd45fe71955")` · `Reporting Presentation` / `None` · `Reporting Generation` / `0` · `Block block:01a1c993-f469-7255-a1ba-982ffc8a62aa` / `cold` · [card bottom].

**Layout and visual:**
- The card's rounded bottom (about 10 pt radius) is at image y about 677, leaving about 15 pt of #0B0F13 above the window's bottom edge.
- The last row has no divider below it.
- No footer, buttons or actions follow; the pane ends with the last block row.

**Behaviour implied:** Block state per session is reported as `open` (attached and streaming, inferred) versus `cold` (not currently attached or loaded, inferred). These match the session's `Demands` (`… 2 content` versus `… 0 content`, inferred correlation).

---

## Visual spec summary (for implementers)

| Token | Value |
|---|---|
| Window (as captured) | 1091 x 655 pt, large continuous corner radius, drop shadow |
| Sidebar width / bg | 144 pt (fixed, splitter disabled) / #1C2024 |
| Sidebar row | 124x32 cell, 10 pt side inset, 32 pt pitch, about 13 pt text #BFC7CD, outlined SF-Symbol-style icon |
| Sidebar selection | solid #006FFE pill, about 7 pt radius, white text and icon |
| Title block | `RexSync` bold about 13 pt #CACBCC over `Diagnostics` about 11 pt #8C8E90, x = sidebar edge + 4; transparent toolbar |
| Inspector button | glass capsule about 54x36, `info.circle` glyph #C1C9CF |
| Content bg | #0B0F13 |
| Card | #12161A, about 10 pt radius, 16 pt outer horizontal margin, no border |
| Section header | about 11 pt regular #BCC3CA, x = card + 10, 10 pt above the card |
| Row | 39 pt pitch (48 pt when wrapped), label x = card + 5, value right edge = card − 27, copy button 12x15 at card − 18 |
| Label text | about 13 pt regular, #767C81 (muted) |
| Value text | about 13 pt regular proportional, #B8C0C6 to #E1E6EA, right-aligned, wraps at about 400 pt |
| Divider | 1 px #26292D, inset about 6 pt both sides, none after the last row |
| Scroll thumb | 6 pt wide overlay, #4A4C4F |

---

## Feature inventory

- [diagnostics.sidebar] Diagnostics window sidebar with categories: Rex Binary, Rex Server, Rex API, RexSync, Network, GPU (fixed 144 pt width, icon + label, blue selected pill)
- [diagnostics.sidebar.toggle] Hide Sidebar button in the title area
- [diagnostics.inspector.toggle] Show Inspector button (info glyph) at the top right of the Diagnostics window
- [diagnostics.title] Window title `<category>` with subtitle `Diagnostics`
- [diagnostics.copy-row] Per-row copy-to-clipboard button on every diagnostic value (accessibility description `Copy <label>`)
- [diagnostics.rexsync.layout] RexSync pane as a scrolling list of grouped inset cards with section headers; long values wrap right-aligned
- [diagnostics.rexsync.terminal-policy.scheme] Terminal Policy > Scheme (e.g. `dark`)
- [diagnostics.rexsync.terminal-policy.cursor-style] Terminal Policy > Cursor Style (e.g. `block`)
- [diagnostics.rexsync.terminal-policy.cursor-blink] Terminal Policy > Cursor Blink (e.g. `true`)
- [diagnostics.rexsync.terminal-policy.option-key] Terminal Policy > Option Key (e.g. `actsAsAlt`)
- [diagnostics.rexsync.event-sessions] Event Sessions count (e.g. `2`)
- [diagnostics.rexsync.hosts-migrated] Hosts Migrated, with migration key (e.g. `Yes (rex.sync.hosts-migrated.v1)`)
- [diagnostics.rexsync.pending-mutations] Pending Mutations count
- [diagnostics.rexsync.pending-pane-commands] Pending Pane Commands count
- [diagnostics.rexsync.launch-to-first-drawable] Launch to First Drawable timing (e.g. `777 ms`)
- [diagnostics.rexsync.launch-prewarm] Launch Prewarm state (e.g. `Finished (presented)`)
- [diagnostics.rexsync.last-host-reset] Last Host Reset (e.g. `None`)
- [diagnostics.rexsync.host-section] One section per host, headed by the host endpoint URL
- [diagnostics.rexsync.host.id] Host ID (UUID)
- [diagnostics.rexsync.host.kind] Host Kind (e.g. `transient`)
- [diagnostics.rexsync.host.name] Host Name
- [diagnostics.rexsync.host.icon] Host Icon (e.g. `None`)
- [diagnostics.rexsync.host.connection] Connection state (e.g. `ready`)
- [diagnostics.rexsync.host.resumes] Resumes counter (`N control, N blocks`)
- [diagnostics.rexsync.host.last-resume-failure] Last Resume Failure, with attempt number and error text
- [diagnostics.rexsync.host.resolved-endpoint] Resolved Endpoint URL
- [diagnostics.rexsync.host.instance-id] Server Instance ID
- [diagnostics.rexsync.host.instance-generation] Server Instance Generation
- [diagnostics.rexsync.host.host-id] Host ID (server-issued HostID)
- [diagnostics.rexsync.host.client-id] Client ID assigned by the server
- [diagnostics.rexsync.host.tailscale] Tailscale status (`state=… dns=… tailnet=…`)
- [diagnostics.rexsync.host.last-failure] Last Failure (e.g. `None`)
- [diagnostics.rexsync.session.id] Per-session Session ID
- [diagnostics.rexsync.session.label] Per-session Session Label (e.g. `lunar-basin`, `still-atoll`)
- [diagnostics.rexsync.session.status] Per-session Session Status (e.g. `live`)
- [diagnostics.rexsync.session.structure-status] Per-session Structure Status (e.g. `current`)
- [diagnostics.rexsync.session.demands] Per-session Demands (`N structure, N attachment, N content`)
- [diagnostics.rexsync.session.attachment-status] Per-session Attachment Status (e.g. `attached(<ClientID>)`)
- [diagnostics.rexsync.session.window-count] Per-session Window Count
- [diagnostics.rexsync.session.presented-window] Per-session Presented Window (WindowID)
- [diagnostics.rexsync.session.reporting-presentation] Per-session Reporting Presentation (UUID or `None`)
- [diagnostics.rexsync.session.reporting-generation] Per-session Reporting Generation counter
- [diagnostics.rexsync.session.block-state] Per-block row `Block <block id>` with state (e.g. `open`, `cold`)
