# Rex Linux

An unofficial, independent native Linux client for [Rex](https://www.superlogical.com/rex), the terminal multiplexer from Superlogical. It is not affiliated with, endorsed by or supported by Superlogical or the Rex authors.

![Rex Linux with the sessions sidebar open and a three-pane split showing a file tree, source code and test results](docs/screenshot.png)

Rex Linux attaches to an existing Rex server. Saved hosts contain sessions, each session's Rex windows become tabs, and the server defines the terminal splits. Sessions and their programs stay on the server when the app closes, so any other Rex client can attach to the same sessions.

It is written in Rust on [Rune](https://github.com/anomalyco/rune) (wgpu/winit, `rune-kit` components) with libghostty-vt for terminal emulation, and talks to the server through the [`rexrs`](https://github.com/a10y/rexrs) control SDK.

> **Status:** early. The protocol layer, workspace, terminal rendering and input are implemented and tested against real servers, but some areas (see [Current scope](#current-scope)) still need hands-on validation.

## Features

- **Settings → About** shows the app version, build commit, build date and build channel (`official`, `nix` or `source`), and the active server’s negotiated API revision when connected. Open the repository, this version’s release notes or MIT license, or **Copy version info** for a bug report (OS and architecture included; no host addresses or credentials). Source archives without Git show `unknown` for build metadata. Server software version is not inferred.
- **In-app update notice and Settings → Updates:** official and Nix builds check the signed GitHub release manifest (`latest.json` and `latest.json.minisig`) against embedded CI and backup Minisign public keys about 30 seconds after launch and at most once every 24 hours while running. When a newer stable release is verified, a non-blocking toast offers **What's new** (opening the GitHub release page in your browser), **Updates…** and **Dismiss for this version**. Guidance matches the detected install channel (`REX_LINUX_CHANNEL`, `/usr/share/rex-linux/channel` and `~/.local/share/rex-linux/install.json`): `sudo pacman -Syu rex-linux-bin` on Arch (`omarchy update` on Omarchy), the verified `install.sh` one-command upgrade for tarball/script installs, `.deb` upgrade instructions for Debian/Ubuntu, and `nix flake update rex-rune` / `nix profile upgrade rex-linux` for Nix. Source builds never check automatically; turn automatic checks off in Settings → Updates (`updates.check` in `workspace.json`) or with `REX_LINUX_NO_UPDATE_CHECK=1`, or check on demand with **Check now** or `rex-linux --check-update`. Background check failures are silent, and the client never self-updates or downloads binaries.

- Multiple saved hosts, each with its sessions; session windows are tabs and splits follow the server's layout.
- Unix-socket, HTTPS/WSS and SSH endpoints. Add host lists the machines on your tailnet: a server that answers shows its reachable endpoint and, on API 12, a review of its listener addresses and access rules before connecting. Other machines take an SSH user name and are reached by forwarding their Rex socket over SSH (Tailscale SSH needs no keys).
- A single toolbar with session identity, tabs, commands and the window's own controls. A tab with one pane draws it edge to edge; splits are rounded pane cards. Tabs close from their right-click menu, a middle-click or `Ctrl Shift W`, always behind a confirmation. Host-grouped session navigation opens on demand, with an optional sidebar (Ctrl Shift B) that opens at launch as it was left. Tabs can sit in the toolbar or, while the sidebar is open, under each session in the sidebar.
- Searchable session switcher (`Ctrl K`) and command palette (`Ctrl Shift P`).
- Snapshot restore, live output, physical-layout key press and release forwarding with dead-key composition, bracketed paste, text selection and copy, local scrollback, and mouse/wheel reporting for terminal programs. Right-click always opens the pane menu; Shift+right-click sends the click to a program that tracks the mouse, and Shift with the left button selects text locally.
- Terminal search (`Ctrl Shift F`) over the focused pane's screen and scrollback, with Ghostty's search: matches are highlighted in the chrome's accent and the field counts them; Enter selects the next older match and Shift+Enter the next newer one, scrolling it into view; Escape closes the search. In a pane showing fewer rows than the terminal has, the view follows the selected match. New output is searched as it arrives, and the search survives a reconnect. Known limit: once a terminal restored from a snapshot has been resized, Ghostty can start from an older match and step toward newer ones; every match is still reached.
- Cursor shapes programs choose with DECSCUSR (block, bar or underline) in the focused pane; other panes show a hollow block. Blinking cursors draw steadily: terminal content does not animate.
- Underline styles (single, double, curly, dotted and dashed) and colours that programs set with SGR 4:x and 58, and overlines (SGR 53).
- Box-drawing and block element characters (U+2500–U+259F) are drawn as shapes on whole device pixels, as Ghostty draws them, rather than from the font, so borders, tables and block graphics meet their neighbours without seams or overlaps at any font or display scale.
- Inline Kitty graphics (RGB, RGBA and PNG), including image-bearing snapshots on reattach. Ghostty resolves placement, crop and paint order; the client caches decoded pixels and textures. Image storage is capped at 64 MiB per terminal. File, temporary-file and shared-memory image media are disabled: remote terminal output cannot read local files.
- Synchronized output (mode 2026): while a program holds an update open (for up to a second, as in Ghostty) the pane keeps its last frame, so full-screen programs redrawing after a resize don't flash blank.
- Links: Ctrl+click opens an `http`, `https`, `ftp` or `mailto` link with `xdg-open`, whether it is an OSC 8 hyperlink or a URL in the text (including one that soft-wraps across rows). Holding Ctrl over a link underlines it, shows a hand pointer and, after a moment, a tooltip naming its real target, so OSC 8 text cannot disguise where it goes. Ctrl+click works over programs that track the mouse too; clicks elsewhere still go to the program. Other schemes, `file://` among them, are never opened, since output may come from a remote host.
- File downloads (API revision 13): **Download file…** in the pane menu and Commands saves a file from the terminal's machine into a folder on this one, over the Rex connection (also through SSH hosts). Nothing is overwritten, opened or run by itself. See [Downloading files](#downloading-files).
- Reconnect handling: a transport failure pauses input on affected panes (input is never replayed) while parsed output survives reconnects and session changes. TCP connections are probed when idle, so a server that dropped this client while the network was gone (a laptop asleep, for example) is noticed within about 30 seconds and the host and its terminals reconnect.
- Program activity on tabs. Programs that report their status with OSC 7501 (coding agents, for example) mark their tab with a word when they need you ("Needs approval", "Question", "Needs sign-in", "Needs input") or fail ("Error"), and with "Done" when they finish in a tab you weren't looking at. A terminal bell that rings out of sight marks its tab "Bell" until you see it. Progress that programs report with OSC 9;4 draws as a thin bar under the tab's label (and under a split pane's name) until they clear it or leave the foreground, with "Paused" (and its percentage, if it gave one) or "Error" as a word when the report says so; a working program's OSC 7501 percentage draws the same way when it sends no OSC 9;4. In a split, each pane's header carries its own word, so you can see which pane needs you. Each tab shows its focused pane's program icon (Claude Code, Codex, OpenCode, Pi and OMP, Vim, Git, Cargo, SSH; otherwise `>_`), and a tab or pane you haven't renamed takes the title its program sets (OSC 0 or 2), as a terminal window would, without changing its name on the server. Needs a server with API revision 8.
- Desktop notifications through the freedesktop notification service when a tab you aren't looking at needs you, fails or finishes, and for notifications programs send themselves (OSC 9 and OSC 777). Clicking one raises the window and shows that tab. Each kind can be turned off in Settings → Notifications.
- Agent integrations (Settings → Integrations): installs, updates or removes Rex's skill and session hooks for Claude Code, Codex, OpenCode, Pi and OMP on this machine with the local `rex skill` command. OMP reads the shared `~/.agents/skills/rex` skill used by Codex, OpenCode and Pi; it does not have a Rex session hook. `rex skill` has no OMP agent yet, so until it does, OMP's row changes Pi's skill: installing or removing it for one does the same for the other. Every change is previewed with a dry run first, including every agent affected by that shared skill. "Let agents name sessions" is passed to every setup explicitly.
- Interface zoom (70–150%) and a configurable terminal font and size. Settings → Terminal → Pane padding sets the space around every pane's content (0–32 pt, default 8 pt), including zero, with no restart.
- Chrome palettes (Settings → Interface → Theme): **Midnight** (the default), **Rosé Pine**, **Rosé Pine Moon** and **Gruvbox Dark**, all dark, applied at once without restarting. Each pairs with a built-in terminal theme of the same name whose background is the chrome's ground. Until you pick a terminal theme yourself, choosing a palette moves the terminal theme from Follow server to **Follow chrome**, which declares the palette's paired theme to the server as below; a terminal theme you picked is left alone.
- Terminal themes (Settings → Terminal → Theme). **Follow server**, the default, declares nothing and shows whatever colours the server has, as long as this client has never declared a theme. **Rex default colours**, **Follow chrome** (the chrome palette's paired theme), the built-in **Midnight**, **Rosé Pine**, **Rosé Pine Moon** and **Gruvbox Dark**, or any Ghostty theme file are declared to the server with Rex's `set_theme`, which the server applies to panes this client owns or nobody owns; while another client owns a pane, the server keeps that client's colours and applies ours when this client takes the pane. New tabs and splits are created with the theme. Ghostty themes are read from `$XDG_CONFIG_HOME/ghostty/themes` (default `~/.config/ghostty/themes`), then `$GHOSTTY_RESOURCES_DIR/themes`, then `ghostty/themes` under each `$XDG_DATA_DIRS` entry; the first file with a name wins. Selection and cursor-text colours from a theme are drawn by this client. Rex keeps this client's last theme for each pane across launches and applies it whenever this client owns the pane, so once this client has declared any theme (remembered as `terminal_theme_declared`, never cleared automatically), Follow server sends one empty `set_theme`, Rex's own colours, to each pane this client owns or nobody owns: once per pane per connection, when its stream starts, when this client takes it, and again after a reconnect. A server without `set_theme` is reported once and the choice is kept for other hosts.
- Borderless window with its own close button, edge resizing and drag region. There are no minimize or maximize buttons; double-clicking the drag region maximizes, and the window manager handles the rest.
- Keys are sent by physical position (Ghostty key names), so non-US layouts work.

**Attachment-only by default.** Attaching and reconnecting never start, stop, replace or reconfigure a Rex server and do not need a `rex` executable. A missing default server on an SSH host offers a separately confirmed, one-time **Start personal server…** action (see [SSH setup](#ssh-setup)); it never enables automatic starts on future connections. Settings → Integrations runs local `rex skill` with `--autostart=false`. Both explicit CLI features search `PATH`, then `$XDG_BIN_HOME` or `~/.local/bin` for desktop launches. An unavailable server is shown as disconnected and retried. Closing the client leaves server programs running.

## Install a binary release

### One-command user install or upgrade (0.5.2+)

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh | sh
# Explicit upgrade, check, pin (including deliberate downgrade), or uninstall:
# ... | sh -s -- --upgrade
# ... | sh -s -- --check
# ... | sh -s -- --version 0.5.1
# ... | sh -s -- --uninstall
```

The signed, immutable release asset detects x86_64/aarch64 Linux and requires
**glibc 2.39+**, `curl`, `tar`, coreutils, `bash`, `desktop-file-utils`, and either
**minisign or OpenSSL 3+**. It verifies the signed checksums with embedded CI and
backup keys, verifies the tarball, then installs to `~/.local/bin` with a launcher
and icons. `XDG_BIN_HOME`, `XDG_DATA_HOME` and `XDG_CACHE_HOME` are respected.
No root, server changes or automatic downgrade. Uninstall keeps preferences.
NixOS users should use [Nix](docs/nix.md); musl is unsupported.

**Pipe-to-shell is trust on first use:** HTTPS and GitHub deliver the script you
execute. Its detached `install.sh.minisig` can be verified with a separately
trusted key before running a saved copy; retain that reviewed copy for upgrades.
The script's body runs only after the complete function has downloaded. Package
verification is mandatory; there is no unsigned mode. A saved script with old
keys fails closed after rotation. **Only** explicit `--trust-repo-keys` expands
trust to keys at the requested release's protected tag (never `master`), printing
the key ID and fingerprint. Prefer fetching and reviewing the current script.

`/usr/bin/rex-linux` or a mismatching install receipt blocks replacement;
`--force` deliberately overrides those guards, **not signature checks or downgrade
protection**. A receipt requires Python 3 to validate; pre-receipt installs such
as 0.5.1 upgrade without it. Avoid mixing user installs and package managers.

### Manual download

Download a package and `SHA256SUMS` from [Releases](https://github.com/shuv1337/rex-rune/releases). Choose **x86_64 / amd64** for Intel/AMD or **aarch64 / arm64** for ARM64. For signed releases, [verify the signature](#verify-a-download) before installing.

```sh
sha256sum --ignore-missing -c SHA256SUMS

# Debian/Ubuntu: install the one .deb matching your architecture.
sudo apt install ./rex-linux_0.6.0_amd64.deb

# Or extract the matching tarball and install for your user (no root).
tar -xzf rex-linux-0.6.0-x86_64-linux-gnu.tar.gz
cd rex-linux-0.6.0-x86_64-linux-gnu
bash install.sh
```

The tarball installs into `~/.local/bin` with a desktop launcher and icons, independently of any source checkout. It needs `desktop-file-utils`; uninstall with `bash scripts/install-desktop.sh --uninstall`. Uninstall a `.deb` with `sudo apt remove rex-linux`. Neither removes preferences or changes Rex servers. Avoid mixing both installation methods: a user installation can shadow `/usr/bin/rex-linux`.

Prebuilt packages are built on Ubuntu 24.04 and require **glibc 2.39+**. They are dynamically linked, not self-contained AppImages. Install your distribution's graphics drivers, fontconfig (including `fc-match`), xkbcommon, Wayland/X11, EGL/OpenGL/Vulkan libraries, CA certificates and `xdg-utils`. On Debian/Ubuntu the `.deb` declares these dependencies. Rust and Zig are **not** needed to run a binary. A running Rex server with API revision 10, 11, 12 or 13 is required.

### Verify a download

Releases from 0.5.0 include `SHA256SUMS.minisig`, `latest.json`, and
`latest.json.minisig`; from 0.5.2, `install.sh` and `install.sh.minisig` too.
Both release keys are configured; v0.4.2 and earlier remain unsigned. The CI public key is
committed as [`packaging/keys/release.pub`](packaging/keys/release.pub):

- Minisign key ID: `A1FD78C27652816E`.
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `9e779605311a46a5287d6c9b25eb2083fb17447454239c7d66f1ecebe336d0c6`.

The offline recovery public key is committed as
[`packaging/keys/release-backup.pub`](packaging/keys/release-backup.pub):

- Minisign key ID: `368EA1A12E2721F3`.
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `d9fe424ca8b20725e11d6039c39445d38a33023a8d366c69e0612a1c3528de76`.

See the [public-key registry](packaging/keys/README.md). Both public trust roots
are embedded in the client for signed release metadata checks (`latest.json`); tag publication still fails closed on missing secrets or invalid
signatures. The client never self-updates or downloads binaries.

For a signed release, obtain and retain a trusted copy of those public keys
separately from the downloaded assets. With `minisign` installed, and the package,
`SHA256SUMS` and `SHA256SUMS.minisig` from the **same release** in your directory:

```sh
# Stop if verification fails. For a recovery release, use release-backup.pub instead.
minisign -Vm SHA256SUMS -p /path/to/trusted/release.pub &&
  sha256sum --ignore-missing -c SHA256SUMS
```

Require both commands to succeed before installing. A checksum alone detects
corruption, not a replaced download. First obtaining the public keys still relies
on trusting the repository; retain them for later verification. The signed
manifest is at `https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json`
with its `.minisig` beside it. Verify its exact bytes before parsing it; it has no expiry.

Release builds also produce GitHub provenance attestations for both package formats:
`gh attestation verify <downloaded-package> --repo shuv1337/rex-rune`.
Attestations identify the build workflow; they do not replace minisign verification.
Older releases do not gain signatures or attestations retroactively.

## Build requirements

- Rust 1.96 or newer (edition 2024; see `rust-version` in `Cargo.toml`).
- [Zig](https://ziglang.org/) 0.17.x. The `ghostty-vt` crate builds Ghostty's VT library from a checksum-pinned source archive with Zig.
- Linux with Wayland or X11, plus the usual graphics/input libraries: Vulkan, xkbcommon, fontconfig and the Wayland/X11 client libraries.
- A running Rex server that supports API revision 10, 11, 12 or 13. Verified with Linux servers **0.0.1794323808-g757adf (API 13)** and **0.1.0-preview.14 (`7e460d0`, API 12)**; see [API-13 compatibility results](docs/server-api13.md) and [preview.14 results](docs/server-preview14.md). Rex is in alpha, so older revisions aren't supported; update Rex on a host that refuses the connection. Uploading files needs revision 11.

## Build

```sh
cargo build --locked --release --bin rex-linux
```

Optionally, [mise](https://mise.jdx.dev/) can provide the toolchain: [`mise.toml`](mise.toml) pins Rust 1.96.1 (with Clippy and rustfmt) and Zig 0.17.1, and `mise run build` runs the command above. Run `mise trust` and `mise install` once first. Mise does not provide the native Linux libraries listed in [Requirements](#requirements).

The repository is also a Nix flake: `nix build .#rex-linux` builds a package with the desktop entry and icons, and `nix develop` gives a shell with the toolchain and libraries. See [Nix packaging](docs/nix.md).

`Cargo.lock` pins the dependency graph. Server connections use Rustls with the system trust roots; release metadata checks use Rustls with WebPKI roots and verify Ed25519 Minisign signatures against the embedded release public keys. The workspace also builds three probes (`rex-probe`, `rex-workspace-probe`, `rex-update-probe`) that do not need the UI; use `--no-default-features` to build only those.

## Run

```sh
# Create a persistent session on the local Rex server.
target/release/rex-linux --new work

# Reattach to an existing session by exact label or ID.
target/release/rex-linux --session work

# Attach to a remote server (for example one exposed over Tailscale).
target/release/rex-linux --endpoint https://my-host.example.ts.net --session work

# Attach to the Rex server on another machine through its SSH login.
target/release/rex-linux --endpoint ssh://me@my-host.example.ts.net --session work

# Choose a terminal font and size.
target/release/rex-linux --session work --font 'JetBrainsMono Nerd Font' --font-size 15
```

Options:

| Option | Meaning |
| --- | --- |
| `--endpoint <URL>` | `unix:///absolute/path.sock`, `https://…`, `http://…`, `wss://…`, `ws://…` or `ssh://[user@]host[:port][/socket]` (see [Hosts](#hosts)). Defaults to `$REX_SERVER`, then `unix://$XDG_RUNTIME_DIR/rex/server.sock` (or `unix:///run/user/1000/rex/server.sock` if `XDG_RUNTIME_DIR` is unset). |
| `--session <LABEL\|ID>` | Select a session by exact label or ID. Missing or ambiguous names never pick an arbitrary session. |
| `--new <LABEL>` | Explicitly create a persistent session (conflicts with `--session`). Ordinary attachment never creates or destroys sessions. |
| `--block <ID>` | Initially focus this terminal block. |
| `--font <FAMILY>` | Override the saved terminal family for this launch. Fonts are resolved through fontconfig; an unavailable primary family uses bundled IBM Plex Mono and reports it on stderr. |
| `--font-fallback <FAMILY>` | Override the saved fallback list for this launch. Repeat for an ordered list. System script/symbol fallbacks remain available. |
| `--font-size <PT>` | Override the saved terminal size (8–48 pt, initially 15). |
| `--config <PATH>` | Use a separate workspace preferences file. |
| `--window-size <WxH>` | Requested startup size (default `1440x900`); tiling compositors may override it. |
| `--check-update` | Verify the latest signed release manifest, print channel-specific upgrade guidance, and exit without starting the UI. |

Without `--session`, the workspace restores the saved host and session; on a new host it selects the first listed session. Rune's own developer flags go after `--` (for example `-- --snap out.png`). `REX_FRAME_STATS=1` prints per-frame CPU percentiles.

### Desktop install

```sh
bash scripts/install-desktop.sh [--copy] [path/to/rex-linux]
bash scripts/install-desktop.sh --uninstall
```

This installs `dev.shuv.rex-linux.desktop` under `$XDG_DATA_HOME/applications` (normally `~/.local/share/applications`) pointing at the release binary of this checkout (or the binary you pass), and the app icon at 32, 128 and 256 px into the user's `hicolor` theme. Without `--copy`, re-run it if you move the checkout. With `--copy`, the binary is copied to `$XDG_BIN_HOME` (normally `~/.local/bin`) and the launcher points there, so it no longer depends on the checkout. `--uninstall` removes the launcher, the icons and the copied binary. It needs `desktop-file-install`, `desktop-file-validate` and `update-desktop-database`; no root access. See [icon provenance](resources/icons/README.md).

With mise, `mise run install` builds the release binary and runs the script with `--copy`, and `mise run uninstall` runs it with `--uninstall`.

## Configuration

Preferences are stored as JSON in `$XDG_CONFIG_HOME/rex-linux/workspace.json` (default `~/.config/rex-linux/workspace.json`): saved hosts (label, endpoint, collapsed state, optional default shell), the last host and session per host, whether the sidebar is open and its width, tab position (`vertical_tabs`), pane density (`pane_density`: `"compact"` or `"comfortable"`, the default), pane padding (`pane_padding`: 0–32 pt, default 8 pt), terminal font family (`font_family`), ordered fallback families (`font_fallbacks`), font size, the chrome palette (`chrome_theme`: `"midnight"`, `"rose_pine"`, `"rose_pine_moon"` or `"gruvbox_dark"`; an unknown value is Midnight), the terminal theme (`terminal_theme`: `"follow_server"`, `"builtin_default"` for Rex's own colours, `"follow_chrome"`, or `"named:<name>"` with `rex/midnight`, `rex/rose-pine`, `rex/rose-pine-moon`, `rex/gruvbox-dark` or a Ghostty theme's file name), whether you have picked a terminal theme yourself (`terminal_theme_chosen`) and whether this client has ever declared one (`terminal_theme_declared`), interface zoom (`ui_scale`), the corner notices appear in (`notice_position`), the activity and notification choices (`notifications`), automatic update checks (`updates.check`, default `true`), and whether agents may name sessions (`agent_session_naming`). Update check state and dismissed versions are stored in `$XDG_STATE_HOME/rex-linux/update.json`. Use `--config <path>` to keep a separate workspace.

Settings (the sliders button in the toolbar, `Ctrl ,`, or Settings… in Commands) has Interface (chrome theme, zoom, pane density, sidebar, tab position, notice corner), Terminal (font family, fallback families, size, pane padding, theme, each host's default shell), Notifications (activity in tabs, desktop notifications and which changes send one), Integrations (Rex's skill and hooks for coding agents on this machine), Remote Hosts (add, edit or remove saved hosts), Shortcuts, Updates and About sections. A host's name, endpoint and default shell can also be edited from its sidebar header's right-click menu, from Edit host… in Commands, or from an offline session's menu. Removing a host only stops this client connecting to it; its sessions keep running on the server. A changed endpoint connects to that server as a new host. How hosts are added is described under [Hosts](#hosts).

In Interface settings, **Pane density → Compact** removes pane headers, outer borders, shadows, rounded corners, gaps and outer margins. One straight separator remains at each split boundary. Split placement and divider dragging stay available, and reconnect controls float over the affected pane. Pane actions remain on right-click or `Ctrl Shift Space`; per-pane activity words and progress bars are hidden, but tab activity remains. **Comfortable** is the default and keeps pane cards. A single, unzoomed pane stays edge to edge in either mode.

In Interface settings, **Tab position → Sidebar** lists each session's tabs under it in the sidebar, with their program icon, label and activity word, and takes the tab strip out of the toolbar while the sidebar is open; closing the sidebar brings the strip back without changing the setting. Choosing Sidebar opens the sidebar. Clicking a tab selects its session and window, and Commands and the keyboard shortcuts then act on it as usual. To list the tabs of sessions this client hasn't attached, it reads their layouts with read-only `session.view` requests, one at a time, for expanded hosts while the sidebar shows tabs; it never attaches them or opens their terminals, and a failed read is retried after 1, 5 and then every 30 seconds. **Top** keeps tabs in the toolbar.

In Terminal settings, **Change…** saves the primary family and optional comma-separated fallback families. Family changes take effect when you reopen the client; they do not restart the server or its sessions. The initial family is `JetBrainsMono Nerd Font`, with bundled IBM Plex Mono used if unavailable. Missing glyphs come from the named fallbacks in order, then bundled IBM Plex Mono, then automatic system fallbacks for other scripts and symbols; an empty fallback list uses only the latter two. Only the families you name are resolved through fontconfig, and their files are memory-mapped rather than copied. Linux discovery of the automatic fallbacks includes fontconfig's fonts, including fonts installed through Nix. Fallback faces supply missing glyphs without changing the primary font's grid metrics; install a suitable CJK font, such as Noto Sans CJK, for Chinese, Japanese and Korean. `--font` and `--font-fallback` take precedence over saved family choices without replacing them, so remove those flags from launchers when you want Settings to control the family.

A host's default shell (empty means the server user's login shell) is used for every new session, tab and split this client creates on that host. New tabs and splits start in the source pane's shell directory on that server: a new tab or split uses the focused pane; a pane-menu split uses the pane it names. The client asks the server for the shell's process directory, so no OSC 7 or local path lookup is needed; a foreground editor's directory is not used. If the read is unavailable or takes over one second, creation uses the server's default directory. Rex falls back to its home directory if the inherited directory no longer exists. New sessions keep their existing default. Host endpoints must be `unix://` with an absolute path, `https://`, `http://`, `wss://` or `ws://` URLs, or `ssh://[user@]host[:port][/socket]`.

## Hosts

**Add host** (the sidebar's footer, Settings → Remote Hosts, Commands, or the switcher's footer) lists the machines on this device's tailnet, from `tailscale status --json`, searchable by name. Only machines Tailscale shows to this device appear; this machine itself and iPhones and iPads (which run neither Rex nor `sshd`) are left out. A tailnet without MagicDNS lists machines by their Tailscale IP. Each online machine is checked for a Rex server reachable over the tailnet, at the same addresses the official clients try for a bare host name: `https://<machine>` on port 443 (built-in Tailscale or `tailscale serve`), then `https://` and `http://` on port 7297. The check negotiates a control connection and reads `server.status`; it never attaches to a session or changes the server.

- **Rex servers** show the endpoint that actually answered. On API 12, selecting one opens **Connect to Rex**, showing each reported listener's configured endpoint, actual bound addresses, transport and admission rules before **Connect**. Bound addresses are server facts, not proof that an alternate address is reachable or admits you; only the checked endpoint is selected. Unknown rules are shown verbatim. Revision 10/11 servers, empty listener lists and failed status reads say **listener details unavailable** and retain one-press attachment.
- Any other machine, online or offline, opens a form with its name, an **SSH user** (your user name is offered) and an optional **Rex socket**, and connects as `ssh://user@<machine>[/socket]`. The form says what the check found: nothing listening on those ports, or that it couldn't tell (a timeout or a certificate this client doesn't trust), in which case an `https://` endpoint can still be entered by hand. It also says whether Tailscale SSH is on there, in which case the tailnet's policy signs the login in with no keys; otherwise your SSH keys and configuration apply.
- A machine already added stays listed, marked with the hosts that reach it; pressing it adds another login over SSH (another user, or another socket). The same endpoint is never added twice.
- **Enter an endpoint…** takes any endpoint by hand, as before. Without Tailscale, the list says so and this is what remains.

An `ssh://` host is reached by forwarding the Rex server's Unix socket on that machine to a local one with `ssh -L` (OpenSSH's Unix-socket forwarding, so only `sshd` is needed there). Unless the endpoint names the socket (`ssh://user@host/run/rex/server.sock`), the client first asks the machine where it is: the path `rex server status --autostart=false` prints, then the login user's own server (`$XDG_RUNTIME_DIR/rex/server.sock` on Linux, `~/Library/Application Support/rex/server.sock` on macOS), then a shared server's `/run/rex/server.sock` or `/var/run/rex/server.sock`; "No Rex server found on … for …" names those places when none is a socket and suggests a manual `rex server install 'ssh://[user@]host[:port]'` (preview.14 or newer) for a Linux host. That command asks before installing or configuring anything; the app never runs it. `ssh` runs with `BatchMode=yes` (it never prompts; use an agent for keys with passphrases), `StrictHostKeyChecking=accept-new` (a machine's key is trusted the first time and a changed key is refused), keep-alives and `ControlMaster=no` (a shared connection in your SSH configuration is not used, since it would keep the forward after this client lets go). The local end lives in a directory of this process's own under `$XDG_RUNTIME_DIR/rex-linux`, so two instances (or a probe alongside the app) forwarding the same machine don't disturb each other, and it is removed when the forward ends; directories left by a crashed instance are swept on the next forward. When the forward drops, the host reconnects through a new one like any other disconnect, and the reason `ssh` gave (for example "tailnet policy does not permit you to SSH as user …") is shown next to Retry. Edit host… shows the `ssh://` endpoint, where the user name and socket can be changed.

Ordinary attachment keeps using socket forwarding and **never auto-starts** a server. The separate, confirmed setup action below can invoke the Rex CLI's native `-S ssh://host` auto-start once. No local Rex executable or Tailscale is required for SSH attachment; your OpenSSH configuration and aliases still apply. Discovery uses `server.status` listener metadata after a documented tailnet endpoint answers; it never guesses alternate reachable addresses from wildcard, loopback or Unix bindings. Built-in Tailscale is not in the server's listener list.

The SSH user is the login on that machine, which is also the user whose Rex server is forwarded; it is not a Rex account in the sense of Rex's login server, which is chosen in an `https://user@host` address instead.

### SSH setup

When SSH succeeds but finds no default server, **Host unavailable**, the host's menu and Commands offer **Start personal server…**. It is not offered for SSH authentication/transport failures, an already connected host, or a missing explicitly named socket.

The dialog names the host and full `ssh://` endpoint, previews the command and explains what changes: Rex starts a personal server as that SSH login, may create its user config/state, and leaves it running after the client closes. It creates no sessions, installs no packages and creates no system services. **Not now** is the default (including Enter); only **Start personal server** authorizes a single attempt. Closing the busy dialog does not cancel that already-authorized start.

Setup requires a local and remote `rex` reporting API 12 or newer (preview.14). Version checks run before starting. A missing or old executable, SSH refusal or failed start is shown with the command's error and never retried automatically. After success, the client reconnects through the original `ssh -L` path; aliases, optional users/ports and SSH configuration still apply. No persistent auto-start preference is saved.

The native command is `rex --autostart=true --server ssh://host --timeout 20s api call server.status '{}'`. Using `api call server.status`, rather than `rex server status`, matters: the latter deliberately does not auto-start even with the flag. This read-only API call creates no session.

Installation is deliberately not automated: if Rex is missing, run `rex server install 'ssh://host'` yourself in a terminal. Rex's installer asks before each change and adds the host; the app neither answers its prompts nor installs packages/services. You can instead edit the endpoint to an existing custom socket and remain attachment-only.

## Keyboard shortcuts

Global tab navigation uses cached tab order without enabling extra prefetch: a session whose view is not loaded is one stop, and selecting it loads its tabs. Crossing sessions or hosts settles after 150 ms without another cycle, coalescing fast repeats rather than attaching each intermediate session. The first stop can attach before key repeat begins. During settling there is no target preview, and typing still goes to the displayed pane. Back/Forward history is local to this window and survives reconnects; unavailable targets are skipped. Commands also exposes these navigation actions. Ctrl Tab is a client default; remove it (see recipes) to pass it to editors such as Neovim.

| Shortcut | Behavior | Action (and args where required) |
| --- | --- | --- |
| Ctrl K | Search sessions across saved hosts | `session.switch` |
| Ctrl Shift P | Search commands, sessions and current-session tabs | `client.palette.toggle` |
| Ctrl Shift N / T | New session on the active host / new tab | `session.new` / `client.tab.new` |
| Ctrl Shift D / Ctrl Alt D | Split right / down | `pane.split`, `{"direction":"right"}` / `{"direction":"down"}` |
| Ctrl Tab / Ctrl Shift Tab | Next / previous tab across connected hosts and sessions, in sidebar order (wraps) | `client.tab.next_global` / `client.tab.previous_global` |
| Ctrl PageUp / PageDown | Previous / next tab within the current session (server-controlled, no wrap) | `client.tab.previous` / `client.tab.next` |
| Mouse Back / Forward | Revisit tabs, sessions and hosts in navigation history | `client.history.back` / `client.history.forward` |
| Ctrl Alt arrows | Focus an adjacent pane (Alt arrows reach terminal programs) | `pane.focus`, `{"direction":"left"}` (or `right`, `up`, `down`) |
| Ctrl Shift Enter | Zoom / restore the focused pane | `pane.zoom` |
| Ctrl Shift Space | Focused-pane actions | `pane.actions` |
| Ctrl Shift W | Close the focused pane, or the tab when it has one pane (asks first) | `client.close` |
| Ctrl , | Settings | `client.settings` |
| Ctrl Shift B | Collapse / restore the sidebar | `client.sidebar.toggle` |
| Ctrl Shift C / V | Copy selection / paste text (bracketed-paste aware), or upload copied files or an image | `pane.copy` / `pane.paste` |
| Ctrl Shift F | Search the focused terminal's screen and scrollback | `client.search` |
| Ctrl Shift R | Resume the focused terminal's data stream | `pane.reconnect` |
| Shift PageUp / PageDown, wheel | Scroll local history | `pane.scroll`, `{"lines":-20}` / `{"lines":20}` (wheel is pointer input) |
| Ctrl + / Ctrl − / Ctrl 0 | Enlarge / shrink / reset the whole interface | `client.zoom.in` / `client.zoom.out` / `client.zoom.reset` |

These are the client defaults; bound chords do not reach terminal programs, and every other key goes to the focused pane. Interface zoom changes in 10% steps (70–150%), is saved as `ui_scale`, and is independent of the terminal font size. `Ctrl =` and the numpad `+`/`−` also work.

The following actions have no default key binding:

| Action | Args schema | Behavior |
| --- | --- | --- |
| `client.tab.goto` | `{"index": integer >= 1}` | Focus the 1-based tab in the current session |
| `client.tab.close` | None | Confirm closing the current tab |
| `client.tab.rename` | None | Open the current tab's rename dialog |
| `session.rename` | None | Open the current session's rename dialog |
| `pane.close` | None | Confirm closing the focused pane |
| `pane.send_key` | `{"key": "single chord"}` | Send a synthesizable key directly to the PTY, bypassing Lua bindings |
| `pane.download` | None | Open the Download file… dialog for the focused pane (API revision 13). Never available to `client.perform` |

Omit `args` for actions with no arguments (an empty object also works).
For argument-taking actions, supply exactly the required field: `pane.split`
accepts `direction` = `right` or `down`; `pane.focus` also accepts `left` and
`up`; `pane.scroll` takes a signed 32-bit integer `lines` (negative scrolls up).
`pane.send_key` takes a single supported chord, not a sequence; for example
`{"key":"ctrl+b"}`.

### Configuring shortcuts

For the default workspace, edit `$XDG_CONFIG_HOME/rex-linux/shortcuts.json` (normally
`~/.config/rex-linux/shortcuts.json`). With a custom `--config /path/demo.json`,
edit `/path/demo.shortcuts.json`. The app never writes this file, except for the
one-time import of custom bindings from nivox's legacy `keybindings` field.

The preferred format is an object; a bare array also works. A missing, empty or
whitespace-only file means no overrides. Bad entries are skipped; invalid JSON
uses defaults and appears in Settings → Shortcuts → Problems.

```json
{
  "shortcuts": [
    {"keys": "ctrl+b>c", "action": "client.tab.new"},
    {"keys": "ctrl+tab", "action": null},
    {"keys": "ctrl+shift+tab", "action": null}
  ]
}
```

Layers run lowest to highest: **Linux defaults → each server’s Rex config → client overrides**.
The app sends its defaults, override layer and action registry to `client.keymap`;
`rex.bind`, `rex.unbind`, Lua-function sequences and config-file actions participate
in the same resolution. The active host’s map is used—not a union of hosts.
When that host is offline, has not answered, or rejects the keymap request, local
defaults plus overrides work instead. A keymap error never disconnects the host;
Settings → Shortcuts retains its explanation. Config changes refresh by hash and
server instance, and every reconnect registers again.

### Remote control

Settings → Remote Hosts has a **Remote control** switch for each saved host,
**off by default, including local hosts**. Changing an endpoint turns it off again.
Commands can turn it on or off for the host on screen. With it on, `rex do` and
`client.perform` can run exactly eight actions: `session.select`, `client.tab.goto`,
`client.tab.next`, `client.tab.previous`, `pane.focus`, `pane.zoom`, `pane.scroll`,
and `client.sidebar.toggle` on that host.
These requests cannot create or close terminals, type, access the clipboard, open
dialogs (so none can start a download), change interface zoom, or affect any host other than the requesting host. Refusals name the
server command to use instead where one exists (`rex window new`, `rex send-key`,
`rex capture`, etc.). Actions remain listed even with the switch off.

`session.select` shows a session on the requesting host, switching to that host if
needed; an optional `window_id` selects a tab. It replies only after navigation is
acknowledged, or fails after five seconds. Unknown sessions/tabs return `not_found`.
All other requests must match the host, session and pane on screen. Navigation runs only
after a pause in typing, and never while a text field has focus. The pause must
last at least 1.5 seconds. All remote actions are refused while a text field has
focus, a panel is open or a key sequence is pending. Requests not acted on within
five seconds are discarded, including time buffered during a busy connection
step (counted conservatively from that step's start). Replies never cross a reconnect.
Every performed request gets a six-second in-app notice naming host, requester
and action; nothing raises the window or sends a desktop notification. The first
request refused because control is off shows one explanatory notice per host per
launch. Requester identity is displayed, not used as a second permission filter.

The **Client label**, also in Remote Hosts, defaults to this computer's current
hostname (or “Rex Linux” if empty), without saving that default as an override.
It is visible to clients on every connected server and can be edited (reopen the
app to apply it). Use `rex -C LABEL actions` to list actions, then, for example,
`rex -C LABEL do client.sidebar.toggle`. Local shortcuts and queued actions from
this client's own `client.action` are independent of Remote control.

```sh
rex -C LABEL do session.select session_id="$REX_SESSION"
rex -C LABEL do session.select session_id=SESSION window_id=WINDOW
rex -C LABEL do client.tab.goto index=2
rex -C LABEL do client.tab.goto index=-1  # last tab
```

`session.select` can also be bound with fixed arguments in `shortcuts.json` or
queued by Lua; it does not add an argument-free Commands entry. Cross-host remote
control was checked against two private preview.14 servers, with the second
reached through a real `ssh://` socket forward and disposable SSH keys. Run
`scripts/remote-control-acceptance.py` with `REX_BIN` pointing to a built client
and preview.14's `rex` on `PATH`; it owns its servers, sockets, HOME/XDG state and
Xvfb display under `~/.cache/agent-ws/`, and stops all its processes by PID. It also
captures Settings → Remote Hosts and the performed-request notice. If a private
`sshd` cannot run, it reports that and checks two local sockets instead.

Later entries win; `null` frees those exact keys for the terminal. Hand edits reload
when the window regains focus or Settings opens, gated by file mtime and length.
`cmd`/`super` means Super, not Ctrl. Physical keys use W3C names such as
`ctrl+[KeyT]`; sequences use `>`, Escape cancels, and a failed tail goes to the
terminal without retrying it as a new shortcut. Non-ASCII bindings such as
`ctrl+é` work in the client but are not readable by the server's parser, like the
client-only `ctrl++` extension. Ordinary terminal keys still use `client.key`.

Server action calls use a dedicated 35-second timeout (Rex allows an action 30 seconds).
The host's control loop is serial: later input and control commands queue behind a
running action. Failed or timed-out input is never replayed automatically. Known
returned client actions are performed once, in order; unknown names are ignored.
No cross-server `servers` list is advertised. A returned action whose original
host, session, pane or local keymap generation is no longer current is skipped
with an informational notice rather than applied to a different target.

Settings → Shortcuts shows effective keycaps, “then” between sequence steps,
sources, persistent Problems and both config paths. Commands shows the highest
binding for each action and argument set, or no shortcut when unbound. Only
settings-source diagnostic sets toast, once per distinct set per host; file and
default-layer diagnostics stay in Problems. A pending sequence shows `Ctrl B …`
in the toolbar. Host/pane/session/focus/map changes and opening a dialog cancel it.

Only popup navigation/editing keys and Escape cancelling a sequence stay reserved;
zoom is global (including fields), but it is still rebindable. Hold-to-repeat works
for zoom, pane focus/scroll, within-session tab navigation (Ctrl PageUp/PageDown)
and global tab cycling. Other completed shortcuts and
prefix repeats are suppressed, and releases only reach terminals when presses did.

#### More recipes

```json
{
  "shortcuts": [
    {"keys":"ctrl+shift+t", "action":null},
    {"keys":"ctrl+b>c", "action":"client.tab.new"},
    {"keys":"ctrl+b>shift+5", "action":"pane.split", "args":{"direction":"right"}},
    {"keys":"ctrl+b>ctrl+b", "action":"pane.send_key", "args":{"key":"ctrl+b"}},
    {"keys":"ctrl+1", "action":"client.tab.goto", "args":{"index":1}}
  ]
}
```

`pane.send_key` is the explicit escape hatch through `block.event` (press and
release); normal keys and forwarded Lua sequences use `client.key`, with the
server’s original prefix spellings. Action arguments are checked before dispatch.
Keys accept modifier aliases, named keys, F1–F12, W3C physical codes and `>` sequences.
Letters follow the keyboard layout, with physical US-position fallbacks on
non-Latin layouts; Shift stays a modifier instead of being folded into the key.
Key modes, a recording editor, server actions in Commands and pane-resize bindings
are not implemented. Cross-server `client_call_requested` relaying is also not
implemented ([#20](https://github.com/shuv1337/rex-rune/issues/20)). Agents should use
Rex's session, window and block commands for server state, and show refusal messages.

The parser, sequence machine, action-registry structure and repeat suppression
adapt Andrea Zito ([nivox](https://github.com/nivox))’s `8037d78` work. Importing a
legacy `keybindings` field drops copied defaults, writes only custom entries once,
and reports a move only when at least one custom entry remains.

Double-click selects a word (including shell paths), and triple-click selects a line. Selection stays attached to terminal content as history or output scrolls; dragging above or below the grid autoscrolls. Hold Shift to select locally or scroll local history when a fullscreen TUI owns the mouse. Without Shift, ordinary TUI mouse reporting is unchanged.

Terminal-initiated clipboard writes (OSC 52) are described under [Clipboard (OSC 52)](#clipboard-osc-52).

**Uploading files.** Drop files on a pane, or paste with Ctrl Shift V when the clipboard holds copied files or only an image, and Rex stores them on the machine that terminal runs on and types their shell-quoted paths into it, as a paste, without pressing Enter. This works the same for local and remote hosts and needs no SSH access: the bytes go over the Rex connection with `upload_begin` (API revision 11). The server keeps each file in a private directory under its state directory; files dropped together are typed together, and nothing is typed unless every file arrived. Folders can't be uploaded. A pasted image is uploaded as `clipboard.png`; when the clipboard holds text as well, the text is pasted instead. A failed upload shows a notice and is never retried. An older server shows a notice asking you to update Rex on that host.

Other interactions: click the session/host identity for grouped navigation, or right-click it for the current session's actions; in the switcher, rename a session in place (right-click its row, F2 or the pencil; Enter saves, Esc cancels) or close it with its ×; right-click a pane for actions targeting that pane; drag dividers to resize splits and the sidebar's edge to resize it (double-click resets); double-click a tab to rename it. Closing a pane or tab asks for confirmation, with **Keep running** as the default. The active window takes size ownership of a terminal automatically. On the borderless window, the toolbar's empty space moves it (double-click maximizes; right-click requests the window manager's menu on Wayland).

## Downloading files

Needs a server with API revision 13 or newer (`rex` snapshots from 2026-11-10; preview.14 is revision 12). On an older server the dialog says "Rex on <host> is too old to send files; update Rex there." and nothing is sent.

- **Starting one.** Choose **Download file…** in the pane menu (right-click, or `Ctrl Shift Space`) or in Commands (`Ctrl Shift P`), or bind `pane.download` to a key (it has no default; see [Configuring shortcuts](#configuring-shortcuts)). The dialog is titled with the host of the pane it was opened from. Type a path on that machine: absolute, `~/…`, or relative to the pane's foreground program's directory (so it follows `cd`). The path is always on the machine running Rex, even when the pane shows an `ssh` session or a container: with `ssh remote` in the foreground, a relative name resolves in `ssh`'s own directory on the Rex host, not on `remote`, so type an absolute path there. The resolved path is the row's tooltip in the Downloads popover. A single-line selection in the pane fills the field. Enter starts; Escape closes. Only a person at this client starts a download: terminal output and `client.perform` cannot.
- **Where files go.** The **Save in** folder shows under the field: the XDG download folder (`xdg-user-dirs`, else `~/Downloads`), or `downloads.directory` in `workspace.json` (absolute or `~/…`), which **Change** in the dialog writes. The file keeps the server's file name after cleaning: control characters, direction overrides and invisible characters are removed, a leading dot becomes `_`, trailing dots and spaces go, and a name that ends up empty becomes `download`. A leading dash and non-ASCII names are kept as they are. The folder must already exist: it is never created, and a missing one fails with "<folder> doesn't exist." (use **Change**, or create it first; minimal desktops often have no `~/Downloads`).
- **Never overwritten.** If the name exists, the new file is `name (1).ext`, then `(2)`. The saved file is mode 0644 less the umask and never executable, whatever the source's mode. While running, the data goes to a hidden `.name.<uuid>.rex-part` in the same folder, renamed into place only when the transfer finished completely.
- **Never opened by itself.** A finished transfer leaves one notice ("Saved build.log from devbox to ~/Downloads"); nothing is shown in a file manager or run. A finished row in the Downloads popover offers **Show in folder** (the file manager's `ShowItems`, else `xdg-open` on the folder) and **Open** (`xdg-open`). **Open is offered only for a short allow list** of types: text, Markdown, PDF, images, JSON, CSV and archives, that is the final extension (compared without case; for `x.tar.gz` it is `gz`) `txt`, `text`, `log`, `md`, `markdown`, `csv`, `tsv`, `json`, `pdf`, `png`, `jpg`, `jpeg`, `gif`, `webp`, `zip`, `tar`, `gz`, `tgz`, `xz`, `zst`, `bz2` or `7z`. The file's first 512 bytes must also pass: a file that starts as a program or script (ELF, `#!`, `MZ`, `%!PS`), as HTML, SVG or XML markup (after any byte order mark and white space: `<!doctype`, `<html`, `<head`, `<body`, `<script`, `<svg`, `<?xml`, `<!--`, `<h1`, `<title`) or as a desktop entry gets no **Open** whatever its name, and a `pdf`, `png`, `jpg`, `jpeg`, `gif`, `webp`, `zip`, `gz`, `xz`, `zst`, `bz2` or `7z` file must start with that type's own magic bytes, so a file that is not what its name says gets none either. **Everything else gets only Show in folder**: no extension, SVG, HTML, XML, PostScript, Office and OpenDocument files, audio, video, scripts, packages and every unknown type. The known cost: a `README.md` or `.txt` that begins with an HTML comment or `<h1` gets no **Open** (use Show in folder). The check is repeated when you press **Open**, so a file rewritten after it was saved is refused.
- **Progress.** While bytes arrive, a chip in the toolbar shows `↓ build.log 42%` (or `↓ 3 files`); click it, or choose **Downloads** in Commands, for a row per transfer of this launch with its state and **Cancel**. Cancelling closes the stream and deletes the partial file. A failure is a sticky notice and a row that says why; nothing is retried or resumed.
- **Limits and confirmation.** A file of 1 GiB or more, or one that would leave under 256 MiB free, is not started until you answer a question in the dialog ("build.img is 3.2 GB. ~/Downloads has 41 GB free."), with **Cancel** focused. A file that cannot fit with 16 MiB left over is refused at once. After a yes to the low-space question the transfer stops if free space falls under 16 MiB; after a yes to a large file it stops under 256 MiB. A transfer ends if the file grows past the larger of twice Rex's size and that size plus 64 MiB (a file being appended to), or if no data arrives for 30 seconds. A file that changes while it is sent is saved as it arrived: one that grew by less than that is saved longer than Rex first reported, and one that shrank or whose reported size was wrong (`/proc` and `/sys` files) is saved shorter; the Saved notice adds the size when it differs from what Rex reported ("6 bytes; Rex reported 4096"). A log, database or archive that is being written to can therefore be saved as a torn copy and still say Saved, so download files that are not being written to. At most three downloads run at once per host; a fourth is refused. There is no other size limit.
- **Leftovers.** Quitting the app with the Close button or the window manager's close cancels every running download without asking, and a transfer is never resumed on the next launch, so a large file that is nearly done is lost. A partial file is deleted on cancel, on failure and when the app quits that way; each is recorded in `$XDG_STATE_HOME/rex-linux/downloads.json`. The app has no signal handler: a signal (SIGTERM, SIGINT, SIGHUP) or a crash ends it at once and leaves the partial file until the next launch, which deletes it. Only recorded files are touched; the folder is never swept.
- **Known limits.** Filesystems that support neither a no-replace rename nor hard links (sshfs, CIFS without Unix extensions) fail cleanly (the transfer ends with an error and the partial file is deleted). A download folder that is itself a symlink is refused, and the message names the real folder. A transfer ends if the connection to the host drops, and is never resumed or retried; Rex ends every stream when a control connection goes.

`python3 scripts/download-acceptance.py` checks this behaviour against the built binary (see [Development](#development)); HTTPS and Tailscale hosts, the real file-manager and `xdg-open` calls and Wayland pointer input were not part of that run.

## Clipboard (OSC 52)

A program in a terminal copies to your desktop clipboard by writing an OSC 52 sequence (`ESC ] 52 ; c ; <base64> BEL`). Rex Linux puts the text on the system clipboard of the machine the client runs on, with no environment variable and no setting, for a local server and for an `ssh://` host alike (checked live on X11 and on a private Wayland compositor, and through a private `ssh://` login).

- A copy is applied only from the focused pane of the selected session on the active host, while the Rex window has keyboard focus and no dialog or popup is open. Any other copy (a background tab, an unfocused split pane, another session, an unfocused window) is dropped without a notice and leaves the clipboard as it was.
- The selections `c`, `p` and `s` (and an empty one) all go to the clipboard; the primary selection is never set. The server drops a target of several letters such as `cp`.
- Read requests (`?`) are never answered, so a remote program can't read your clipboard.
- The server drops, without any event or notice here, a payload that is not valid base64, is empty or is not UTF-8 text, and a very large one: 6,000,000 bytes arrived and 6,291,456 did not in the server used for the check; the exact boundary was not measured. The client refuses text above 8 MiB from the server (`MAX_COPY_BYTES` in `src/connection/clipboard.rs`; this limit was read in the code, not reached in a test).
- Only the newest copy counts when several arrive together.

### Programs that don't copy through OSC 52

Some programs send OSC 52 only when an `SSH_*` variable is set, or prefer the clipboard of the machine they run on. Neither is the client: **Rex Linux cannot set environment variables in a terminal on a remote server, because the Rex server owns the terminal's environment** (the session and block options have no environment field; an `env` option is accepted and ignored). What the sources and tests showed:

- **OpenAI Codex CLI** (read from the source of `rust-v0.167.1`; Codex itself was not run): it always tries the native clipboard first, so on a server that has `DISPLAY` or `WAYLAND_DISPLAY` the text lands in the server's own clipboard whatever else is set. It additionally sends OSC 52 when `SSH_TTY` or `SSH_CONNECTION` (any value) or `HERDR_ENV=1` is set, even if the native copy succeeded; under `TMUX`/`TMUX_PANE` it forwards through tmux instead (`tmux load-buffer -w`, then a passthrough sequence), which reaches Rex with tmux's default `set-clipboard`. It falls back to OSC 52 alone only if the native copy fails.
- **Neovim 0.10.x** enables its OSC 52 provider only when `SSH_TTY` is set (any value) and the `clipboard` option is empty (the default); without both there is no provider (checked with 0.10.4). **Neovim 0.11 and later** don't check `SSH_TTY` and, when the `clipboard` option is empty (the default), use OSC 52 on their own, which Rex advertises (checked with 0.12.6), but they pick `xclip`, `xsel` or `wl-copy` first when `DISPLAY` or `WAYLAND_DISPLAY` is set, whatever `SSH_TTY` says. **LazyVim** sets `clipboard` to empty only when `SSH_CONNECTION` is set, and that is what turns its OSC 52 provider on (checked live: `SSH_TTY` alone does not).
- `rex server start` hands `DISPLAY` and `WAYLAND_DISPLAY` from the shell that starts the server on to every terminal.

Workarounds on the machine that runs the Rex server (checked live):

- Put this in the file your login shell reads (`~/.profile` or `~/.bash_profile` for bash, `~/.zprofile` for zsh; not only `~/.bashrc` or `~/.zshrc`, since Rex starts terminals as login shells). It keys on `TERM_PROGRAM=rex`, which the server sets in every terminal, and leaves an existing `SSH_TTY` alone:

  ```sh
  if [ "$TERM_PROGRAM" = rex ] && [ -z "${SSH_TTY+x}" ]; then
    _rex_tty=$(tty 2>/dev/null)
    case $_rex_tty in /dev/*) export SSH_TTY=$_rex_tty ;; esac
    unset _rex_tty
  fi
  ```

  It worked in a default login bash, `rex run`, splits, a nested `bash` and a zsh login shell; it does nothing for a `--shell=none` terminal. To keep it off machines with a desktop session, add `&& [ -z "$DISPLAY$WAYLAND_DISPLAY" ]` to the condition (this variant was not checked); with that guard, Codex on a machine that has a display keeps copying only to that machine's clipboard.
- Or start a new server with `rex server start --env SSH_TTY=/dev/tty`. It applies to every terminal of that server and only when a new server is launched.

Both make every terminal on that server look like an SSH session, local clients included: prompts such as Starship show the host and an SSH mark (checked with Starship), some dotfiles branch on `SSH_*`, and by its source Codex then also treats the session as remote: it turns off right-click paste, reading text from the clipboard and primary-selection copy. Neither workaround sets `SSH_CONNECTION`, which LazyVim needs, and neither stops a program that prefers the server's own clipboard (Neovim 0.11 and later) from using it while `DISPLAY` or `WAYLAND_DISPLAY` is set there.

## Development

```sh
cargo test --locked --all-features
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings

# Protocol probes against a Rex server (endpoint optional; they create and remove their own sessions).
cargo run --locked --no-default-features --bin rex-probe
cargo run --locked --no-default-features --bin rex-workspace-probe

# File downloads against a disposable server you control (creates a session there; refuses the default per-user socket).
cargo run --locked --no-default-features --bin rex-download-probe -- unix:///path/to/private.sock /path/to/workdir
```

With mise, `mise run test`, `fmt`, `clippy` and `test-rexrs` run the first three commands and the vendored SDK tests; `mise run check` runs them all.

`python3 scripts/keymap-acceptance.py` checks layered bindings, native sequences,
Lua forwarding, queued client actions, file/server reload, removals and the
send-key escape hatch and explicit remote-action refusal on its own disposable Rex server and Xvfb display. It uses
`target/release/rex-linux` unless `REX_BIN` selects another build, and preserves
logs/captures in the printed temporary directory.

The `scripts/` directory holds integration checks; each script's docstring lists its requirements (Xvfb, xdotool, a `rex` binary, `wayland-info`, …). `x11-smoke.py`, `rune-acceptance-x11.py` and `wayland-seat-smoke.py` use a server you start: they require `REX_SERVER` to name a disposable one (autostart is off, and they exit rather than fall back to the default server); every other script starts its own private server. Every script that launches the client honours `REX_BIN`, and runs it with a `tailscale` that finds no tailnet and no update check. X11 checks use an isolated display, and those that select text by pointer pin the pane padding in their private `workspace.json`, so pointer positions come from the layout they set rather than the app's default. Of the Wayland scripts, `mouse-check.py`, `readme-screenshot.py` and `demo-record.py` start a private headless compositor; `wayland-seat-smoke.py` opens its window on the compositor `WAYLAND_DISPLAY` names, so point it at a disposable one.

- `connection-only-smoke.py`: a missing or stale socket must never execute the Rex CLI.
- `ssh-setup-smoke.py`: the real opt-in setup UI against a private localhost sshd and preview.14 executable, with isolated HOME/XDG state and SSH config. Checks attachment-only, Not now, confirmed native start, zero created sessions and subsequent `ssh -L` attachment; stops its server and sshd by PID. Requires explicit `REX_CLI` and `REX_BIN`; no installation or real host setup is performed.
- `download-acceptance.py`: file downloads through the built binary and the real UI, on its own private API 13 and API 12 servers, a private Xvfb and (when `sshd` can run as this user) a private SSH host. Covers names with spaces and Unicode (and, through the core library only, a newline), a leading dash, `~` and relative paths, a symlink, a folder and a FIFO refused, an empty file, a 1 GiB file cancelled partway, a file growing during the transfer, the same file twice, the "too old" message on API 12, the fourth concurrent transfer, low space (a size-limited tmpfs in a private user namespace), one transfer over `ssh://`, and quitting or killing the app mid-transfer; it checks every saved file's SHA-256, that nothing is executable and that a stub `xdg-open` is never called. It also checks that **Open** is offered for a `.txt` row and not for a program, a script, a file with no extension or an `.svg`, and that the server holds the 1 GiB question's file until Escape releases it. No case presses **Open** or **Show in folder**, so the stub only proves nothing opens a file by itself. Prints PASS, FAIL or SKIP per case; a skipped case (no API 12 `rex`, `sshd`, `unshare` or probe) does not fail the run unless `--strict` is given, which is the form to use for a release (it also requires at least 15 saved files to have been compared). The artifact folder under `~/.cache/agent-ws` is deleted when every case passed and kept otherwise, or with `--keep`. Also needs the client (`REX_BIN`) and `rex-download-probe` built; `REX12_BIN` selects the API 12 `rex`. `private_ssh.py` is the private sshd helper it shares with `remote-control-acceptance.py`.
- `navigation-x11.py`: isolated single-host private-server checks for global tab shortcuts, reverse wrapping and mouse Back/Forward across sessions; also exercises named history Commands. Cross-host navigation and reconnect behavior are covered by headless/protocol tests, not this script.
- `process-cwd-contract.py`: private-server process-directory and missing-directory fallback checks without OSC 7; set `REX_BIN` to also check keyboard/pane-menu splits and new tabs through the real client on X11. Keeps logs/captures under `~/.cache/agent-ws/`.
- `x11-smoke.py`: real X11 events (forms, clipboard, exact pointer copy and raw Kitty key isolation). Requires `REX_SERVER` explicitly set to a disposable server (no autostart); `REX_BIN` selects a debug or release client. Runs the client with its own HOME and the pane padding pinned.
- `scroll-selection-x11.py`: private-server Unicode copy checks after scrolling, drag autoscroll, and Shift selection in a fullscreen mouse-tracking TUI without breaking ordinary mouse reports.
- `kitty-graphics-x11.py`: private-server pixel checks for live RGBA, chunked PNG, image restoration after reattach and deletion.
- `clipboard-x11.py`: isolated private-server acceptance check for this feature; see its docstring.
- `alt-arrow-x11.py`: isolated private-server acceptance check for this feature; see its docstring.
- `sidebar-x11.py`: private-server check that the sidebar's open state survives relaunch, with captures of both tab positions.
- `rune-acceptance-x11.py`: window-corner, terminal-key and frame-CPU acceptance checks on X11. Requires `REX_SERVER` (disposable); also the shared helpers (`Desktop`, `grid_origin`) the other X11 scripts import.
- `chrome-spike-x11.py` (with the `chrome_spike` example): borderless window chrome on X11.
- `wayland-seat-smoke.py`: Wayland seat/resource selection on a multi-seat compositor. Requires `REX_SERVER` (disposable) and uses `REX_BIN`.
- `mouse-check.py`: terminal mouse reporting with real pointer input on a private headless Wayland display.
- `readme-screenshot.py`: regenerates `docs/screenshot.png` from a private Rex server and headless compositor, with a throwaway home and prompt so no local details appear.
- `docs-captures.py`: regenerates the documentation screenshots through Rune's `--script`/`--snap`/`--tree` tooling, headless, against two private servers; the sidebar is opened by the saved config, the Add host list sees no tailnet and the `cargo test -q` pane prints fixed text, so reruns are reproducible.
- `demo-record.py`: records a demo video of the app on a private headless compositor (uses the `tools/vptr` virtual-pointer helper).

See [AGENTS.md](AGENTS.md) for the code layout and contributor conventions, and [DESIGN.md](DESIGN.md) / [PRODUCT.md](PRODUCT.md) for the design system and product intent.

## Vendored dependencies

Three dependencies are vendored with small, documented patches and wired in through `[patch]` in `Cargo.toml`:

- [`vendor/rune/`](vendor/rune/README.md): the `rune`, `rune-motion`, `rune-staging`, `rune-taffy` and `procwatch` crates from `anomalyco/rune` at `d70929a` (MIT), with dead-key forwarding and window-chrome additions for a borderless Linux window (maximized state, minimize, edge resize, window menu, native window icon).
- [`vendor/rexrs/`](vendor/rexrs/REX-LINUX.md): `a10y/rexrs` at `780be3f` (MIT OR Apache-2.0), plus revision-13 generated types, explicit API-revision range negotiation and preservation of an empty `attach_session_ids`.
- [`vendor/ghostty-vt/`](vendor/ghostty-vt/REX-LINUX.md): the crates.io `ghostty-vt` 0.8.0 source (Apache-2.0; Ghostty itself is MIT), plus a safe wrapper for Ghostty's render-state image iterator, image-aware snapshot decoding, a fixed temporary-file medium setting and grayscale PNG normalization.

Each patch is meant to be dropped once the corresponding upstream release has equivalent APIs.

## Current scope

Still needing hands-on validation: physical Wayland pointer gestures (move, edge resize, double-click, window menu), multi-seat Wayland desktops, input methods beyond dead-key composition, and sustained daily use. On X11, winit has no window-menu request, so right-clicking the drag region does nothing there.

Not implemented: animated Kitty graphics. Packaging includes the Nix flake, Linux tarballs and Debian/Ubuntu packages; RPM, Flatpak and AUR packages are not provided.

### Accessibility

Rune's AT-SPI tree is enabled (Rune's default `accessibility` feature). Rune builds it only while the desktop's accessibility bus has `org.a11y.Status IsEnabled` set. It was inspected with an AT-SPI client script on a private bus; **it has not been tried with Orca or any other screen reader, so what is spoken is unknown.**

What a client sees:

- **Chrome.** Toolbar, sidebar, Settings, dialog and notice buttons have accessible names (the resize handles do not). Toolbar tabs, Settings sections, switches (toggle buttons with a pressed state), segmented controls (radio buttons with a checked state) and text fields (with caret, selection and edit events, and `EditableText`) have roles and states. Dialogs have role dialog, are named by their title, move focus to a button or tab inside, drop the window behind them from the tree while open, and close on Escape. The switcher and Commands lists report the highlighted row as the focused item as you move through it (the search field then no longer reports focus itself). Notices (toasts and the update notice) are status regions and raise AT-SPI announcements when they appear while a client is attached. Rex has no password fields.
- **Terminal panes.** A pane is a button named `Terminal <pane name>` holding one panel per visible row and one label per text segment. The labels carry the visible text and update as output arrives (about 0.1 to 0.2 s after a shell command's output was sent, in a test); nothing off screen, from another tab, session or host, or from scrollback is exposed. There is no text interface, caret, selection or terminal role, so a client reads node names rather than lines, words or characters, and each redraw replaces nodes. The pane is not focusable and keyboard focus is reported on the window, not the terminal.
- **Keyboard.** Ordinary keys go to the terminal by design. Without a pointer you can reach the rest through Commands (Ctrl Shift P): it lists Settings, Add host, tabs, panes, sessions and the other chrome actions. Inside Settings and dialogs, Tab, arrow keys and Escape work. No item of the pane menu took focus when it opened and Down was pressed, and the toolbar and sidebar buttons are not focusable. Clients can still press every named button, tab, switch and radio button with the AT-SPI `click` action, and focus Settings controls with `grab_focus`.
- **Cost.** With the bus enabled the tree is built and sent on every redraw, even when no screen reader is attached (about 215 nodes per update, 0.3 to 0.4 ms each, 95th percentile under 0.7 ms). While a full-screen program redrew 20 times a second, the app used about 0.71 CPU cores with the bus enabled and no client of mine attached, about 0.78 while a script walked the whole tree once a second, and about 0.59 with the bus disabled (the pre-bump build: 0.66); three interleaved rounds on a busy machine, so treat the differences as rough. Resident memory rose by about 5 MB. A client walking the tree during a redraw sometimes hit stale nodes (3 to 5 failed reads in 30 walks). The release binary is about 13 MB larger than before the bump; about 5 MB of that is the accessibility feature (51.2 MB without it, 56.1 MB with it).

Known gaps: terminal text interface and terminal role; terminal pane focus; keyboard access to the pane menu and the toolbar and sidebar buttons; the sidebar's session rows (buttons whose names end in "(current)") have no selected state; group labels for segmented controls and setting titles are separate nodes, not associated with their controls; resize handles, the dialog scrim and the window-sized press targets are unnamed nodes; a notice raises four announcement events (its two nested status regions, text and Dismiss button), which a screen reader might read more than once.

## Third-party software and assets

| Component | License |
| --- | --- |
| [anomalyco/rune](https://github.com/anomalyco/rune) (UI toolkit; vendored crates plus `rune-kit` and `rune-theme-opencode` from git) | MIT |
| [a10y/rexrs](https://github.com/a10y/rexrs) (Rex control SDK; vendored) | MIT OR Apache-2.0 |
| [Ghostty](https://github.com/ghostty-org/ghostty) (terminal core and snapshot format, built by `ghostty-vt-sys`) | MIT |
| [`ghostty-vt`](https://crates.io/crates/ghostty-vt) Rust bindings, pinned `=0.8.0` (vendored with patches) | Apache-2.0 |
| Inter and IBM Plex Mono (bundled through `rune-theme-opencode`) | SIL Open Font License 1.1; license texts also appear in `vendor/rune/crates/rune/assets/` |
| Rex app icon (`resources/icons/`) | Superlogical artwork, not covered by this repository's license; see [resources/icons/README.md](resources/icons/README.md) |

Other crates are used under their own licenses; see `Cargo.lock`.

## Acknowledgements

- [Superlogical Rex](https://www.superlogical.com/rex), the server and protocol this client speaks. The implementation follows the server's `/llms.txt` API revision 13 contract; revisions 10–12 are also supported.
- [anomalyco/rune](https://github.com/anomalyco/rune) and [a10y/rexrs](https://github.com/a10y/rexrs).
- [Ghostty](https://github.com/ghostty-org/ghostty) for the terminal core.
- [m4nik1/rex-linux-client](https://github.com/m4nik1/rex-linux-client) and [noahdunnagan/rmx](https://github.com/noahdunnagan/rmx), whose protocol work served as references.

## License

MIT. See [LICENSE](LICENSE). Vendored code keeps its own licenses in `vendor/`.
