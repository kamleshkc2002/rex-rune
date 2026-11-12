# Rex Rune SDK patch

Vendored from `https://github.com/a10y/rexrs` at
`780be3f9f0455c758273e57199af30bdaa145cf1`. The upstream manifest declares
`MIT OR Apache-2.0`; that revision ships no license text files, so the standard
`LICENSE-MIT` and `LICENSE-APACHE` texts were added here.

Local changes:

- `codegen/help.json` and `version.json` capture Rex `0.0.1794323808-g757adf`,
  commit `757adf99fad394c515be6313aa5dbe17008dc7d2`, API revision 13 (`rex api list --json` and `server.version` from a disposable
  server). `python3 codegen/generate.py` regenerates the committed types and
  example tests. The API additions since upstream are
  `session.bootstrap.data_tickets` (8), layer `state` and
  `session.set_layer_state` (9), the Tailscale `control_url` (10) and
  `terminal.upload_begin` (11), and optional `listeners` in `server.status`
  and `session.bootstrap.server_status` (12), and `terminal.download_begin` (13).
  Revision 13 adds only that terminal method; existing methods, events, auth,
  handshake, keymap and action contracts are unchanged. No upstream revision
  bump or generator changes were needed. See `docs/server-api13.md` and
  `docs/server-preview14.md` in the client repository for the comparisons.
- `ClientBuilder::api_revisions(min..=max)` passes an explicit supported range
  to HELLO. The SDK default still advertises only its generated revision.
  Rex Rune advertises 10–13, validates WELCOME and gates uploads on 11.
- A focused negotiation test accompanies the builder change.
- Hand-written `tests/listeners.rs` covers the new optional listener metadata in
  status and bootstrap, including omitted/null lists and unknown enum-like strings.
- Hand-written `tests/download.rs` covers API-13 request serialization, optional
  cache validators, ticket-bearing and not-modified responses, and required
  metadata. The app does not invoke downloads or advertise a download UI action.
- The generator preserves `attach_session_ids` as `Option<Vec<String>>`:
  omitting it selects a default session, while an explicit `[]` attaches none.
  A serialization test covers this distinction.

Tests: `cargo test --manifest-path vendor/rexrs/Cargo.toml --all-features`.

Remove this patch once upstream has equivalent generated types and negotiation.
