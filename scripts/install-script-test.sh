#!/usr/bin/env bash
# Container integration suite; never touches the host's installation or servers.
set -euo pipefail
cd "$(dirname "$0")/.."
images=("${@}")
if ((${#images[@]} == 0)); then
    images=(
        "archlinux:base@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899"
        "ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852"
        "debian:trixie@sha256:9c6e381421d0f48d1b8133e573f33c185ee16cbf505eda9c4a8a4791b25ea8a6"
        "fedora:latest@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c"
        "ubuntu:22.04@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11"
        "debian:12@sha256:268fb24c39bca92d326dd83ea3abebe4a3f7751c804ffef23bb6d857dd343ff8"
        "alpine:3.22@sha256:49340b346798875b5592e5d420241f2f9cfba5f47e62ebb1446a1b34734e898f"
    )
fi
for image in "${images[@]}"; do
    docker run --rm -v "$PWD:/src:ro" "$image" sh -euc '
        export DEBIAN_FRONTEND=noninteractive
        if command -v apt-get >/dev/null; then
            apt-get update -qq
            apt-get install -y -qq python3 python3-tomli curl ca-certificates tar coreutils bash desktop-file-utils openssl dash busybox shellcheck
            if apt-cache show minisign >/dev/null 2>&1; then apt-get install -y -qq minisign
            else
                # Ubuntu 22.04 predates the distro package; build a pinned test tool.
                apt-get install -y -qq cmake make gcc pkg-config libsodium-dev
                curl --proto =https -fsSL https://github.com/jedisct1/minisign/archive/refs/tags/0.12.tar.gz -o /tmp/minisign.tar.gz
                echo "796dce1376f9bcb1a19ece729c075c47054364355fe0c0c1ebe5104d508c7db0  /tmp/minisign.tar.gz" | sha256sum -c -
                tar -xzf /tmp/minisign.tar.gz -C /tmp
                cmake -S /tmp/minisign-0.12 -B /tmp/minisign-build -DCMAKE_INSTALL_PREFIX=/usr
                cmake --build /tmp/minisign-build
                cmake --install /tmp/minisign-build
            fi
        elif command -v pacman >/dev/null; then
            pacman -Syu --noconfirm python curl ca-certificates tar coreutils bash desktop-file-utils openssl minisign dash busybox shellcheck
        elif command -v dnf >/dev/null; then
            dnf install -y python3 curl ca-certificates tar coreutils bash desktop-file-utils openssl minisign dash busybox ShellCheck
        else
            apk add python3 curl ca-certificates tar coreutils bash desktop-file-utils openssl minisign dash busybox shellcheck
        fi
        shellcheck /src/packaging/install/install.sh /src/scripts/install-script-test.sh
        dash -n /src/packaging/install/install.sh
        openssl version
        python3 /src/scripts/test-install.py
    '
done
