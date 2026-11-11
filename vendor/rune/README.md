# Vendored Rune: window-chrome additions

`crates/rune`, `crates/rune-motion`, `crates/rune-staging`, `crates/rune-taffy` and
`crates/procwatch` are [anomalyco/rune](https://github.com/anomalyco/rune) at
`d70929a4dae263d16d2e8088e7bfe53f58f619ad` (MIT, see `LICENSE`), with the changes below.
The only manifest changes are replacing `edition.workspace`/`license.workspace` with their
values and dropping `[lints] workspace = true`, since these crates sit outside Rune's
workspace. `rune-taffy` (the layout store `rune` depends on) is an unpatched snapshot.
`procwatch` is an optional dependency of `rune` (the `frame-profile` feature).

`rune-kit`, `rune-highlights` and `rune-theme-opencode` still come from the same git
revision. The root `Cargo.toml` patches `rune`, `rune-motion` and `rune-staging` to these
paths so the whole dependency graph sees a single `rune`. Nothing outside the vendored
`rune` depends on `rune-taffy` or `procwatch`, so they need no `[patch]` entry.

## Changes

- `Runtime::physical_key_input` is public so Rex's headless tests can inject the native
  physical/logical pair, base character, repeat and release without scripts or private
  runtime access. No dispatch behavior changes. `src/app/physical_input_tests.rs` checks
  base/code/modifier delivery, repeat reporting and release routing through this API.

Rex runs in a borderless window and draws its own chrome: resize bands, a drag region and
the window manager's menu on right-click. It has no minimize or maximize buttons;
`examples/chrome_spike.rs` exercises those. A borderless Linux window cannot minimize,
edge-resize or open the window manager's menu, and its view cannot tell whether it is
maximized. The patch adds the following without changing existing behavior on macOS
(maximizing is `Tx::toggle_maximized`, which Rune has):

- `Cx::window_maximized()`, refreshed on every `Resized`; the view re-runs when it changes.
- `Tx::minimize()` works on borderless windows off macOS (used by the spike only).
- `El::window_resize(WindowEdge)` (winit `drag_resize_window`, started inside the press),
  `WindowEdge`, `TitlebarAction::Resize` and `Cursor::ResizeEdge(WindowEdge)`.
- `Tx::show_window_menu(x, y)` (winit `show_window_menu`; Wayland and Windows only in winit).
- Off macOS, a `window_drag` press waits for 3pt of travel before `drag_window`, so the
  region's double-click (maximize) still reaches Rune: an X11 move grab otherwise takes the
  second press.
- `set_app_icon()` registers a native window icon on Linux, decoding any image the `image`
  dependency reads into unmodified RGBA before startup. All top-level windows opened after
  the call receive it through `WindowOptions`. This supplies X11's `_NET_WM_ICON` for task
  lists that do not resolve desktop-file icons; Wayland still uses the app ID and the
  installed desktop entry. Invalid images return `Err`. macOS and Windows keep Rune's own
  `set_app_icon`.
- `El::no_wrap()` for styled display text without ellipsis or fade. Rex owns terminal line
  breaks, so fractional-DPI width rounding must not wrap a text run onto another row
  (`rich()` turns wrapping on).
- Native dead keys are retained as `Key::Dead` events instead of being dropped, so terminal
  clients can send their physical press and `composing` state before the completed text.
  A completed key carries winit's composed text, so a dead key followed by Space types its
  accent rather than a space. Headless scripts accept
  `{"type":"key","dead":"~","code":"backquote"}` for this path.
- `text::set_app_font_paths()` and `TextSystem::new_with_paths()` register native font files by
  path, so large system fonts stay memory-mapped instead of being copied into process-lifetime
  byte buffers. Symbol and math fallbacks go through the same per-file cache as CJK and script
  fallbacks, so a file the app registered is not loaded twice. The embedded-font API is
  unchanged.

Spike: `examples/chrome_spike.rs` and `scripts/chrome-spike-x11.py`.

Drop this copy once upstream Rune has equivalent APIs.
