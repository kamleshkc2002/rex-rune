# Rex API 13 compatibility

Verified on Linux x86-64 on **2026-11-10 (PST)**, based on Rex Linux 0.5.1.
The client now advertises **API revisions 10–13**. Uploads still require 11;
no new remote permissions or server lifecycle behavior was added.

**Current state:** the client now has a file-download UI that requires revision
13 (see [Client implementation and acceptance](#client-implementation-and-acceptance)).
The statements below that no download UI or data-stream consumer exists describe
the 0.5.1 snapshot this section was first written against.

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
A download implementation must require revision 13 and handle absent
validators, conditional responses and changing stream sizes. The original PR did
not implement that data-stream consumer; the client does now (see
[Client implementation and acceptance](#client-implementation-and-acceptance)).

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

**No API-13 compatibility decision is needed.** Downloads are optional. The
original compatibility change kept them SDK-only; the file-download UI that
followed is described under
[Client implementation and acceptance](#client-implementation-and-acceptance).
It has no cache, and no remote permission: `client.perform` cannot start a
download. None of it is needed to attach safely or retain existing
remote-control behavior. No Wayland, macOS or aarch64 claim is made.

## Download contract (D0)

Observed on **2026-11-11 (PST)**, 00:24 to 01:00, with `scripts/download-contract.py`
against a private server started from `/usr/bin/rex`: `0.0.1794323808-g757adf`,
commit `757adf99fad394c515be6313aa5dbe17008dc7d2`, API 13, executable SHA-256
`0436f853…d60a` (the snapshot described above). The API 12 comparison used the
cached preview.14 binary, `0.1.0-preview.14`, commit `7e460d02f51573990bfcdc6c44a5606d11c7d47a`,
which does run (`rex version` prints `api: revision 12 (supports 0 to 12)`).

Everything was observed over a **Unix socket**, from a Python client that speaks
WebSocket itself so that every frame, opcode and close code is visible. Nothing
here was measured over HTTPS, Tailscale or an SSH forward (see "Not observed").
The script starts its own server (own socket, HOME, XDG directories, empty config,
`--restore=false`), scrubs `REX_*`, and stops the server by PID; it never touches
another server. Raw evidence (every control request and response, every stream's
summary, per-scenario results, the pass/fail of each pinned expectation, the
server's `/llms.txt`) is under `~/.cache/agent-ws/rex-downloads/d0/` and is not in
the repository. Rerun the script on every server bump: it exits 1 when a pinned
expectation no longer holds.

### What the server's own documents say, and where they stop

`/llms.txt` (326,544 bytes, SHA-256 `11d25f84…cb81`), `rex api describe` and
`vendor/rexrs/codegen/help.json` agree on the method; they do not describe the
stream. All of the text about `download_begin` and `rex.data.bytes.v1` is:

> Open a file on the terminal's machine for download, and get a ticket for the
> rex.data.bytes.v1 data stream that carries its bytes from the terminal. A
> relative path is relative to the working directory of the terminal's
> foreground process. Only regular files can be downloaded.

with the argument and result descriptions already tabulated above, plus
"`size`: The file's size in bytes when it was opened, for showing progress only.
The stream may carry more or fewer bytes if the file changes" and "`version`: … Left
out when the file changed within the last second, or when the server can't tell,
and then the download must not be cached." `rex help`, `rex api call --help` and
`rex skill show` say nothing about downloads. The "Data Connections" chapter
describes the HELLO/WELCOME/REJECT handshake, RESUME and the close table for the
**terminal** stream (`rex.data.terminal.v1`); the bytes protocol is not defined
there or anywhere else in `/llms.txt`.

| Where the docs say or imply | Observed |
| --- | --- |
| "Each [stream] message holds 1 to 65,536 bytes" (terminal stream) | Download messages hold **at most 32,768 bytes**, and the stream ends with one **empty (0-byte) binary message** before the close. Nothing in the docs mentions the empty message. |
| "When your client closes a data connection with status 1000, its stream ends. Any other close, or a connection that drops, leaves the stream waiting for a resume." | For a download, **every** client-side close (1000, 1001, FIN without a close frame, RST) releases the file and the slot within a second. A RESUME after a drop gets REJECT `0x03`. A download stream cannot be resumed. |
| REJECT `0x02`: a client can have only one connection to the same data source of a block | Not seen: 16 downloads from one terminal and one control connection ran at once, each on its own data connection. |
| REJECT `0x04` "Opening the connection would pass a limit. This server doesn't send it yet." | Still not sent. The limit (16 per terminal) is a control-side `conflict` from `download_begin`. |
| `args` schema has `additionalProperties: false` | An unknown extra argument is **accepted** (the reply carries a ticket). Do not rely on schema rejection. |
| `version` is withheld "when the file changed within the last second" | The rule looks at the inode's **ctime**, not the modification time (table below). |
| Close table: `1000` "The stream ended" | For a download `1000` is the only success code, and it is also what a file that was truncated or that grew ends with. It does not say the file was complete. |

### Request and reply

`com.superlogical.terminal.download_begin` is a block method, on revision 13 and up.

| Item | Observed |
| --- | --- |
| Envelope | `{"type":"request","id":…,"method":"com.superlogical.terminal.download_begin","payload":{"session_id":"session:…","block_id":"block:…","args":{"ref":"download:<uuid>","path":"…"}}}` |
| Reply | `payload` with `ticket` (22 base64url characters, 16 bytes), `expires_in_ms` (**60000**), `path`, `name`, `size`, `modified_at`, and `version` only when it can be given (see below). No other keys were ever present. |
| `modified_at` | RFC 3339 with nanoseconds and the server's local offset, e.g. `2026-11-11T00:24:53.664802448-08:00`; a whole-second time has no fraction (`1969-12-31T16:00:01-08:00`). |
| `size` | Integer, exact up to at least 1 TiB (a 5 GiB and a 1 TiB sparse file were reported exactly). `0` for `/proc` files that have content, so a zero hint does not mean empty. |
| Needs an attached session | A connection that has not called `session.attach` or `session.bootstrap` for the session gets `conflict` 409 `stream is not attached to session session:…`. |
| Envelope errors | Unknown session: `session_not_found` 404. Unknown block: `not_found` 404 `block not found: block:…`. Missing `args`: `bad_request` `validating input: validating root: required: missing properties: ["ref" "path"]`. |
| Revision gate | On a revision 10, 11 or 12 **connection to an API 13 server**: `revision_too_low` 400 `method "com.superlogical.terminal.download_begin" needs API revision 13, but this connection uses revision 12 (the server supports 0 to 13)`. On the real **API 12 server** (preview.14): `method_not_found` 404 `method not found: "download_begin"`, and `api list` does not list it. Gate on the negotiated revision, and treat both codes as "too old". |
| Reserved slot | Each reply with a ticket takes one of **16 slots per terminal** until its stream ends or is cancelled, or the ticket expires. The 17th: `conflict` 409 `conflict: 16 downloads are already reserved, the most a terminal allows`. |

#### `ref`

| Case | Observed |
| --- | --- |
| `download:` + lower-case canonical UUID (any version, the nil UUID, a v7) | Accepted. |
| Upper-case UUID; no prefix; `upload:` prefix; `Download:`; empty; prefix only; not a UUID; UUID without hyphens; braced; trailing space; trailing NUL; doubled prefix; 5,000 characters | `bad_request` 400, e.g. `ref "download:CC1E…" does not end with a UUID in canonical form` or `ref "…" does not start with "download:"` |
| Same `ref` begun again while the first is reserved (unopened, or streaming), from the same or another connection | `bad_request` 400 `download download:<uuid> is already reserved` |
| Same `ref` after its stream finished or was cancelled | Accepted. A UUID is not remembered after use. |
| `ref` of another type, `path` of another type, `path` null, `if_none_match` a number | `bad_request` 400 with a Go decoding message |
| Missing `ref` or `path` | `bad_request` 400 `required: missing properties: ["ref"]` |

#### `path`

Paths are resolved by the server, on the server's machine. Reply `path` is
**lexically cleaned** (`..`, `.`, `//` and a trailing `/` collapse) and symlinks are
left unexpanded.

| Input | Observed |
| --- | --- |
| Absolute | Used as is, cleaned. `/../../x` collapses to `/x`. |
| `~/x` | The home directory in the `process` reply's `home` field (the server's `$HOME`); this is the server's HOME and not the passwd entry when they differ. `~` alone, `~/`: `not a regular file: <home> is a directory`. |
| `~root/x`, `~nosuchuser/x`, `$HOME/x`, a glob, `file://…`, `C:\x` | **Not expanded**: treated as relative names, e.g. `not found: <cwd>/~root/x does not exist`. |
| Relative | Relative to the **foreground process's** working directory, the `foreground.cwd` of `com.superlogical.terminal.process`. With the shell in `dir one` and `sh -c 'cd "dir two"; exec sleep 600'` in front, `both.txt` came from `dir two`, and `one.txt` was not found. After Ctrl-C it came from `dir one` again. It follows `cd` in the shell. |
| Relative, shell at its start | The terminal's start directory (HOME here). |
| Relative where the cwd is a deleted directory | `not found: <cwd> (deleted)/x does not exist`: the Linux `(deleted)` suffix is kept in the path. |
| Relative where the cwd name is not UTF-8 | Resolved correctly, but the reply `path` has U+FFFD in place of the bad bytes, so it cannot be reopened from the reply. |
| `.`, `..` | Resolve to the foreground cwd and its parent, then `not a regular file: … is a directory`. |
| Empty | `bad_request` `path is empty`. A single space is a relative name `" "`. |
| NUL in the path | `bad_request` `opening …\u0000.png: stat …: invalid argument` |
| Longer than the OS allows | `bad_request` `…: file name too long`. **The message repeats the whole path**: 5,000 characters in, about 10,000 out. |

#### What each kind of path answers

All errors have no ticket and take no slot. Messages quoted verbatim, paths shortened.

| Target | Code | Message |
| --- | --- | --- |
| Missing file | `not_found` 404 | `not found: <path> does not exist` |
| Dangling symlink | `not_found` 404 | `not found: <path> does not exist` |
| Directory, `/`, symlink to a directory | `bad_request` 400 | `not a regular file: <path> is a directory` (a trailing slash is dropped from `<path>`) |
| File given with a trailing `/` | **accepted**: the slash is dropped from `path` | |
| `file.txt/x` | `bad_request` 400 | `opening <path>: stat <path>: not a directory` |
| Symlink loop | `bad_request` 400 | `opening <path>: stat <path>: too many levels of symbolic links` |
| Unreadable file (mode 000) | `bad_request` 400 | `opening <path>: open <path>: permission denied` |
| File in a directory with no search permission | `bad_request` 400 | `opening <path>: stat <path>: permission denied` |
| FIFO | `bad_request` 400, immediately (the server did not block) | `not a regular file: <path> is a p---------` |
| Unix socket | `bad_request` 400 | `… is a S---------` |
| `/dev/null`, `/dev/zero` | `bad_request` 400 | `… is a Dc---------` |
| Symlink to a file, or to a symlink to a file | accepted; `path` and `name` are the **link's**, the bytes are the target's | |
| Empty file | accepted; `size` 0; the stream is WELCOME, one empty message, close 1000 (no data bytes) | |
| `/proc/cpuinfo` | accepted with `size` 0; 46,082 bytes came | |
| `/proc/self/mem` (a read fails with EIO at offset 0) | accepted; WELCOME, then close **1011 `data provider failed`** with no empty message | |

So `not_found` is the only code for "no such file", and `bad_request` is the code for
everything else, including permissions. The message text is the only way to tell
them apart, and it is English written for people.

#### Names

`name` is the last element of the cleaned `path` in every case (`name_is_last_component_of_path`
held for all 23). **It is returned verbatim, with no sanitizing**:

| File name | `name` in the reply |
| --- | --- |
| spaces, Unicode with an emoji, combining accent | unchanged |
| newline, tab, CR, BEL, ESC (`esc\x1b[31mred.txt`), DEL | unchanged, control characters and all |
| U+202E (right-to-left override), U+200B (zero-width space) | unchanged |
| leading dash (`-rf.txt`, `--help`), leading dot (`.hidden`), `...`, trailing dot or space | unchanged |
| quote, backslash, `%25`, colon | unchanged |
| 255 bytes | unchanged |

A name that is not valid UTF-8 cannot be requested (a JSON string cannot carry it,
and invalid UTF-8 on the control socket closes the connection), and none could be
made to surface in `name`. The reply for a path inside a non-UTF-8 directory is
described above. The client must sanitize `name` itself, as the plan says.

### The data stream

#### Opening it

| Step | Observed |
| --- | --- |
| Endpoint and subprotocol | `GET /api/data` upgraded with `Sec-WebSocket-Protocol: rex.data.bytes.v1`, the same call `src/upload.rs` makes (`transport::socket(endpoint, "/api/data", PROTOCOL)`), the **same HELLO** (`0x01` + 16 ticket bytes decoded from base64url) and the same WELCOME (`wire::handshake`). The reply's `Sec-WebSocket-Protocol` echoes the request. |
| Control connection | The data socket presents only the ticket. The control connection that issued the ticket must stay open for the whole transfer (see "Tickets"). |
| After the WELCOME | The server sends binary messages only. The client sends nothing. |
| Upgrade quirks | A request with no subprotocol is upgraded with no `Sec-WebSocket-Protocol` header; an unknown subprotocol is upgraded **and echoed**. Check what came back, as `transport::open_socket` already does. |

The WELCOME is 32 bytes: `0x81`, a 16-byte reconnect token, three zero bytes,
`retention_ms` (60000) as u32 little-endian, and a non-zero u64 channel nonce. The
token, retention and nonce have no use in a download: a stream cannot be resumed,
and no nonce-marked control sequences appear in it. Verbatim, for a 12-byte file
(`d0/basic-raw-frames.json`):

```text
client -> HELLO     binary, 17 bytes  01 <16 ticket bytes>
server -> WELCOME   binary, 32 bytes  81 6b7ccb59795b1612ddd62a60c49b1a3e 000000 60ea0000 015f62ad47c54f46
server -> data      binary, 12 bytes  68656c6c6f20776f726c640a            ("hello world\n")
server -> end       binary,  0 bytes
server -> close     opcode 8, payload 03e8                                (1000, empty reason)
```

#### Framing

| Question | Observed |
| --- | --- |
| Message type | Every message is **binary** (opcode 2), FIN set. No text, no continuation frames, no pings or pongs seen during any transfer, no RSV bits. |
| Message size | **32,768 bytes** exactly for every message but the last data message. A 256 MiB file: 8,192 messages of 32,768. A 1 MiB + 17 byte file: 32 of 32,768, one of 17. No stream in the run had a larger message. |
| Header, trailer, length prefix, control JSON | **None.** The data messages concatenated are the file. |
| Throughput | 256 MiB sparse in 0.14 s, about **1.8 GB/s** over the Unix socket (a Python reader, so a lower bound). Over TCP, TLS or SSH it will be far lower; not measured. 16 concurrent 24 MiB downloads: 0.63 s in all, 613 MB/s combined. |
| Read-ahead | The server runs ahead of a slow reader by several MiB (a file truncated after the reader had 128 KiB still delivered 9.1 MB). The bytes the client gets are not what is on disk when it reads them. |

#### How it ends

| Case | What arrives |
| --- | --- |
| **Success** (this is the only success signal) | the last data message, then **one empty binary message**, then a Close frame with status **1000** and an **empty reason**. Seen on all 65 logged streams that ended with 1000. The empty message was never seen on any stream that ended another way (18 logged). |
| Server stopped the stream | no empty message; a Close with a code and reason from the table below. |
| The client must send | **Nothing**: no ack, no finish, no cancel. After the server's Close the server leaves the TCP connection open until the client answers the Close (it did not hang up in 0.5 s), so a client should reply with a Close frame and drop the socket. |

A transfer is therefore complete **only if** it got the empty message and then a 1000
close. A close with any other code, or the connection ending with no Close, is a
failure whatever bytes arrived. `bytes received == size` is not part of the rule (see
"A file that changes").

| Close | Reason | Seen when |
| --- | --- | --- |
| 1000 | empty | the file was read to EOF (even if shorter or longer than `size`) |
| 1001 | `data provider canceled` | the control connection that issued the ticket closed; `session.detach` of the attached session; the server was told to stop (SIGTERM) |
| 4000 | `block closed` | the block was closed, or the session destroyed, during the transfer |
| 1011 | `data provider failed` | a read failed (EIO) |
| 1008 | `data handshake rejected` | after REJECT `0x01`, `0x03` or `0x07` |
| 1002 | `data handshake rejected` | after REJECT `0x05` |

#### Cancel

| Client action | Server behavior |
| --- | --- |
| Close frame 1000, or 1001, or FIN without a Close frame, or RST | The server stops and **closes its descriptor on the file within a second** (checked in `/proc/<pid>/fd`); the slot is free again. A client can cancel by closing the socket; the normal way is a Close frame with 1000. |
| Empty binary, other binary and text messages sent mid-stream | **Ignored.** An upload-style empty message does not cancel. |
| RESUME after a drop | REJECT `0x03`, then close 1008. Not resumable. |

#### REJECT reasons seen on a bytes stream

| Case | Reply |
| --- | --- |
| Unknown, already used, expired or revoked ticket; the all-zero ticket | REJECT `0x01` |
| A ticket from `data.connect` (terminal) on `rex.data.bytes.v1`, or a download ticket on `rex.data.terminal.v1` | REJECT `0x07` |
| Empty HELLO, `0x01` alone, a 15-byte ticket, first byte `0x03`, a text message | REJECT `0x05`, close 1002 |
| No HELLO within 5 s | REJECT `0x05`, close 1002 at 5.0 s |
| HELLO with extra bytes after the 17 | accepted (ignored, as documented) |
| RESUME | REJECT `0x03` |

Each REJECT is one 2-byte binary message (`82 <reason>`) and then a Close.

### Tickets

| Question | Observed |
| --- | --- |
| Lifetime | `expires_in_ms` = 60000, measured: used at 30 s it works; at 62.5 s it is REJECT `0x01`. |
| Held while unused | The server **holds the file open** from `download_begin` until the ticket is used or expires (an `fd` on the file at 0 s and 30 s, none at 62.5 s), and the slot is held: with 16 unopened tickets the 17th was refused for 56 s and accepted at 60 s. |
| Giving an unopened ticket up | Opening it (HELLO, WELCOME) and sending Close 1000 at once frees the file and the slot immediately (an `fd` count of 16 went to 0, and the 17th `download_begin` was accepted). Without it a cancelled or abandoned ticket holds both for the full 60 s. The Linux client does this when a transfer is cancelled, expires or is refused for space before its stream opened. A ticket whose control connection closed is already revoked and cannot be released this way. |
| Reuse | A second HELLO with a finished ticket, or with one that is mid-stream: REJECT `0x01`. |
| Two connections at once with one ticket | One gets the WELCOME and the file; the other gets REJECT `0x01`. |
| The issuing control connection closes | Unopened ticket: REJECT `0x01` afterwards (revoked). Mid-stream: the stream ends with 1001 `data provider canceled`. A client reconnecting its control connection therefore loses its transfers. |
| Different user | Not observed (one user only). |

### Concurrency and limits

| Limit | Observed |
| --- | --- |
| Downloads reserved or running per terminal | **16**, shared by every client attached to that terminal. The 17th `download_begin`: `conflict` 409. A second terminal (a split) has its own 16. |
| What frees a slot | the stream finishing; a client close of any kind; RST; the ticket expiring (60 s); the issuing control connection closing (a client that held 16 unopened tickets and left freed all 16 at once). |
| 3, 8 and 16 at once, one connection, one terminal | All completed byte-exact; no REJECT `0x02`. Combined rate grew with the count. |
| Two clients, one terminal, the same file | Both exact. |
| The same `ref` from two clients | The second: `already reserved`. |
| Ordinary calls during a download | `com.superlogical.terminal.write` answered in under 1 ms with a stream running. |
| A stalled reader | A reader that stopped reading for **100 s** then read on and got the **whole 64 MiB**, byte-exact, close 1000. No keepalive close, no ping seen in that time. The server will wait. The client needs its own stall timer. |
| Idle control connection | No frame or ping in 65 s. The ping interval is longer than that or pings are only sent in another state; unknown. |
| Not sent | REJECT `0x04`, ever. |

### A file that changes

`size` is a hint: the server reads until end of file **at the time it reads**, and does not
count against `size`.

| Change | Stream result (all ended `1000`) |
| --- | --- |
| Appended after `download_begin`, before the HELLO | all of it: 4 MiB + 2 MiB gave 6 MiB; 4 MiB + 64 MiB gave 68 MiB |
| Appended during the transfer | all of it that was appended before the server reached the end: 16 MiB + 2 MiB gave 18 MiB |
| Appended continuously faster than the reader reads | **never ends**: 120 MiB read against a 16 MiB hint, still going when the reader gave up. The stream has no cap of its own. |
| Truncated before the HELLO (4 MiB to 1 MiB) | 1 MiB, close 1000, no error |
| Truncated during the transfer | stops at the new length, close 1000 (256 MiB truncated to 64 MiB gave exactly 64 MiB). Bytes the server had already read ahead are still delivered. |
| Replaced by a rename over it, or deleted, before or during | the **old** file, complete and exact (the server holds the descriptor) |
| `chmod 000` during | unaffected |
| Overwritten in place with the same length | a mixture of old and new bytes; nothing signals it |

The client cannot tell a truncated file from a complete one by the close. Whether a
shorter stream than `size` is an error is a client decision; `size` is only a hint.
A longer one needs the client's own cap, which the plan already has.

### `version`, `if_none_match`, `not_modified`

| Question | Observed |
| --- | --- |
| When it is present | When the inode's **ctime** is more than about one second old. A file written or `chmod`ed, renamed, hard-linked or `utime`d within the last second has none, **whatever mtime says** (mtime 100 s ago, 0.5 s ago, at the epoch: absent while ctime is fresh; present 1.3 s later). A file whose mtime is **in the future** got one even with a fresh ctime. |
| Form | 22 base64url characters. |
| What changes it | A content change of the same size, an mtime-only change, a `chmod`, growth. A same-size rewrite with the old mtime restored changed it too (it includes ctime). |
| `if_none_match` equals the current version | `payload` has `not_modified: true`, `path`, `name`, `size`, `modified_at`, `version`; **no `ticket` and no `expires_in_ms`**, no slot taken. |
| `if_none_match` does not match, or is `""` | A normal reply with a ticket, and the current `version` if there is one. |
| `if_none_match` a stale version after the file changed | A normal reply with a ticket. |
| The plan's client | Sends no `if_none_match`, so `not_modified` is an error. A `ticket` that is absent without `not_modified` was never seen. |

### What a client must defend against

- A name or path with control characters, direction overrides, a leading dash or dot, or 255 bytes: sanitize, never trust `name`.
- An error message that repeats a very long path: truncate before showing.
- Whether a path resolved where the person meant: the reply `path` is the only evidence, and it can be lossy (U+FFFD).
- A stream that ends in `1000` shorter or longer than `size`, or never ends: own byte cap and stall timer.
- A `1000` close **without** the empty message: not seen, but treat it as failure.
- Any other close code, or an abrupt end: failure, delete the partial file.
- Tickets that are never opened: they hold a descriptor and one of 16 slots for 60 s each, so open or cancel promptly.
- Leaving the control connection: it kills every in-flight transfer and revokes unopened tickets.

### Not observed

- **HTTPS, Tailscale, SSH-forwarded and TCP endpoints.** Unix socket only. Framing, close codes and the 32 KiB message size are expected to be the same, but TLS and `ssh -L` were not exercised, and nothing was measured about how a slow link interacts with the server's read-ahead.
- A different user presenting a ticket (the docs say REJECT `0x01`).
- The server's ping interval on a data connection (none seen in 100 s with the reader stalled; none on an idle control connection in 65 s).
- A read error part way through a file: only EIO at offset 0 was produced (`/proc/self/mem`), which gave close 1011.
- How much the server reads ahead. At least 9 MiB was seen.
- Whether the 16-slot cap and the `ref` uniqueness are per terminal or per server (two terminals each had their own 16; `ref` uniqueness was checked on one terminal only).
- `~` where the server's `$HOME` and the passwd home differ (here they did: it used `$HOME`, but it was not determined whether it is the server's or the terminal process's).
- macOS or other servers, `permessage-deflate` (the client did not offer it), files on network or FUSE file systems, paths over `PATH_MAX`, a hard cap on open descriptors from unopened tickets.
- `/dev` and `/proc` behavior beyond the cases listed.

## Client implementation and acceptance

Rex Linux implements the contract above as the **Download file…** action
(`pane.download`; see the README's Downloads section). It gates on the negotiated
revision (13 or higher), sends no `if_none_match` (a `not_modified` reply is an
error), opens the data stream with `rex.data.bytes.v1` on the host's own data
endpoint, counts a transfer as complete only after the empty message and a 1000
close, enforces its own growth cap and stall timer, and gives an unopened ticket
back at once (HELLO, WELCOME, Close 1000) when a transfer is cancelled or refused.

[`scripts/download-acceptance.py`](../scripts/download-acceptance.py) exercises
the built binary through the real UI against private API 13 and API 12 servers
(and one transfer through a real private `ssh://` host). Its cases are listed in
its docstring and in the README's Development section; it prints a PASS, FAIL or
SKIP line for each. Rerun it, and `scripts/download-contract.py`, on every server
bump.

Still not observed with downloads: HTTPS, Tailscale and plain TCP endpoints (a
Unix socket and an SSH forward were), and the real file-manager `ShowItems` call
and `xdg-open` on the folder (the acceptance script puts a stub `xdg-open` on `PATH` and
checks that it is never called).
