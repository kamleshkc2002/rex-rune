#!/usr/bin/env bash
# End to end, in a scratch HOME: the updater from Rex Linux 0.7.5 installs this checkout's
# release build from its bridge tarball, then this client's first start gives the
# installation its rex-rune names. Run it before the release that carries the rename, and
# after any change to the bridge, the manifest or update::installer.
#
#   REX_RUNE_CHANNEL=official cargo build --locked --release --bin rex-rune
#   bash scripts/bridge-e2e.sh
#
# The 0.7.5 updater is built from the v0.7.5 tag with one change: it downloads the signed
# URL's file from a loopback fixture instead of GitHub. Its manifest and signature checks,
# archive rules, version check, launcher step and transaction are that release's own. The
# fixture is signed with a throwaway minisign key, which that updater is given to trust.
#
# Needs git, the Rust and Zig toolchains, minisign, desktop-file-utils, python3, network
# access for the v0.7.5 clone, and a GPU or software renderer for the headless first start.
# It takes a few minutes to build the old updater; REX_BRIDGE_OLD=<dir> keeps and reuses
# that checkout. Nothing outside a temporary directory (and dist/) is written, and no Rex
# server is contacted.
set -euo pipefail
new="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/rex-bridge-e2e.XXXXXX")"
server=""
trap '[[ -z "$server" ]] || kill "$server" 2>/dev/null || true; rm -rf -- "$work"' EXIT
old="${REX_BRIDGE_OLD:-$work/v0.7.5}"
version="$(python3 -c 'import tomllib,sys; print(tomllib.load(open(sys.argv[1],"rb"))["package"]["version"])' "$new/Cargo.toml")"
arch="$(uname -m)"
[[ -x "$new/target/release/rex-rune" ]] || { echo "Build first: cargo build --locked --release --bin rex-rune" >&2; exit 1; }

echo "== 1. the 0.7.5 updater, with only its download transport substituted"
if [[ ! -x "$old/target/release/examples/bridge_harness" ]]; then
    [[ -d "$old/.git" ]] || git clone -q --depth 1 --branch v0.7.5 https://github.com/shuv1337/rex-rune.git "$old"
    python3 - "$old" <<'PY'
import pathlib, sys
root = pathlib.Path(sys.argv[1])
path = root / "src/update/installer.rs"
text = path.read_text()
before = """    let url = validate_fetch_url(&artifact.url, false)?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)"""
after = """    let base = std::env::var("REX_BRIDGE_TEST_BASE").expect("REX_BRIDGE_TEST_BASE");
    let name = artifact.url.rsplit('/').next().unwrap();
    let url = validate_fetch_url(&format!("{base}/{name}"), true)?;
    eprintln!("   {} served from {url}", artifact.url);
    let _ = rustls::crypto::ring::default_provider().install_default();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(false)"""
if after not in text:
    assert text.count(before) == 1, "v0.7.5's download function is not as expected"
    path.write_text(text.replace(before, after))
(root / "examples/bridge_harness.rs").write_text('''//! Rex Linux 0.7.5's updater, called as Settings > Updates > Update now calls it.
fn main() {
    let argument = std::env::args().nth(1).unwrap();
    if argument == "--version" {
        println!("rex-linux 0.7.4");
        return;
    }
    let base = std::env::var("REX_BRIDGE_TEST_BASE").unwrap();
    let key = std::fs::read_to_string(std::env::var("REX_BRIDGE_TEST_KEY").unwrap()).unwrap();
    let home = std::env::var("HOME").unwrap();
    let config = rex_linux::update::CheckConfig {
        current_version: "0.7.4".into(),
        manifest_url: format!("{base}/latest.json"),
        signature_url: format!("{base}/latest.json.minisig"),
        public_keys: vec![key],
        state_path: format!("{home}/.local/state/rex-linux/update.json").into(),
        allow_loopback_http: true,
    };
    eprintln!("   install channel: {:?}", rex_linux::update::InstallChannel::detect());
    let mut last = String::new();
    let result = rex_linux::update::installer::install(&config, &argument, |progress| {
        let phase = format!("{progress:?}");
        let phase = phase.split(' ').next().unwrap().to_owned();
        if phase != last {
            eprintln!("   {phase}");
            last = phase;
        }
    });
    match result {
        Ok(binary) => println!("   installed {}", binary.display()),
        Err(error) => {
            eprintln!("FAILED: {error:#}");
            std::process::exit(1)
        }
    }
}
''')
PY
    (cd "$old" && REX_LINUX_CHANNEL=official cargo build --locked --release --no-default-features --example bridge_harness 2>&1 | tail -1)
fi

echo "== 2. this release ($version), packaged and signed as a release is"
(cd "$new" && python3 scripts/package-release.py >/dev/null)
python3 "$new/scripts/test-bridge.py" "$new/dist/rex-linux-$version-$arch-linux-gnu.tar.gz" --binary "$new/target/release/rex-rune"
site="$work/site"
mkdir "$site"
cp "$new/dist/rex-rune-$version-$arch-linux-gnu.tar.gz" "$new/dist/rex-linux-$version-$arch-linux-gnu.tar.gz" "$site/"
other="$([[ "$arch" == x86_64 ]] && echo aarch64 || echo x86_64)"
for name in "rex-rune-$version-$other-linux-gnu.tar.gz" "rex-linux-$version-$other-linux-gnu.tar.gz" \
    "rex-rune_${version}_amd64.deb" "rex-rune_${version}_arm64.deb" \
    "rex-rune-bin-$version-1-x86_64.pkg.tar.zst" "rex-rune-bin-$version-1-aarch64.pkg.tar.zst"; do
    echo "fixture $name" > "$site/$name"
done
(cd "$site" && sha256sum -- *.tar.gz *.deb *.pkg.tar.zst > SHA256SUMS)
python3 "$new/scripts/release-manifest.py" --dist "$site" --version "$version" --update-from 0.4.2
minisign -G -W -p "$work/test.pub" -s "$work/test.key" >/dev/null
minisign -S -s "$work/test.key" -m "$site/latest.json" -t "rex-linux v$version latest.json" >/dev/null
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
(cd "$site" && exec python3 -m http.server "$port" --bind 127.0.0.1 >"$work/http.log" 2>&1) &
server=$!
sleep 1

export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export HOME="$work/home"
unset XDG_DATA_HOME XDG_BIN_HOME XDG_CONFIG_HOME XDG_STATE_HOME XDG_CACHE_HOME
export REX_BRIDGE_TEST_BASE="http://127.0.0.1:$port" REX_BRIDGE_TEST_KEY="$work/test.pub"
bin="$HOME/.local/bin" data="$HOME/.local/share"
files() { (cd "$HOME" && find . \( -type f -o -type l \) -not -name '*.cache' -not -name '*.lock' | sort | sed 's/^/   /'); }

echo "== 3. a 0.7.x install.sh install, made by 0.7.5's own install-desktop.sh, with settings"
mkdir -p "$HOME/.config/rex-linux"
bash "$old/scripts/install-desktop.sh" --copy --receipt-version 0.7.4 "$old/target/release/examples/bridge_harness" >/dev/null
printf '{"hosts":[{"label":"Kept","endpoint":"unix:///tmp/rex-bridge-e2e-none.sock"}]}\n' > "$HOME/.config/rex-linux/workspace.json"
echo '{}' > "$HOME/.config/rex-linux/shortcuts.json"
files

echo "== 4. Update now, as 0.7.5 runs it"
"$bin/rex-linux" "$version"
cmp "$bin/rex-linux" "$new/target/release/rex-rune"
test "$("$bin/rex-linux" --version)" = "rex-linux $version"
files

echo "== 5. the first start of the new client"
timeout 120 "$bin/rex-linux" -- --snap "$work/first-start.png" 2>&1 | grep -v '^rune: ' | sed 's/^/   /' || true
files

echo "== 6. one binary, one launcher, one set of icons, a verified receipt, settings and state moved"
test "$(readlink "$bin/rex-linux")" = rex-rune
cmp "$bin/rex-rune" "$new/target/release/rex-rune"
test "$("$bin/rex-rune" --version)" = "rex-rune $version"
test "$("$bin/rex-linux" --version)" = "rex-linux $version"
test "$(ls "$data"/applications/*.desktop)" = "$data/applications/dev.shuv.rex-rune.desktop"
desktop-file-validate "$data/applications/dev.shuv.rex-rune.desktop"
grep -qx "TryExec=$bin/rex-rune" "$data/applications/dev.shuv.rex-rune.desktop"
test -z "$(find "$data" -name '*rex-linux*')"
test "$(find "$data/icons" -name 'dev.shuv.rex-rune.png' | wc -l)" = 3
python3 - "$bin/rex-rune" "$data/rex-rune/install.json" "$version" <<'PY'
import hashlib, json, pathlib, sys
binary = pathlib.Path(sys.argv[1])
receipt = json.load(open(sys.argv[2]))
assert receipt == {"schema": 1, "path": str(binary), "version": sys.argv[3],
                   "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}, receipt
PY
test ! -e "$HOME/.config/rex-linux"
grep -q Kept "$HOME/.config/rex-rune/workspace.json"
test -e "$HOME/.config/rex-rune/shortcuts.json"
test ! -e "$HOME/.local/state/rex-linux"
grep -q "\"highest_verified\": \"$version\"" "$HOME/.local/state/rex-rune/update.json"
test -s "$work/first-start.png"

echo "== 7. a second start changes nothing"
before="$(files | sha256sum)"
timeout 120 "$bin/rex-rune" -- --snap "$work/second-start.png" >/dev/null 2>&1 || true
test "$before" = "$(files | sha256sum)"
echo "PASS: 0.7.5 Update now -> bridge tarball -> first start -> rex-rune, one launcher, settings and state kept"
