# M2.10 — generated titles (unreleased)

Initially implemented independently on `00c07aa`, then integrated onto `628cb5d` after PR #59, without the broader W2 Terminal/font/settings-navigation work. The setting is a single Titles section on the existing Terminal page, implemented in `src/app/titles.rs`; it does not change the Settings shell or Shortcuts page.

## Behavior

`generated_titles` is a local boolean in `workspace.json`, default **false**, including files written before this setting existed. **Settings → Terminal → Prefer generated titles** saves it immediately and updates the window title as well as the drawn labels.

For a default tab or terminal-pane label:

1. A current title set by a foreground program wins. A title with unknown ownership also stays.
2. When the preference is on, a title set by the terminal's own shell, or no title, yields to the foreground program and directory: `vim - ~/rex`.
3. Without a directory, show the program alone. Without a program, keep the current title or default label.

User labels always win. As before, a name identical to the default (`Shell` for a tab, `shell` for a shell pane, or that window's generated `Window <suffix>`) is indistinguishable from an unrenamed label. Rename fields start from the server label, not the generated display name. The Opus review caught the tab form retaining the display name; the fix and a headless open-form/save-unchanged regression now cover default, explicit and missing labels.

One resolver supplies toolbar/sidebar tabs, pane headers and menus, Commands and search results, notification context, close targets and the window title. Names are display-only: no title/label writes, output changes, ownership claims or layout changes.

The terminal's inherited home is abbreviated only on a directory boundary, never using the client's HOME. Long directories keep their trailing folders after `…/`; Unicode counts as characters. The foreground cwd wins, then its nearest readable ancestor's cwd. A nested shell/script's title is not treated as the terminal's original shell title. macOS `login` ancestry is handled by the parser.

## Metadata and request budget

`process_changed` already supplies foreground process, cwd and ancestors. Existing `terminal.title` reads/events supply title ownership; no OSC-output scanning or `terminal.pwd` request is needed. Metadata is held per block and cleared on removal/reconnect. A known home survives subsequent events that omit it, without being shared across terminals.

The existing once-per-block metadata pass for the **selected session only** now reads `terminal.process` for streamed blocks too: one additional initial read per streamed block per connection, even with the preference off, so toggling is immediate. The read has a **one-second timeout**, does not retry missing/unsupported metadata and does not poll. Hidden/non-streamed blocks already had this read. No new session prefetch or background-session query is introduced. If the read fails, events still supply what they can; a home not reported stays an absolute path.

## Fork adaptation and attribution

Source: [Matt Porter (@ohporter), `e94fe1da4725b80a412c27be6fd01a00c1ec1caa`](https://github.com/ohporter/rex-rune/commit/e94fe1da4725b80a412c27be6fd01a00c1ec1caa), **Name untitled panes by program and directory**, from `linux-enhancements`. Author identity comes from that commit: `Matt Porter <mporter@konsulko.com>`.

Adapted its unnamed-pane program/directory fallback, retained-home behavior and renamed-pane regression into the shared M2.10 resolver preserved in `cc8bdad`. Not a blind cherry-pick: the fork used pane-only `Neovim ~/src` names and a host-wide home cache. M2.10 uses `vim - ~/rex` on both panes and tabs, a preference, shell-title ownership checks and per-terminal homes. Marketing-name aliases and icon/header work are deliberately excluded. See the [harvest record](../fork-harvest-2026-11.md#later-focused-harvest-generated-titles-m210).

## Verification — November 11, 2026 (PST)

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed; existing dependency warnings in vendored Rune remain.
- `cargo test --locked --all-features`: passed, **272 library + 395 app + 7 integration tests**. Three existing ignores remain: Ghostty restored/resized search order, environment-dependent CJK fallback and the private-server OSC 133 contract.
- After integration with #59 and the rename-form review fix, parent verification passed with **693 tests**, three ignored; fmt and clippy passed again.
- Debug client build: passed. Built-client captures on a disposable Rex server, private HOME/config/state/socket: generated `vim - ~/rex` in both pane and tab; sidebar consistency; switch off restores the shell title; program-set title and explicit pane/tab renames win. The server was stopped by its owned PID.
- Rendered and inspected Settings at **1440×1000** and **1024×700**, including the scrolled Titles row, switch, wrapped footnote and narrow-window layout. The existing card/switch tokens are unchanged. Impeccable's mechanical check reported no findings.
- `.wtfm.yaml` manifest validation passed. README link/drift lint retains three pre-existing findings: missing TL;DR, missing tour heading, and the repository's intentional `Co-authored-by` policy text. Do not scrub required contributor attribution to satisfy that generic lint rule. The new evidence/harvest links resolve; linting the historical GAP-REPORT also finds its existing untracked screenshot-archive links, and applying README-only style/visual rules to supporting notes is not a clean docs check. These are reported, not silently rewritten.

### Test matrix

| Case | Evidence |
| --- | --- |
| Shell title vs foreground title; unknown owner | `connection::titles` unit tests, recorded title-event fixture, live Vim captures |
| Shell prompt, nested shell/script, direct program, macOS login | `connection::titles` unit tests (macOS is mock/fixture-only) |
| Default, absent and user-renamed pane/tab labels; preference off/on | `app::activity` matrix and live captures |
| Home, non-home, prefix collision, Unicode, long paths | `connection::titles` unit tests |
| Missing program/cwd/home; ancestor cwd; changed cwd; distinct terminal homes | `connection::titles` unit tests |
| Old settings, boolean serialization, save/relaunch | settings unit tests and `app::tests::titles_tests` |
| Toolbar/sidebar/header/menu/window names agree; no request on toggle or label/layout mutation | `app::tests::titles_tests` |
| Existing selected-session query scope and one-shot metadata | scripted connection test |

Limitations: no fresh full WTFM sandbox/install run, no platform packaging or release validation, and no new X11/Wayland input smoke run. Live capture uses Rune's headless rendered client, not a desktop compositor; macOS ancestry and old-server missing metadata are unit-tested rather than live-tested. This change does not claim parity for W2 typography/settings navigation, PR #59 Shortcuts search, deferred icons/headers or tray behavior.
