# Changelog

## Unreleased

### Added

- **Server version in About:** each host connection now asks its Rex server for its version once with a read-only `server.version` request, and keeps the status the session list already carries (process, sessions, listeners, Tailscale state). Settings → About shows `Rex Server <version>` for the active host with the negotiated API revision and the range the server supports; a server that doesn't answer is shown as not reported and nothing else changes.
- **Copy debug info**, next to **Copy version info** in Settings → About: the version info plus the install channel, display server, desktop, interface theme and zoom, terminal font, and each saved host's kind, connection state, API revision and server version. Neither copy contains a host name, address, session name, client id or credential. Commands gains **About Rex Linux**, **Copy version info** and **Copy debug info**, and About links to the issue tracker.

### Changed

- **About page:** a centred header with the app icon, version, install channel and the active server's version replaces the version rows; commit and build date share one row. Versions are printed one way everywhere (`0.6.0`, no `v` on Settings → Updates).

- **Settings layout:** Settings is a wider dialog (816×680, clamped to the window) whose pages are grouped into headed cards with a divider between every row, one-sentence helpers and controls aligned to one edge. The window palette, terminal theme and notice position are popup buttons whose lists open over Settings beside the control instead of replacing it; font family, client label and each host's default shell are rows that open their form; pane density and tab position are segmented controls with glyphs. Long pages fade at the scrolled edge and show a scroll thumb. Settings and saved preferences are unchanged.

## 0.6.0 - 2026-11-10

### Added

- **In-app update notice and Settings → Updates:** on startup (after 30 seconds) and at most once every 24 hours while running, checks the signed GitHub release manifest (`latest.json` and `latest.json.minisig`) against the embedded CI and backup Minisign public keys (`packaging/keys/release.pub` and `packaging/keys/release-backup.pub`). When a newer stable release is verified, shows a non-blocking update toast with **What's new** (opens the release page in your browser without rendering remote HTML or markdown), **Updates…** and **Dismiss for this version**. Guidance matches the detected install channel (`REX_LINUX_CHANNEL`, `/usr/share/rex-linux/channel` and `~/.local/share/rex-linux/install.json`): `sudo pacman -Syu rex-linux-bin` on Arch (`omarchy update` on Omarchy), the verified `install.sh` one-command upgrade for tarball/script installs, `.deb` upgrade instructions for Debian/Ubuntu, and `nix flake update rex-rune` / `nix profile upgrade rex-linux` for Nix. Includes `rex-linux --check-update`, a **Check now** action, and opt-out via Settings → Updates (`updates.check` in `workspace.json`) or `REX_LINUX_NO_UPDATE_CHECK=1`; source builds never check automatically, background check failures back off for 24 hours silently, and the client never self-updates or downloads binaries.
- **Pane density and padding:** Settings → Interface → **Pane density** adds a **Compact** mode alongside **Comfortable** (default), removing split pane headers, rounded card borders, shadows, gaps and outer margins in favour of 1 pt split separators while keeping split geometry, divider dragging and floating reconnect controls. Settings → Terminal → **Pane padding** adjusts uniform terminal content padding from 0 to 32 pt (default 8 pt) live without restarting. Adapted from [Loric ANDRE (@LoricAndre)](https://github.com/LoricAndre) ([LoricAndre/rex-rune](https://github.com/LoricAndre/rex-rune), `fd4e753`, `e0be986`, `d3a9cbe`).

### Changed

- **Uniform default pane padding:** terminal content now uses uniform 8 pt padding on all four sides in every pane by default (replacing the previous 12 pt horizontal, 8 pt bottom, and 0 pt split-card top padding), so single panes and split cards share identical inset geometry at a given **Pane padding** setting.

## 0.5.3 - 2026-11-10

### Added

- Signed Arch pacman repository at `https://pkg.shuv.dev/arch/$arch`, with x86_64 and aarch64 packages, signed repository databases and package signatures. See the [setup and migration instructions](docs/auto-update-operator-walkthrough.md#5-the-arch-repository-u2).
- Install channel markers and installer receipts; About identifies the installation channel. Nix installation guidance is available in `docs/nix.md`.

## 0.5.2 - 2026-11-10

### Added

- Signed one-command user installer, published as immutable `install.sh` and `install.sh.minisig` release assets. Embedded CI and backup keys verify release checksums with minisign or OpenSSL 3+ before any downloaded code runs. Supports upgrades, checks, explicit version pins and uninstall; refuses automatic downgrades. Repository-key fallback is opt-in only (`--trust-repo-keys`).

### Changed

- Support Rex API revisions **10–13**, including the official Arch snapshot `0.0.1794323808-g757adf`. Refresh the vendored SDK for the additive `terminal.download_begin` method; existing attachment, upload gating and per-host remote-control consent remain unchanged. File-download UI is not enabled. See [API-13 compatibility notes](docs/server-api13.md).

## 0.5.1 - 2026-11-10

### Added

- Settings → About with app version, build commit/date, connected server API revision, repository/release/license links and Copy version info for bug reports.

## 0.5.0 - 2026-11-10

### Added

- Opt-in Remote control per saved host (off even locally), eight view-only client actions, explicit refusals with server-command alternatives, host/target/panel/typing guards, five-second stale-request limits and reconnect-scoped replies. Every performed request names its host and requester in a six-second in-app notice.
- `session.select session_id=… [window_id=…]` shows a session on the requesting host, with acknowledgement, a five-second deadline and history integration. `client.tab.goto index=-1` selects the last tab. Checked live against private preview.14 servers, including a real SSH host.
- A client label in Settings → Remote Hosts, defaulting to the hostname and sent in HELLO; edits apply after reopening.

- Signed releases with minisign signatures for `SHA256SUMS` and `latest.json`, protected release approval, package provenance attestations, and committed CI and offline recovery public keys (#21, #28). No client auto-update is included.

### Changed

- Update GitHub Actions checkout to 7.0.1, upload-artifact to 7.0.2, and mise-action to 5.2.7 (Dependabot #22–#24).

### Verifying downloads

Verify the metadata signatures before checking package hashes. See the [README's minisign public keys and verification instructions](https://github.com/shuv1337/rex-rune/blob/v0.5.0/README.md#verify-a-download). Releases through 0.4.2 remain unsigned.

## 0.4.2 - 2026-11-09

### Added

- Offer a separately confirmed, one-time personal-server start when SSH finds no default Rex server. Preview the target and command, default to Not now, check local/remote API-12 support, report failures without retrying, and reconnect through the unchanged `ssh -L` transport. Ordinary attachment never starts servers; installation remains manual.
- Add host reads API-12 `server.status` listeners, showing the verified endpoint and a review of bound addresses and admission rules. Older servers and unavailable metadata retain attachment without guessed access rules.
- New tabs and splits start in the source pane's shell directory on its server, including shells that do not report OSC 7; a pane-menu split inherits from the pane the menu names. Missing or slow directory reads fall back to the server's default; new sessions are unchanged. (@kitten-lily)

### Fixed

- Clicking a disabled control, divider or padding inside a dialog no longer dismisses it. Empty space preserves field focus; clicking outside still closes the dialog. (@kitten-lily)

## 0.4.1 - 2026-11-09

### Changed

- Support Rex server 0.1.0-preview.14 (Linux build `7e460d0`), negotiating API revisions **10–12**. Refresh the vendored SDK capture and generated types for optional listener metadata in `server.status` and `session.bootstrap`; uploads still require revision 11.
- When SSH finds no Rex socket, explain attachment-only behavior and suggest the manual `rex server install ssh://host` setup. Socket lookup explicitly disables autostart; existing OpenSSH forwarding, aliases and custom sockets remain supported.

### Fixed

- Answer remote `client_perform_requested` actions with `not_supported` and an explanation instead of leaving agents waiting for a timeout. Remote interface control remains unimplemented; local shortcuts and their queued actions are unchanged.

See [preview.14 compatibility notes](docs/server-preview14.md) for the exact protocol diff, test results and deferred features.

## 0.4.0 — 2026-11-09

Changes since 0.3.2, including work adapted from [Joe Goldin](https://github.com/joegoldin) ([joegoldin/rex-rune](https://github.com/joegoldin/rex-rune)), [Lily](https://github.com/kitten-lily) ([kitten-lily/rex-rune](https://github.com/kitten-lily/rex-rune)) and [Andrea Zito (nivox)](https://github.com/nivox). Thank you; their contributions are credited below and in the commits.

### Added

- **Tailnet host discovery:** Add host lists and searches your Tailscale machines, using Tailscale IPs when MagicDNS is unavailable. Online machines are checked for Rex at the addresses the official clients use; connect to a detected server with one press, or enter an SSH login and optional socket path. Manual endpoints and multiple logins to the same machine remain supported.
- **SSH socket forwarding:** connect with `ssh://[user@]host[:port][/socket]` using an existing SSH login, including Tailscale SSH, without reconfiguring Rex. The client discovers the remote Rex socket when no path is given and forwards it with `ssh -L`. Each client instance keeps its own forwards; disconnects show SSH's reason next to Retry.
- **Global navigation and visit history:** Ctrl Tab and Ctrl Shift Tab cycle through tabs across connected hosts and sessions in sidebar order. Mouse Back/Forward and Commands revisit this window's history, including across reconnects, skipping unavailable targets. Rapid cross-session cycling briefly settles before attachment without enabling extra prefetch; Ctrl PageUp/PageDown still use the server's current-session navigation. (@joegoldin)
- **Configurable shortcuts through Rex's `client.keymap`:** bindings in a host's Rex config now apply, layered with Linux defaults and user overrides in `~/.config/rex-linux/shortcuts.json`. Rebind or remove any workspace shortcut, including Ctrl Tab, to let it reach terminal programs. Supports multi-key sequences, layout-aware logical and physical keys, Lua forwarding and server actions, plus explicit `pane.send_key`, tab selection and pane resizing. (@nivox)
- Settings → Shortcuts shows effective bindings, their sources, file paths and Problems; Commands follows rebinding and a toolbar chip shows pending sequences. Override files reload on window focus or opening Settings and are not rewritten by workspace saves. Invalid files leave defaults working, and unavailable server keymaps use local defaults plus overrides. Custom bindings from nivox's format import once without copying defaults or replacing an existing override file. (@nivox)
- **Terminal search:** Ctrl Shift F searches the focused pane's screen and scrollback with Ghostty's search. Matches are highlighted; Enter and Shift Enter step through them, and Escape returns the keyboard to the terminal. Search-field typing and clicks stay out of the running program. (@kitten-lily)
- **Activity, progress and bells:** OSC 9;4 progress appears beneath tab labels and split-pane names, with OSC 7501 percentages as a fallback and words for Paused or Error. Each split pane shows its own activity and, unless renamed, its program's terminal title. An unseen terminal bell marks its tab Bell until viewed. Progress clears with its reporting program but survives while a child holds the foreground. (@kitten-lily)

### Changed

- **Terminal fidelity:** programs can select block, bar or underline cursors; curly, dotted, dashed and double underlines use their requested colours, and overlines are supported. Box-drawing and block characters meet on whole device pixels without seams, and decoration thickness respects fractional display scaling. (@kitten-lily)
- A pane owned by this client waits up to half a second for a frame at its own size before drawing, avoiding a flash of the previous client's dimensions without waiting indefinitely. (@kitten-lily)
- Settings → Terminal identifies saved font changes that need a client restart and font families that are not installed; saving a family change also shows a restart notice.

### Fixed

- SSH connections no longer get stuck reconnecting when SSH config enables `ControlMaster auto`. Forwards stay off shared control masters and keep their login open for the connection's lifetime.
- Silently dropped server connections, such as after laptop sleep, are detected within about 30 seconds using TCP keepalives so hosts and terminals can reconnect. (@kitten-lily)
- Shift Space, Shift Tab and Shift Enter reach Kitty-keyboard programs such as Claude Code as distinct keys. Shift is retained unless the keyboard layout used it to produce the character. Held within-session navigation works again, stale consumed-key state no longer swallows fresh presses, and forwarded terminal input releases focus from UI controls. (@kitten-lily for modified-key handling)
- Search highlights and selected matches stay correctly positioned in clipped or scrolled panes. Closing search during a host reconnect clears its highlights, and a failed search reports a notice instead of dropping the terminal stream. (@kitten-lily)
- Back/Forward retains the right history position after failed or superseded navigation; expired queued visits no longer attach later, and selecting a reconnecting host's session no longer bounces back. (@joegoldin for the navigation foundation)
- Terminal sizes, themes and activity are restored before refreshing shortcuts after reconnect. Local action-request errors no longer disconnect the host, returned actions can finish after pane-focus updates, and changed overrides also reach hosts that were offline.
- The X11 smoke test now holds modifiers through key releases and checks pointer selection at the correct cell boundary. It requires an explicit disposable server and stubs tailnet discovery rather than contacting real tailnet machines.

## 0.3.2 — 2026-11-06

- First prebuilt Linux releases for x86-64 and ARM64: user-installable tarballs and Debian/Ubuntu packages, with SHA-256 checksums.
- Automated native builds and tests on Ubuntu 24.04. No application behavior changes since 0.3.1.

## 0.3.1 — 2026-11-06

- Only Rex API revisions 10 and 11 are supported; servers older than revision 10 are refused with a message to update Rex. The revision-7 and revision-8 compatibility paths and `scripts/revision8-check.py` are gone.

## 0.3.0 — 2026-11-06

Changes since the 0.2.0 public release on 2026-11-01. Many of them come from two forks; thank you to [Joe Goldin](https://github.com/joegoldin) ([joegoldin/rex-rune](https://github.com/joegoldin/rex-rune)) and [Lily](https://github.com/kitten-lily) ([kitten-lily/rex-rune](https://github.com/kitten-lily/rex-rune)), credited on each item below and as authors or co-authors of the commits.

### Terminal

- Drop files on a pane, or paste copied files or a screenshot with Ctrl+Shift+V, and Rex uploads them to the machine that terminal runs on and types their paths, local or remote, with no SSH setup. Needs a Rex server with API revision 11; older servers say so. ([#2](https://github.com/shuv1337/rex-rune/issues/2), requested by @joegoldin)
- The client speaks Rex API revisions 7 to 11, and doesn't draw floating layers another client minimized.
- Inline images with the Kitty graphics protocol, restored when you reattach. Adds a patched copy of `ghostty-vt` under `vendor/`. (@joegoldin)
- Copies made by terminal programs (OSC 52) reach your clipboard, accepted only from the focused pane while the window is focused. Remote programs still can't read your clipboard. (@joegoldin)
- Double-click selects a word or shell path and triple-click a line; a selection stays attached to its text while output scrolls, and dragging past the edge autoscrolls. (@joegoldin)
- Ctrl+click opens web and mail links, including OSC 8 hyperlinks; the tooltip shows the real target. `file://` links are never opened. (@kitten-lily)
- Full-screen programs no longer flicker while they redraw (synchronized output). (@kitten-lily)
- Tabs with a server-generated name show their program's title, as tabs called "Shell" already did. (@joegoldin)
- Terminal themes: Settings → Terminal → Theme sends a theme to the server with Rex's `set_theme`, so programs that ask for colours get the ones on screen. Other clients viewing a terminal see the theme while this client owns it, as they see its size. Choose Rex's default colours, Midnight, any Ghostty theme you have installed, or follow the chrome palette. The default, Follow server, changes nothing. (@kitten-lily)
- The terminal font family and fallback families are saved in Settings → Terminal; only the families you name are loaded. (@joegoldin, @kitten-lily)

### Keyboard

- Dead-key accents are typed into the terminal, including an accent followed by Space. (@kitten-lily)
- Programs can tell Shift+Enter, Shift+Tab and Shift+Backspace from the unshifted keys. (@kitten-lily)
- Key releases are sent correctly when nothing in the window has focus. (@kitten-lily, @joegoldin)
- Alt+arrows reach terminal programs (word movement in shells and editors); switching panes moved to Ctrl+Alt+arrows. (@joegoldin)
- Ctrl+Shift+V pastes only into the pane that asked, even if focus moves before the clipboard read returns.

### Window and navigation

- Chrome palettes, switchable live in Settings → Interface → Theme: Midnight (the default, unchanged), Rosé Pine, Rosé Pine Moon and Gruvbox Dark. (@joegoldin, @kitten-lily)
- Program activity on tabs (OSC 7501: "Needs approval", "Error", "Done" and so on), program icons, and desktop notifications for tabs you aren't looking at.
- Settings in sections (Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts), including adding, editing and removing hosts.
- A single pane draws edge to edge; tabs and toolbar restyled.
- Tabs in the sidebar: Settings → Interface → Tab position → Sidebar lists each session's tabs under it while the sidebar is open. (@joegoldin)
- The sidebar opens at launch as you left it. (@kitten-lily)
- A Settings button in the toolbar; the toolbar's buttons share one plain hover style, and the minimize and maximize buttons are gone.
- Edit a host (name, endpoint, default shell) from its sidebar menu, from Commands or from an offline session's menu.
- Rename and close sessions in place in the switcher and from the session chip.
- The pane menu opens on right-click even over programs that track the mouse.

### Integrations and packaging

- Settings → Integrations installs, updates or removes Rex's skill and session hooks for Claude Code, Codex, OpenCode and Pi with the local `rex skill`, always previewed first.
- OMP (oh-my-pi) in Settings → Integrations. It shares Pi's skill directory, so it uses Rex's Pi setup until Rex knows OMP. (@kitten-lily)
- `rex` is also found in `~/.local/bin` (or `$XDG_BIN_HOME`), not only on `PATH`. (@kitten-lily)
- A Nix flake with pinned packages and desktop integration; see [docs/nix.md](docs/nix.md). (@joegoldin)
- Optional mise tasks, and `scripts/install-desktop.sh --copy` / `--uninstall`. (@kitten-lily)
