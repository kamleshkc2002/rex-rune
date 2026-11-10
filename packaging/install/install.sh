#!/bin/sh
# Rex Linux's release installer. The complete function must arrive before it runs.
rex_install() (
    set -eu
    umask 077
    REPO='https://github.com/shuv1337/rex-rune'
    CI_KEY='RWRugVJ2wnj9oVdIvBhQFK1/BDvL1KkA4aH1HB2Q0uA30OBHXIvTlHwL'
    BACKUP_KEY='RWTzIScuoaGONtqf+4m48HAGuwNgUxvd/AFZareIgMwCGCKVjbTjcmJe'

    die() { printf 'rex-linux installer: %s\n' "$*" >&2; exit 1; }
    stable() {
        case "$1" in *[!0-9.]*|'') return 1 ;; esac
        printf '%s\n' "$1" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'
    }
    newer() { # Numeric component comparison, including components beyond shell integer range.
        awk -v a="$1" -v b="$2" 'BEGIN {split(a,x,"."); split(b,y,"."); for(i=1;i<=3;i++) {if(length(x[i])!=length(y[i])) exit !(length(x[i])>length(y[i])); if(("x" x[i])!=("x" y[i])) exit !(("x" x[i])>("x" y[i]))} exit 1}'
    }
    requested='' check=false force=false trust=false uninstall=false
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --upgrade) ;;
            --uninstall) uninstall=true ;;
            --check) check=true ;;
            --force) force=true ;;
            --trust-repo-keys) trust=true ;;
            --version) shift; [ "$#" -gt 0 ] || die '--version needs X.Y.Z'; requested=$1; stable "$requested" || die 'expected stable X.Y.Z' ;;
            --help) printf '%s\n' 'Usage: install.sh [--upgrade | --check | --uninstall] [--version X.Y.Z] [--force] [--trust-repo-keys]'; exit 0 ;;
            *) die "unknown option: $1" ;;
        esac
        shift
    done
    if $uninstall && { $check || [ -n "$requested" ]; }; then die '--uninstall cannot be combined with --check or --version'; fi
    : "${HOME:?HOME must be set}"
    bin=${XDG_BIN_HOME:-$HOME/.local/bin}
    data=${XDG_DATA_HOME:-$HOME/.local/share}
    cache=${XDG_CACHE_HOME:-$HOME/.cache}
    for dir in "$bin" "$data" "$cache"; do
        case "$dir" in /*) ;; *) die 'HOME and XDG directories must be absolute paths' ;; esac
        [ "$dir" != / ] || die 'refusing an XDG directory at filesystem root'
    done
    # Delete only the documented user-install files. Never remove preferences or directories.
    if $uninstall; then
        rm -f -- "$bin/rex-linux" "$data/applications/dev.shuv.rex-linux.desktop" "$data/rex-linux/install.json"
        for size in 32 128 256; do rm -f -- "$data/icons/hicolor/${size}x${size}/apps/dev.shuv.rex-linux.png"; done
        if command -v update-desktop-database >/dev/null 2>&1 && [ -d "$data/applications" ]; then update-desktop-database "$data/applications"; fi
        if command -v gtk-update-icon-cache >/dev/null 2>&1 && [ -d "$data/icons/hicolor" ]; then gtk-update-icon-cache -f -t "$data/icons/hicolor" >/dev/null 2>&1 || :; fi
        printf '%s\n' 'Removed Rex Linux user install; preferences kept.'
        exit 0
    fi
    # /etc/os-release is root-owned, but source it in a subshell so its variables
    # cannot overwrite installer state.
    # shellcheck disable=SC1091
    distro=$(if [ -r /etc/os-release ]; then . /etc/os-release; fi; printf '%s %s' "${ID:-}" "${ID_LIKE:-}")
    case " $distro " in
        *nixos*) die 'NixOS: use the Nix package; see docs/nix.md' ;;
        *arch*) hint='sudo pacman -Syu curl tar coreutils bash desktop-file-utils openssl'; manager='sudo pacman -Syu' ;;
        *debian*|*ubuntu*) hint='sudo apt install curl tar coreutils bash desktop-file-utils openssl'; manager='sudo apt update && sudo apt upgrade' ;;
        *fedora*|*rhel*) hint='sudo dnf install curl tar coreutils bash desktop-file-utils openssl'; manager='sudo dnf upgrade' ;;
        *) hint='install curl, tar, coreutils, bash, desktop-file-utils and minisign or OpenSSL 3+'; manager='your package manager' ;;
    esac
    if [ -e /usr/bin/rex-linux ] && ! $force; then die "package install detected; use $manager (or --force for a user install)"; fi
    [ "$(uname -s)" = Linux ] || die 'only Linux is supported'
    case "$(uname -m)" in x86_64) target=x86_64-linux-gnu ;; aarch64|arm64) target=aarch64-linux-gnu ;; *) die 'supported architectures: x86_64 and aarch64' ;; esac
    for cmd in curl tar sha256sum bash desktop-file-install desktop-file-validate awk base64 dd od; do
        command -v "$cmd" >/dev/null 2>&1 || die "missing $cmd; $hint"
    done
    libc=$(getconf GNU_LIBC_VERSION 2>/dev/null) || die 'glibc 2.39+ required; musl is not supported'
    libc=${libc#glibc }
    awk -v v="$libc" 'BEGIN {split(v,a,"."); exit !(a[1]>2 || (a[1]==2 && a[2]>=39))}' || die "glibc $libc is too old; need 2.39+"
    if command -v minisign >/dev/null 2>&1; then verifier=minisign
    elif command -v openssl >/dev/null 2>&1 && openssl version | grep -Eq '^OpenSSL ([3-9]|[1-9][0-9]+)\.'; then verifier=openssl
    else die "need minisign or OpenSSL 3+; $hint"; fi

    if [ -e "$data/rex-linux/install.json" ] && ! $force; then
        # U3 writes the receipt. Python is only needed when a receipt actually exists.
        command -v python3 >/dev/null 2>&1 || die 'receipt present: install python3 to validate it (or --force)'
        python3 - "$data/rex-linux/install.json" "$bin/rex-linux" <<'PY' || die 'receipt does not match the installed binary; use --force only to replace it deliberately'
import hashlib, json, pathlib, sys
r = json.loads(pathlib.Path(sys.argv[1]).read_text())
p = pathlib.Path(sys.argv[2])
if pathlib.Path(r["path"]).resolve() != p.resolve() or r["sha256"] != hashlib.sha256(p.read_bytes()).hexdigest():
    sys.exit(1)
PY
    fi
    installed=''
    if [ -e "$bin/rex-linux" ]; then
        actual=$("$bin/rex-linux" --version) || actual=''
        installed=${actual#rex-linux }
        if [ "$actual" != "rex-linux $installed" ] || ! stable "$installed"; then
            $force || die 'installed binary cannot report a stable version; --force to repair deliberately'
            installed=''
        fi
        if [ ! -e "$data/rex-linux/install.json" ]; then printf '%s\n' 'Existing install has no receipt (normal before U3); checking its version.'; fi
    fi
    mkdir -p "$cache"
    work=$(mktemp -d "$cache/rex-install.XXXXXXXX")
    trap 'rm -rf -- "$work"' 0
    trap 'exit 1' HUP INT TERM
    fetch() {
        curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL --connect-timeout 20 --max-time 300 \
            --max-filesize "$3" -o "$2" "$1" || return 1
        [ "$(wc -c < "$2")" -le "$3" ]
    }
    # Decode minisign records without putting binary data (NULs) in shell variables.
    record() { printf '%s' "$1" | base64 -d > "$2" 2>/dev/null && [ "$(wc -c < "$2")" -eq "$3" ]; }
    key_id() { dd if="$1" bs=1 skip=2 count=8 2>/dev/null | od -An -tx1 | tr -d ' \n'; }
    verify_key() {
        record "$1" "$work/key" 42 || return 1
        [ "$(key_id "$work/key")" = "$(key_id "$work/sig")" ] || return 1
        [ "$(dd if="$work/sig" bs=1 count=2 2>/dev/null)" = ED ] || return 1
        [ "$(dd if="$work/key" bs=1 count=2 2>/dev/null)" = Ed ] || return 1
        if [ "$verifier" = minisign ]; then
            minisign -Vm "$work/SHA256SUMS" -x "$work/SHA256SUMS.minisig" -P "$1" -q >/dev/null 2>&1 || return 1
        else
            # Minisign ED = Ed25519 over BLAKE2b-512. The global signature binds
            # the raw 64-byte signature AND trusted comment, without its newline.
            printf '\060\052\060\005\006\003\053\145\160\003\041\000' > "$work/key.der"
            dd if="$work/key" bs=1 skip=10 count=32 2>/dev/null >> "$work/key.der"
            dd if="$work/sig" bs=1 skip=10 count=64 of="$work/raw.sig" 2>/dev/null
            openssl dgst -blake2b512 -binary "$work/SHA256SUMS" > "$work/digest" || return 1
            openssl pkeyutl -verify -pubin -keyform DER -inkey "$work/key.der" -rawin -in "$work/digest" -sigfile "$work/raw.sig" >/dev/null 2>&1 || return 1
            cp "$work/raw.sig" "$work/comment"
            printf '%s' "$comment" >> "$work/comment"
            openssl pkeyutl -verify -pubin -keyform DER -inkey "$work/key.der" -rawin -in "$work/comment" -sigfile "$work/global.sig" >/dev/null 2>&1 || return 1
        fi
        printf 'Verified with %s; key ID %s; SHA-256 %s\n' "$verifier" "$(key_id "$work/key" | awk '{for(i=15;i>0;i-=2) printf "%s", toupper(substr($0,i,2))}')" "$(sha256sum "$work/key" | cut -d ' ' -f1)"
    }
    verify() {
        [ "$(wc -l < "$work/SHA256SUMS.minisig")" -eq 4 ] || return 1
        sig=$(awk 'NR==2 {print}' "$work/SHA256SUMS.minisig")
        comment=$(awk 'NR==3 {print}' "$work/SHA256SUMS.minisig")
        global=$(awk 'NR==4 {print}' "$work/SHA256SUMS.minisig")
        case "$comment" in 'trusted comment: rex-linux v'*' SHA256SUMS') ;; *) return 1 ;; esac
        comment=${comment#trusted comment: }
        version=${comment#rex-linux v}; version=${version% SHA256SUMS}
        stable "$version" || return 1
        [ -z "$requested" ] || [ "$requested" = "$version" ] || return 1
        record "$sig" "$work/sig" 74 && record "$global" "$work/global.sig" 64 || return 1
        if verify_key "$CI_KEY" || verify_key "$BACKUP_KEY"; then return 0; fi
        $trust || return 1
        # This is an explicit expansion of trust, only for an UNKNOWN key ID.
        for key in "$CI_KEY" "$BACKUP_KEY"; do
            record "$key" "$work/key" 42 || return 1
            [ "$(key_id "$work/key")" != "$(key_id "$work/sig")" ] || return 1
        done
        printf 'Explicit --trust-repo-keys: reading public keys at tag v%s, not a branch.\n' "$version"
        for name in release.pub release-backup.pub; do
            fetch "$REPO/raw/refs/tags/v$version/packaging/keys/$name" "$work/repo.pub" 4096 || return 1
            key=$(awk 'NR==2 {print}' "$work/repo.pub")
            if verify_key "$key"; then return 0; fi
        done
        return 1
    }
    base="$REPO/releases/latest/download"
    [ -z "$requested" ] || base="$REPO/releases/download/v$requested"
    verified=false
    for attempt in 1 2; do
        [ "$attempt" -eq 1 ] || printf '%s\n' 'Retrying signed metadata once (possible latest-release transition).'
        if fetch "$base/SHA256SUMS" "$work/SHA256SUMS" 65536 && fetch "$base/SHA256SUMS.minisig" "$work/SHA256SUMS.minisig" 4096 && verify; then verified=true; break; fi
    done
    $verified || die 'signature verification failed; nothing installed (unknown keys require explicit --trust-repo-keys)'
    name="rex-linux-$version-$target"
    archive="$name.tar.gz"
    hash=$(awk -v f="$archive" '$2==f {n++; h=$1; if(NF!=2 || length($1)!=64 || $1 ~ /[^0-9a-f]/) bad=1} END {if(n!=1 || bad) exit 1; print h}' "$work/SHA256SUMS") || die 'signed checksums missing or ambiguous for this tarball'
    if [ -n "$installed" ]; then
        if [ "$installed" = "$version" ]; then printf 'Rex Linux %s is already current.\n' "$version"; exit 0; fi
        if newer "$installed" "$version" && [ -z "$requested" ]; then die "refusing downgrade $installed -> $version; use --version $version deliberately"; fi
    fi
    if $check; then printf 'Rex Linux %s available (installed: %s).\n' "$version" "${installed:-none}"; exit 0; fi
    fetch "$REPO/releases/download/v$version/$archive" "$work/$archive" 268435456 || die 'tarball download failed (limit 256 MiB)'
    printf '%s  %s\n' "$hash" "$work/$archive" | sha256sum -c - || die 'tarball checksum mismatch; nothing installed'
    tar -tzf "$work/$archive" > "$work/entries" || die 'invalid tarball'
    awk -v root="$name" '{if($0!=root && index($0,root "/")!=1) exit 1; n=split($0,a,"/"); for(i=1;i<=n;i++) if(a[i]==".." || a[i]==".") exit 1}' "$work/entries" || die 'tarball paths escape expected directory'
    # Official packages contain only directories and regular files, not links/devices.
    tar -tvzf "$work/$archive" > "$work/types" || die 'cannot inspect tarball'
    awk 'substr($0,1,1)!="d" && substr($0,1,1)!="-" {exit 1}' "$work/types" || die 'tarball contains links or special files'
    mkdir "$work/unpack"
    tar -xzf "$work/$archive" -C "$work/unpack" --no-same-owner --no-same-permissions || die 'cannot extract tarball'
    package="$work/unpack/$name"
    [ "$("$package/bin/rex-linux" --version)" = "rex-linux $version" ] || die 'downloaded binary cannot run or reports wrong version; old install untouched'
    bash "$package/install.sh" || die 'desktop installation failed'
    printf 'Installed Rex Linux %s into %s.\n' "$version" "$bin"
    case ":$PATH:" in *":$bin:"*) ;; *) printf 'Add %s to PATH.\n' "$bin" ;; esac
)
# A complete command group also prevents a truncated final call losing its args.
{ rex_install "$@"; }
