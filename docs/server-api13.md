# Rex API 13 compatibility

Verified on Linux x86-64 on **2026-11-10 (PST)**, based on Rex Linux 0.5.1.
The client now advertises **API revisions 10–13**. Uploads still require 11;
no download UI, new remote permissions or server lifecycle behavior was added.

**Important correction to the original compatibility concern:** this snapshot
supports older negotiated revisions. The installed Rex Linux **0.5.0 actually
connects** to it, rather than being refused. Direct HELLO checks with ranges
10–12 and 10–13 receive WELCOME revision 12 and 13 respectively. This update
adds explicit revision-13 support; the tested snapshot does not strand 0.5.0.

## Server provenance and isolation

- Official Arch snapshot package:
  `rex-0.0.1794323808_g757adf-1-x86_64.pkg.tar.zst`, already cached and verified
  by the preceding server-update task. Snapshot manifest:
  <https://blob.superlogical.com/api/releases/rex/snapshot>.
- Version `0.0.1794323808-g757adf`, commit
  `757adf99fad394c515be6313aa5dbe17008dc7d2`, API 13.
- Published **2026-11-10 07:32 PST**; binary build timestamp
  **2026-11-10 07:16:48 PST**.
- Package SHA-256:
  `faf3de00c773759d76fd2ecfadb41822ff5ee7def3d3283a4d3248680fe88edb`.
- Extracted executable SHA-256:
  `0436f853fb12f3790e1054e0018d652a0fe9f19a66495741e31ede0de211d60a`;
  identical to `/usr/bin/rex` on disk. Only `usr/bin/rex` was extracted; no
  installer, package hook, service installation or package upgrade was run.
- API-12 comparison: cached preview.14 binary, commit
  `7e460d02f51573990bfcdc6c44a5606d11c7d47a`, with provenance recorded in
  [the previous verification](server-preview14.md).
- Both foreground servers ran under `~/.cache/agent-ws/rex-api13/`, each with
  its own socket, `REX_SERVER`, HOME, XDG directories, empty Lua config and state
  directory, with `--restore=false`. CLI calls explicitly disabled autostart.
- Keymap and remote-control acceptance scripts created additional private
  servers using the selected binary through a scratch-only PATH. UI checks used
  private Xvfb displays and the branch's explicitly selected `REX_BIN`.
- All throwaway servers and test clients/displays were stopped by their owned
  PIDs. The separately authorized live-service restart is described below.

## Complete protocol diff: 12 → 13

Captured both servers' `rex --autostart=false --server <private-endpoint> api
list --json`, `api call server.version`, and `/llms.txt` over their private Unix
sockets. The API-12 help capture is semantically identical to the previous SDK
input. Recursive comparison finds **exactly one addition**:

```text
+ block_types[com.superlogical.terminal].methods.download_begin
```

- **74 top-level methods before and after**, unchanged in full: schemas,
  descriptions, examples, validation notes and action declarations.
- Terminal methods: **17 → 18**. The existing terminal methods and the other
  block type's two methods are unchanged.
- No methods, fields or events removed or renamed; no changes to existing
  required fields or event payloads.
- HTTP endpoints, control/data subprotocols, authentication, HELLO/WELCOME,
  reconnect identity, bootstrap and terminal snapshot/resume contracts are
  unchanged. No new mandatory capability or handshake field.
- `client.keymap`, `client.key`, `client.action`, `client.perform`, `client.reply`
  and their error/event contracts are identical. **No new UI action declaration**
  accompanies the new server method.
- The complete `/llms.txt` diff is the revision-number substitutions and the
  new method's index entry and reference section; all other prose is identical.

### Added `com.superlogical.terminal.download_begin`

This is a block method: supply `session_id`, `block_id` and the following `args`:

| Argument | Contract |
| --- | --- |
| `ref` | Required string: `download:` plus a client-chosen lowercase canonical UUID |
| `path` | Required string; absolute, relative to the foreground process's working directory, or leading `~` for home |
| `if_none_match` | Optional opaque version from a previous result for that path |

Only regular files can be downloaded. The response contains:

| Field | Contract |
| --- | --- |
| `path`, `name` | Required strings: absolute path with symlinks unexpanded, and final path component |
| `size` | Required integer, opened-file byte size; progress hint only, not a stream length guarantee |
| `modified_at` | Required RFC 3339 string; not a reliable cache validator |
| `ticket` | Optional single-use ticket for the existing `rex.data.bytes.v1` protocol, restricted to the same user |
| `expires_in_ms` | Optional unsigned integer ticket lifetime |
| `not_modified` | Optional boolean; true means the cached version matches and there is no ticket or download |
| `version` | Optional opaque cache validator; absent for recently changed files or when the server cannot establish a version, in which case the download must not be cached |

Downloads send bytes from server to client, unlike uploads. No existing client
path calls this method, so **no breaking-change revision branch is needed**.
A future download implementation must require revision 13 and handle absent
validators, conditional responses and changing stream sizes. This PR does not
implement that data-stream consumer or claim a download-transfer test.

## Implementation

- Raise the negotiated maximum from 12 to 13, retaining minimum 10. Mock tests
  accept 10, 11, 12 and 13 and reject 9 and 14 with the explicit supported-range
  message. Add read-only listener discovery coverage at 13.
- Refresh `vendor/rexrs/codegen/help.json` and `version.json`; regenerate with
  `python3 codegen/generate.py` from `vendor/rexrs`. The same upstream SDK pin and
  generator suffice; generated `terminal::DownloadBegin` and response types
  preserve all optional fields. Existing explicit-empty-bootstrap and
  negotiation-range patches remain intact. No Cargo.lock change.
- Add four SDK tests for request/cache-validator serialization, ticket-bearing
  responses without caching metadata, not-modified responses without tickets,
  and required response metadata.
- Preserve 0.5.x's per-host default-off remote-control consent and exact eight
  view-only actions. Unsupported actions still return `not_supported`; supported
  actions with consent off return `refused`. A new headless test checks both
  consent states for an unknown action and the download method name passed as
  an action: only a refusal is dispatched, never a download or UI action.
- Make `remote-control-acceptance.py` check structured API revision 12/13 rather
  than requiring the literal `preview.14` version string. Optional
  `REX_EXPECT_REVISION` also verifies the selected server binary.
- Update the README's supported revisions and add an Unreleased changelog entry.

## Verification

Used a scratch `CARGO_TARGET_DIR` (reflink copy of existing build artifacts,
then rebuilt from this checkout); UI acceptance used the newly built **debug**
client, not a stale installed release. No release build/package is claimed.

| Check | API 13 snapshot | API 12 preview.14 |
| --- | --- | --- |
| `rex-probe`, with `REX_EXPECT_REVISION` | Pass, 13 negotiated | Pass, 12 negotiated |
| `rex-workspace-probe`, with `REX_EXPECT_REVISION` | Pass | Pass |
| `scripts/keymap-acceptance.py` | Pass | Pass |
| `scripts/x11-smoke.py` | Pass on unchanged retry; see caveat | Pass |
| `scripts/remote-control-acceptance.py` | Pass, real private SSH host B | Pass, real private SSH host B |

Probes check snapshot/resume, Unicode, stable reconnect identity, bootstrap
bundled tickets, explicit/default/missing selection, ownership, input, layouts,
split/resize/zoom and external changes. Keymap tests cover layered bindings,
sequences, Lua forwarding, queued local actions, reload, removals, Kitty keys,
`pane.send_key` and explicit remote refusals. Remote acceptance covers the eight
allowed view actions, cross-host session selection, negative tab index, host
consent, invalid targets/arguments, hazardous actions, typing, dialogs, search
fields and pending sequences.

- `cargo test --locked --all-features`: **343 passed, 2 ignored** (152 library,
  190 headless UI, one fixture). Ignored tests remain the existing CJK-font
  environment check and Ghostty restored/resized search-order limitation.
- `cargo test --manifest-path vendor/rexrs/Cargo.toml --all-features`:
  **160 passed**, including six doctests; ten live tests ignored by default.
- The **10 live SDK tests** also passed separately against the API-13 private
  socket (nine blocking, one async). SDK defaults target its generated revision
  13; the app's explicit range is exercised against API 12 by the matrix above.
- `cargo build --locked --bins`: **passed**.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: **passed**.
  Vendored Rune still emits its pre-existing dependency `unused_mut` warning.
- `cargo fmt --all -- --check` and SDK formatting check: **passed**.
- Revisions **10/11**: mock negotiation/state tests, not live servers. No attempt
  was made to run the replaced old installed executable.

One initial suite run exposed an error in the newly added discovery fixture
(the shared assertions expected a TCP address, but it supplied Unix metadata).
Corrected that fixture; subsequent complete runs passed. The first API-13 X11
run produced an empty clipboard paste at `X11_CLIPBOARD_OK`; the unchanged
rerun passed all checks, as did API 12. Recorded the remaining timing flake as
papercut `pc_540f0bd8abba`; no product change was made to hide it.

## Authorized live check on shuvdev

After private API-13 probes, keymap, remote-control and full Rust tests passed,
the user explicitly lifted the no-restart restriction for shuvdev. At
**2026-11-10 11:22:53 PST**, ran only:

```sh
systemctl --user restart rex-server.service
```

The unit path and ExecStart remained unchanged:
`~/.config/systemd/user/rex-server.service`, executing
`/usr/bin/bash -lc 'exec /usr/bin/rex server run --tailscale'`.
Main PID changed from **431123 to 956205**. Read-only `server.version` through
`unix:///run/user/1000/rex/server.sock` confirms the exact API-13 snapshot/commit
above. The original session IDs were preserved.

Both this branch's build and installed **`~/.local/bin/rex-linux` 0.5.0** were
launched on a private Xvfb, with separate scratch workspace/HOME/XDG settings,
notifications disabled, and attached to the existing session. `client.list`
confirmed each process connected and attached; `client.inspect` confirmed its
32 registered actions. Captures showed the shell rendered by each client.
No terminal input, session creation/deletion or saved user settings changes
were needed. Both test clients were stopped by PID afterward.

**The requested old-client refusal does not occur.** Stock 0.5.0 connects
successfully, consistent with the measured HELLO 10–12 → WELCOME 12 negotiation.
We did not deliberately break backward negotiation to manufacture a refusal.
The supported-range error is pinned by mocks for genuinely unsupported WELCOME
revisions. A supplemental attempted check used a stale local release artifact
that reported 0.3.2, not 0.5.1; it is excluded from the 0.5.x verification.

**shuvdev was left running API 13** on its original service. No restart or other
change was made to shuvmbp-1 or shuvbot, no package was installed, and no release
or merge was performed. Raw captures, test logs and live-check evidence remain
under `~/.cache/agent-ws/rex-api13/`; private runtime data is not committed.

## Decisions and scope

**No API-13 compatibility decision is needed.** Downloads are optional and
remain SDK-only. A future save/download/open-file UI, cache policy and any
remote permission for it should be a separate feature decision; none is needed
to attach safely or retain existing remote-control behavior. No Wayland,
macOS, aarch64, real remote-host restart or download-transfer claim is made.
