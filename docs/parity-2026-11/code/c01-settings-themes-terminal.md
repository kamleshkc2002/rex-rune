# c01: Rex Linux settings, themes and terminal rendering (code inventory)

Source: this repo at branch `t3/0a321b52` (HEAD ad196ff, version 0.6.0). All paths are relative to the repo root. Line numbers refer to the files as read. Compiled from the code, with DESIGN.md/PRODUCT.md/AGENTS.md cross-checked.

## 1. Persistence

- Settings file: `$XDG_CONFIG_HOME/rex-linux/workspace.json`, default `~/.config/rex-linux/workspace.json` (`src/settings.rs:233-242`). `--config <path>` swaps it (`src/connection.rs:58-59`). Written pretty-printed, atomically via `.json.tmp` + rename (`src/settings.rs:283-291`). Every field is `#[serde(default)]`, so old and partial files load (`src/settings.rs:150-151`).
- Shortcut overrides: separate user-owned file `shortcuts.json` beside it (`<stem>.shortcuts.json` for custom config paths), never written by the app, reloaded on focus when mtime/length changes (`src/settings/shortcuts.rs:20-80`). Accepts a bare array or `{"shortcuts":[...]}`. Imports nivox `keybindings` once (`src/settings.rs:272-280`).
- Update check state is not in workspace.json: `$XDG_STATE_HOME/rex-linux/update.json` (README.md:208).
- Load-time validation and clamps: hosts validated, at least one host required (`src/settings.rs:259-265`); font names normalized (`:266`); `font_size` clamp 8-48 (`:268`); `pane_padding` 0-32 (`:269`); `sidebar_width` 200-360 (`:270`); `ui_scale` snapped to 0.1 steps in 0.7-1.5, non-finite becomes 1.0 (`:334-340`).
- No per-window, per-host or per-profile settings exist apart from `Host` fields below.

### 1.1 Persisted keys (workspace.json)

| Key | Type | Default | Options / range | Where |
| --- | --- | --- | --- | --- |
| `hosts` | array of Host | one host: label = `/etc/hostname` (else "Local"), endpoint = local Unix socket, `remote_control` false, `collapsed` false, `shell` none | endpoint `https://`, `http://`, `ws(s)://`, `ssh://`, `unix:///abs/path`; at least 1 host | `settings.rs:10-22, 189-198, 341-354` |
| `hosts[].label` | string | see above | non-empty | `settings.rs:12` |
| `hosts[].endpoint` | string | local socket | see above | `settings.rs:13` |
| `hosts[].remote_control` | bool | false (omitted from JSON when false) | on/off | `settings.rs:14-15` |
| `hosts[].collapsed` | bool | false | sidebar group collapsed | `settings.rs:16-17` |
| `hosts[].shell` | string or null | null (server user's login shell) | free text, e.g. `zsh`, `/bin/zsh` | `settings.rs:18-21` |
| `client_label` | string or null | null (follows hostname; empty string is treated as unset; "Rex Linux" if hostname empty) | free text | `settings.rs:154-155, 224-231, 294-305` |
| `last_host` | string or null | null | host id | `settings.rs:156` |
| `sessions` | map host -> session | empty | last session per host | `settings.rs:157` |
| `sidebar_width` | f32 | 236 | 200-360 | `settings.rs:158, 270` |
| `sidebar_open` | bool | false | restored at launch | `settings.rs:159-160` |
| `vertical_tabs` | bool | false | false = tabs in toolbar, true = tabs in sidebar | `settings.rs:161` |
| `pane_density` | enum | `comfortable` | `compact`, `comfortable` | `settings.rs:33-40` |
| `pane_padding` | f32 pt | 8 | 0-32 | `settings.rs:163, 269` |
| `font_size` | f32 | 15 | 8-48 | `settings.rs:164, 268` |
| `font_family` | string | `JetBrainsMono Nerd Font` | single family, no commas/control chars | `settings.rs:165, 307-313` |
| `font_fallbacks` | array of string | empty | ordered; case-insensitive de-dup, drops entries equal to primary | `settings.rs:166, 315-332` |
| `ui_scale` | f32 | 1.0 | 0.7-1.5 in 0.1 steps | `settings.rs:167, 334-340` |
| `notice_position` | enum | `top_right` | `top_right`, `bottom_right` | `settings.rs:24-31` |
| `notifications.show_in_tabs` | bool | true | | `settings.rs:46, 59` |
| `notifications.system_notifications` | bool | true | desktop notifications for tabs not in view | `settings.rs:47, 60` |
| `notifications.working` | bool | false | notify when a background tab's programs start working | `settings.rs:48-49, 61` |
| `notifications.blocked` | bool | true | needs approval/answer/sign-in | `settings.rs:50, 62` |
| `notifications.error` | bool | true | | `settings.rs:51, 63` |
| `notifications.done` | bool | true | | `settings.rs:52, 64` |
| `notifications.program_messages` | bool | true | pass on OSC 9 / OSC 777 | `settings.rs:53-54, 65` |
| `updates.check` | bool | true | background signed release check | `settings.rs:71-80` |
| `agent_session_naming` | bool | false | passed to `rex skill setup --session-naming` explicitly | `settings.rs:171-172` |
| `chrome_theme` | enum | `midnight` | `midnight`, `rose_pine`, `rose_pine_moon`, `gruvbox_dark`; unknown value reads as Midnight (`#[serde(other)]`) | `settings.rs:82-108` |
| `terminal_theme` | string-encoded enum | `follow_server` | `follow_server`, `builtin_default`, `follow_chrome`, `named:<id>` (id = Ghostty theme file name, or `rex/midnight`, `rex/rose-pine`, `rex/rose-pine-moon`, `rex/gruvbox-dark`); unknown or empty `named:` reads as follow_server | `settings.rs:110-148` |
| `terminal_theme_chosen` | bool | false | user has explicitly picked a terminal theme; chrome palette changes then leave it alone | `settings.rs:175-177` |
| `terminal_theme_declared` | bool | false | this client has declared a theme to some server at least once; never auto-cleared | `settings.rs:178-181` |

### 1.2 Command-line overrides (this launch only, never persisted)

- `--font <family>`, repeated `--font-fallback <family>`, `--font-size <pt>` (clamped 8-48, rejects non-finite) (`src/connection.rs:43-50, 65-83`). Explicit `--font-fallback` replaces saved fallbacks.
- `--endpoint`, `--session`, `--block`, `--new`, `--window-size` (default `1440x900`), `--config`, `--check-update` (exit 0/10/1), Rune flags after `--` such as `--snap` (`src/connection.rs:27-62`).

## 2. Settings UI

Settings is a 760x520 dialog (clamped to 92% x 88% of viewport) with a 190 pt section list and a scrolling body; each row = label + one muted line + control right (`src/app/settings_view.rs:780-832`, `setting()` at `:14-35`). Reached by `Ctrl ,` or Commands. Eight sections in order (`src/app/panel.rs:181-205`): Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts, Updates, About.

### 2.1 Interface (`settings_view.rs:89-192`)

- Theme: chrome palette, label plus "Change..." opening a list of 4 palettes (`:98-114`; list `panel.rs:526-537`). Instant switch, no crossfade.
- Interface zoom: stepper, 70-150% by 10, plus Reset; also Ctrl +/- /0 (`:116-131`; `workspace.rs:806-840`). Effective scale is capped to preserve a 640x480 layout floor (`workspace.rs:798-804`). A notice reports "showing N% to fit", "maximum" or "minimum". The description says "Chrome and terminals together", but DESIGN.md says terminal font size is independent; the code scales the whole root element (`app/mod.rs:60-95`), so zoom does affect terminal points on screen while `font_size` stays separate.
- Pane density: segmented Compact / Comfortable (`:133-152`). Applies instantly.
- Session sidebar: switch (`:154-163`), also `Ctrl Shift B`.
- Tab position: segmented Top / Sidebar; choosing Sidebar opens the sidebar (`:164-175`; `workspace.rs:1155-1161`).
- Notices: segmented Top right / Bottom right (`:177-191`).

### 2.2 Terminal (`settings_view.rs:274-373`)

- Font family: "Change..." opens a form (`Form::Fonts`); description reports the active resolved family, "applies when you reopen the client", or that the saved family is not installed and IBM Plex Mono is in use (`:275-293`).
- Font size: stepper, step 1, 8-48 pt, applies live and resizes owned terminals (`:295-304`; `workspace.rs:2012-2022`).
- Pane padding: stepper, step 1, 0-32 pt, live (`:306-321`).
- Fallback fonts: read-only display ("Automatic" or comma list) (`:323-328`). Edited in the same Fonts form as the family (`Form::Fonts`, `panel.rs:162`). Applies after restart.
- Theme (terminal): label plus "Change..." opening the picker (`:330-346`).
- Default shell: one "Change..." row per saved host, sets `hosts[].shell` (`:347-364`).
- Footer: selectable path of workspace.json (`:365-372`).

### 2.3 Notifications (`settings_view.rs:194-272`)

Switches saved on flip: Activity in tabs; Desktop notifications; then under "Notify when a program": Needs you (`blocked`), Fails (`error`), Finishes (`done`), Starts working (`working`); then Messages from programs (OSC 9/777). The four state switches and Messages are disabled when Desktop notifications is off.

### 2.4 Other sections (summarised; outside this file's core scope)

- Integrations: agent skill/hook setup via local `rex skill`, previews every change, "Let agents name sessions" switch (`src/app/integrations.rs:664-670`).
- Remote Hosts: Client label edit; per host: Edit, Remove (disabled for the last host), Retry when offline, Remote control switch, "Add host..." (`settings_view.rs:375-493`).
- Shortcuts: effective bindings of the active host grouped Navigate, Create and close, Panes, Text and view, Interface, Server; Pointer row; Problems; Files paths; read-only (`:592-728`). Section 6 lists defaults.
- Updates: installed version and channel, automatic-check switch, release status + check button, upgrade notice with Copy command (`src/app/updates.rs:186-250`).
- About: version, commit, build date, server API revision, links, Copy version info (`src/app/about.rs:74-125`).

## 3. Themes

### 3.1 Interface (chrome) themes

Four dark palettes, token structs in `src/app/theme.rs:50-161`, selected by `palette_of` (`:164-171`). Tokens: ground, card, popover, ink, muted, accent, accent2, live, warning, danger, on_accent, terminal_selection.

| Palette | ground=card | popover | ink | muted | accent | accent2 | live | warning | danger | selection |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Midnight (default) `theme.rs:68-81` | 001528 | 0d1524 | e8eef6 | 93a3b8 | 7cc4ff | a78bfa | 5eead4 | f5d48a | ff8a8a | 2f4a6e |
| Rose Pine `:84-97` | 191724 | 1f1d2e | e0def4 | 908caa | c4a7e7 | 9ccfd8 | 31748f | f6c177 | eb6f92 | 403d52 |
| Rose Pine Moon `:100-113` | 232136 | 2a273f | e0def4 | 908caa | c4a7e7 | 9ccfd8 | 3e8fb0 | f6c177 | eb6f92 | 44415a |
| Gruvbox Dark `:116-129` | 282828 | 3c3836 | ebdbb2 | a89984 | 83a598 | d3869b | b8bb26 | fabd2f | fb4934 | 504945 |

- No light chrome palette, no system light/dark following, no custom palettes, no accent colour picker (DESIGN.md: "There is no light chrome palette").
- The palette is thread-local, set at the root of every view build (`theme.rs:173-185`, `app/mod.rs:59`).
- The kit is pinned to palette tokens: hover 7% white, pressed 11%, borders 6-20% white, focus ring = accent, radii sm 8 / md 10 (`theme.rs:213-252`).
- Surfaces: chip (5.5% white fill, gradient hairline, radius 10) (`theme.rs:276-281`); glass popover (95% popover fill, gradient rim, 22 px backdrop blur, two shadows) (`:284-296`).
- Motion springs: SNAPPY 0.28 s, POP 0.32 s bounce 0.12, STILL 0.001 s for terminal content (`:204-216`); popover entrance scale 0.96 (`:225-227`).
- Chrome fonts: Inter (bundled via `rune_theme_opencode`), weights 450/560/620; group labels 10.5 pt caps tracking 0.6 (`theme.rs:254-267`).
- Layout constants: toolbar 52, gutter 8, margin 10, card radius 12, card header 30 (`theme.rs:196-200`).
- Icons: authored 20x20 stroke icons, 1.5 stroke (`theme.rs:298-310` onward).

### 3.2 Terminal themes (declared to the server)

- Choices (`src/settings.rs:116-126`; labels `workspace.rs:1094-1103`; picker `panel.rs:463-523`):
  - Follow server (default): declares nothing until the client has ever declared a theme, after which it declares an empty theme (Rex's own colours) once per terminal per connection (`workspace.rs:1063-1091`).
  - Rex default colours (`builtin_default`): declares an empty SetTheme.
  - Follow chrome: declares the built-in theme paired with the current chrome palette; auto-selected the first time the chrome is changed by a user who never chose a terminal theme (`workspace.rs:1046-1065`).
  - Built-in (4): Midnight, Rose Pine, Rose Pine Moon, Gruvbox Dark (ids `rex/midnight`, `rex/rose-pine`, `rex/rose-pine-moon`, `rex/gruvbox-dark`) (`src/terminal_theme.rs:58-83`).
  - Ghostty themes: any file in `$XDG_CONFIG_HOME/ghostty/themes`, `$GHOSTTY_RESOURCES_DIR/themes`, `$XDG_DATA_DIRS/ghostty/themes` (default `/usr/local/share:/usr/share`), listed Dark / Light by background luminance, user ones tagged "Custom", sorted case-insensitively, first directory wins on duplicates (`terminal_theme.rs:244-312`). Picker has a search box (query filter in `panel.rs:463-480`).
- Ghostty parser reads `background`, `foreground`, `cursor-color`, `cursor-text`, `selection-background`, `selection-foreground`, `palette = N=#hex` (0-255); accepts `#rrggbb`, `rrggbb`, `#rgb`, optional quotes; `#` comments; later lines win; everything else ignored (X11 names, `cell-foreground`, etc.) (`terminal_theme.rs:9-13, 164-209, 329-345`). A file with none of those keys is rejected as "not a Ghostty theme".
- Built-in contents: Midnight = background 001528 + dark scheme only (Rex default text); Rose Pine / Moon = full set (bg, fg, cursor, cursor-text, selection bg/fg, 16-colour palette); Gruvbox Dark = bg, fg, cursor, 16-colour palette, dark (selection stays chrome's) (`terminal_theme.rs:99-162`).
- Protocol: only `background`, `foreground`, `cursor`, `palette` (as `"N=#rrggbb"`) and `scheme` (`dark`/`light`) go to `terminal.set_theme` (`terminal_theme.rs:213-235`). Cursor-text and selection colours have no protocol field and are drawn locally (`terminal_theme.rs:6-8`, `app/term.rs:975-1004`).
- Incoming theme metadata is applied to the Ghostty terminal: default fg/bg/cursor and palette; each theme replaces the previous palette (`src/terminal.rs:149-170`).
- Declared to every host on change (`workspace.rs:1075-1080`), and applied by the server only when this client owns the block or nobody does (PRODUCT.md).
- Light terminal themes can be declared and shown; light detection `is_light` = perceived brightness > 128 (`terminal_theme.rs:238-242`).

## 4. Terminal rendering

### 4.1 Engine and grid

- libghostty-vt decodes snapshots and live output; the client builds a `Frame` of cells per render (`src/terminal.rs:236-256, 374-475`). Each cell carries text (grapheme cluster), spacer_tail (wide char), fg, bg, underline colour, full Ghostty style, OSC 8 link (`:224-234`).
- Fixed column grid: rows are text segments placed at `col x cell_width`; a character joins a segment only when it advances exactly one cell; wider/other characters become individual glyph runs; rows are memoized by content hash (`src/app/term.rs:571-725, 1025-1110`; AGENTS.md).
- Cell metrics: width = advance of "M" in the mono slot; height = `ceil(font_size * 1.4)`; line height is fixed at 1.4 and not configurable (`term.rs:154-166`).
- Ligatures/contextual alternates/kerning all off: `'liga' 0, 'calt' 0, 'clig' 0, 'kern' 0` (`term.rs:152`).
- Synchronized output (mode 2026) honoured, with a 1 s hold limit as in Ghostty (`terminal.rs:69-116`).
- Frame fields: cols, rows, cursor, cursor_color, cursor_shape, mouse_tracking, images, selection rows/text, soft-wrap flags, search results (`terminal.rs:236-256`).
- Inverse, invisible handled in the frame; faint is pre-mixed 65% toward the cell background; bold/italic via font faces (`terminal.rs:421-427`, `term.rs:547-568`).
- Display crops to the visible rows locally while keeping the newest rows, the cursor, or the selected search match; the server size is never changed by this (`terminal.rs:310-327`).

### 4.2 Fonts

- Terminal family from `font_family` (default JetBrainsMono Nerd Font), resolved through fontconfig `fc-match` in 4 styles (regular, bold, italic, bold italic), rejecting unrelated substitutes unless the name is a generic (`monospace`/`sans-serif`/`serif`) (`src/app/fonts.rs:50-82`). Bundled families recognised by name are the kit's chrome Inter and IBM Plex Mono (`fonts.rs:27-35`).
- Fallbacks: user's ordered `font_fallbacks`, then bundled IBM Plex Mono, then system script/symbol fallbacks; missing primary replaced by IBM Plex Mono with a stderr message (`fonts.rs:84-136`).
- Fonts change only on restart (family and fallbacks); size is live.
- No per-style font families, no bold-as-bright option, no font weight/variation settings, no line-height/cell-adjust, no font-thicken, no minimum contrast.

### 4.3 Cursor

- Shape from the program's DECSCUSR (block, bar, underline) or Ghostty's hollow block (`terminal.rs:214-221, 405-412`).
- Focused pane: program's shape (block default); unfocused/non-keyboard pane: hollow outline block always (`term.rs:1115-1130`). Bar is 2 pt wide, underline 2 pt tall, hollow is a 1 pt border (`term.rs:1119-1129`).
- Colour: the terminal's cursor colour (OSC 12 or declared theme `cursor`), else the foreground (`terminal.rs:404`). Text under a block cursor is redrawn in theme `cursor-text` if declared, else the body (background) colour (`term.rs:1131-1144`).
- Blink is ignored: cursors draw steadily ("terminal content never animates") (`terminal.rs:403`, DESIGN.md).
- IME: preedit text drawn at the cursor with underline; the cursor registers an IME rect (`term.rs:1131-1160`, `src/app/input.rs:290-312`).
- No cursor style/blink/colour settings in the UI.

### 4.4 Selection and clipboard

- Mouse: single click/drag = cell selection; double click = word; triple click = line (`term.rs:415-423`, `terminal/selection.rs:101-110`). Ghostty's native gesture engine (tracked anchors, content-pinned across scrolling and output) owns the selection (`selection.rs:1-132`); edge-drag autoscroll at 50 ms ticks (`:168-195`).
- Word boundaries: space, tab, newline and ``!"#$&'()*,;<>?[]^`{|}``; shell path characters stay inside words (`selection.rs:56-58`). A local fallback range uses alphanumerics plus `_-.~/\:@%+=` (`terminal.rs:289`).
- A bare focus click never copies (`term.rs:425-428`).
- Shift+left drag selects locally even when a program tracks the mouse; shift+right click goes to the program (`panes.rs:329-335, 455-467`; README.md:22).
- Selection overlay colour: declared theme `selection-background` else palette `terminal_selection` (`term.rs:998-1002`); optional `selection-foreground` recolours selected text including decorations (`term.rs:798-811`). Drawn as an overlay; cell colours themselves are never altered.
- Copy: `Ctrl Shift C` (`pane.copy`) copies the selection text via Ghostty formatter (unwrap + trim), 8 MiB cap (`selection.rs:19, 196+`; `actions.rs:361-365`). Wide characters kept whole; spacer tails and Kitty placeholders skipped (`terminal.rs:329-355`).
- Paste: `Ctrl Shift V` (`pane.paste`); text is sent as `Command::Paste` (bracketed paste per README); file paths on the clipboard and clipboard images are uploaded (API 11+) (`src/app/upload.rs:50-74`). IME commit also sent as paste (`input.rs:303-306`).
- OSC 52 and server-side "selection copied": terminal-initiated copies are written to the clipboard only for the terminal the user is interacting with; clipboard reads are deliberately unsupported (`src/connection/clipboard.rs:1-30`); 8 MiB limit.
- Absent: copy-on-select, X11/Wayland primary selection (middle-click paste), rectangular/block selection, select-all, "trim trailing whitespace" option. The middle button only reaches a program that tracks the mouse (`panes.rs:306-309`).

### 4.5 Links

- OSC 8 hyperlinks and plain-text URLs (`https://`, `http://`, `ftp://`, `mailto:`), the latter continuing across soft-wrapped rows, trimming trailing punctuation and unmatched brackets; quotes/angle brackets/box-drawing stop a URL (`src/links.rs:5-9, 20-88, 97-160`).
- Only `https`, `http`, `ftp`, `mailto` open; `file:` and other schemes are refused (`links.rs:7-9, 89-95`). No filesystem-path or `file://` linkification, no regex-configured links.
- Interaction: only Ctrl+click opens (press and release on the same link); holding Ctrl underlines the link (1 px, own text colour), shows the hand pointer, and a tooltip "Ctrl+click to open <target URL>" (`panes.rs:339-373, 408-432`; `term.rs:1167-1212`). Works even over mouse-tracking programs. Opened with `xdg-open` off the UI thread; failure becomes a notice (`workspace.rs:768-795`).
- No setting to change the modifier or disable links.

### 4.6 Search

- `Ctrl Shift F` (`client.search`) opens a 280x34 field at the pane's top-right corner, "Search terminal" placeholder; trailing text is `esc`, `N/M`, or `No matches` (`panes.rs:552-585`). Ghostty search over screen and scrollback, incremental with new output, survives reconnect (`src/terminal/search.rs:1-165`). Enter = next older, Shift+Enter = next newer, Escape closes (README.md:23). Highlights: accent at 25% (all) and 55% (selected), drawn as overlays (`term.rs:769-776`).
- No case-sensitivity, whole-word or regex toggles in the UI.

### 4.7 Scrolling and mouse

- Wheel: line-accumulating (`dy / (cell_height * ui scale)` for precise devices); when the program tracks the mouse and Shift is not held the wheel goes to the program (max 8 lines per event), otherwise scrolls local scrollback (`panes.rs:473-492`; `term.rs:406-413`). `Shift PageUp/PageDown` scroll 20 lines (`keymap.rs:410-412`).
- Mouse reporting: press/release/motion for programs tracking DEC 9/1000/1002/1003, with modifiers (`term.rs:366-405`, `panes.rs:339-450`).
- No scroll-speed multiplier, no scrollback size setting (server/Ghostty owns it), no scroll-to-bottom-on-input setting.

### 4.8 Graphics, sprites and decorations

- Kitty graphics: RGB, RGBA, PNG; placement, crop and layers (below-background, below-text, above-text) resolved by Ghostty; 64 MiB per terminal; file, temp-file and shared-memory media disabled; PNG dimensions bounded before decode (`src/terminal/graphics.rs:12-67`; `term.rs:929-965, 1075-1100`). Animated Kitty graphics not implemented (README.md:495). No Sixel/iTerm2 inline image support.
- Sprites: box drawing U+2500-257F (light/heavy/double lines, dashes, rounded arcs, diagonals) and blocks U+2580-259F drawn as shapes on whole device pixels instead of font glyphs (`src/app/sprites.rs:1-120`, `term.rs:803-818`). Powerline/Nerd glyphs come from the font.
- Underlines: single, double, curly, dotted, dashed, with SGR 58 colour; overline; strikethrough via text span; all snapped to device pixels (`term.rs:855-927`).
- Default background substitution: the server's default background is drawn as the declared theme's background (else palette card) when both are dark or both light; explicit backgrounds and inverse video untouched (`term.rs:975-1022`; DESIGN.md).

## 5. Density and padding

- Comfortable (default): server-defined splits as rounded cards (radius 12) with 30 pt header (pane name, activity word, exception strip), 8 pt gutters, 10 pt outer margin, focused card gets accent outline (`panes.rs:26-70`, DESIGN.md "Pane cards").
- Compact: server placements with no headers, outer borders, shadows, rounded corners, gutters or outer margins; one 1 pt separator per split boundary; focus shown by the terminal cursor only; per-pane activity words/progress hidden (tab activity stays); exception strips float at each pane's top-right (`panes.rs:26-70, 540-548, 729-761`; DESIGN.md).
- Single pane (not zoomed) draws edge to edge, no card/header/border/margin in both modes (DESIGN.md).
- Pane padding: uniform on all four sides in every pane, 0-32 pt (default 8); 0 keeps card borders but removes content padding; a non-card terminal background fills inside the card border (`panes.rs:500-520`).
- Divider drag previews immediately and sends a ratio clamped to 0.1-0.9 (`workspace.rs:2025-2045`, DESIGN.md).
- No separate horizontal/vertical padding, no padding-colour option.

## 6. Commands, menus and shortcuts relevant to settings/terminal

Defaults ("linux" layer, `src/keymap.rs:370-413`); each is overridable or removable via the server keymap or `shortcuts.json`.

| Keys | Action |
| --- | --- |
| `Ctrl K` | Switch session (`session.switch`) |
| `Ctrl Shift P` | Commands (`client.palette.toggle`) |
| `Ctrl ,` | Settings |
| `Ctrl Shift B` | Toggle sidebar |
| `Ctrl Shift N` / `T` / `W` | New session / New tab / Close focused pane or tab |
| `Ctrl Tab` / `Ctrl Shift Tab` | Next / previous tab across hosts |
| `Ctrl PageUp` / `PageDown` | Previous / next tab |
| `Ctrl Shift Enter` | Zoom pane |
| `Ctrl Shift Space` | Pane actions menu |
| `Ctrl Shift C` / `V` | Copy selection / Paste |
| `Ctrl Shift F` | Search terminal |
| `Ctrl Shift R` | Reconnect terminal |
| `Ctrl =` `Ctrl Shift =` `Ctrl +` `Ctrl NumpadAdd` | Enlarge interface |
| `Ctrl -` `Ctrl NumpadSubtract` | Shrink interface |
| `Ctrl 0` `Ctrl Numpad0` | Reset interface size |
| `Ctrl Shift D` / `Ctrl Alt D` | Split right / down |
| `Ctrl Alt Arrows` | Focus adjacent pane |
| `Shift PageUp` / `PageDown` | Scroll 20 lines up / down |
| Mouse Back / Forward | Navigation history |

- Action registry of 32 named actions (`src/app/actions.rs:65-98`), including `pane.send_key` (bypasses Lua bindings), `client.tab.goto/close/rename`, `session.rename`, `pane.close`.
- There are no actions for font size increase/decrease/reset (zoom actions scale the whole UI), theme switching, density toggling, clear screen, or toggling padding.
- Pane menu entries: Split right, Split down, Zoom pane, Rename pane..., Reconnect terminal, Close pane... (`panel.rs:717-726`). No Copy/Paste items in the pane menu.
- Right-click on a pane opens the pane menu always (Shift+right-click goes to mouse-tracking programs) (`panes.rs:448-467`).
- Keys: ordinary keys go to the focused pane via `client.key` (physical-layout press/release with dead-key composition, `src/app/keys.rs`, `input.rs:231-290`); releases only forwarded when their press was.
- Remote availability of actions: view-only subset (`Spec::remote`, `actions.rs:106-142`); pane copy/paste/zoom-in etc. are never remote.

## 7. Absent or partial

Absent (the code has no equivalent):
- Light interface theme, auto light/dark following the OS, custom or user-defined chrome palettes, accent colour option, window opacity/blur or transparency, window decorations toggle.
- Terminal theme import is read-only from Ghostty files; no theme editor, no per-pane/per-session/per-host theme, no separate light and dark pairing (single theme choice only).
- Minimum contrast, bold-as-bright, faint strength, cursor-text contrast options, cursor blink, cursor style/colour overrides, cursor-hold-after-typing.
- Line height / cell adjust, per-style font families, font weight/thicken, font features toggle (ligatures permanently off), Unicode width/emoji presentation options, text variation settings.
- Scrollback size, scroll multiplier, scroll-on-output/keystroke settings; bell sound or visual bell (bell only marks an unseen tab "Bell").
- Copy-on-select, primary-selection paste (middle click), rectangular selection, select-all, selection clear-on-copy option, trailing-whitespace option.
- Link customisation: no file-path or `file://` links, no regex links, no modifier choice, no disable switch, no hover-without-Ctrl underline.
- Search options (case, whole word, regex), search across panes/tabs.
- Option-as-Alt equivalent (not applicable on Linux, but no Meta/Alt handling setting), no global quick-terminal/hotkey window, no tab-bar auto-hide, no window padding colour, no background image, no ligatures.
- Per-host settings beyond label, endpoint, remote-control and default shell (no per-host theme/font).
- Kitty graphics animation; Sixel; iTerm2 inline images; AT-SPI accessibility (README.md:495, PRODUCT.md "Out of scope").
- No settings search, no import/export of preferences, no reset-to-defaults button.

Partial:
- Font family and fallbacks edit via a form (`Form::Fonts`) but apply only after relaunch; the Settings row shows fallbacks read-only.
- Terminal theme: cursor-text, selection-background and selection-foreground are honoured locally from the theme, but the server only receives background/foreground/cursor/palette/scheme; Midnight and Gruvbox built-ins carry no selection colours (chrome's are used).
- Follow server: "declares nothing" only until the client has once declared a theme; afterward it resets to Rex's colours per terminal per connection.
- Interface zoom scales chrome and terminal together on screen (UI root scale), while `font_size` is the cell size; the size readout and PTY size follow font_size x zoom indirectly. Cap by viewport so zoom may be silently reduced (notice shown).
- Compact density keeps terminal padding but hides per-pane headers, so pane renaming and activity words are reachable only through menus.
- Kitty graphics: static images only.
- Settings keys `terminal_theme_chosen` / `terminal_theme_declared` are bookkeeping flags written to the user-visible file, not user settings.
- Shortcut editing is file-based; the Shortcuts section is read-only (no in-app rebind UI).

## 8. Constraints (product rules that limit parity)

From AGENTS.md, PRODUCT.md, DESIGN.md:

1. Terminal content, colours and split geometry are server-owned. The client never recolours terminal output locally or invents layout. The only exceptions are the default-background stand-in (card or declared-theme background, only when both are dark or both light) and local overlays for selection, search matches, link underline and cursor-text (DESIGN.md Principles; `term.rs:975-1022`).
2. A terminal theme can only be declared through `terminal.set_theme`; the server applies it when this client owns the block (or nobody does) and otherwise defers until ownership, the same rule as resize. Declarations persist server-side per client per block, which is why Follow server behaves differently after the first declaration (PRODUCT.md, `settings.rs:178-181`).
3. Fields the protocol has no slot for (cursor-text, selection colours) can only be local overlays; other clients on the same terminal do not see them.
4. Terminal content never animates: cursor blink requests draw steadily; no smooth scrolling, no cursor animation, no crossfades on palette change; chrome motion only via the `theme.rs` springs.
5. Attachment is passive: ordinary attach must never start/stop/reconfigure a server or create/destroy sessions; creation is explicit (AGENTS.md). Failed input is never replayed; a transport failure pauses panes.
6. Size ownership is automatic (active window owns the size; typing claims it); selecting, copying or navigating must not claim ownership. The terminal is cropped locally, never resized, when it is taller than the pane (`terminal.rs:310-327`, DESIGN.md "Don't").
7. Workspace shortcuts consume their keys, but every binding is removable or replaceable; ordinary keys always go through `client.key`; only explicit `pane.send_key` bypasses Lua bindings (AGENTS.md). Parity of key behaviour is therefore server-keymap-layered rather than a local table.
8. Remote availability is decided exhaustively in `Spec::remote()`, default deny; the allowed set test is pinned. Clipboard-reading, typing, creation and destruction actions can never be remote (AGENTS.md; `actions.rs:106-142`).
9. Security: only web/mail schemes open from terminal links; Kitty file/shm media disabled; clipboard reads from remote programs unsupported; copy size capped at 8 MiB; images capped at 64 MiB (`links.rs:7-9`, `graphics.rs:12-67`, `clipboard.rs:1-30`).
10. Chrome design: dark palettes only, colour-only palette changes (fills, borders, motion, layout shared); trouble states always carry a word; no status dots, "Connected" labels, terminal size readouts, keyboard-hint footers; no illustrative sessions shown as runtime state (DESIGN.md "Do and don't"). Chrome tokens must live in `theme.rs`, not hard-coded in views.
11. Single toolbar row, no minimize/maximize buttons (tiling WM friendliness); settings is a 760x520 dialog with rows of label + one muted line + control.
12. Native Rune look: on Rune bumps take upstream defaults and rendering, keep accessibility on, do not restore old pixels (project memory note). Rune/kit is the widget layer, so macOS-native controls and rendering are not a target.
13. Platform: Linux Wayland/X11 only; Unix socket, HTTPS (Tailscale) and SSH-forwarded endpoints; API revisions 10-13; fonts resolved via fontconfig; links via `xdg-open`; notifications via the desktop's service.
14. README claims must stay honest: do not document features that are not implemented or checked (AGENTS.md).
15. Vendored Rune/rexrs/ghostty-vt are patched minimally; any parity feature needing engine changes (e.g. new Ghostty options) means a documented vendor patch, not ad hoc edits.
