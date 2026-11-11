# Rex Linux 0.6.0: client label, host management, discovery, shortcuts (screenshots 18–29, plus 06)

Source: `rex-rune-2026-11-11/screenshots/18-*.png` to `29-*.png`. Captured November 11, 2026 (PST) on Omarchy/Hyprland, 2560 × 1410 px at 2× scale. Logical sizes below are pixel measurements divided by 2. Colours come from sampled pixels. Screenshot 06 (the top of the Shortcuts page) is outside the 18–29 range, but I read it as well so the shortcut list is complete. There is no accessibility tree, so all text was read from the pixels.

## Common backdrop (all screenshots)

- **Window chrome (behind every dialog):** a toolbar at the top left with these controls:
  - A sidebar-toggle icon (a rectangle with a left bar).
  - A session pill with a server-stack icon, the bold label **still-atoll**, the dimmer subtitle `shuvbot-rex` and a small chevron on the right.
  - A tab pill with a `>_` prompt glyph and the label `shuv@shuvbot:~`.
  - Four icons at the top right: ⌘ (Commands), + (new), a sliders icon (settings, inferred from the glyph) and × (close).
- **Terminal content:** a starship-style prompt (Apple glyph, home icon, `~`, block cursor) on the left. On the right are `✔ < shuv@shuvbot < 00:21:37` and a clock glyph, in a monospace font.
- **Modal scrim:** the terminal is dimmed to near black. The window background around the dialogs samples `#000A13`/`#000B14`. There is no visible blur.
- **Dialog surface:** fill `#0C1523`. The edge has a slightly lighter 1px ring (`#111A23`) and a soft, dark drop shadow below. The corner radius is about 10–12 logical px. Dialog dialogs are horizontally centred and are 338 logical px wide (676 px).
- **Text fields:** the fill is the same as the dialog (`#0C1523`), with a 1px hairline border of about `#18202E`. The radius is about 6 px and the height is about 20 logical px (40 px). Text is proportional (Inter-like) in near white, with a thin white I-beam caret.
- **Field labels:** small, medium weight and grey (about `#93A3B8`), placed above the field.
- **Buttons:**
  - Secondary buttons, such as **Cancel**, are pills with a dark grey fill (`#19222F`) and bold white text.
  - Primary buttons, such as **Save** and **Connect**, are pills with a light blue gradient (`#7FC0FE` on the left to `#90A9FC` on the right), dark navy bold text and a faint outer glow ring.
  - The destructive button is a dark pill with salmon-red text (`#E17B7D`/`#EA8081`) and a thin red-tinted outline.
- **Helper text:** small, grey (about `#99A3B0`), wrapping, with left edges aligned to the fields.

---

## 18: Client label (`18-client-label.png`)

**What it is:** a modal that names this client, as other Rex clients see it (inferred from the title). The screenshot does not show how it is reached. The README groups it with the Remote Hosts surfaces, so it is probably a button in Settings → Remote Hosts (inferred).

**Controls, in visual order**

1. Title **Client label**: bold, about 16 px logical, near white `#E8EEF6`.
2. Field label **Label**.
3. Text field with the value `shuvdev`. It has focus, with the caret after the text. No placeholder is visible.
4. A right-aligned button row: **Cancel** (secondary) and **Save** (primary, blue gradient).

**Layout**

- The dialog is 338 × 115 logical px, centred slightly below the window's middle (it spans y ≈ 590–820 px of 1410).
- The inner padding is about 11 logical px.
- The title sits about 8 px above the label, and the label about 4 px above the field. The field spans the full inner width.
- The buttons sit about 6 px below the field. They are about 16 px tall, with a gap of about 5 px between them.

**Behaviour implied:** Save stores a local name for this client. There is no helper text explaining where the label is shown.

**Polish notes**

- There is no explanation of what the label is for.
- The bottom padding below the buttons (about 11 px) roughly matches the top padding, so it reads as balanced.

---

## 19: Edit host (`19-edit-host.png`)

**What it is:** the edit form for the saved host `shuvdev`, probably reached from the Remote Hosts list or the host context menu (inferred).

**Controls, in visual order**

1. Title **Edit shuvdev**.
2. **Host name**: a text field with the value `shuvdev`. It has focus, with the caret at the end.
3. **Endpoint**: a text field with the value `unix:///run/user/1000/rex/server.sock`.
4. **Default shell**: a text field with the value `zsh`.
5. Helper text, wrapped over two lines: "A new endpoint connects to a different server; sessions on the old one keep running. The shell applies to new sessions, tabs and splits."
6. Buttons: **Cancel** and **Save**.

**Layout**

- About 338 × 183 logical px.
- The label-and-field pairs repeat at a pitch of about 35 logical px. The fields are full width and the same height.
- There are no section dividers.

**Behaviour implied**

- Renames the host.
- Changes its endpoint, which reconnects to a different server and leaves the old server's sessions running.
- Sets the per-host default shell used for new sessions, tabs and splits.

**Polish notes**

- The Default shell field is a free-text field, not a popup or picker. That is inconsistent with the dedicated default-shell form in Settings (screenshot 12, outside this range; inferred from the README).
- There is no validation hint for the endpoint format here, unlike in screenshot 22.

---

## 20: Remove host confirmation (`20-remove-host-confirmation.png`)

**What it is:** a destructive confirmation shown after choosing to remove the host `shuvdev`.

**Controls, in visual order**

1. Title **Remove shuvdev?**
2. Body text: "Rex Linux stops connecting to shuvdev and forgets it. Its sessions and programs keep running on that server, and you can add it again later."
3. Buttons: **Keep host** (secondary, dark pill) and **Remove host** (destructive: salmon-red text `#E17B7D` on a dark pill with a red-tinted outline).

**Layout:** about 338 × 78 logical px. The buttons are right-aligned and sit about 6 px below the body text.

**Behaviour implied:** removing a host only forgets it locally and does not stop server sessions. "Keep host" cancels.

**Polish notes**

- The buttons use verb labels rather than OK/Cancel, which is good.
- The destructive button has no filled red background, so its emphasis is moderate.
- There is no default-button glow on either button, so it is unclear which one Enter activates (inferred risk).

---

## 21: Add host via tailnet discovery (`21-add-host-tailnet-discovery.png`)

**What it is:** the Add host popup, a searchable command-palette-style list of the machines in `tailscale status --json`, grouped by whether each one runs Rex. It is probably reached from + → Add host or from Remote Hosts → Add (inferred). It is anchored at the top centre of the window, just below the toolbar, and is not a modal dialog.

**Controls, in visual order**

1. **Search field:** a magnifier icon, a focused caret, the placeholder **Search your tailnet...** and a right-aligned `esc` hint in small grey text. The field fill is `#202A39`, a raised pill inside the popup.
2. Section header **REX SERVERS**, preceded by a server-stack icon. It is small caps, letter-spaced, grey and semibold. Its rows are:

   | Name (left) | Right-hand metadata | Notes |
   | --- | --- | --- |
   | `rex` | `https://rex.tail586a6d.ts.net · view listeners · Linux` | Chevron `›` |
   | `..` | `Added as shuvbot-rex · https://shuvbot-rex.tail586a6d.ts.net · view listeners · r` | Checkmark ✓ before the name. The name is truncated to "..", the metadata overflows and is clipped at the popup edge ("r…"), and no chevron is visible. |
   | `shuvmbp-r...` | `https://shuvmbp-rex.tail586a6d.ts.net · view listeners · Linux` | Name truncated with "...". Chevron. |

3. Section header **TAILNET**, with no icon. Each row has a name on the left, metadata in grey on the right and a chevron:

   | Name | Metadata |
   | --- | --- |
   | dc-1 | SSH · Android |
   | dock | Rex check failed · SSH · Linux |
   | firewalla-1 | SSH · Linux · firewalla |
   | grokbot-user-6490604 | Rex check failed · SSH · Linux |
   | hermes-bots | SSH · Linux |
   | homeassistant | SSH · Linux |
   | latitudes-signal | SSH · Linux |
   | ltc-executor | SSH · Linux |
   | ltc-tools | Rex check failed · SSH · Linux |
   | picokvm | Rex check failed · SSH · Linux |
   | shuvbot | Rex check failed · SSH · macOS |
   | shuvdesk-relay | SSH · Linux |
   | shuvhost | SSH · Linux |
   | ✓ shuvmbp | Added as shuvmbp · SSH · Linux |

4. Section header **OFFLINE**:

   | Name | Metadata | State |
   | --- | --- | --- |
   | latitudes-firstmate | Offline · Linux | |
   | paperclip-spike | Offline · Linux | Highlighted row: pointer hover or keyboard selection, a rounded `#222A38` fill |
   | sarah-athena | Offline · Linux | |
   | secondmate-box | Offline · Linux | |
   | shuv-firstmate | Offline · Linux | |
   | shuvdeck-tab5 | Offline · Linux | |
   | shuvtest-bb | Offline · Linux | |
   | vault-ops | Offline · Linux | |

5. A hairline divider, then a footer row **Enter an endpoint...** with a chevron, which opens screenshot 22.

**Layout**

- The popup is about 298 logical px wide (595 px) and about 450 logical px tall, from just below the toolbar to y ≈ 975 px of the 1102 px display. Its fill is `#0C1524` and it has a 1px lighter border and rounded corners of about 10 px.
- Rows are about 19 logical px tall. Names are regular weight and near white; metadata is smaller grey text (about `#616E80` to `#8A94A3`), right-aligned against the chevron column.
- Section headers are small, uppercase, letter-spaced and grey.
- Only the REX SERVERS header has an icon.

**Behaviour implied**

- Each item is "· view listeners", so choosing a Rex server opens screenshot 24 with its listeners.
- Plain tailnet machines can be added over SSH (screenshot 23).
- Offline machines are listed and may still be chosen, since they have chevrons.
- A ✓ with "Added as X" marks hosts that are already saved.
- "Rex check failed" means an HTTPS probe for Rex failed.

**Polish notes**

- **Truncation bug:** in the ✓ shuvbot-rex row the name collapses to "..", and the metadata runs past the right padding with no ellipsis, cutting off at "r". The chevron is missing on that row.
- `shuvmbp-r...` is truncated even though the row has room. The long metadata takes priority over the name.
- Metadata is right-aligned, so the dot-separated tokens do not line up in columns ("SSH" sits at different x positions).
- The ✓ indent shifts the name text right compared with unchecked rows.
- The REX SERVERS header has an icon but TAILNET and OFFLINE do not, which is inconsistent.
- Showing "Rex check failed" in grey gives an error state the same weight as normal metadata.

---

## 22: Add host by endpoint (`22-add-host-manual-endpoint.png`)

**What it is:** a manual entry form reached from **Enter an endpoint...** in screenshot 21.

**Controls, in visual order**

1. Title **Add host by endpoint**.
2. **Host name**: an empty text field with the placeholder `my-host` in grey. It has focus.
3. **Endpoint**: an empty text field whose placeholder fills the whole field: `https://host.example.ts.net, ssh://user@host[/socket] or unix:///path`.
4. Helper text: "https:// reaches a Rex server over the network (a bare host name is tried on port 443, then 7297); ssh://user@host[/socket] forwards a server's socket over SSH; unix:// is a socket on this machine."
5. Buttons: **Cancel** and **Connect** (primary).

**Layout:** about 338 × 155 logical px.

**Behaviour implied**

- Accepts the `https://`, `ssh://` and `unix://` schemes.
- A bare host name is tried on port 443, then on 7297.

**Polish notes**

- The endpoint placeholder runs to the right edge of the field with almost no inner right padding.
- The placeholder grey is fairly bright, close to real-value colour, so placeholders and values are easy to confuse.
- Connect appears enabled even though both fields are empty. That may be intentional, with validation on submit (inferred).

---

## 23: Add host over SSH (`23-add-host-ssh.png`)

**What it is:** the SSH add form for a tailnet machine, reached by choosing `dock` in screenshot 21.

**Controls, in visual order**

1. Title **Add dock over SSH**.
2. **Host name**: a text field with the value `dock`. It has focus.
3. **SSH user**: a text field with the value `shuv`.
4. **Rex socket**: an empty text field with the placeholder `Found automatically — or e.g. /run/rex/server.sock`.
5. Helper text, five lines: "Forwards the Rex server's socket on dock.tail586a6d.ts.net over SSH; Rex must be running there. Couldn't tell whether Rex listens there (https 443: HTTP error: 404 Not Found); an https:// endpoint can also be entered by hand. Your SSH keys and configuration apply. Leave Rex socket empty to look it up there (rex server status, the user's runtime directory, /run/rex)."
6. Buttons: **Cancel** and **Connect**.

**Layout:** about 338 × 205 logical px.

**Behaviour implied**

- Uses `ssh -L` forwarding with the user's SSH keys and configuration.
- Finds the socket automatically through `rex server status`, the runtime directory or `/run/rex`.
- Includes the diagnostic from the failed HTTPS probe.

**Polish notes**

- The helper paragraph is long and dense, and mixes a diagnostic error ("HTTP error: 404 Not Found") into normal grey helper text without any warning styling.
- There is no separation between the explanation and the error.

---

## 24: Discovered server listeners (`24-discovered-server-listeners.png`)

**What it is:** the **Connect to Rex** form, reached by choosing a Rex server (`rex`, "view listeners") in screenshot 21.

**Controls, in visual order**

1. Title **Connect to Rex**.
2. **Host name**: a text field with the value `rex`. It has focus.
3. **Endpoint**: a text field with the value `https://rex.tail586a6d.ts.net`.
4. Status line: "Reachable from this device: https://rex.tail586a6d.ts.net".
5. A listener block, all grey helper text:
   - "Server-reported listeners (bound addresses are not reachability or permission checks):"
   - "unix:///run/user/1000/rex/server.sock (primary) (unix, bound: /run/user/1000/rex/server.sock) — local processes identified by OS peer credentials (normally only the server's user)"
   - Blank line, then "login-server:///run/user/1000/rex/login-server.sock (unix, bound: /run/user/1000/rex/login-server.sock) — users authenticated by the login server"
6. Buttons: **Cancel** and **Connect**.

**Layout:** about 338 × 210 logical px. The listeners are plain wrapped paragraphs, not a table or list.

**Behaviour implied:** this form shows the reachable endpoint, read-only information about the server's listeners and their trust model, and then connects.

**Polish notes**

- The listener paths are not monospace and break in the middle of a path (`/run/user/1000/rex/` | `server.sock`).
- Everything is the same small grey weight, so the positive "Reachable" status gets no emphasis such as a green check.
- The redundant "(unix, bound: same path)" text adds noise.

---

## Settings → Shortcuts (06, 25, 26, 27, 28, 29)

**What it is:** the **Shortcuts** page of the Settings modal, reached from the settings icon or with Ctrl+, (the Settings shortcut listed on the page). It is a read-only reference list. There are no edit controls; overrides are made by editing a JSON file.

**Settings modal layout**

- The panel is about 493 × 337 logical px (986 × 675 px), centred. Its fill is `#0C1523`, the corners are about 12 px and it has a soft shadow.
- **Left navigation column:** about 96 logical px wide. A 1px divider (`#141D2A`) separates it from the content.
  - The column has a bold heading **Settings**, followed by items: Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts, Updates, About.
  - Items are at a pitch of about 17 logical px, in regular weight and light grey.
  - The selected item, **Shortcuts**, has a rounded `#222A37` pill fill and white semibold text.
- **Content area:**
  - The heading **Shortcuts** is bold and about 16 px. A circular close button (×) sits at the top right with a dark teal-navy fill (`#061F32`) and a hairline ring.
  - The content scrolls under a fixed heading. There is no visible scroll bar.
- **Shortcut rows:**
  - Key chips are on the left. Each key is a separate small chip with fill `#152D42`, text about `#899AAF`, a radius of about 2–3 px and a font of about 9 px. Chips are about 2 px apart.
  - The action label is in a second column starting about 103 logical px into the content area, regular weight, near white.
  - Rows are at a pitch of about 21 logical px.
  - The note "also numpad" appears in smaller grey text under some chip groups.
  - Group headings (for example NAVIGATE) are small, uppercase, letter-spaced, grey and semibold.

### Complete shortcut list (top to bottom)

| Group | Binding | Action |
| --- | --- | --- |
| NAVIGATE | Ctrl PageDown | Next tab |
| NAVIGATE | Ctrl PageUp | Previous tab |
| NAVIGATE | Ctrl Tab | Next tab across hosts |
| NAVIGATE | Ctrl Shift Tab | Previous tab across hosts |
| NAVIGATE | Ctrl K | Switch session |
| CREATE AND CLOSE | Ctrl Shift W | Close focused pane or tab |
| CREATE AND CLOSE | Ctrl Shift T | New tab |
| CREATE AND CLOSE | Ctrl Shift N | New session |
| PANES | Ctrl Alt D | Split pane · down |
| PANES | Ctrl Shift D | Split pane · right |
| PANES | Ctrl Shift Enter | Zoom pane |
| PANES | Ctrl Alt Arrows | Focus adjacent pane |
| PANES | Ctrl Shift Space | Pane actions |
| PANES | Ctrl Shift R | Reconnect terminal |
| TEXT AND VIEW | Ctrl Shift F | Search terminal |
| TEXT AND VIEW | Ctrl Shift C | Copy selection |
| TEXT AND VIEW | Ctrl Shift V | Paste |
| TEXT AND VIEW | Shift PageUp | Scroll up · 20 lines |
| TEXT AND VIEW | Shift PageDown | Scroll down · 20 lines |
| INTERFACE | Ctrl Shift P | Commands |
| INTERFACE | Ctrl , | Settings |
| INTERFACE | Ctrl Shift B | Toggle sidebar |
| INTERFACE | Ctrl + (also numpad) | Enlarge interface |
| INTERFACE | Ctrl Shift = | Enlarge interface |
| INTERFACE | Ctrl = | Enlarge interface |
| INTERFACE | Ctrl - (also numpad) | Shrink interface |
| INTERFACE | Ctrl 0 (also numpad) | Reset interface size |
| POINTER | Mouse Back / Forward | Navigation history |

**FILES** section, at the end of the page (screenshot 29):

- "Server: /Users/shuv/.config/rex/init.lua (not created)". This is the server's Lua bindings file. The server here is macOS, hence the `/Users` path.
- "/home/shuv/.config/rex-linux/shortcuts.json", the local overrides file.
- "Edit the overrides file, then return to Rex to reload. Escape cancels a sequence; unbound keys reach the terminal."

### Per-screenshot scroll positions

- **06** (top of the page, the anchor): NAVIGATE through to the start of PANES (Split pane · down).
- **25:** scrolled to the CREATE AND CLOSE heading, showing CREATE AND CLOSE, all of PANES and the TEXT AND VIEW heading at the bottom.
- **26:** shows the end of PANES, all of TEXT AND VIEW and the INTERFACE heading. The top row ("Split pane · right") is clipped in half under the fixed heading.
- **27:** the end of TEXT AND VIEW, then INTERFACE from Commands to the third Enlarge interface.
- **28:** INTERFACE from Settings onward, then POINTER and FILES. The last helper line is cut off at the bottom edge.
- **29:** scrolled to the end, with the FILES helper text fully visible.

### Behaviour implied

- Bindings are customised through `shortcuts.json`, and the client reloads it when the window regains focus.
- Server Lua bindings live in `init.lua`.
- Escape cancels a multi-key sequence, so chords are supported (inferred from "sequence").
- Unbound keys pass through to the terminal.

### Polish notes

- **Hard clipping:** rows are cut off under the sticky heading and at the bottom edge with no fade or mask. In 26 and 27 half-rendered chips and text are visible; in 28 a cut helper line is visible.
- **Repeated rows:** "Enlarge interface" appears three times as separate rows, where it could be one row with alternate bindings. "Ctrl +" is also visually confusing, because the "+" chip looks like a separator.
- **Uneven row heights:** rows with "also numpad" are taller, and the label column then centres against the two-line block, which shifts the rhythm.
- **Low contrast chips:** the chip text (`#899AAF` on `#152D42`) is small at about 9 px and fairly low contrast.
- **No editing UI:** there is no in-app rebinding, conflict display or reset control.
- **Plain FILES paths:** the paths are not monospace, not clickable and have no "Open" button.
- **Wide gap:** with no leader dots, the gap between the chip column and the label column is wide.
- The panel is short (337 logical px), so the 28 rows need five scroll positions.

---

## Feature inventory

- [hosts.client-label] Client label: a text field ("Label") with Save/Cancel, setting this client's display name (current value `shuvdev`).
- [hosts.edit.name] Edit host: Host name (text field).
- [hosts.edit.endpoint] Edit host: Endpoint (text field). Changing it connects to a different server and the old sessions keep running.
- [hosts.edit.default-shell] Edit host: per-host Default shell (free-text field, e.g. `zsh`), applied to new sessions, tabs and splits.
- [hosts.remove] Remove host confirmation: "Keep host" / "Remove host" (destructive). It forgets the host locally and leaves its sessions running.
- [hosts.add.tailnet-search] Add host popup: searchable tailnet machine list ("Search your tailnet...", Esc closes).
- [hosts.add.groups] Discovery groups: REX SERVERS, TAILNET and OFFLINE.
- [hosts.add.rex-server-row] Rex server rows: https endpoint · "view listeners" · OS.
- [hosts.add.tailnet-row] Tailnet rows: SSH availability, OS and tags (e.g. "firewalla"). "Rex check failed" is shown when the HTTPS probe fails.
- [hosts.add.already-added] An already-added host shows ✓ and "Added as <name>".
- [hosts.add.offline] Offline machines are listed with "Offline · <OS>".
- [hosts.add.manual-entry] "Enter an endpoint..." footer opens the manual form.
- [hosts.add.endpoint-form] Add host by endpoint: Host name plus Endpoint (https://, ssh://user@host[/socket], unix:///path). A bare host name tries 443, then 7297. Connect/Cancel.
- [hosts.add.ssh-form] Add <machine> over SSH: Host name, SSH user and Rex socket (optional, found automatically). Uses SSH keys and configuration and shows the HTTPS probe diagnostic.
- [hosts.add.listeners] Connect to Rex: shows "Reachable from this device", server-reported listeners (unix primary and login-server) and their auth model. Connect/Cancel.
- [settings.nav] Settings sections: Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts, Updates, About.
- [shortcuts.reference] Shortcuts page: a read-only list of 28 bindings in six groups (NAVIGATE, CREATE AND CLOSE, PANES, TEXT AND VIEW, INTERFACE, POINTER).
- [shortcuts.navigate.next-tab] Next tab: Ctrl PageDown
- [shortcuts.navigate.prev-tab] Previous tab: Ctrl PageUp
- [shortcuts.navigate.next-tab-hosts] Next tab across hosts: Ctrl Tab
- [shortcuts.navigate.prev-tab-hosts] Previous tab across hosts: Ctrl Shift Tab
- [shortcuts.navigate.switch-session] Switch session: Ctrl K
- [shortcuts.create.close] Close focused pane or tab: Ctrl Shift W
- [shortcuts.create.new-tab] New tab: Ctrl Shift T
- [shortcuts.create.new-session] New session: Ctrl Shift N
- [shortcuts.panes.split-down] Split pane · down: Ctrl Alt D
- [shortcuts.panes.split-right] Split pane · right: Ctrl Shift D
- [shortcuts.panes.zoom] Zoom pane: Ctrl Shift Enter
- [shortcuts.panes.focus-adjacent] Focus adjacent pane: Ctrl Alt Arrows
- [shortcuts.panes.actions] Pane actions: Ctrl Shift Space
- [shortcuts.panes.reconnect] Reconnect terminal: Ctrl Shift R
- [shortcuts.text.search] Search terminal: Ctrl Shift F
- [shortcuts.text.copy] Copy selection: Ctrl Shift C
- [shortcuts.text.paste] Paste: Ctrl Shift V
- [shortcuts.text.scroll-up] Scroll up · 20 lines: Shift PageUp
- [shortcuts.text.scroll-down] Scroll down · 20 lines: Shift PageDown
- [shortcuts.interface.commands] Commands: Ctrl Shift P
- [shortcuts.interface.settings] Settings: Ctrl ,
- [shortcuts.interface.sidebar] Toggle sidebar: Ctrl Shift B
- [shortcuts.interface.enlarge] Enlarge interface: Ctrl + (also numpad) | Ctrl Shift = | Ctrl =
- [shortcuts.interface.shrink] Shrink interface: Ctrl - (also numpad)
- [shortcuts.interface.reset] Reset interface size: Ctrl 0 (also numpad)
- [shortcuts.pointer.history] Navigation history: Mouse Back / Forward
- [shortcuts.files.server-lua] Shows the server Lua config path (`~/.config/rex/init.lua`, with "(not created)" status).
- [shortcuts.files.overrides] Local overrides file `~/.config/rex-linux/shortcuts.json`, reloaded when you return to Rex.
- [shortcuts.sequences] Multi-key sequences are supported (Escape cancels). Unbound keys reach the terminal.
