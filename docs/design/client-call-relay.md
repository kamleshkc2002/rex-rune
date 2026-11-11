# Cross-server relay: consent and host boundary for `client_call_requested`

Design for [issue #20](https://github.com/shuv1337/rex-rune/issues/20), written 2026-11-11 (PST) against `master` at `ad196ff` (Rex Linux 0.6.0, API window 10–13). Status: proposed. Nothing here is implemented: Rex Linux still sends no `servers` list, and the two tests that pin that stay. It builds on the Remote control work for [#12](https://github.com/shuv1337/rex-rune/issues/12) ([plan](../plans/remote-ui-execution.md)) and replaces the sketch in [roadmap 0.5.2, section 7](../plans/roadmap-0.5.2.md#7-20-cross-server-relay-client_call_requested).

## Recommendation

- **The relay is a separate permission from Remote control and a stronger one.** Remote control lets host A change what this window shows. The relay lets A's Rex config act on host B *as this user on B*. At the Control level, that is code execution on B.
- **Consent belongs to an ordered pair of saved hosts (A → B). It is remembered, set only in Settings, and off by default.** It has three levels: **Off**, **Read layout** and **Control terminals**. No prompts, no one-shot grants, no pair implied by another (including this machine's own server).
- **A request is accepted only while this client's own key-press `client.action` is still waiting on A**, and only for a target listed in that request. Methods are classed by an exhaustive allowlist, default deny. Rex-level persistence, data tickets, chaining and server lifecycle are never relayed at any level.
- **Every relayed call goes over its own connection to B, named after A,** so B can see it and nothing queues behind or in front of B's input.
- **Don't build it yet** (decision 1). Approve this policy as the gate for #20, keep `servers` empty, and implement it when someone names a binding they can't get with `rex -S B …`.

## 1. What the relay is

Sources: `vendor/rexrs/codegen/help.json` (the `client.action` and `client.reply` notes) and the schema strings in the installed server (`rex` build `7b4cbcb`, 2026-11-11). The event's field names come from those strings and **have not been observed**. Before implementing, check them against a private server with a stand-in client, as [#12's appendix](../plans/remote-ui-execution.md#appendix-how-the-contract-was-checked) did.

- `client.action` accepts `servers: [{name, id}]`, the other servers this client is connected to. A config action (one whose name has no dot) on A can then call `rex.server(name)`, which returns the full Lua API (`rex.session`, `rex.block.call`, `rex.call`) pointed at that server. `client.key` takes no `servers`, so Lua key handlers cannot relay; only key presses bound to config actions can.
- When the action calls the other server, A sends this client `client_call_requested {call_id, server, method, args}`, where `server` is the `id` the client chose. The client makes that one call on B and answers with `client.reply` on A's connection within 10 seconds. If no reply comes, or the reply is an error, that call fails inside the action and the action keeps running. The action as a whole has 30 seconds.
- `client.reply` is not ordered, so it never waits behind the `client.action` it serves. `call_id` is never reused on a connection.
- Today Rex Linux sends `client.action` only for a key bound to a server action (`src/app/input.rs`, `Kind::Server`), with no `servers` (pinned in `src/connection/keymap_tests.rs` and `src/app/bindings_integration_tests.rs`). If a `client_call_requested` event did arrive, `handle_event` would drop it and the action would wait the full 10 seconds.
- The SDK's `call()` buffers every event until its own reply arrives (`vendor/rexrs/src/async_client.rs`). The relay therefore needs A's worker to read events while its `client.action` is still waiting (section 8).

## 2. Threat model

The asset is **this user's identity on every other host**. B admits the call because it comes from this client's connection, so it runs with the user's Unix uid or Tailscale login on B. B cannot tell it from the user typing.

| Threat | What it gains without care | Mitigation |
| --- | --- | --- |
| **A is compromised or malicious** (its server, or its config file, writable by anyone with A's account) | Pivots to every host this window is attached to. | Consent per ordered pair, default off; levels; the never-relayable list; no chaining. |
| A sends unsolicited events | Calls on B while the user is idle. | Accept only while one of this client's `client.action` requests is outstanding on that connection and epoch; otherwise `refused`, plus a warning notice. |
| A binds common keys to config actions | Opens the window above on almost every keystroke. | The gate limits timing, not content, so consent is the real control. Settings → Shortcuts already shows A's server-layer bindings; the pair row shows when it was last used. |
| A forges or guesses a target | Reaches a host it was not given. | Target IDs are random per launch and mapped back by the client. Each request's list is snapshotted, and an ID outside the snapshot is refused. |
| A maps the topology | Learns endpoints and the names of other hosts. | Endpoints are never sent. Only consented targets are listed, by label. |
| **Persistence on B** | Leaves a hook that outlives the action. | `client.set_script`, `hosts.put`/`hosts.remove` (B's host list is shared by all of the user's devices) and config reloads are never relayed. Control still allows shell-level persistence (type into a shell on B), and the Settings text says so. |
| Data exfiltration B → A | Files, clipboard, terminal content. | Data tickets, uploads, downloads, blob and clipboard reads, and directory listing are never relayed. Terminal content needs Control. |
| Chaining A → B → C | Turns one grant into many. | Relayed calls never carry `servers`, and `client.*` is never relayed. The relay connection lists no servers, so B's config cannot relay through it. |
| Replay or late execution | A call answered on the wrong connection, or retried after a timeout. | Replies carry the source connection's epoch, as `client.perform` replies do. Nothing is ever retried. |
| Flooding | Many calls, or a stuck window. | One call in flight per source; 64 calls per action; `args` capped at 256 KiB; 8 seconds per call. |

Out of scope: B attacking A through results. Results only go back to A's own Lua, which already runs on A.

## 3. Consent

**Shape.** The grant is stored on the source host as a list of target endpoints with a level. It is omitted from the file when empty:

```json
{"label": "studio", "endpoint": "https://studio.example.ts.net", "relay": [{"to": "ssh://devbox", "level": "read"}]}
```

- **Per ordered pair.** studio → devbox says nothing about devbox → studio.
- **Per level, not per action.** A's config owns its action names and code, so allowing `dev_layout` by name binds nothing: a compromised A puts its payload inside `dev_layout`. Per-method scoping would be finer, but no one can judge which methods a config needs before it runs. Two levels are the honest granularity.
- **Remembered, never one-shot.** A prompt would appear right after a keystroke, take the keyboard while the user is still typing (Enter could approve it), and race the server's 10-second limit. This is the same reasoning that ruled out prompts for Remote control.
- **Cleared:** if the source's endpoint changes, its whole list is cleared. If a target's endpoint changes, or the target is removed, every entry naming it is removed. Renaming a host keeps its grants, because the server has not changed. Remote control and the relay never imply each other.

**Where it is shown.** In Settings → Remote Hosts, each host with at least one other saved host gets a group **Key bindings on studio can reach**. It has one row per other host, following the existing row pattern:

- The name: **devbox**.
- One muted line: by level, "Off", "Can list devbox's sessions, tabs and panes" or "Can create, type into, read and close terminals on devbox — this runs commands as you there". While a grant is in use, the line becomes "Last used 2 min ago by dev_layout (3 calls)" for this launch.
- On the right, a three-way choice: Off · Read layout · Control terminals.

**Revoking.** Set the row to Off. Commands also has **Stop relaying from studio** for the host on screen whenever it has any grant; it sets all of its rows to Off. Revoking takes effect on the next call, closes any open relay connection for that pair, and drops the target from the next `client.action`. A call already running finishes.

## 4. What is relayed

Methods are classed in one function by an exhaustive `match` on the method name, with `_ => Never`, like `Spec::remote()`. `block.method` is classed by its inner method name. A test pins both allowed sets, so a new Rex method stays unrelayable until someone decides otherwise.

| Level | Methods |
| --- | --- |
| **Read layout** | `session.list`, `session.view`, `session.list_blocks`, `session.describe_block`, `target.resolve`, `target.complete`, `block_creator.list`, `help.all`, `session.help`; terminal `title`, `pwd`, `process`, `program_status`, `size`. |
| **Control terminals** (adds) | `session.create`; `session.new_window`, `new_split`, `new_block`, `new_layer`; `session.close_window`, `close_layer`, `block.close`; the `set_*_label`, `focus_*`, `move*`, `swap_blocks`, `zoom_block`, `float_block`, `resize_pane`, `resize_layer`, `raise_layer`, `lower_layer` and `set_layer_state` methods; terminal `write`, `clear`, `reset`, `format`, `modes`, `input_mode`. |
| **Never, at any level** | `server.*` (lifecycle, config reload, Tailscale, version, whoami); `client.*` (`action`, `key`, `keymap`, `perform`, `reply`, `set_script`, `set_label`, `list`, `inspect`); `hosts.*`; `data.connect` and terminal `upload_begin`, `download_begin`, `blob_read`, `clipboard_read`, `list_dir`; `block.event`; `session.attach`, `attach_all`, `bootstrap`, `detach`, `detach_block`; `session.own_block`, `disown_block`, terminal `resize` and `set_theme` (size and theme belong to the user's own window on B); `session.destroy`; `session.list_clients`; any block type other than the terminal; anything unknown. |

Control means arbitrary code on B: `session.new_window` takes a `command`, and `write` types into a shell. Splitting creation from typing would not change that, so there is one Control level (decision 3).

## 5. How servers are advertised once approved

On each `client.action` the client sends to host A:

- `servers` is sent only for a config action (the name has no dot) with `origin: "key"`. It is never sent for block actions, and the `server` field is never sent: running A's action against a session on B is a different feature.
- Each entry is a target B with a grant from A at Read layout or above, other than A itself. Hosts whose `server.status` reports the same `host_id` as A are skipped. A target is listed even while it is offline, so that `rex.server("devbox")` fails with Rex Linux's message rather than "no server named devbox".
- `name` is the saved host's label, the same name `rex hosts` and `rex.servers()` use. If two saved hosts share a label, neither is listed, and Settings says why.
- `id` is a random 128-bit token per target per launch, never the endpoint. The client keeps the list it sent with the outstanding request.

With no grants, the request is byte-for-byte what it is today, so the existing pinning tests stay valid as the default-path tests.

## 6. Handling a request

The checks run in this order. The first failure answers with the message shown and sends nothing to B. The waiting action sees only the message.

1. No `client.action` from this client is outstanding on this connection and epoch: `refused`, "Rex Linux relays calls only while one of your key bindings is running." This also shows a warning notice, once per host per launch.
2. `server` is not in that request's list: `refused`, "That host was not offered to this action."
3. The grant is now Off or below the method's level (re-read at call time): `refused`, "studio may not call session.new_window on devbox. Change this in Rex Linux: Settings → Remote Hosts."
4. The method is Never or unknown: `not_supported`, "Rex Linux never relays hosts.put."
5. The action already made 64 calls, another call is in flight, or `args` is over 256 KiB: `refused`, naming the limit.
6. B is offline or reconnecting in Rex Linux: `refused`, "devbox is offline in Rex Linux; nothing was sent."
7. Otherwise, make the call on B's relay connection (below) with an 8-second budget, including connection setup, and reply with B's result, or with B's error code and message unchanged.

**The relay connection.** One connection per target, opened on first use, using the same transport as the target's saved endpoint (including an existing `ssh://` forward). Its HELLO names it "Rex Linux relay from studio", followed by the client label, so it shows up in `rex client ls` on B. It never attaches and never sends `servers`. It closes after 60 seconds idle, on revocation, on a timeout, or when the app quits. The alternative, B's own control connection, would put relayed calls in the same serial queue as the user's keys on B. It could also stall for up to 10 seconds when A's action calls B while B's worker is waiting on an action that calls A, and B could not tell relayed calls from the user's.

## 7. Audit and visibility

- **Per action:** when a `client.action` that made at least one relayed call finishes, one informational notice (about 6 seconds, coalesced like other notices) names the source, the action, and each target with its call count: "dev_layout on studio: 3 calls on devbox". If a call failed, the notice adds "1 refused".
- **Per refusal:** the first policy refusal (checks 3 and 4) per pair per launch shows one informational notice naming the method, so a config author can see why it failed. Check 1 shows a warning, because a well-behaved server never sends that event.
- **Per pair:** the Settings row shows when the grant was last used, by which action and with how many calls. It is kept in memory for this launch only.
- **Per call:** one log line on stderr with source, target, action, method, outcome and duration. Never `args` or results, which can hold typed text or terminal content.
- **On B:** the named relay connection in `rex client ls`.
- Nothing raises the window, opens a dialog or sends a desktop notification. There is no healthy-state badge (DESIGN.md, "quiet when healthy").

## 8. Failure and offline behaviour

| Situation | Behaviour |
| --- | --- |
| B offline when the action starts | B is listed anyway; each call to it is refused immediately (check 6). |
| B unreachable, or its API revision outside 10–13 | The connection attempt fails within the 8-second budget; the reply names the reason. |
| B does not answer in time | The reply says "devbox did not answer within 8 s; the call may have run." The relay connection is closed, nothing is retried, and the next call opens a fresh connection. |
| A disconnects mid-action | Pending calls are dropped. A call already sent to B finishes, and its result is discarded (its reply would carry a stale epoch). |
| Revoked mid-action | The next call is refused (check 3); the call in flight finishes. |
| A's worker is busy | The relay requires A's worker to keep reading events while its `client.action` is waiting. Today `call()` buffers them until the reply arrives. The fix is either a minimal vendored `rexrs` patch (send now, then wait while handing events to a callback), documented in `vendor/rexrs/REX-LINUX.md`, or a split send-and-wait in the worker. This is the first implementation step. |
| A stray event before any of this ships | Today it waits out the server's 10 seconds. Answering `not_supported` at once is a small hardening worth taking separately. |

## 9. Test plan

**Mock server** (the WebSocket harness in `src/connection/keymap_tests.rs`):

- With no grants, `servers` and `server` are absent (the existing pins). With a grant, `servers` lists the label and a random ID that is not the endpoint and changes per launch. Absent for dotted action names, for A itself and for the same `host_id`.
- An event with no outstanding action gets `refused` and a warning. An event during an outstanding action is answered *before* the `client.action` reply, on the same connection, with its `call_id`.
- Unknown ID, revoked grant and wrong level each get `refused`; Never and unknown methods get `not_supported`; nothing reaches the B mock in any of these cases.
- The pinned Read and Control sets. `block.method` is classed by its inner method. A Never sample is spot-checked: `hosts.put`, `client.set_script`, `client.action`, `data.connect`, `upload_begin`, `server.stop`, `session.destroy`, `block.event`.
- B's error code and message pass through. An 8-second B timeout gives the "may have run" reply, nothing is retried, and the relay connection is closed.
- The 64-call limit, the size cap, and one call in flight.
- After A reconnects, a stale reply is not sent. The relay connection's HELLO name, and that it never attaches or sends `servers`.

**Headless UI** (`src/app/tests.rs`):

- The relay group appears only when there are at least two saved hosts. Every pair defaults to Off. Grants survive save and load; an old file loads with none.
- A source endpoint change clears its list. A target endpoint change or removal removes its entries. A label change keeps them. Duplicate labels are not listed, and Settings shows why.
- **Stop relaying from studio** sets every pair from the host on screen to Off.
- One notice per action with call counts. The refusal notice appears once per pair per launch.
- Remote control and the relay are independent in both directions.
- The rows are checked in a rendered build (`-- --snap`).

**Live** (`scripts/relay-acceptance.py`, built binary, Xvfb, two private servers on their own sockets and state directories, started and stopped by PID, as `remote-control-acceptance.py` does):

- First, confirm the event's field names (section 1).
- A's config defines `relay_probe`, which calls `session.list`, then `session.new_window`, then `hosts.put` on B through `rex.server("b")`, and is bound to a key.
- At Off, the key press gives "no server named b" from Rex, and B is unchanged. At Read, only the list succeeds. At Control, the window is created, `hosts.put` is still refused, and B's host list is unchanged.
- While the action runs, B's `rex client ls` shows the relay connection. The notice text is checked in a capture.

Each PR also runs the suite in AGENTS.md.

## 10. Decisions for you

1. **When to build it.**
   - (a) **Park it (my pick).** Approve this policy, keep `servers` empty, and build it when there is a concrete binding that `rex -S B …` cannot do. No workflow needs the relay today, and it would be the widest permission in the client.
   - (b) Build consent and the Read level now. This is low risk but also low value: an action can look at B but not act on it.
   - (c) Build both levels now, as two PRs: the event-handling fix, consent and Read, then Control. That is about 600 lines of source and as many of tests.
2. **Consent shape.**
   - (a) **Remembered per ordered pair, set in Settings, levels Off / Read layout / Control terminals (my pick).**
   - (b) The same, plus an allowlist of action names per pair. This adds UI without adding protection against a compromised A.
   - (c) A first-use prompt, "Allow once / Always / Deny". It takes the keyboard mid-typing and races the 10-second limit.
3. **What Control may do.**
   - (a) There is no Control level; Read layout only, ever.
   - (b) **Create, type into, read and close windows and blocks, including `session.create`; never destroy a session or take size or theme ownership (my pick).**
   - (c) Control without `write` and content reads. This is mostly illusory, because `session.new_window` with a `command` already runs code.
