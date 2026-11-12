# M3.7 — prompt navigation verified and implemented

Verified 2026-11-11 (PST), from base `c24f8c4`. Version remains **0.7.0**.
This closes the prompt-navigation portion of #44 (d3 T35, d4 D24), not the
separate move-to-new-tab gap.

## Spike before implementation

`parity/w3-a` resolves to `cc8bdad48a91d8d99d10f98d5f70816cb514a0fd`, the
preserved W2 review-fix commit, not a prompt implementation. Searches of the
relevant `~/.cache/agent-ws/rex-parity`, `rex-issue44` and `rex-osc` artifacts
found no reusable prompt spike. Old worktrees were not modified.

A fresh private server was launched with `--restore=false --config /dev/null`,
a private HOME/XDG environment, state directory and Unix socket. No configured
personal server was contacted. The server was stopped by its own process handle
in the acceptance script's `finally` block.

Observed server:

- Rex `0.0.1794418166-g7b4cbc`, commit
  `7b4cbcbc6b67172a4c191ae243cd385b15578924`.
- Built 2026-11-11 09:29:26 PST; negotiated API **13**.
- Ghostty wrapper/sys pin: `0.8.0+ghostty.1.4.0-main.83fb956`.

The fixture starts `/bin/sh` without personal rc files. It emits OSC 133 A/B/C/D
around three prompts with eight output rows each, then waits on a local file
barrier. Only after that barrier is ready does the client attach its data
stream. This ensures the first marks come from a **real server snapshot**, not
from VT bytes fed after attachment. Releasing a second barrier emits three more
prompts over the **live stream**. A fresh data connection then restores all six.

The probe uses the application's `wire::handshake`, `SnapshotBuffer`, APC parser
and `TerminalStream::feed`, splitting received data into 17-byte chunks. It
checks both `Row::semantic_prompt() == Prompt` and the first cell's
`Cell::semantic_content() == Prompt` at every reported mark.

Recorded final successful run (`rex-prompt-pqxfxgo3/contract.log` under
`~/.cache/agent-ws`):

```text
negotiated API 13
snapshot: bytes=1975 marks=[0, 9, 18] scrollback=20
live: bytes=2433 marks=[0, 9, 18, 27, 36, 45] scrollback=48
reattach snapshot: bytes=2724 marks=[0, 9, 18, 27, 36, 45]
worker viewport prompts=[6, 5, 4, 3, 2, 1, 1, 2, 3, 4, 5, 6, 6]; input_events=0; server windows unchanged
```

Rows are zero-based in the terminal's full screen/history. Byte counts are
cumulative per connection and can vary with metadata. Session creation, sizing
and fixture input are explicit test setup; the navigation phase sends none of
those operations. A separate ordinary `HostConnection` exercises the actual
`Command::PromptStep` → data-worker path. Each step waits for a newer rendered
frame and checks its first line, smaller-pane cropping, the zero input-event
counter and unchanged `session.view.windows` (including layout and ownership).

## Implementation and behavior

- `client.prompt.previous` / `client.prompt.next`, **Ctrl Shift Up / Down**,
  and two Commands rows. Defaults remain removable/rebindable.
- The roadmap spellings `pane.prompt_previous` / `pane.prompt_next` are aliases.
  All four spellings are `Remote::Never`; the pinned eight-action allowed set is
  unchanged. PR #57's unsupported diagnostic is removed only for prompt actions.
- The pinned Ghostty C bindings already have
  `GHOSTTY_SCROLL_VIEWPORT_DELTA_PROMPT`. The eight-line vendored Rust addition
  exposes that existing operation as `ScrollViewport::DeltaPrompt`; it adds no
  server request, protocol, ABI, source-pin change or client-side mark index.
- Ghostty navigates prompt groups **relative to the viewport top**, not the PTY
  cursor. Previous/next do not wrap. No matching mark means no movement. A newer
  prompt in the active area clamps there; a final continuation may also return
  to the active area. Wrapped and explicit continuation rows are one group.
- Navigation sends no keys, claims no ownership, changes no server geometry and
  does not animate. It remains local while the host reconnects; input is still
  discarded rather than replayed.
- A pane shorter than the server terminal keeps the top of a scrolled viewport
  visible instead of cropping away the prompt. Active-area cursor following and
  search-match visibility retain their existing rules.

## Reproduce and checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
python3 scripts/prompt-acceptance.py
```

`CARGO_TARGET_DIR` is honoured (this run used
`~/.cache/agent-ws/rex-review-target`). The live script needs `rex`, `cargo` and
the usual build dependencies. It always starts its own server; artifacts and
logs remain in its printed `~/.cache/agent-ws/rex-prompt-*` directory. The
integration test is ignored in ordinary test runs specifically to avoid silently
creating server sessions. The script opts into that test against its own socket.

Checks passed:

- Formatting and clippy with `-D warnings`.
- All-features tests: **661 passed**, three ignored (two existing tests and the
  explicit live contract); the live contract passed separately.
- Five terminal tests cover restored scrollback boundaries, repeated steps,
  manual viewport origin, active-area clamping, no marks, alternate screen,
  explicit/soft-wrapped continuations, split live OSC sequences and resize.
- Headless app tests cover dispatch, aliases, no input/layout requests, removed
  diagnostics, Commands/hints, remote denial, repeats/releases and unbinding.
- `cargo build --locked --bin rex-linux` passed. Live acceptance verifies the
  real worker's rendered frames, not physical X11/Wayland keyboard input or a
  screenshot of the desktop window.

No PR, merge, release, version change, mode-indicator work or pane-move change
was made. Notification-icon and saved-window-size work is left intact.
