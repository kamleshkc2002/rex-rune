# Remote control: carrying out `client.perform` requests

Plan for [issue #12](https://github.com/shuv1337/rex-rune/issues/12), written 2026-11-09 (PST) against `master` at `da8da40` (Rex Rune 0.4.1, Rex server `0.1.0-preview.14`, API revision 12). Status: implemented and tested for 0.5.0; awaiting review, not merged or released. R1–R9 are decided as recommended, with the client label folded into PR 1 (two stacked PRs, released together). PR 1 implements consent, guards, notices, the seven existing actions and the label; PR 2 adds `session.select`. The original rationale and contract evidence below are retained. Both implementation branches are based on 0.4.2 (`origin/master` at `5b2c9e2`), with PR 2 stacked on PR 1. `client_call_requested` is tracked separately in [#20](https://github.com/shuv1337/rex-rune/issues/20).

## Summary

Today Rex Rune answers every `client.perform` request with `not_supported`. The plan is to carry out a small set of them, under a setting the user turns on per host.

- **What gets performed:** only actions that change what this window shows. That is one new action, `session.select` (show a session, optionally one of its tabs), and seven existing ones: go to a tab, next tab, previous tab, focus a pane, zoom a pane, scroll a terminal, and toggle the sidebar.
- **What stays refused:** the other 24 registered actions. They create or close things, open a dialog that takes the keyboard, touch the clipboard, type into a terminal, change a saved preference, or move the window to another host. Most have a `rex` command that does the same job on the server with the requester's own identity, and the refusal names it.
- **Consent:** a **Remote control** switch on each saved host, off by default, in Settings → Remote Hosts. Off answers `refused`, which is what Rex's skill tells agents to expect and to show to the person. There is no per-request prompt.
- **Host boundary:** a request arrives from one host's server and may only affect that host. Except for `session.select`, it is performed only while that host is the one on screen.
- **Visibility:** each performed request shows one notice naming the host, the requester and the action. Turning the switch off takes effect on the next request.
- **Size:** a stack of two pull requests, with an optional third for a client label.

## Implementation verification

Stack: [PR 1 — consent, guards, existing actions and label](https://github.com/shuv1337/rex-rune/pull/26), then [PR 2 — session.select](https://github.com/shuv1337/rex-rune/pull/27). Both remain unmerged; ship together in 0.5.0.

- `cargo test --locked --all-features`: protocol mocks and headless UI tests pass, including expiry/busy-loop/reconnect, exhaustive remote classes, hazards, consent, arguments, target/panel/typing guards, navigation acknowledgement/failure/timeout and local binding independence.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, and vendored rexrs tests pass.
- `scripts/keymap-acceptance.py`, `scripts/x11-smoke.py`, and the new `scripts/remote-control-acceptance.py` pass on isolated preview.14 servers. The remote script drives `rex do` and `rex api call client.perform`, including all eight allowed actions, both consent states, non-supported hazards, malformed arguments and target failures.
- Real `ssh://` attachment to the second private server was exercised with an unprivileged private sshd and disposable keys. No user servers, installed packages or units were changed; owned processes were stopped by PID.
- Rendered Settings → Remote Hosts and performed-request notices were inspected at 1440×900 in Xvfb.
- Implementation details: requests also refuse while another navigation is pending, label edits apply after reopening (stated in Settings), and unknown tabs in uncached sessions are resolved by the server during navigation rather than refusing an unpopulated cache. No vendored SDK changes or API gate were needed.

## 1. The problem

### What "remote UI execution" means here

Rex has two kinds of action. Server actions (`rex window new`, `rex split`, `rex send-key`) change session state that every attached client shares. Client actions change one app's interface: which session and tab it shows, its sidebar, its command palette. A program asks an app to run a client action by calling `client.perform` with the app's client ID; the server relays the request to that app as a `client_perform_requested` event and waits for the app's `client.reply`.

Rex Rune keeps one control connection to each saved host and has a separate client ID on each. So a request always arrives on one host's connection, from a client of that host's server. The window shows one host at a time, and the host that sent a request need not be the one on screen. "Remote UI execution" for this client is: **a program on host H asks this window to do something, and the window does it, for H.**

The requester is almost always the `rex` command, run by a coding agent or by the user in a terminal on H:

```sh
rex -C CLIENT do session.select session_id="$REX_SESSION"    # on H
rex -S devbox -C CLIENT do session.select session_id="$remote_session"   # from another machine, about devbox
```

The second form is what the skill's "Work on other machines" section describes: an agent starts work on `devbox` with `rex -S devbox …`, then asks an app connected to `devbox` to show it. A Lua script run with `rex do` reaches the same path through `rex.client.queue`, and another app could call `client.perform` directly.

### What users would want performed

| Request | Action | Why only the client can do it |
| --- | --- | --- |
| "Show me that session" (and a particular tab in it) | `session.select session_id=… [window_id=…]` | Which session a window shows is client state. Nothing in the server API reports or changes it. If H is not on screen, the window must switch to H first. |
| "Go to tab 2", "next tab" | `client.tab.goto`, `client.tab.next`, `client.tab.previous` | They act on the session this window is showing. |
| "Focus the pane on the right", "maximize it" | `pane.focus`, `pane.zoom` | They act on the pane focused in this window. |
| "Scroll up a page" | `pane.scroll` | Scroll position is local to this window. |
| "Hide the sidebar" | `client.sidebar.toggle` | Window chrome. |

`session.select` is the important one. It is the action Rex's skill names for "Show a session in the app", it is what the Mac app calls it, and Rex Rune does not have it: its `session.switch` opens the switcher instead of taking a target.

### What users can already do without this

Creating a tab, splitting a pane, typing, renaming and closing all have server commands (`rex window new`, `rex split`, `rex send`, `rex window rename`, `rex block close`) that act with the requester's own identity, return the new IDs, and work whether or not an app is open. Rex's skill tells agents to use those and to keep client actions for the interface. Rex Rune picks the new state up from the server's events. Performing these through the app adds no capability, so they are not part of the first version.

### Out of scope

- **`client_call_requested`**, where a Lua action asks this client to call a method on another server. The server sends it only to a client that listed other servers in `client.action`'s `servers`, and Rex Rune lists none. It is a relay between servers, not an interface action, and needs its own design (see question 8).
- Starting, installing or configuring a server on another host. Attachment only, as now.
- New client actions other than `session.select`, such as the Mac app's `client.font.*` or `client.config.reload`.

## 2. The protocol contract

Sources: the committed help capture (`vendor/rexrs/codegen/help.json`), the server's `/llms.txt` event reference, the guide printed by `rex skill show`, and the Mac action and settings reference in Rex's documentation. Everything marked **verified** was observed on 2026-11-09 (PST) against a private preview.14 server with a stand-in client; the appendix says how.

### Request: `client.perform`

| Field | Meaning |
| --- | --- |
| `client_id` (required) | The client to ask. |
| `action` (required) | A name the client declared in `client.keymap`. |
| `args` | A JSON object. |
| `origin` | Free text from the caller: `key`, `palette`, `api`, `cli`. |
| `session_id`, `block_id` | What the action applies to. Passed on to the client. |
| `timeout_ms` | Default 10 000, capped at 30 000. |

The server answers some requests itself, and the client never sees them (**verified** except where noted):

- An action the client did not declare fails with `not_supported`.
- Arguments that do not match the schema the client declared fail with `bad_request`. An action declared without a schema is **not** checked: `client.sidebar.toggle` with `{"extra": 1}` reached the client. The client must validate arguments itself.
- A `session_id` that names no session fails with `session_not_found`, although the documentation says the server does not check it. A `block_id` that names no block is passed through unchecked.
- An unknown `client_id` fails with `not_found`. A client that is reconnecting, or already has 16 requests waiting, fails with `client_unavailable` (documented, not exercised).

### Event: `client_perform_requested`

Delivered only to the named client, on its control connection, whatever it is attached to.

```json
{
  "call_id": "3",
  "action": "session.select",
  "args": {"session_id": "session:…"},
  "origin": "cli",
  "session_id": "session:…",
  "block_id": "block:…",
  "requester": {
    "client_id": "client:…",
    "info": {"kind": "cli", "platform": "linux", "name": "rex", "version": "0.1.0-preview.14"},
    "principal": {"kind": "peer", "transport": "unix", "display": "uid 1000", "uid": 1000, "gid": 1000, "pid": 2851534}
  }
}
```

- `call_id` is a counter on that connection: `"1"`, `"2"`, … (**verified**; each new connection started at `"1"`). A reply must therefore never cross a reconnect.
- `args`, `origin`, `session_id` and `block_id` are left out when the request had none. `rex do` sends `origin: "cli"`; it sends `session_id` and `block_id` only for explicit `-s` and `-b`, not from `REX_SESSION` or `REX_BLOCK` (**verified**).

### Identity of the requester

`requester.principal` is filled in by the server from the requester's connection: Unix peer credentials (`uid`, `gid`, `pid`), a Tailscale login (`user_id`, `login_name`, `node`, `tags`), or `anonymous`. `requester.info` is what the requester says about itself and is for display only. `requester.client_id` is not stable enough to key anything on: every `rex` invocation is a new client (**verified**).

The requester cannot fake its principal, but all of it reaches Rex Rune through the host's server. It is exactly as trustworthy as that server. That is why consent in this plan is given per host and not per requester.

### Reply: `client.reply`

```json
{"call_id": "3", "result": null}
{"call_id": "3", "error": {"code": "refused", "message": "…"}}
```

- Exactly one of `result` and `error`. A reply with neither fails with `bad_request` (**verified**), and the documentation says the same of both. Success with nothing to return is an explicit `"result": null`.
- Sent on the connection that received the event. It is not one of the ordered methods, so on the server it never waits behind this connection's `client.key` or `client.action`.
- The requester receives `error.code` unchanged and sees `error.message`; `rex do` prints the message after `Error:` (**verified**). The documentation names `not_supported`, `refused` and `internal`, and gives `not_found` as an example. Other codes pass through too: `bad_request` arrives as a 400, an empty code becomes `internal` (**verified**).
- A reply to a call that already timed out fails with `not_found` (**verified**). The current code logs that and carries on, which stays right.

### Ordering and timeouts

- Requests to one client wait in a line. The server sends the next event only after the previous one is answered or times out (**verified**: a second request sent 0.3 s into a 3 s action was delivered when the first was answered). So at most one request is outstanding, unless an earlier one timed out unanswered.
- `timeout_ms` counts from when a request reaches the front of the line.
- On timeout the requester gets `timeout` and the client is told nothing. `rex do` prints "did not answer within 10s; the action may still run" (**verified**). A slow client can therefore perform an action after its requester gave up.
- When a request ends, connections attached with `event_scope: all` receive `client_performed` with the outcome: `performed`, `refused` (for `refused`, `not_supported` and `bad_request`), `failed` or `timeout` (**verified**). Rex Rune attaches with that scope, so it also sees the events for other clients; it has no use for them.

### What Rex's skill expects of an app

From `rex skill show` in preview.14:

- "Client actions need Remote Control, which is off by default. `actions` still lists them while it's off. If a client refuses, show the user its message. Don't change the setting yourself."
- "Check the client's action list before calling an action, because available actions vary by app and version."
- "To show a host's session, select a client on that host. Clients connect to each host separately."
- Its examples: `session.select session_id=SESSION`, optionally with `window_id=`; `client.tab.goto index=2`, or `index=-1` for the last tab; `client.sidebar.toggle`; and `client.font.*` and `client.config.reload`, which Rex Rune does not have.

The Mac app has one switch, Settings → Server → Client Info → **Remote Control** ("Let Rex and other clients perform actions in this app"), and next to it a **Label** ("Name this Mac in client lists and for targeting it from rex commands").

## 3. Safety model

### What the requester gains

Anyone who can send `client.perform` is already a client of H's server, and can already create, close, read and type into every terminal there through the server API. So an action that only changes H's server state gives them nothing new. What a client action can reach that the server API cannot is **this machine and this window**:

1. **The clipboard.** `pane.paste` types the local clipboard into a terminal on the server; `pane.copy` replaces the local clipboard with terminal text. One leaks local data to the host, the other plants text the user may paste elsewhere.
2. **The keyboard.** An action that opens a dialog, a popup or a field takes the keys the user is typing. Keys meant for a shell would land in a rename field or a confirmation.
3. **Other hosts.** The window is attached to several servers. `pane.send_key`, `client.close` and the rest act on whatever is focused, which may be a terminal on a different host from the one that asked. Performed blindly, the client would let host A act on host B.
4. **Saved preferences.** Interface zoom is written to the settings file and applies to every host.
5. **Attention.** Any navigation moves the user's view and where their next key goes.

The model below removes 1 to 4 completely and bounds 5.

### Rules

1. **Default deny, decided per action in code.** Each registered action carries a remote class. The class is an exhaustive `match` in the registry, so adding an action does not compile until someone decides it, and a test pins the allowed set.
2. **A request from H affects only H.** `session.select` makes H the host on screen and shows a session of H. Every other allowed action is performed only while H is already on screen.
3. **Never a different target.** A `session_id` on the request must be the session on screen, and a `block_id` the focused pane, or the request is refused. This is the rule queued client actions already follow.
4. **Never behind a dialog or popup.** While any panel is open, requests are refused. This also means opening Settings stops a misbehaving requester at once.
5. **No keyboard capture, no clipboard, no typing, no creating, no closing, no preferences.** Those actions are refused whatever the setting is.
6. **Consent is per host and off by default.** Changing a host's endpoint turns it off again, since a new endpoint is a different server.
7. **The requester is shown, not judged.** Its name and principal appear in the notice and the log. `origin` is caller-supplied and never affects the decision.

### The 32 actions

**Performed when Remote control is on for the host (8):**

| Action | What happens |
| --- | --- |
| `session.select` (new) | Show the session, and the tab if `window_id` is given. Switches the window to H if needed. |
| `client.tab.goto` | Focus tab `index` of the session on screen. |
| `client.tab.next`, `client.tab.previous` | Focus the neighbouring tab. |
| `pane.focus` | Move pane focus in a direction. |
| `pane.zoom` | Toggle zoom on the focused pane. |
| `pane.scroll` | Scroll the focused terminal by lines. |
| `client.sidebar.toggle` | Show or hide the sidebar. |

None of these creates, closes, types, reads the clipboard or opens anything. Performed late or twice, the worst outcome is a view the user did not expect.

**Never performed remotely (24), answered `not_supported`:**

| Actions | Reason | What the refusal suggests |
| --- | --- | --- |
| `pane.paste`, `pane.copy` | Local clipboard. | `rex send`, `rex capture` |
| `pane.send_key` | Types as this user into whatever is focused. | `rex send-key` |
| `session.new` | Creates a session. Creation is always an explicit user action. | `rex new` |
| `client.tab.new`, `pane.split` | Create terminals. | `rex window new`, `rex split` |
| `client.close`, `client.tab.close`, `pane.close` | Destructive, and open a confirmation that takes the keyboard. | `rex block close`, `rex window close` |
| `client.tab.rename`, `session.rename` | Open a field that takes the keyboard. | `rex window rename`, `rex session rename` |
| `client.settings`, `client.palette.toggle`, `session.switch`, `pane.actions`, `client.search` | Open a dialog, popup or field that takes the keyboard. | None: these need the person. |
| `client.zoom.in`, `client.zoom.out`, `client.zoom.reset` | Change a saved preference that applies to every host. | None. |
| `client.tab.next_global`, `client.tab.previous_global`, `client.history.back`, `client.history.forward` | Move the window to other hosts. | `session.select` |
| `pane.reconnect` | Transport control with no use to a requester. | None. |

There is no quit action in the registry, and none is added.

All 32 stay in the list sent with `client.keymap`. The server needs the full list to check the user's key bindings, and the skill expects `actions` to list everything even while Remote control is off. Each never-remote action gets a `description` that says so, so `rex -C CLIENT actions --json` shows it before an agent tries.

### Reply codes

| Situation | `error.code` | Notes |
| --- | --- | --- |
| Action is never performed remotely, or unknown | `not_supported` | Checked first, so an agent is never told to ask for a setting that would not help. |
| Remote control is off for the host | `refused` | "Remote control is off for this host in Rex Rune. The person at that computer can turn it on in Settings → Remote Hosts." |
| Arguments fail the action's own check | `bad_request` | Nothing is performed. |
| A dialog is open, the host is not on screen, the named session or pane is not the current one, or the person is typing | `refused` | The message says which, and that nothing was done. |
| `session.select` names a session or tab the host does not have | `not_found` | |
| Navigation failed, or the app did not act in time | `internal` | |
| Performed | `result: null` | |

### Options considered

| Option | Verdict |
| --- | --- |
| A prompt for each request | No. It takes the keyboard, which is the hazard being avoided; callers time out after 10 seconds; agents retry. |
| One global switch, as in the Mac app | Workable, but Rex Rune shows several servers in one window and each is its own trust decision. A global switch would let every saved host steer the window as soon as one needed it. |
| **A switch per host, off by default** | Recommended. Stored as `"remote_control": true` on the host in the settings file. |
| An action allowlist in the config file | Not now. The built-in set is small and fixed. A second tier (new tab, split) could be added later as a second switch if it is wanted. |
| Filtering by requester principal | Not now. The main requester is `rex` over the host's Unix socket, whose principal (a uid on H) cannot be compared with this client's when it connects over Tailscale. Per-host consent already covers "everyone this server admits". See question 7. |

## 4. UX

### Turning it on, off and seeing it

- **Settings → Remote Hosts.** Each host gets a switch row in the existing pattern: the name **Remote control**, one muted line ("Let `rex do` and other clients on this host switch sessions and tabs in this window"), the switch on the right. Off for every host until the user turns it on, including hosts added later.
- **Commands.** One entry for the host on screen, "Turn on remote control for devbox" or "Turn off remote control for devbox", so revoking is two keystrokes away.
- **Revoking** takes effect on the next request, because the setting is read on the UI thread when a request is handled. Removing a host, or changing its endpoint, clears it.
- **No indicator in the chrome.** DESIGN.md's "quiet when healthy" rules out a badge for a setting that is working as intended. The state lives in Settings; use is announced by notices.

### When a request arrives

- **Performed:** one informational notice, which leaves after about 6 seconds like any other, naming host, requester and action. For example: `devbox: rex (uid 1000) showed “Tests”`. Identical notices coalesce, and at most three notices are kept, as now. A navigation is recorded in the window's history, so mouse Back, or Back in history from Commands, returns to where the user was.
- **Refused because the switch is off:** one informational notice per host per launch: `rex on devbox asked to show a session. Remote control is off for devbox (Settings → Remote Hosts).` It tells the user the setting exists at the moment they would want it, and cannot be turned into spam.
- **Refused for any other reason:** no notice. The requester gets the message; the app writes a log line.
- The window is never raised, focused or flagged for attention by a request, and no desktop notification is sent. The request changes what the window shows, not where the user is looking.
- Terminal content does not animate; a remote navigation looks exactly like a local one.

### What the requester sees

`rex do` prints the refusal message as is, and the skill tells agents to pass it on. So the messages are written for the person: they say what was refused, that nothing was done, and what to use instead or where the setting is.

### How it fits the Phase 6 keymap

- **One registry.** `declarations()` stays the single list. It already feeds key-binding checks, Settings → Shortcuts and Commands; the server now also uses it to accept or reject `client.perform`. `session.select` joins it with a schema (`session_id` required, `window_id` optional), so the server rejects malformed requests before they arrive.
- **Bindings and remote requests are independent.** A request names an action, not a key. Removing or rebinding a shortcut in `shortcuts.json` does not disable the action remotely, and the Remote control switch does not affect any shortcut.
- **Queued client actions are unchanged.** Actions returned by this client's own `client.action` come from the user's key press, run as they do now, and ignore the switch. `rex.client.queue` inside a `rex do` script is different: it arrives as `client.perform` (**verified**) and follows the rules above.
- **Pending sequences.** A remote navigation cancels a pending key sequence, as a local host, session or focus change already does.
- **`session.select` locally.** It can be bound in `shortcuts.json` with a fixed `session_id`, or queued from Lua. It has no entry in Commands, because it needs an argument; the switcher remains the way to choose a session by hand.
- **`client.tab.goto index=-1`** is accepted as the last tab, as the Mac app and the skill use it.

## 5. Implementation plan

### How a request flows

The host worker in `src/connection.rs` runs one serial loop per host; the workspace lives on the UI thread. A request has to cross from one to the other and back.

1. The worker reads `client_perform_requested`, parses it and puts it in `HostModel.performs`, tagged with the connection's epoch. It wakes the UI as it does for any model change.
2. `Workspace::sync` takes the list, as it already takes `client_actions`, and hands each request to the policy: class, host switch, arguments, dialog, host on screen, target. The request is then performed through the existing `actions::run`, or refused.
3. The UI sends `HostCommand::PerformReply { epoch, call_id, outcome }`. The worker sends `client.reply` with the existing 8-second timeout and error handling: never retried, and an API error or timeout does not disconnect the host.
4. `session.select` replies when its navigation is acknowledged (the existing `navigation_result`), or with `internal` after 5 seconds. That makes its success mean "the session is showing", which an agent has no other way to check.

Guards in the worker:

- **No UI answer in 5 seconds:** if the request has not been taken, the worker removes it and replies `internal`, so it cannot run later. If the UI has taken it, the worker waits for the answer.
- **Reconnect:** untaken requests are dropped, and a reply carrying an older epoch is discarded. Without this, a late reply with `call_id` `"1"` could answer a different request on the new connection.
- **Busy loop** (see question 4): the worker is serial, so a request can sit unread behind a slow call such as a file upload or a 35-second Lua action. If the loop was busy for more than 5 seconds when the request is found, the worker replies `internal` ("Rex Rune was busy; nothing was done") and does not queue it.

No change to the vendored SDK is needed: `client::Reply` and `ReplyError` exist, and the event is parsed with a small `serde` struct in the client that ignores unknown fields. No API revision gate is needed either; a server without `client.perform` never sends the event.

### PR 1: Remote control, the switch and the existing actions

Roughly 500 lines of source and 450 of tests.

| File | Change |
| --- | --- |
| `src/connection/perform.rs` (new) | `Request`, `Requester`, `Outcome`, parsing, building the reply. The reply code moves here from `handle_event`. |
| `src/connection.rs` | `HostModel.performs`; `HostCommand::PerformReply`; epoch; the 5-second and busy-loop guards; a silent arm for `PerformReply` while disconnected. |
| `src/app/actions.rs` | `Spec::remote()`, an exhaustive `match` on `Handler`; `description` for never-remote actions in `declarations()`. |
| `src/app/remote.rs` (new) | The policy as one function from (action, arguments, host setting, workspace context) to perform or refuse, with messages; the once-per-host notice. |
| `src/app/workspace.rs` | Take `performs` in `sync`; clear the switch when an endpoint changes; the time of the last terminal key (question 3). |
| `src/settings.rs` | `Host.remote_control`, default false, left out of the file when false. |
| `src/app/settings_view.rs`, `src/app/panel.rs` | The switch row; the Commands entry. |
| `README.md`, `PRODUCT.md`, `DESIGN.md`, `AGENTS.md`, `CHANGELOG.md` | Replace the "not implemented" paragraphs with the setting, the allowed set and the refusals. Add a convention: remote availability is decided in `Spec::remote()`, default deny. |
| `scripts/keymap-acceptance.py` | Its refusal check uses `client.tab.new`, which stays `not_supported`; update the expected message. |

With default settings nothing new is performed. What a requester sees changes in two ways: the seven allowed actions answer `refused` with the Remote control message instead of `not_supported`, and the other refusals are reworded to name the `rex` command to use.

### PR 2: `session.select`

Roughly 250 lines of source and 300 of tests, on top of PR 1.

| File | Change |
| --- | --- |
| `src/app/actions.rs` | `session.select` with its schema and argument check; `client.tab.goto` accepts `-1`. |
| `src/app/remote.rs`, `src/app/navigation.rs` | Start the navigation through the existing `navigate_to`, switching host when needed; hold the reply until `navigation_result` or 5 seconds; `not_found` for an unknown session or tab. |
| `scripts/remote-control-acceptance.py` (new) | The live check below. |
| `README.md`, `CHANGELOG.md` | The action, the `rex` commands that reach it, and the live result. |

### PR 3 (optional): client label

About 60 lines. Send a label in HELLO so `rex -C LABEL` can address this app when several apps are connected. See question 6.

### Tests

**Mock server** (the WebSocket harness in `src/connection/keymap_tests.rs`):

- A request is parsed and queued with its requester, for an unattached session and below the bootstrap sequence. An event without `call_id` is ignored.
- A success reply is `{"call_id", "result": null}` with `result` present and no `error`; each error code and message goes out as given.
- `not_found`, `bad_request` and `internal` responses to the reply, and no response at all, leave the host connected, and the reply is not retried (the existing tests, adapted).
- No answer from the UI in 5 seconds gives `internal`, and the request is gone from the model.
- A request found after the loop was busy for more than 5 seconds is answered `internal` and not queued.
- After a reconnect, an old reply is not sent and untaken requests are dropped.

**Headless UI** (`src/app/`):

- Every registered action has a class; the allowed set is exactly the eight names.
- Switch off: each allowed action gives `refused`, nothing in the workspace changes, no command is sent, and there is one notice per host. Never-remote actions give `not_supported` with the switch on and off.
- Each never-remote action is checked for its hazard: `pane.paste` does not read the clipboard, `pane.send_key` sends nothing to a terminal, `session.new` sends no create, `client.close` opens no dialog.
- Switch on: each allowed action does what its shortcut does and replies `null`.
- Bad arguments give `bad_request` and nothing runs.
- Refused, with the workspace unchanged: a panel is open; the host is not on screen; `session_id` or `block_id` names something else; a key was typed a moment ago.
- `session.select`: switches host; unknown session and unknown tab give `not_found`; the reply waits for the navigation; a failed navigation gives `internal`; a disconnect before the navigation completes sends no reply; otherwise exactly one reply.
- Settings: an old file loads with the switch off; the value survives a save and load; an endpoint change clears it.
- The switch row in Settings → Remote Hosts is checked in a rendered build (`-- --snap`), as AGENTS.md asks for UI changes.

**Live** (`scripts/remote-control-acceptance.py`, built binary, Xvfb, two private preview.14 servers on their own sockets and directories, started and stopped by PID, as `keymap-acceptance.py` does):

- `rex -C CLIENT actions` lists `session.select` with `session_id, window_id?`.
- Switch off: `rex do session.select …` exits non-zero and prints the Remote control message; the window title is unchanged.
- Switch on in the test's own `--config` file: `session.select` on the second server makes the window show that host and session (read from the window title); `client.tab.goto index=2` and `index=-1` change the active tab (read with `rex session inspect`); `client.sidebar.toggle` succeeds.
- `client.tab.new`, `pane.paste` and `pane.send_key` exit non-zero with their messages; the tab count and the terminal's contents are unchanged.
- A real `ssh://` host is checked only if `sshd` is available on the test machine. Otherwise the README says cross-host behaviour was checked between two local servers.

Each PR also runs the suite in AGENTS.md: `cargo test`, `fmt`, `clippy -D warnings`, the SDK tests, and `keymap-acceptance.py`.

### Risks

| Risk | Handling |
| --- | --- |
| A requester moves the view while the user is typing, and keys land in another terminal. | Question 3's guard; no dialogs are ever opened; requests are refused while a panel is open. |
| A request is performed long after its requester gave up. | The 5-second UI limit, the busy-loop guard, and an allowed set that is harmless when late. |
| A reply answers the wrong request after a reconnect. | Epoch on every request and reply. |
| A new action becomes remotely callable by accident. | The exhaustive `match` and the pinned-set test. |
| A request loop makes the window flap. | Requests arrive one at a time; notices coalesce; opening Settings refuses everything; the switch is right there. No rate limit is planned. |
| The server's behaviour differs from its documentation (it checks `session_id`; error codes pass through). | The client treats every field as unchecked and validates it. `bad_request` and `not_found` are the only codes that rely on pass-through; if a server ever rewrote them, the requester would still get an error and a message. |
| Rex changes the contract in a later preview. | The live script runs against the pinned server; `docs/server-preview14.md` is the model for re-checking on a bump. |
| Scope growth toward the Mac app's full action list. | The principle in section 1: perform only what the server API cannot do. |

## 6. Open questions

1. **Is the allowed set right?** Recommended: the eight actions above. Leave out new tab and split (use `rex window new` and `rex split`), and interface zoom (a saved preference for every host; the skill's "make the text bigger" is the Mac's terminal font action, which Rex Rune does not have). Either could become a second, separate switch later.
2. **Per host, and off everywhere?** Recommended: a switch per saved host, off by default, including for this machine's own server. A local server's requesters already run as you, so on-by-default there would be defensible, but a window that moves without ever having been asked is the wrong first experience, and off matches the Mac app and the skill's wording.
3. **Should a request wait while you are typing?** The failure: an agent finishes background work and shows its session while you are mid-command in another, and your next keys go to the wrong terminal. Likely enough once agents use this unprompted. Recommended: yes. Refuse navigation (`session.select`, the tab actions, `pane.focus`) for 1.5 seconds after the last key sent to a terminal, with a message saying to try again. Cost: one timestamp and one comparison.
4. **Should requests found after a busy spell be dropped?** The failure: a request arrives while the host's serial loop is waiting on an upload or a slow Lua action; the requester times out; when the loop frees up, the window jumps to another session, possibly a minute later. Unlikely, since it needs the two to overlap on one host, and harmless, but surprising. Recommended: yes, the 5-second check in section 5. Cost: about ten lines in the worker loop and one test. Without it, the late action is simply performed.
5. **How loud should a performed request be?** Recommended: one 6-second notice for every performed request, coalesced. Alternatives: notices for navigation only, or none once the switch is on.
6. **Give this client a label?** With the Mac app and Rex Rune both attached, `rex do` without `-C` is ambiguous, and Rex Rune shows in `rex client ls` with an empty label, so an agent has to copy an ID that changes on every launch. Recommended: yes, as PR 3. Default to this machine's hostname, editable in Settings. It needs your say because the label is visible to everyone on each server the client attaches to.
7. **Filter requesters beyond the host switch?** For example, always refuse `anonymous` principals or tagged Tailscale devices. Recommended: no. On a server that admits anonymous clients, they can already type into every terminal, and the allowed set gives them less than that. Revisit if a second tier is added.
8. **`client_call_requested`?** Recommended: keep it out of this work and track it separately. It makes the client a relay between servers, which is a different permission from steering a window.
9. **Stack and release?** Recommended: PR 1 then PR 2, released together as 0.5.0 so the setting never ships without `session.select`; PR 3 if question 6 is yes.

## Appendix: how the contract was checked

The preview.14 binary already extracted under `~/.cache/agent-ws/rex-preview14/bin` for #10 was reused (archive SHA-256 `36ece396…ff940a`, as recorded in `docs/server-preview14.md`); no installer was run and no existing Rex server, package or service was touched. A foreground server ran on a private Unix socket with its own home, XDG and state directories under `~/.cache/agent-ws/rex-rune-issue12/`, with `--restore=false` and an empty Lua config, and was stopped by PID. Every `rex` call used that socket explicitly with `--autostart=false`.

A short Python WebSocket script connected as an `app` client, declared a handful of actions through `client.keymap` (two with Rex Rune's names, the rest test stand-ins), logged every `client_perform_requested` it received, and answered in scripted ways (success, each error code, late, never, malformed). A second connection attached with `event_scope: all` to log `client_performed`. The requests came from `rex -C … do`, `rex do` with no client named, `rex do` scripts using `rex.client.queue`, and `rex api call client.perform`.

The script and its logs are scratch files, not part of the repository. PR 2's acceptance script replaces them with a check that drives the real app.
