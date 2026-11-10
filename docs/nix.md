# Linux Nix packaging

The flake exposes `packages.<system>.rex-linux` (also `default`), a development shell, a formatter and a package check for `x86_64-linux` and `aarch64-linux`. An output being exposed does not mean it has been built on that architecture.

From a checkout:

```sh
nix build .#rex-linux
./result/bin/rex-linux --help
nix develop
cargo test --locked --all-features
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

`nix build .#rex-linux` uses only files tracked by git; use `path:.#rex-linux` to include uncommitted files. Another flake can take this repository as an input (`github:shuv1337/rex-rune`) and install `inputs.rex-rune.packages.${pkgs.stdenv.hostPlatform.system}.default`, for example in a Home Manager configuration. Keep this input's own nixpkgs pin: the build needs a newer Rust and Zig than some system configurations provide.

The package installs `rex-linux` and the `rex-probe` / `rex-workspace-probe` diagnostic binaries, plus the desktop entry and icons. It wraps the GUI with the Linux graphics and input libraries and font discovery. The development shell puts the same runtime libraries, including the Vulkan loader, in `LD_LIBRARY_PATH` so `cargo run --locked --bin rex-linux` can load them. Your system must still provide a working Vulkan graphics driver. After a flake change, exit and enter `nix develop` again to update the environment. Run the probes only against disposable servers: they create and remove their own sessions.

The package's check phase runs rustfmt, Clippy with warnings denied, the Rust unit and headless UI tests, and `scripts/connection-only-smoke.py`, which checks that a missing or stale socket never makes the client execute `rex`. The icon has its own provenance and licensing notice in `resources/icons/README.md`; the application's MIT license does not cover that artwork.

## Staying current

| Install | Update |
| --- | --- |
| Flake input `github:shuv1337/rex-rune` | `nix flake update rex-rune`, then rebuild your configuration |
| `nix profile install github:shuv1337/rex-rune#rex-linux` | `nix profile upgrade rex-linux` (use the name shown by `nix profile list`) |
| Tag pin `github:shuv1337/rex-rune/vX.Y.Z#rex-linux` | Change the tag to the desired release, update the lock, then rebuild |

The unqualified GitHub reference follows **master**, which can be ahead of a release.
There is no `stable` branch. Nix builds set `REX_LINUX_CHANNEL=nix`; the app never
replaces a Nix store binary. This release supplies provenance only: the in-app
update notice is a later roadmap item, not an implemented feature.

## Reproducibility

- `flake.lock` pins nixpkgs and the Zig overlay (Zig 0.17.1). `Cargo.lock` pins Rust dependencies, including Rune's git source; fixed-output hashes protect fetched git and Zig dependencies.
- The vendored crates (`vendor/rune`, `vendor/rexrs`, `vendor/ghostty-vt`) are path dependencies and are built from the checkout.
- The Ghostty source archive is pinned to the same `83fb95624b0439ee660c2679b43b38c8b63769cd` revision and SHA-256 as `ghostty-vt-sys` 0.8.0's `GHOSTTY.lock`.
- Ghostty's sources and all lazy Zig dependencies are fetched before the main build, and Zig's cached archives are extracted into the layout `--system` expects. Cargo's build script receives `GHOSTTY_SOURCE_DIR`, `GHOSTTY_ZIG_SYSTEM_DIR` and `ZIG`, so the build needs no network access inside the sandbox.
- When updating `ghostty-vt`, update the archive pin and the Zig dependency hash together. When updating Rune, update the `rune-kit` git dependency hash and review the vendored patches.

## Session safety

Installing or starting the package never starts, replaces, reconfigures or stops a Rex server. It connects to your existing `REX_SERVER` endpoint or the default local socket; run the Rex server separately. Tests and probes that create sessions must use disposable servers, never your daily sessions.
