# Cross-platform Rex: how far from a macOS and a Windows source build

Exploratory study, written 2026-11-12 (PST) against `master` at `20f64b4` (Rex Linux 0.7.5, Rune at `d70929a`, `ghostty-vt` 0.8.0 on Ghostty `83fb956`). Status: **findings and a proposed order of work; nothing here is implemented.** Signing, notarization and publishing are out of scope.

The question: Rex ships for Linux only. Rune is built for cross-platform apps. What would it take for someone to clone the repository on macOS, then on Windows, follow documented steps, run `cargo build`, and get a client that connects to a Rex server and runs terminal sessions?

## Summary

- **macOS is close.** With 23 added lines in four files (a scratch patch, not committed), `cargo build --locked` produced a binary on an Apple Silicon Mac. That binary passed all three protocol probes against a private Rex server, opened a real window on a live session, took typed input, and attached to a Linux server over `ssh://`. What remains for a usable build is small Rex work: the compile gates, the default local socket path, two `/proc` lookups that misfire when two copies run, and build documentation.
- **Windows is a porting project, not a blocker hunt.** Every dependency, including the vendored Rune with Rex's patches, type-checks for `x86_64-pc-windows-msvc`. Rex's own library has 54 errors in five files, all Unix system calls. With those five files stubbed the UI binary has two more causes, both shared with macOS. The real work is porting the download store, the paths, and the transports that assume a Unix socket.
- **Rune is not the blocker on either platform.** It has macOS and Windows back ends for windows, GPU, text, clipboard, IME, accessibility and folders. The gaps are Rex code that calls Linux directly.
- **Biggest blocker on macOS:** none that stops a build. The largest remaining piece of work is window chrome and Mac keyboard conventions, which decide whether the result feels like a Mac app.
- **Biggest blocker on Windows:** two things could not be verified without a Windows machine. The first is whether the pinned Ghostty VT library builds with Zig for MSVC. The second is `ssh://` hosts, which forward to a local Unix socket that tokio does not offer on Windows.

Sizes used below: **S** is half a day or less, **M** is one to three days, **L** is a week or more.

## What was run

| Where | What | Result |
| --- | --- | --- |
| shuvbot (macOS 27.3, arm64, Xcode 27.2 beta) | `cargo check --locked`, default toolchain 1.95 | Refused: the crate needs Rust 1.96. Ran the rest with the installed 1.96.1. |
| shuvbot | `cargo check --locked --no-default-features` (library and probes) | 4 errors in the library, [listed below](#errors-on-macos). Zig built `libghostty-vt` for `aarch64-macos` without trouble. |
| shuvbot | Same, after a scratch patch for those 4 | Passes. |
| shuvbot | `cargo check --locked` (the app) | 3 errors, one cause: the primary selection. |
| shuvbot | `cargo check --locked`, then `cargo build --locked`, after patching that too | Both pass. From cold, checking took 1 min 48 s for the library and its dependencies and about 1 min more for the app's; the build took 1 min 29 s. |
| shuvbot | `rex-probe`, `rex-workspace-probe`, `rex-download-probe` against a private server (`rex 0.1.0-preview.17`, API 13) on a private socket | All pass: 4, 10 and 17 checks. |
| shuvbot | `rex-linux -- --snap` (headless), then a real window for a few seconds | Renders a live session on Metal. The window opened at the requested size with the session's title. Typed text reached the PTY and Ctrl+K opened the session switcher (scripted, headless). |
| shuvbot to shuvdev | `rex-probe`, `rex-workspace-probe` and the app over `ssh://` to a private Linux server (`0.0.1794418166-g7b4cbc`) | All pass; the app showed the Linux shell. |
| shuvbot | `cargo test --locked --all-features` | Library: 315 pass, 12 fail, 1 hangs. App: 632 pass, 7 fail, 2 ignored. Integration tests pass. [Causes below](#tests-on-macos). |
| shuvdev (Linux), `cargo-xwin` 0.23.1 | `cargo xwin check --locked --target x86_64-pc-windows-msvc` | Stops in the `ghostty-vt-sys` build script: Zig cannot build for MSVC from Linux without Microsoft's C headers. A limit of cross-building, not a product error. |
| shuvdev | Same with `DOCS_RS=1`, which skips the Zig step and uses the committed bindings | 54 errors, all in Rex's library. Every dependency checks. |
| shuvdev | Same, in a scratch copy with the five failing files stubbed | Library passes. The app has 5 errors of 2 causes. With those stubbed too, the app and three probes check clean. |

Private servers used their own socket, state folder and `HOME`. No user server was touched. The scratch clone, build output, temporary files and both private servers were removed afterwards.

Not verified, and called out again where it matters:

- Nothing was linked or run on Windows. No Windows machine was available.
- The Zig build of `libghostty-vt` for MSVC.
- Real keyboard input in the macOS window (only scripted input through Rune's headless driver), and resizing the window by its edges with the pointer.
- Intel Macs.

## 1. What the client needs

### No local PTY

Sessions and their programs run on the Rex server. The client holds no pseudo-terminal:

- There is no `openpty`, `forkpty`, `portable-pty` or ConPTY call in `src/` or `vendor/rexrs/src/`, and none of those crates is in `Cargo.lock`.
- Terminal bytes arrive over a WebSocket and are decoded by `ghostty-vt` (`src/terminal.rs`). A resize is a request to the server (`com.superlogical.terminal.resize`, `src/connection.rs:2201`).
- Keys go out as `client.key` payloads (`src/app/keys.rs`).

So **ConPTY is not needed on Windows**, and nothing on the client depends on a Unix terminal layer.

### What it does need from the platform

A window and GPU surface, text shaping and system fonts, keyboard and IME, the clipboard, a way to reach a server, a few per-user folders, and optional desktop services (notifications, opening links and folders).

## 2. What Rune supports

Rune describes itself as "a native GPU-rendered Rust UI framework for macOS, Windows, Linux and the web" (upstream `README.md` at the pinned revision). The vendored copy backs that up:

| Area | macOS | Windows | Evidence in `vendor/rune/crates/rune/` |
| --- | --- | --- | --- |
| Windowing | winit 0.30 with AppKit | winit 0.30 with Win32 | `Cargo.toml:72`; `src/platform/macos/` (19 files), `src/platform/win32/` (7 files) |
| GPU | Rune's own Metal back end, or wgpu Metal | wgpu Direct3D 12 and Vulkan, with precompiled DXBC shaders | `Cargo.toml:51,70,193`; `uber.fs.dxbc`, `src/render/dxbc.rs` |
| Text | Parley and swash, with Core Text matching | Same, with DirectWrite rasterizing and ClearType | `Cargo.toml:77`; `src/text/macos/`, `src/text/windows/` |
| System fonts | fontique on Core Text | fontique on DirectWrite | `Cargo.toml:109` |
| Clipboard | arboard and `NSPasteboard` | arboard | `Cargo.toml:112-116`; `src/clipboard/macos.rs`, `native.rs` |
| IME and keys | winit; `Mods::cmd` is the Command key | winit; `Mods::cmd` is Ctrl; AltGr handled | `README.md:296-323` |
| Custom title bar | Traffic lights over the app's own bar | Caption buttons, resize edges, Aero Snap | `src/window/titlebar.rs:32`, `README.md:1036-1049` |
| Accessibility | AccessKit on NSAccessibility | AccessKit on UI Automation | `Cargo.toml:56,145,180` |
| Per-user folders | `~/Library/Application Support/<id>` and friends | `%APPDATA%\<id>`, `%LOCALAPPDATA%\<id>` | `src/dirs.rs:10-20` |
| Open a link | `NSWorkspace` | `ShellExecuteW` | `src/open_url.rs:9` |
| File dialogs, tray, spelling | Yes | Yes | `Cargo.toml:127-128`; `src/tray/win32.rs`, `src/spell/` |

Maturity, as far as it can be read from the source:

- **macOS is Rune's first platform.** Its only CI workflow runs the test suite on `macos-latest` (`.github/workflows/macos-tests.yml` upstream). The crate has 808 `target_os = "macos"` gates, against 250 for Windows and 107 for Linux.
- **Windows is implemented but has no upstream CI.** There is a UI Automation test (`tests/accessibility_windows.rs`) and about 2,100 lines of Win32 and DirectWrite code. Nothing upstream builds it on every change.
- **Here:** the vendored Rune with Rex's window-chrome patches compiled and ran on macOS, and type-checked for Windows with no errors.

What Rune does not offer, so Rex must supply per platform:

- **Desktop notifications.** There is no notification module.
- **Showing a file in the file manager.** `Tx::open_url` takes only `http:`, `https:` and `mailto:` links.
- **The primary selection.** That is an X11 and Wayland idea; macOS and Windows have none.
- **A public list of installed font families.** `installed_family_files` is private to the text module (`src/text/system_fonts.rs:150`).

## 3. Where Rex is tied to Linux

The source has almost no platform gates today: one `#[cfg(unix)]` (`src/ssh.rs:278`) and one `cfg!(target_os = "macos")` (`src/app/keys.rs:93`). Everything else assumes Linux. There is no GTK, GLib or gio anywhere, and no single-instance lock (the only file lock is the updater's).

"Compiles" below means it builds and fails or degrades at run time.

| Area | Where | macOS | Windows |
| --- | --- | --- | --- |
| Unix socket transport | `src/transport.rs:27` | Works | No `UnixStream` in tokio: compile error |
| Default local endpoint | `src/transport.rs:18-23` (`$XDG_RUNTIME_DIR/rex/server.sock`) | Wrong path. The Mac server listens at `~/Library/Application Support/rex/server.sock`, so the default host shows offline | Wrong. The Windows server listens on a named pipe |
| TCP user timeout | `src/transport.rs:51` | Linux-only socket option: compile error | Same |
| SSH tunnel | `src/ssh.rs:298-315` (`ssh -L local.sock:remote.sock`), `:324` | Works (checked live) | Needs a local Unix socket: compile error, and a design question |
| Is that process alive? | `src/ssh.rs:220` (`/proc/<pid>`) | Always "no". A second copy deletes the first copy's tunnel sockets | Same |
| User id | `src/ssh.rs:224-232` | Falls back to `user`; harmless | Compile error |
| Download store | `src/download/store.rs` (`openat`, `linkat`, `fstatvfs`, `fchmod`, `O_NOFOLLOW`) | Works except two calls below | 25 compile errors: the whole file is POSIX |
| Rename without replacing | `src/download/store.rs:295` (`renameat2`) | Linux-only: compile error. The `linkat` fallback beside it works | As above |
| File mode type | `src/download/store.rs:234` | `mode_t` is 16-bit: compile error | As above |
| Who owns a partial download | `src/download/store.rs:438` (`/proc/<pid>/stat`) | Never knows. A second copy deletes the first copy's downloads in progress (two tests fail on this) | Same |
| Download file names | `src/download/name.rs:75` | Fine | Does not reject `: * ? " < > \|` or device names such as `CON` and `NUL` |
| Random ids | `src/upload.rs:66` (`/dev/urandom`) | Works | Fails at run time: no uploads or downloads |
| Settings path | `src/settings.rs:302-311` (`$XDG_CONFIG_HOME`, else `$HOME/.config`) | Works, in `~/.config/rex-linux` | `HOME` is unset, so the path is relative to the working folder |
| State and data paths | `src/update.rs:1129-1145` | Works, in `~/.local` | Same problem as settings |
| Ghostty theme folders | `src/terminal_theme.rs:251-267` | Finds `~/.config/ghostty`; misses the app bundle's themes | Finds nothing |
| Default download folder | `src/download/store.rs:354-380` (`user-dirs.dirs`, else `~/Downloads`) | Works | Needs `USERPROFILE` |
| Updater: lock | `src/update.rs:1116` (`flock`) | Works | Compile error. `std::fs::File::try_lock` replaces it on every platform |
| Updater: installer | `src/update/installer.rs` (receipt, `~/.local` layout, `desktop-file-install` at `:537`, glibc check at `:307`) | glibc call is a compile error. Otherwise inert: a source build is channel `Source` and never installs | 16 compile errors; inert for the same reason |
| Restart after update | `src/main.rs:53-54` (`exec`) | Works | Compile error |
| Fonts | `src/app/fonts.rs:61` (`fc-match`), `:146` (`fc-list`) | Works only if Homebrew's fontconfig is on `PATH`, which an app started from Finder does not have. Without it, only the bundled IBM Plex Mono and Inter | No fontconfig: bundled fonts only |
| Notifications | `src/app/notify.rs:181,294` (D-Bus) | Compiles. No session bus, so Settings reports the service unavailable | Same |
| Show in folder | `src/app/downloads.rs:747` (`xdg-open`), `:767-781` (`FileManager1`) | Compiles, then fails | Same |
| Open a link | `src/app/workspace.rs:822` (`xdg-open`) | Compiles, then fails | Same |
| Primary selection | `src/app/select_copy.rs:134` (arboard's Linux extension) | Compile error | Compile error |
| Clipboard and OSC 52 | Rune's `Tx::copy` and `read_clipboard` | Portable | Portable |
| IME | `src/app/input.rs:381` on Rune's `on_ime` | Portable | Portable |
| Window chrome | `src/app/mod.rs:316` (`borderless`), `:69-84` (resize bands), `src/app/toolbar.rs:478-479` | Opens with no traffic lights. winit does not support edge-drag resizing on macOS | winit supports both calls; not run |
| Shortcuts | `src/app/bindings.rs:68,137` (Command folded into Ctrl) | Defaults work with the Control key. ⌘K arrives as `ctrl+cmd+k` and matches nothing | Fine: Rune's `cmd` is Ctrl there |
| Tailnet discovery | `src/tailnet.rs:175` (`tailscale status --json`) | The Mac app's CLI is not on `PATH` by default | Works if `tailscale.exe` is on `PATH` |
| Local `rex` helper | `src/ssh/setup.rs:141-153`, `src/app/integrations.rs:284-292` (`~/.local/bin/rex`) | The Mac app keeps `rex` inside its bundle | Not looked for in the right place |
| About and debug info | `src/app/about.rs:202-208` (display server, desktop), `src/update.rs:462` (`/etc/os-release`) | Says X11 or Wayland wrongly | Same |
| Build date | `build.rs:28-35` (`date --date`) | Only with `SOURCE_DATE_EPOCH`, which BSD `date` rejects; falls back to "unknown" | No `date` program; "unknown" |
| Names | `Cargo.toml:2,57` (`rex-linux`), `src/app/mod.rs:280` (`dev.shuv.rex-linux`) | The menu bar reads "rex-linux" | A decision, not a defect |

Two Linux-flavoured dependencies need no change: arboard's `wayland-data-control` feature (`Cargo.toml:15`) and `zbus` (`Cargo.toml:46`) both compile on macOS and for Windows. They are unused there.

## 4. macOS in detail

### Errors on macOS

Eight compile errors, six causes. All are in Rex.

Library (`--no-default-features`):

1. `src/download/store.rs:296`: `libc::SYS_renameat2` does not exist. Linux-only system call.
2. `src/download/store.rs:234`: `fchmod` wants `mode_t`, which is `u16` on macOS.
3. `src/update/installer.rs:307`: `libc::gnu_get_libc_version` does not exist. glibc only.
4. `src/transport.rs:51`: `set_tcp_user_timeout` does not exist. `TCP_USER_TIMEOUT` is Linux-only.

App:

5. `src/app/select_copy.rs:134,172,184` (three errors): arboard's `LinuxClipboardKind` and its `Get`/`Set` extensions are gated out.

Tests:

6. `src/transport.rs:287`: the test for item 4 reads the same option.

The scratch patch gated items 1, 3, 4 and 6 to Linux, cast item 2, and gave item 5 a stand-in that reports "no primary selection". It is 23 added lines and 2 removed.

Build requirements met along the way:

- **Rust 1.96.** The Mac's default toolchain was 1.95.
- **Zig 0.17.x.** Homebrew's `zig` formula is 0.17.1. The Mac had `zig@0.15` linked, so a 0.17.1 tarball in the scratch folder was used through `PATH`.
- **Xcode command-line tools.** Already present.
- Nothing else. No Homebrew library was needed to build.

### What worked at run time

- All three probes, against the Mac server and (two of them) through `ssh://` to a Linux server.
- Rendering through Rune's Metal back end, headless and in a window.
- A session over a Unix socket, and over an SSH tunnel made by the system's OpenSSH 10.5.
- Scripted typing into the terminal, and the Ctrl+K session switcher.
- `--check-update`: "Rex Linux 0.7.5 is up to date (source)". A source build never offers to install.
- A file download end to end, including the symlink refusal and low-space checks.

### What did not

- **The default host is offline**, because the default endpoint is the Linux runtime path.
- **Prompt glyphs drew as boxes** when fontconfig was not on `PATH`: the saved Nerd Font could not be found and IBM Plex Mono has no such glyphs. With `/opt/homebrew/bin` on `PATH`, `--font Menlo` resolved. `SF Mono` is not visible to fontconfig at all.
- **The window has no close, minimize or zoom buttons** (the accessibility API reports none). Rex's own × and drag region are drawn, and the accessibility API could resize the window. Nothing was clicked or dragged by hand. Rex's resize bands call `drag_resize_window`, which winit answers with "not supported" on macOS, so resizing by an edge may not work.
- **Notifications** report unavailable, and "Show in folder" and opening links fail, as the table says.

### Tests on macOS

| Cause | Failing tests |
| --- | --- |
| The Linux installer: `install.sh` receipt, `~/.local` layout, glibc, `bash scripts/install-desktop.sh` | 7 in `update::installer::tests`, and one more that hangs |
| `/proc` for process identity | 3 in `download::tests`, 1 in `app::tests::downloads_tests` |
| Fixtures that assume a short temporary path (macOS's is 49 characters and is shortened for display) | 5 in `app::tests::downloads_tests` |
| A test selects all with Ctrl+A; Rune's text fields take ⌘A on macOS | 1 in `app::tests::directory_tests` |
| A connection to a closed loopback port timed out where the test expects a refusal (possibly this Mac's firewall) | 1 in `tailnet::tests` |
| Not investigated | `connection::tests::server_info_tests::identity_is_read_once_per_connection_and_a_refusal_is_quiet` |

The other 947 library and app tests pass, including every headless UI test outside downloads and directories.

### Work for a usable macOS source build

In dependency order. All of it is Rex code.

| # | Item | Size | Needed for "usable"? |
| --- | --- | --- | --- |
| M1 | Compile gates: the six causes above | S | Yes. Everything else follows |
| M2 | Default local endpoint per platform (`~/Library/Application Support/rex/server.sock` on macOS) | S | Yes |
| M3 | Process identity without `/proc`: whether a pid is alive, and its start time, for the tunnel sweep and the download records | S to M | Yes. Otherwise a second window destroys the first one's tunnels and downloads |
| M4 | Build documentation: Rust 1.96, Zig 0.17.x, Xcode tools, `brew install fontconfig` as the interim font step, and how to name the socket | S | Yes |
| M5 | CI: a macOS job that checks and builds (section 7) | S | Yes, or it will break again within a week |
| M6 | Tests: gate the installer tests to Linux, fix the path fixtures | S to M | For CI to run tests, not for the build |
| M7 | Fonts without fontconfig: list and resolve families through fontique (Core Text). Either a direct dependency or a small Rune patch to expose its lookup | M | No. Bundled fonts and the Homebrew route work. Needed before anyone starts it from Finder |
| M8 | Window chrome: a Rune `Titlebar` with traffic lights on macOS in place of the borderless window | M | Probably. Depends on whether edge resizing works today, which needs a hands-on check |
| M9 | Mac shortcuts: stop folding Command into Ctrl on macOS, add ⌘ defaults (⌘T, ⌘W, ⌘C, ⌘V, ⌘K), decide what Option sends | M | No. Control-key defaults work |
| M10 | Open links and "Show in folder" through `open` and `open -R` | S | No |
| M11 | Notifications | S for a basic one, M for click-through | No. A proper one needs an app bundle, which is packaging |
| M12 | Finding `tailscale` and `rex` inside their Mac app bundles | S | No |

**M1 to M5 are the minimum: about two to three days.** M6 to M9 make it something a Mac user would keep open: about another week.

## 5. Windows in detail

### Errors on Windows

The first failure is not in Rex. `ghostty-vt-sys` runs `zig build -Dtarget=x86_64-windows-msvc`, and from Linux Zig has no MSVC C headers (`'stdlib.h' not found`, then C++ errors in simdutf and highway). Pointing Zig at the headers `cargo-xwin` downloads moved the failure to the host tools the same build compiles, because one libc setting applies to both. This says nothing about a real Windows machine, where Zig finds Visual Studio itself.

With that step skipped, **54 errors, all in Rex's library**:

| File | Errors | Cause |
| --- | --- | --- |
| `src/download/store.rs` | 25 | `std::os::fd`, `std::os::unix`, `openat`, `unlinkat`, `linkat`, `fstatat`, `fstatvfs`, `fchmod`, `renameat2`, `O_NOFOLLOW`, `O_DIRECTORY`, `O_CLOEXEC`, `geteuid` |
| `src/update/installer.rs` | 16 | Unix file metadata (`dev`, `ino`, `uid`, `mode`, `mtime`), `Permissions::from_mode`, `geteuid`, glibc version |
| `src/update.rs` | 8 | `flock` on a raw descriptor |
| `src/ssh.rs` | 3 | `UnixStream`, Unix metadata |
| `src/transport.rs` | 2 | `UnixStream`, TCP user timeout |

With those five stubbed in a scratch copy:

| File | Errors | Cause |
| --- | --- | --- |
| `src/main.rs:53-54` | 2 | `exec` to restart after an update |
| `src/app/select_copy.rs:134,172,184` | 3 | Primary selection, as on macOS |
| `src/bin/download_probe.rs:159,352` | 2 | Unix permissions and symlinks in the probe |

After stubbing the first two, `rex-linux`, `rex-probe`, `rex-workspace-probe` and `rex-update-probe` check clean. **Rune, rune-kit, winit, wgpu, zbus, arboard, fontique, AccessKit and rexrs needed no change.**

The GNU target is not an option: `ghostty-vt-sys` maps only `windows-msvc` targets to Zig and panics on any other (`build.rs:467-483` of the crate).

### What a native Windows run would still hit

None of this was run. It follows from the code.

- **The Ghostty library.** The crate claims MSVC support and names the `.lib` it expects. Whether the pinned Ghostty commit builds under Zig 0.17 for MSVC is the first thing a Windows CI job will answer. If it does not, this is the hardest item on the list and not ours to fix quickly.
- **Reaching a server.**
  - `https://` and `wss://` endpoints (a tailnet host) are plain TCP with rustls and should work unchanged.
  - A local server listens on "the Rex named pipe" (`rex server run --help`), not a Unix socket. Rex needs a pipe transport and the pipe's name.
  - `ssh://` asks OpenSSH to forward the remote socket to a local Unix socket, then connects to it. tokio has no Unix sockets on Windows, and whether Windows' OpenSSH can listen on one was not checked. The portable alternative is to forward to a loopback TCP port, which any local user can reach, where today's socket sits in a private folder. That is a security decision, not just a port.
- **Paths.** With `HOME` unset, settings, update state and download records resolve to paths relative to the working folder.
- **Random ids.** `/dev/urandom` does not exist, so every upload and download fails at its first step.
- **Downloads.** The store's safety argument rests on directory descriptors and `O_NOFOLLOW`. Windows needs its own version (a directory handle, reparse-point checks, `CREATE_NEW`, a move that does not replace), and the name sanitizer must also reject Windows' reserved characters and device names. Until then downloads should be off on Windows, not half working.
- **Fonts.** No fontconfig, so only the bundled families; Nerd Font glyphs draw as boxes.
- **Notifications.** D-Bus is absent. Toasts need the WinRT API and, for an unpackaged program, a registered application id.
- **Links and folders.** `xdg-open` does not exist.
- **The updater.** Inert in a source build. "Restart to update" has nothing to restart into.
- **Window chrome.** winit supports the resize drag and the window menu on Windows, so Rex's borderless chrome may work as it is. A borderless window loses Aero Snap and the system shadow, which Rune's `Titlebar` keeps.
- **Tests.** Fixtures use `/tmp`, Unix permissions and shell scripts throughout. Expect far more failures than on macOS.

### Work for a usable Windows source build

Ordered by dependency. W1 comes first because it can end the effort early.

| # | Item | Size | Rune or Rex | Needed for "usable"? |
| --- | --- | --- | --- | --- |
| W1 | A Windows CI job that builds `rex-probe` with Zig and MSVC: does `libghostty-vt` build? | S to find out; unknown if it fails | Neither: `ghostty-vt` and Ghostty | Yes. Gate for everything below |
| W2 | Compile gates shared with macOS (M1), plus `exec`, `flock` (use `File::try_lock`), user id, and the installer made Linux-only | S | Rex | Yes |
| W3 | Per-platform folders for settings, state, data and downloads | S to M | Rex | Yes |
| W4 | Random ids from `ring` (already a dependency) in place of `/dev/urandom` | S | Rex | Yes |
| W5 | Unix-socket transport gated out; `https://` and `wss://` hosts work | S | Rex | Yes. This is the first point where it connects |
| W6 | Downloads refused with a clear message on Windows | S | Rex | Yes, as the honest interim |
| W7 | Build documentation: Rust MSVC toolchain, Visual Studio Build Tools, Zig 0.17.x | S | Rex | Yes |
| W8 | Named-pipe transport and default endpoint for a local server | S to M | Rex | Only for a server on the same PC |
| W9 | `ssh://` hosts: choose loopback TCP or a Unix socket, then test on a real machine | M | Rex | For most people, yes: it is the common way to reach a Linux box |
| W10 | Download store for Windows, with Windows name rules | M | Rex | No |
| W11 | Fonts through fontique (DirectWrite); shared with M7 | M | Rex, perhaps a small Rune patch | No, but boxes for prompt glyphs will be the first complaint |
| W12 | Window chrome on Rune's `Titlebar`; shared design with M8 | M | Rex | Unknown until it runs |
| W13 | Links, "Show in folder", notifications | S, S, M | Rex | No |
| W14 | Tests and probes on Windows | M to L | Rex | No |

**W1 to W7 reach a client that attaches to a tailnet host: about a week, if W1 comes back clean.** Adding `ssh://` (W9) and fonts (W11) makes it roughly two weeks. Parity with Linux (W10, W12 to W14) is several weeks more.

## 6. A boundary for the platform code

Today the Linux calls are spread through the files that use them. Gating each call site would work for macOS, where six causes exist, but not for Windows. A boundary in two layers, matching the existing split between the library and the app:

**`src/platform/` in the library** (no UI dependency), one public surface with `linux.rs`, `macos.rs` and `windows.rs` behind it:

| Module | Replaces |
| --- | --- |
| `dirs` | Config, state, data, runtime and download folders: `settings.rs:303`, `update.rs:1129-1145`, `terminal_theme.rs:259-267`, `download/store.rs:341-380`, `ssh.rs:192` |
| `process` | Is a pid alive, when did it start, who is the user: `ssh.rs:220-232`, `download/store.rs:438` |
| `local` | The default local endpoint and how to connect to it (Unix socket or named pipe): `transport.rs:18-28` |
| `fs` | What the download store needs: open a folder without following a link, create a private new file, free space, rename without replacing: `download/store.rs:46-58,131-170,291-323` |
| `random` | `upload.rs:66` |

**`src/app/platform/` in the app:**

| Module | Replaces |
| --- | --- |
| `notify` | The D-Bus back end. The seam exists: `trait Backend` at `src/app/notify/worker.rs:55` |
| `open` | Links, folders and "Show in folder": `workspace.rs:822`, `downloads.rs:747-792` |
| `fonts` | Listing and resolving families: `fonts.rs:54-87,143-155` |
| `selection` | The primary selection: `select_copy.rs:133-210`. Absent off Linux, and its setting hidden there |
| `chrome` | The window options and resize bands: `mod.rs:69-84,315-319` |

The **updater** needs no per-platform back end yet. `update::installer` becomes Linux-only, and other platforms always report the `Source` channel, which is what a source build reports today.

Rules worth keeping:

- A `cfg` gate belongs inside `platform`, not at a call site. The two existing gates are the exception to fold in.
- A feature that is not ported says so in the interface (downloads on Windows, the primary selection off Linux). It does not compile to something that fails quietly.
- The library should not gain a dependency on Rune to get `rune::dirs`. The paths are a few lines each, and the probes must keep building without the UI.

## 7. CI

The repository has no general pull-request CI today. Tests, Clippy and formatting run in `release.yml` on a tag, and two path-filtered workflows cover the updater and the Arch repository. Cross-platform support needs the first one.

A `ci.yml` on pull requests and pushes to `master`:

| Job | Runner | Steps | When to add |
| --- | --- | --- | --- |
| Linux | `ubuntu-24.04` | The checks `release.yml` already runs | With M1 |
| macOS | `macos-15` (arm64) | `cargo check --locked --no-default-features`, `cargo build --locked --bin rex-linux`, then `cargo test --locked --all-features` once M6 lands | With M1 |
| Windows | `windows-2025` | `cargo build --locked --no-default-features --bin rex-probe` first (W1), growing to `cargo build --locked --bin rex-linux` | Before any other Windows work |

Notes:

- The repository is public, so hosted macOS and Windows runners cost nothing.
- Each job needs Rust 1.96.1 and Zig 0.17.1. `mise.toml` already pins both, and `release.yml` already uses `jdx/mise-action`. Whether mise's Zig works on the Windows runner needs checking; `winget` or a setup action is the fallback.
- A cold macOS check and build took a little over four minutes on the test Mac. Hosted runners are slower; cache `target/` and the Zig cache.
- Upstream Rune's own macOS CI renders on the hosted runner's virtual Metal adapter, so a headless `--snap` smoke test is plausible there. Whether Windows runners can render (through WARP) is unknown.
- The Python acceptance scripts drive Xvfb and X11 tools. They stay Linux-only.

## 8. Decisions to make first

1. **Is `ssh://` on Windows worth a loopback TCP port?** It is simpler and works with any OpenSSH. It also lets any local user on a shared PC reach the forwarded server for as long as Rex runs.
2. **Where do settings live on macOS?** `~/.config/rex-linux` works today and matches Linux. `~/Library/Application Support` is what Mac users expect. Moving later means a migration.
3. **The name.** The binary, the crate, the app id and the settings folder all say `rex-linux`. Renaming is cheap now and awkward after other platforms have settings on disk.
4. **Mac shortcuts.** Keep Control-based defaults on every platform, or give macOS Command-based ones as the official Mac client has.
5. **How far to go on Windows before someone can test it.** Everything past W7 is guesswork without a machine or a tester.

## Appendix: reproducing the trial

macOS, from a clone, with Rust 1.96 and Zig 0.17.x on `PATH`:

```sh
cargo check --locked --no-default-features   # 4 errors in the library
cargo check --locked                         # after gating those: 3 errors in select_copy.rs
cargo build --locked                         # after gating that: builds
```

A private server for the probes (the Mac app ships the server binary inside its bundle):

```sh
REX="/Applications/Rex Beta.app/Contents/Helpers/Rex Server.rex/Contents/MacOS/rex"
HOME=$work/home "$REX" --autostart=false server run --listen unix://$work/rex.sock \
    --restore=false --config /dev/null --state-dir $work/state
target/debug/rex-probe unix://$work/rex.sock
```

Windows type-check from Linux:

```sh
cargo install cargo-xwin --locked
DOCS_RS=1 cargo xwin check --locked --keep-going --target x86_64-pc-windows-msvc
```

`DOCS_RS=1` makes `ghostty-vt-sys` skip its Zig build, which is enough to type-check and not enough to link.
