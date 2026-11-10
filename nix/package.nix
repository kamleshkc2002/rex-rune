{
  lib,
  rustPlatform,
  fetchurl,
  runCommand,
  zig,
  pkgs,
  pkg-config,
  makeWrapper,
  fontconfig,
  libxkbcommon,
  wayland,
  libGL,
  vulkan-loader,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
}:
let
  ghosttyArchive = fetchurl {
    url = "https://tip.files.ghostty.org/83fb95624b0439ee660c2679b43b38c8b63769cd/libghostty-vt-source.tar.gz";
    hash = "sha256-7Veum5VYHIjyYU6XbB0/FGwC1EOjwmE5IA653m5kXrQ=";
  };
  ghosttySource = runCommand "rex-linux-ghostty-83fb956" { } ''
    mkdir -p "$out"
    tar -xzf ${ghosttyArchive} --strip-components=1 -C "$out"
  '';
  ghosttyDepArchives = (pkgs.zig.fetchDeps.override { inherit zig; }) {
    pname = "rex-linux-ghostty";
    version = "83fb956";
    src = ghosttyArchive;
    # VT enables lazy wuffs/highway dependencies after build configuration.
    fetchAll = true;
    hash = "sha256-82Ife6Av0wfZLmovdEdJFgsG5MI4swCeOB3dbvgBDSU=";
  };
  # Zig 0.17 caches archives, but --system expects extracted package directories.
  ghosttyDeps = runCommand "rex-linux-ghostty-83fb956-system-deps" { } ''
    mkdir -p "$out"
    for archive in ${ghosttyDepArchives}/*.tar.gz; do
      tar -xzf "$archive" -C "$out"
    done
  '';
  runtimeLibraries = [
    fontconfig
    libxkbcommon
    wayland
    libGL
    vulkan-loader
    libx11
    libxcursor
    libxi
    libxrandr
  ];
in
rustPlatform.buildRustPackage {
  pname = "rex-linux";
  version = "0.5.2";
  src = lib.cleanSourceWith {
    src = lib.cleanSource ../.;
    # Keep local build trees out of `path:` builds.
    filter =
      path: type:
      !(
        type == "directory"
        && builtins.elem (baseNameOf path) [
          "target"
          ".mise"
        ]
      );
  };
  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "rune-kit-0.0.1" = "sha256-qWQd1il3ccR69j7jQY3Whl3L7odgkaA8RvP+OVY5stA=";
    };
  };
  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];
  buildInputs = [
    fontconfig
    libxkbcommon
    wayland
  ];
  cargoBuildFlags = [ "--bins" ];
  cargoTestFlags = [ "--all-features" ];
  nativeCheckInputs = [
    pkgs.clippy
    pkgs.rustfmt
    pkgs.python3
    fontconfig
  ];

  preCheck = ''
    cargo fmt --all -- --check
    cargo clippy --offline --locked --release --all-targets --all-features -- -D warnings
  '';

  postCheck = ''
    REX_PROBE="target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/rex-probe" \
      python3 scripts/connection-only-smoke.py
  '';

  # Cargo's Ghostty build must not fetch sources or Zig packages in the sandbox.
  GHOSTTY_SOURCE_DIR = ghosttySource;
  GHOSTTY_ZIG_SYSTEM_DIR = ghosttyDeps;
  ZIG = "${zig}/bin/zig";

  preBuild = ''
    # Zig still needs writable compiler artifacts when dependencies are offline.
    export ZIG_GLOBAL_CACHE_DIR="$TMPDIR/zig-cache"
    mkdir -p "$ZIG_GLOBAL_CACHE_DIR"
  '';

  postInstall = ''
    install -Dm644 resources/dev.shuv.rex-linux.desktop \
      "$out/share/applications/dev.shuv.rex-linux.desktop"
    substituteInPlace "$out/share/applications/dev.shuv.rex-linux.desktop" \
      --replace-fail 'Exec=rex-linux' "Exec=$out/bin/rex-linux"
    for size in 32 128 256; do
      install -Dm644 "resources/icons/''${size}x''${size}/apps/dev.shuv.rex-linux.png" \
        "$out/share/icons/hicolor/''${size}x''${size}/apps/dev.shuv.rex-linux.png"
    done
    wrapProgram "$out/bin/rex-linux" \
      --prefix PATH : ${lib.makeBinPath [ fontconfig ]} \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibraries}
  '';
  meta = {
    description = "Native Linux Rune client for existing Rex terminal servers";
    homepage = "https://github.com/shuv1337/rex-rune";
    # The inherited Rex icon has separate provenance/terms; see its README.
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "rex-linux";
  };
}
