# README captures

The README images show Rex Rune built from `origin/master` at
`9af5bf5d0a7cc1ba5c61e2d2784035a66e30e19b` (0.7.4). They are native X11
captures with real fonts, not mock-ups or Rune's headless test-font renderer.

| Image | What it shows |
| --- | --- |
| `screenshot.png` | Midnight theme, sidebar, three tabs and three panes in the fictional Orbit workspace |
| `images/settings-search.png` | Search results alongside toolbar, pane-outline and header-fading settings |
| `images/settings-fonts.png` | Advanced font metrics, live preview and ordered Noto fallbacks |
| `images/settings-tabs.png` | Three icons with Always colour, the shared preview, tab fill and download folder |

Only the last shot changes the icon preferences: the defaults are one icon and
Never colour. The hero retains default theme, font size, padding and pane chrome.
The two Noto fallback families are example preferences. Orbit's files and check
output are fixtures, not this repository's source or test results.

## Reproduce

Build a clean checkout of the source revision above, then run:

```sh
cargo build --locked --release --bin rex-rune
REX_BIN="$PWD/target/release/rex-rune" python3 scripts/readme-gallery.py
oxipng -o 4 --strip all docs/screenshot.png docs/images/settings-*.png
```

Requirements: Rex server API 13, Xvfb, openbox, xdotool, ImageMagick `import`,
`bat`, `eza`, Python with Pillow, and (for the exact font stack) JetBrainsMono
Nerd Font, Noto Sans Mono and Noto Sans CJK JP. `SHOT_OUT_DIR` redirects the
output directory. `SHOT_KEEP=1` retains private logs for troubleshooting; do
not publish those logs. No recording or real desktop is used.

The script combines the private-server fixture approach of
`readme-screenshot.py` and `docs-captures.py` with the real X11 driver from
`rune-acceptance-x11.py`. It creates its own HOME, XDG directories, config,
server socket, sessions and display, disables tailnet discovery and update
checks, and stops its processes. Every Rex call names that socket with
autostart off. It does not use the personal environment inherited by
`demo-record.py`'s fixture shells.

## Verification and hygiene

- Built the current source with `cargo build --locked --offline --release --bin rex-rune`
  in a disposable WTFM clone, with a temporary HOME and network disabled.
- WTFM command records passed for format checking, release build, app help,
  Python compilation and the native capture script. This documentation-only
  run did not repeat the full Rust test suite or live updater acceptance.
- Captured a 1280×800 workspace; cropped each Settings image to its 840×704
  dialog area. Applied lossless PNG optimization and stripped metadata.
  No UI text, terminal pixels or update state was composited into the images.
- Reviewed all four images and their OCR text: only fictional project/host
  labels, generic paths and the product's built-in preview text appear.
  No personal hosts, usernames, paths, IP addresses or credentials appear.
- Ran WTFM scrub, PII and README link/style lint reports. Existing historical
  docs and product references to coding tools are outside this screenshot
  refresh; required contributor credits and bespoke prose are preserved.
  No changelog, version stamps, security report or vendor files were rewritten.

The existing `.wtfm.yaml` remains the command manifest. WTFM's capture planner
supports `capture.web`, not desktop windows, and returns no pages for this
project even with `--force`. Its browser capture/record phase is therefore
skipped, not treated as a successful capture. This file and
[`images/readme-captures.json`](images/readme-captures.json) record the native
capture source, sizes and hashes instead.
