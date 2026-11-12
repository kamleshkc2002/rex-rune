# Rex Linux patch

This is the crates.io `ghostty-vt` 0.8.0+ghostty.1.4.0-main.83fb956 source from
`prabirshrestha/gpui-cn`, commit `e49d909b2ceae040714aa53cd38f97faa4f85cd2`,
path `crates/ghostty-vt`. The original README and license files are preserved.

The pinned C bindings expose the render-state image iterator, but the Rust
render wrapper does not. `src/render/images.rs` adds a safe wrapper returning
owned placement geometry. It delegates fixed, relative and Unicode-placeholder
placement resolution to Ghostty, rather than implementing Kitty positioning
again in Rex. Unknown image protocols are skipped. The linked Ghostty source,
sys crate, ABI and version are unchanged.

`ScrollViewport::DeltaPrompt` exposes the pinned C API's existing
`GHOSTTY_SCROLL_VIEWPORT_DELTA_PROMPT` operation. Ghostty owns prompt grouping,
continuations and viewport clamping; no new ABI, protocol or mark index is added.
Remove this wrapper addition when upstream provides it. Client tests in
`tests/prompt_navigation.rs` and the disposable `scripts/prompt-acceptance.py`
exercise the operation on both locally fed and Rex-restored terminals.

Additional fixes required by the same image pipeline:

- `Decoder::next_record` consumes history pages and image records without
  querying page-only progress fields. The previous client loop rejected image
  snapshots because those fields correctly report `NO_VALUE` for images.
- Temporary-file medium setters/getters now use the C API's `GhosttyString`,
  not a `bool`. The incorrect getter overwrote the caller's stack. The client
  disables this medium, ordinary files and shared memory on restored terminals.
- The optional PNG decoder normalizes grayscale and grayscale-alpha output to
  RGBA8. PNG's expansion transform alone does not perform this conversion.

The client installs a dimension-bounded PNG decoder on each terminal worker
thread and caps decoded image storage at 64 MiB per terminal. Remove these
patches when the upstream wrapper provides equivalent fixes.
