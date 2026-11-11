# Contributor and agent guide

Rex Linux is a native Linux client for Rex terminal servers, written in Rust on Rune. See [README.md](README.md) for what it does and how to run it, [PRODUCT.md](PRODUCT.md) for product principles and [DESIGN.md](DESIGN.md) for the design system.

## Build and test

```sh
cargo build --locked --release --bin rex-linux          # the app
cargo test --locked --all-features                      # unit and headless UI tests
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings

# Protocol-only binaries (no UI libraries needed):
cargo run --locked --no-default-features --bin rex-probe
cargo run --locked --no-default-features --bin rex-workspace-probe

# Vendored SDK tests:
cargo test --manifest-path vendor/rexrs/Cargo.toml --all-features
```

Requirements are listed in the README (Rust 1.96+, Zig 0.17.x for `ghostty-vt`, Linux graphics and input libraries). `Cargo.lock` is committed; keep `--locked` working.

Some probes connect to the configured Rex server and create sessions there. Of the Python scripts in `scripts/`, `x11-smoke.py`, `rune-acceptance-x11.py` and `wayland-seat-smoke.py` require `REX_SERVER` to name a disposable server and exit without it; the rest start their own. Every script that launches the client honours `REX_BIN`. Run them only against a disposable server or socket you control, and check each script's requirements in its docstring. `rex-download-probe <unix endpoint> <work dir>` creates a session on the server it is given and downloads real files from it; it refuses the default per-user socket path. Scripts that move the pointer over the terminal grid pin `pane_padding` in their private `workspace.json` and compute positions from it (`grid_origin` in `rune-acceptance-x11.py`), so a changed default does not break them. Anything that exercises the UI against real input (`x11-smoke.py`, `rune-acceptance-x11.py`, `wayland-seat-smoke.py`) needs a built binary and the tools it names.

## Code layout

- Protocol and model library (`src/lib.rs`, no UI dependency):
  - `transport.rs`, `wire.rs`: Unix/HTTPS WebSockets, data tickets, snapshot framing.
  - `connection.rs` (and `connection/`): one control connection per host, block-keyed terminal streams, the `Options` CLI definition.
  - `terminal.rs`: Ghostty decoding and render frames. `terminal/search.rs`: searching the screen and scrollback with Ghostty's search. `links.rs`: OSC 8 hyperlinks and text URLs in a frame.
  - `layout.rs`: split placement. `settings.rs`: local configuration.
  - `download.rs` and `download/`: the file download core for API 13 (`name.rs` sanitizes hostile names, `store.rs` the destination folder, partial files and crash records, `stream.rs` the data stream, `open.rs` the "never Open" list); driven by `HostCommand::Download` in `connection/downloads.rs`. No UI yet.
  - `ssh.rs`: `ssh://` endpoints, a Rex socket forwarded with `ssh -L`. `tailnet.rs`: the machines in `tailscale status --json` and the check for a Rex server on each.
- `src/main.rs` and `src/app/`: the Rune front end (behind the default `ui` feature).
  - `workspace.rs`: workspace state and behavior (hosts, sessions, tabs, panes, requests, preferences, notices). Host and terminal workers wake the UI through `rune::AppHandle::update`.
  - `panel.rs`: popup and dialog state and the switcher, Commands and pane-menu lists. `discover.rs`: the Add host list of the tailnet's machines.
  - `input.rs`, `keys.rs`: workspace shortcuts first, then every other key to the focused pane's PTY; Rune key events to Rex `client.key` payloads; IME.
  - `term.rs`: one pane's view state and cell grid. Rows are text segments placed at `column × cell width`; a character joins a segment only if it advances exactly one cell; ligatures and kerning are off; rows are memoized by content.
  - `sprites.rs`: box-drawing and block element characters drawn as shapes on whole device pixels, as Ghostty does, instead of font glyphs.
  - `toolbar.rs`, `sidebar.rs`, `panes.rs`, `popups.rs`, `dialogs.rs`, `widgets.rs`: views on rune-kit, with design tokens pinned onto the kit in `theme.rs`.
  - `tests.rs`: headless UI tests.
- `src/bin/`: `rex-probe` and `rex-workspace-probe`, live protocol probes.
- `examples/`: `chrome_spike` (borderless window chrome).
- `scripts/`: integration and acceptance checks, and the desktop installer. `tools/`: small helpers for test scripts.
- `resources/`: desktop entry and icons.
- `vendor/`: patched copies of Rune, rexrs and ghostty-vt.

## Conventions

- Match the surrounding style; run `cargo fmt` and keep clippy clean with `-D warnings`.
- Terminal content, colours and split geometry are server-owned. The client may *declare* a terminal theme through Rex's `terminal.set_theme`, which the server applies when this client owns the block or no client does, and otherwise defers until this client becomes the owner (the same rule as resize). The client never recolours terminal output locally or invents layout.
- Ordinary attachment must never start, stop or reconfigure a Rex server, and must never create or destroy a session. Creation is always an explicit user action.
- Failed input is never replayed automatically; a transport failure pauses affected panes.
- Effective workspace shortcuts consume their keys, but every binding is removable or replaceable. Keep ordinary keys flowing through `client.key`, including releases only when their press was sent. Only explicit `pane.send_key` uses `block.event` to bypass Lua bindings.
- Remote availability is decided exhaustively in `Spec::remote()`, default deny. Consent belongs to the saved host, never the requester; replies must stay on their connection epoch. Keep the allowed-set test pinned.
- Terminal content never animates; chrome motion uses the springs in `theme.rs`.
- UI changes should follow [DESIGN.md](DESIGN.md). Prefer verifying them in a rendered build (for example with Rune's `-- --snap out.png`) rather than by reading code.
- Add tests for protocol and state-machine changes; headless UI tests in `src/app/tests.rs` cover focus and dialog behavior.
- Keep the README's claims honest: don't document features that aren't implemented or checked.

## Vendored patches

`vendor/rune`, `vendor/rexrs` and `vendor/ghostty-vt` are upstream snapshots plus small, documented patches, wired in through `[patch]` in `Cargo.toml`. Rules:

- Keep each patch minimal and describe it in the vendor directory's README (`vendor/rune/README.md`, `vendor/rexrs/REX-LINUX.md`, `vendor/ghostty-vt/REX-LINUX.md`).
- Do not mix unrelated edits into vendored code; changes that could go upstream should.
- Preserve upstream license files and attribution. When bumping a pinned revision, update `Cargo.toml`, the vendored copy and its README together, then re-run the tests above.
- The Nix package (`nix/package.nix`) pins fetched sources by hash. A Rune bump also means updating the `rune-kit` hash in `cargoLock.outputHashes`; a `ghostty-vt` bump also means updating the Ghostty source archive and the Zig dependency hash there.
- Remove a patch once upstream provides an equivalent.
- Regenerate rexrs types with `python3 codegen/generate.py` (from `vendor/rexrs`) rather than editing generated files by hand.
