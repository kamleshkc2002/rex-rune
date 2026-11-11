# Roadmap from 0.5.2

Written 2026-11-10 (PST) against `master` at `61e0f2a` (API 13 support merged, unreleased). This is a plan only: nothing here is built, no release was cut and no setting was changed. It re-orders the approved [auto-update plan](https://github.com/shuv1337/rex-rune/blob/plan/auto-update/docs/plans/auto-update.md) (on the `plan/auto-update` branch) for two facts that plan did not have: both of the owner's client machines run Arch, and the owner has decided that **Arch packaging comes first, together with a GitHub-hosted install script, and the APT repository moves later**. The operator's view of the same system is in [the walkthrough](../auto-update-operator-walkthrough.md).

## Sprint 1 — approved 2026-11-10 (PST)

One shorter sprint, **items 1–4 only**:
1. **0.5.2:** API 13 and the signed release-asset install script.
2. **Arch:** signed pacman repository, channel markers, install receipt and Nix docs.
3. **In-app update notice**, plus LoricAndre's pane padding and density.
4. **File downloads** (API 13).

**Deferred:** APT, the Activity sidebar, the tarball self-updater and #20.
The later sections retain their design context, not a commitment to this sprint.

**Decisions:** agents drive setup, hosting, PRs, merges and releases, involving the
owner only when physically needed. The permanent package host is **pkg.shuv.dev**
for both Arch and later APT. **No AUR for now.** Repository-key fallback is
**explicitly opt-in with `--trust-repo-keys`**, never automatic or a prompt.
PRs, merges and releases are approved; the release signing gate remains: post
the requested queue item when publish waits, and approve its pending deployment
by API only after the owner's affirmative queue response.

## Summary

- **0.5.2** ships API 13 support and the install script. Cut it when the install script is merged, or on 2026-11-13 (PST) with API 13 alone if it is not.
- **Arch:** a signed pacman repository at `https://pkg.shuv.dev/arch/$arch`, fed by `.pkg.tar.zst` packages that become release assets. The same `PKGBUILD` is kept ready for an AUR `rex-linux-bin` package, which is not published yet (question 2).
- **Install script:** `curl … | sh` from the latest release's assets. It verifies the minisign signature and the checksum before it touches anything, installs into `~/.local/bin`, and the same command upgrades. Verification without `minisign` installed was checked today with OpenSSL against the real 0.5.0 signature.
- **Update notice** moves ahead of the APT repository. **The tarball self-updater moves to the end**, because the install script already upgrades a tarball install with one command.
- **APT** stays in the plan, after the notice. It reuses the site, key and builder the Arch repository needs, so it shrinks to a small increment.
- **File downloads** (API 13): a "Download file…" dialog, saves to `~/Downloads`, a toolbar chip for progress and cancel, never opens a file by itself, no cache in the first version.
- **Fork harvest:** kamleshkc2002 and kitten-lily have no new work of their own since the last harvest, only syncs of our `master`. Scroll geometry stays skipped. The Activity sidebar changes to "adapt, narrowed", scheduled last. A new fork (LoricAndre) has a small pane padding and density change worth adapting.
- **#20 (cross-server relay):** sketched below; do not build it now.

## 1. Where things stand

Checked on 2026-11-10 (PST):

| Fact | State |
| --- | --- |
| 0.5.0 | Published 10:14 AM PST. First signed release; immutable; eight assets. |
| 0.5.1 | **Published 11:43 AM PST**, while this plan was being written. Immutable, eight assets, and `releases/latest/download/latest.json` reports `0.5.1`. |
| `master` | Two commits past `v0.5.1`: #32, API 13 support. No open pull requests. One open issue, #20. |
| `release` environment | Reviewer shuv1337, no administrator bypass, `v*` tags only. Secrets: `RELEASE_MINISIGN_KEY`, `RELEASE_MINISIGN_PASSWORD`. |
| Tag rulesets | Two, active: creation restricted, no updates or deletion. They cover `v*` only. **`master` has no branch protection.** |
| GitHub Pages | Not enabled. `pkg.shuv.dev` has no DNS record. `shuv.dev` is on Cloudflare. |
| OpenPGP package key | Does not exist. |
| Client machines | shuvdev: Omarchy (`ID=omarchy`, `ID_LIKE=arch`), x86_64, glibc 2.44. shuvmbp-1: Arch, aarch64. Both have a user install in `~/.local/bin`. |
| Servers | shuvdev API 13, shuvmbp-1 (`shuvmbp-rex`) API 13, shuvbot preview.14 (API 12). A 0.5.x client negotiates 12 with an API 13 server, so nothing is broken today. |

Stale text found while reading, all to fix in 0.5.2: `CHANGELOG.md` has two `## Unreleased` headings; the release notes template in `release.yml` and `PRODUCT.md` still say revisions 10 to 12; the `.deb` description says "10–12"; the README's "Verify a download" section still describes key setup as a step before the first signed release.

## 2. Recommended order

Sizes: **S** is under 200 changed lines, **M** 200 to 600, **L** more than 600, tests included.

| # | Release | Pieces | Size | Needs from the owner |
| --- | --- | --- | --- | --- |
| 1 | **0.5.2** | R1 release prep; **U1 install script**; D0 download contract probe (no release impact) | S + M + S | The approval click |
| 2 | **0.5.3** | **U2 Arch package and signed pacman repository**; U3 channel markers, receipt, Nix docs | L + S | One DNS record, one script run on the offline machine, the approval click |
| 3 | **0.6.0** | **U4 update check and notice**; H1 pane padding and density (LoricAndre) | L + S | The approval click |
| 4 | **0.7.0** | **D1 to D3 file downloads** | M + M + S | The approval click |
| 5 | rides the next release | **U5 APT repository**, with the `.deb` carrying its source | M | Nothing new |
| 6 | **0.8.0** | H2 Activity view (kitten-lily, narrowed) | L | The approval click |
| 7 | decide then | U6 tarball self-updater | L | Nothing new |
| not now | | #20 cross-server relay | M to L | |

Reasons for the order:

1. **The install script is first** because it is the smallest thing that changes how both of the owner's machines update, it needs no new key, account, DNS record or client code, and every later channel can point to it as the fallback.
2. **The Arch repository is second** because it is what the owner's machines should end up on, and because it forces the shared infrastructure (the Pages site, the OpenPGP key, the stateless site builder) that APT needs later.
3. **The notice is third, ahead of APT.** Every release without it creates installs that never hear of the next one. A package manager covers its own users; nothing covers the rest.
4. **Downloads follow the notice** rather than lead it, because they depend on a server contract that has not been observed yet (D0) and only two of the three servers can serve them.
5. **APT comes after the notice.** No machine of the owner's uses it, so nobody exercises it daily; from U4 on, `.deb` users at least hear about releases; and after U2 it is scripts only, about a third of its original size.
6. **The self-updater is last** and is a decision to take then, not a commitment. After U1 and U4 it turns "copy one command" into "click one button". That is worth roughly 800 lines of code that replaces binaries only if tarball users turn out to exist in numbers.

## 3. Release 0.5.2

### What goes in

| Item | State |
| --- | --- |
| API revisions 10 to 13 (#32) | Merged. |
| R1: version bump (`Cargo.toml`, `Cargo.lock`, `nix/package.nix`), one `## Unreleased` heading, README install examples, the stale revision text listed in section 1 | To write. S. |
| U1: the install script as a release asset (section 4.3) | To write. M. |
| `docs/0.5.2-verification.md` | To write with the checks below. |

Not in it: any download UI, the Arch package, anything that needs Pages or a new key.

### When to cut it

- **0.5.1 is published, so nothing blocks the tag.** The general rule stands: never tag a release while an older tag's run is waiting for approval. The `publish` job ends with `gh release edit --latest`, so approving the older run afterwards would move "latest" backwards.
- Cut 0.5.2 **when U1 is merged and a manual Binary release run has passed with it**.
- If U1 is not merged by **2026-11-13 (PST)**, cut 0.5.2 with API 13 and R1 alone and move U1 to 0.5.3. There is no urgency in either direction: 0.5.1 already attaches to API 13 servers at revision 12, and a release costs one click.

### Checks before the tag

1. The suite in `AGENTS.md` (`cargo test`, `fmt`, `clippy -D warnings`, the SDK tests) and `python3 scripts/test-release.py`.
2. A manual **Binary release** run on `master`; inspect `test-release-metadata`.
3. Attach-only checks with the release build, creating and destroying nothing: shuvdev (API 13), shuvmbp-1 (API 13) and shuvbot (API 12). Settings → About must show 13, 13 and 12.
4. The aarch64 tarball from the manual run, on shuvmbp-1, against its own server. `docs/server-api13.md` makes no aarch64 claim; this adds one.
5. Probes and acceptance scripts only against private servers under `~/.cache/agent-ws/`, as in `docs/server-api13.md`.

### Risks

| Risk | Handling |
| --- | --- |
| The X11 clipboard flake recorded as papercut `pc_540f0bd8abba` fails the acceptance run. | Rerun once unchanged; a second failure blocks the release. |
| Release notes still tell people the server must be revision 10 to 12. | R1 fixes the template in `release.yml`. The notes are built only in `publish`, so the check before the tag is a search of the repository for the old wording, and the check after it is reading the published notes. |
| The install script asset breaks `release-manifest.py`'s "exactly four packages" rule. | The script is signed as its own file and stays out of `SHA256SUMS` (section 4.3). |

## 4. Auto-update, re-ordered

### 4.1 Numbering

The approved plan's PR 1 (signed releases) shipped in 0.5.0. The rest is renumbered here, and the walkthrough uses these names.

| New | Approved plan | Contents |
| --- | --- | --- |
| **U1** | new | Install script |
| **U2** | new | Arch package, signed pacman repository, Pages site, OpenPGP key |
| **U3** | PR 3 (part) and PR 6 | Channel markers, the tarball receipt, Nix docs |
| **U4** | PR 4 | Update check and notice |
| **U5** | PR 2 and the rest of PR 3 | APT repository; the `.deb` installs its source |
| **U6** | PR 5 | Tarball self-updater |

Decisions carried over unchanged: CI custody in the `release` environment with the owner's approval per release; immutable releases; the APT address `pkg.shuv.dev`; the manifest behind `releases/latest/download/`; no expiring metadata, no `by-hash`, no automatic crash rollback.

### 4.2 The Arch decision

| Option | For | Against |
| --- | --- | --- |
| **Signed pacman repository** (chosen) | Updates arrive with `pacman -Syu`, with no helper. Works the same on Arch Linux ARM. Packages and the database are signed by a key behind the same approval click as everything else. Builds the site, key and builder that APT reuses. No new account outside GitHub. | A one-time setup as root on each machine: import a key and add a section to `pacman.conf`. Needs the OpenPGP key and Pages now rather than later. |
| AUR `rex-linux-bin` | Where Arch users look first. No hosting. The `PKGBUILD` pins the tarball's SHA-256. | Needs an AUR account and an SSH key in CI: a new publishing surface outside GitHub. The `PKGBUILD` itself is unsigned, so its integrity is the AUR account's. Needs an AUR helper. Official AUR support is x86_64; aarch64 works but is second-class. |
| Both now | Covers both habits. | Two things to keep in step before either has run once. |
| Pull the self-updater forward instead | Helps tarball installs on any distribution. | The install script gives the same result for a fraction of the code, and a package is the better end state on Arch. |

**Decision: the signed pacman repository now; the AUR package later and only if wanted.** The package is built from a committed `PKGBUILD` that is already a valid AUR `-bin` package (it has `source_x86_64` and `source_aarch64` URLs and pinned checksums), so publishing to the AUR later is one push and one CI step, not new design.

Details:

- **Package:** `rex-linux-bin`, `provides=(rex-linux)`, `conflicts=(rex-linux)`, `arch=(x86_64 aarch64)`, `options=(!strip !debug)`. It repackages the release tarball's binary unchanged: `/usr/bin/rex-linux`, the desktop entry, the three icons, licenses under `/usr/share/licenses/rex-linux-bin/`, and `/usr/share/rex-linux/channel` containing `channel=pacman` and `hint=sudo pacman -Syu`. Dependencies are the Arch names for what the `.deb` declares (`glibc`, `gcc-libs`, `fontconfig`, `libxkbcommon`, `libxkbcommon-x11`, `wayland`, `libx11`, `libxcb`, `libglvnd`, `vulkan-icd-loader`, `xdg-utils`, `ca-certificates`, `hicolor-icon-theme`). The binary links only glibc and `libgcc_s` at load time and opens the rest at run time, so `ldd` cannot confirm the list; starting the app on shuvdev and shuvmbp-1 from the package does.
- **Packages are release assets.** A new `arch-package` job with no secrets runs real `makepkg` in a digest-pinned `archlinux:base-devel` container, once per architecture from the two tarballs (`CARCH=aarch64` for the second; nothing is compiled). It compares the packaged binary with the tarball's, as CI already does for the `.deb`. `SHA256SUMS`, `latest.json` (a `pacman` entry per target) and the attestations then cover six packages.
- **Signing** happens in `publish`, behind the same single approval: a detached OpenPGP signature for each package (`.pkg.tar.zst.sig`, also a release asset), and after publication the repository database.
- **The repository** is a pure function of the published releases and the key, like the approved APT design. `scripts/build-repo-site.py` takes the newest five signed, non-prerelease releases that have pacman packages, verifies `SHA256SUMS.minisig` against the committed keys and then the checksums, lays out `arch/x86_64/` and `arch/aarch64/`, runs real `repo-add` in the container (which embeds each package's signature) and signs `rex-linux.db` and `rex-linux.files` on the runner, so the container never sees the key. The database lists the newest version; the four older package files stay in the directory for `pacman -U <url>`.
- **One OpenPGP key for pacman and APT:** an offline certify-only Ed25519 primary and a signing subkey that expires in three years, as the approved plan describes. Only the subkey reaches CI, as `PACKAGE_SIGNING_KEY` and `PACKAGE_SIGNING_PASSPHRASE` in the `release` environment. The public certificate is committed as `packaging/keys/rex-linux-archive-key.asc` and served at the site root.
- **Address:** one Pages site has one custom domain, and that is `pkg.shuv.dev`, so the pacman repository is `https://pkg.shuv.dev/arch/$arch` (question 1).
- **Setup for a user:**

  ```sh
  curl -fsSLO https://pkg.shuv.dev/rex-linux-archive-key.asc
  sudo pacman-key --add rex-linux-archive-key.asc
  sudo pacman-key --lsign-key <PRIMARY-FINGERPRINT>   # printed in the README
  printf '\n[rex-linux]\nSigLevel = Required\nServer = https://pkg.shuv.dev/arch/$arch\n' | sudo tee -a /etc/pacman.conf
  sudo pacman -Syu rex-linux-bin
  ```

- **Rebuilding without a release:** `repo.yml`, started by hand **on a release tag** (`gh workflow run repo.yml --ref v0.5.3`), because the `release` environment admits only `v*` refs. It takes an `exclude` input for a withdrawn version and needs the same approval click.

Left out on purpose:

| Left out | The case it covers | Cost |
| --- | --- | --- |
| A `rex-linux-keyring` package | A new signing subkey reaching users without them importing the certificate again. | A second package and an install script that runs `pacman-key`. Rotation is once in about three years; the README gives the two commands. |
| Building the pacman database by hand in Python | Avoiding a container in CI. | A hand-written copy of pacman's database format that only real pacman can prove right. |

### 4.3 U1: the install script

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh | sh
```

**Where it is served.** As a release asset, `install.sh`, with `install.sh.minisig` beside it. The source is `packaging/install/install.sh`. A release asset passes the approval click, is immutable, and is withdrawn together with a bad release. The alternative, `raw.githubusercontent.com/…/master/…`, would let anyone who can push to `master` change what new users run, and `master` has no branch protection.

**What it does, in order.** It is POSIX `sh`, and its whole body is one function called on the last line, so a download cut short runs nothing.

1. **Detect.** `uname -m` gives `x86_64-linux-gnu` or `aarch64-linux-gnu`; anything else stops. `/etc/os-release` `ID` and `ID_LIKE` choose the wording of hints (shuvdev reports `ID=omarchy`, `ID_LIKE=arch`, so `ID` alone is not enough). musl and NixOS stop with the reason. It needs `curl`, `tar`, `sha256sum`, `bash` and `desktop-file-install`, and names the distribution's package for any that is missing.
2. **Fetch the signed checksums.** `SHA256SUMS` and `SHA256SUMS.minisig` from `releases/latest/download/`. It uses `SHA256SUMS`, not `latest.json`, because lines can be parsed safely in `sh` and JSON cannot. If the two files straddle a new release, it retries once.
3. **Verify the signature against the keys embedded in the script** (the CI key and the offline backup key). With `minisign` installed it uses that. Otherwise it uses OpenSSL 3 or newer: Ed25519 over the BLAKE2b-512 digest, then the second signature that covers the trusted comment. This route was checked today on OpenSSL 4.1 against 0.5.0's real `SHA256SUMS.minisig` (it verifies, and an altered file fails). With neither tool it stops and installs nothing. There is no "skip verification" option.
4. **Read the version from the signed trusted comment** (`rex-linux v0.5.2 SHA256SUMS`). It must be a stable `vX.Y.Z`. The tarball name follows from it and must have a line in `SHA256SUMS`.
5. **Compare with what is installed.** Same version: say so and exit 0. Installed is newer: stop, unless `--version` asked for exactly that.
6. **Download** `releases/download/vX.Y.Z/<tarball>` into a directory under `$XDG_CACHE_HOME` (not `/tmp`, which may be `noexec`), with a size limit, and **check its SHA-256**.
7. **Unpack and try it.** Entries must sit under the one expected top directory. `bin/rex-linux --version` must print the version; a binary that cannot start here stops the install with the old one untouched.
8. **Install** by running the tarball's own `install.sh`, which is `scripts/install-desktop.sh --copy`: the binary to `$XDG_BIN_HOME` (default `~/.local/bin`), the launcher and the icons. From U3 on that step also writes the receipt. The script warns if the directory is not on `PATH`.

**Upgrade** is the same command. **Uninstall** is `… | sh -s -- --uninstall`: it removes the binary, launcher, icons and receipt and leaves preferences alone. Other flags: `--check` (report only), `--version X.Y.Z` (a signed release from 0.5.0 on), `--force`, `--trust-repo-keys`.

**It refuses** when `/usr/bin/rex-linux` exists (a package owns the install; it names the package manager's command), and when a receipt exists but does not match the binary on disk (something else, such as a source build, replaced it). `--force` overrides both. An install with no receipt, which is every install made before U3, is upgraded with a one-line note.

**Keys and the fallback.** The two public key lines are constants in the script and must equal the second lines of `packaging/keys/release.pub` and `release-backup.pub`; `scripts/test-release.py` fails if they differ, so a rotation cannot change one without the other. The fallback the owner asked for covers a saved copy of the script that predates a rotation: if the signature names a key ID that matches neither embedded key, the script can fetch the two key files from the repository **at the release's `v*` tag** and verify against those. It never reads keys from a branch: tags are protected by the rulesets, `master` is not. It prints the key IDs and fingerprints it used. The review settled question 3: only an explicit `--trust-repo-keys` enables this fallback; no automatic fallback and no terminal prompt.

**Publishing.** `sign-release.py` signs a third file, `install.sh`, with the trusted comment `rex-linux vX.Y.Z install.sh`, and verifies it like the other two. The upload list grows to ten assets. The asset is byte for byte the file at the tag, so anyone can compare it with `git show vX.Y.Z:packaging/install/install.sh`.

**Tests** (`scripts/install-script-test.sh`, also a job in `release.yml`):

- The test makes a copy of the script with the base URL and the two keys replaced by a loopback server and throwaway keys, and asserts it made exactly three replacements. The shipped script has no environment override for either.
- Packages come from the real `package-release.py` with a stub binary, in two versions.
- Containers: `archlinux`, `ubuntu:24.04`, `debian:trixie`, `fedora`; `alpine`, `ubuntu:22.04` and `debian:12` for the refusals (musl; a glibc too old for the binary to start) and for OpenSSL 3.0. Shells: `dash`, `bash`, `busybox sh`. `shellcheck` and `dash -n`.
- Cases: fresh install; upgrade; already current; a lower latest version refused; `--version`; uninstall leaves the home directory's file list exactly as before the install; altered tarball; altered `SHA256SUMS`; signature by an unknown key with and without `--trust-repo-keys`; a script cut off at several lengths does nothing; a missing `desktop-file-install`; `/usr/bin/rex-linux` present; a receipt that does not match.
- Both verifiers must give the same answer on every good and altered fixture.
- After each release, an agent runs the real command in fresh containers and, with a temporary `HOME`, on shuvmbp-1 for aarch64.

**Risks**

| Risk | Handling |
| --- | --- |
| The OpenSSL route behaves differently on OpenSSL 3.0. | The container matrix runs the verifier on 3.0 (`ubuntu:22.04`, `debian:12`). A distribution where it fails is listed as needing `minisign`. |
| `curl … \| sh` is trust on first use. | Stated in the README, as the approved plan states it for every channel. The signature protects a saved or pinned copy and every later run. |
| The script's uninstall list drifts from `install-desktop.sh`. | The file-list comparison in the tests. |
| A user install shadows a package. | The `/usr/bin` refusal; the README already warns against mixing. |

### 4.4 U2: Arch package and repository

| Part | Contents |
| --- | --- |
| Packaging | `packaging/arch/PKGBUILD`; `scripts/build-arch-package.sh` (runs in the container). |
| Metadata | `release-manifest.py`, `sign-release.py` and `test-release.py` accept six packages and write the `pacman` entries; the attestation covers `*.pkg.tar.zst`. |
| Site | `scripts/build-repo-site.py`; a `pages` job; `repo.yml`; `scripts/make-package-key.sh` for the owner's one run on the offline machine. |
| Settings (done when the PR lands, not now) | Pages with source "GitHub Actions"; the `github-pages` environment allowed to deploy from `v*` tags; the custom domain; the two new secrets. |
| Docs | README "Arch Linux" install section; `PRODUCT.md`'s "distribution packages" out-of-scope line; `docs/releases.md`. |

**Tests** (`scripts/arch-repo-test.sh`, in the `archlinux` container, with a throwaway OpenPGP key and two versions of a stub package): the documented setup, then `pacman -Syu` with no warnings; install the old version, publish the new, upgrade; `pacman -Qi` reports "Validated By: Signature"; an altered package and an unsigned database are both refused; an older file installs with `pacman -U`; `pacman -Qkk` and `namcap` are clean; both `.PKGINFO` files name the right architecture; the marker file's content. The manifest tests gain the six-package cases. Live acceptance is shuvdev and shuvmbp-1.

**Risks**

| Risk | Handling |
| --- | --- |
| `makepkg` for aarch64 on an x86_64 runner writes a wrong package. | Nothing is compiled or stripped; CI checks `.PKGINFO` and compares the binary. shuvmbp-1 is the real test before the README claims aarch64. |
| The first Pages deploy from a tag is rejected by the `github-pages` environment. | Allow `v*` there during setup and prove it with a `repo.yml` run before the release that depends on it. |
| The address is permanent. | Question 1, answered before U2 starts. |
| Release assets grow from 10 to 14. | The upload list in `release.yml` stays explicit, and `docs/releases.md` counts them. |
| Omarchy or another derivative differs from Arch. | shuvdev is Omarchy and is one of the two acceptance machines. |

### 4.5 U3: channel markers, receipt, Nix docs

The approved PR 3 without the `.deb`'s source file, which waits for U5: `REX_LINUX_CHANNEL=official` in `release.yml` and `nix` in `nix/package.nix`; `/usr/share/rex-linux/channel` in the `.deb` (`channel=deb`); the receipt at `$XDG_DATA_HOME/rex-linux/install.json`, written by the tarball's `install.sh` and removed by both uninstall paths and by `mise run install`; and the "staying current" table in `docs/nix.md` from the approved plan's section 8. Tests are the approved ones for these parts. Nothing reads the markers until U4, so the risk is only a wrong file, which the package tests check.

### 4.6 U4: update check and notice

As approved (PR 4), with these changes:

- The action table gains **Package (pacman)**: the marker's hint, `sudo pacman -Syu`, selectable, with Copy. A full upgrade, because Arch does not support partial ones.
- **Tarball**, until U6 exists, shows the install command, selectable, with Copy.
- **Package (deb)**, until U5 exists, links to the release's `.deb`.
- The embedded keys are read at compile time from `packaging/keys/*.pub`, the same files CI and the install script are checked against.

Tests and risks are the approved ones. One added check: a headless test for each row above.

### 4.7 U5: APT repository

As approved (PR 2, plus the `.deb` carrying `rex-linux.sources` as a conffile from PR 3), with these changes:

- The repository root is `https://pkg.shuv.dev`. `rex-linux.sources` and the key sit at the root, next to `arch/`.
- `build-repo-site.py` gains the APT layout (`apt-ftparchive`, `InRelease`, `Release.gpg`); the Pages job, the key and `repo.yml` already exist.
- It ships in whichever release is next once it passes `scripts/apt-repo-test.sh` (the approved container test on `ubuntu:24.04` and `debian:trixie`). The `.deb` gains its source in that same release, so no `.deb` ever points at an empty repository.

### 4.8 U6: tarball self-updater

Unchanged from the approved PR 5, but last, and to be confirmed when its turn comes. If it is built, the install script and the updater must agree on one receipt format, and the end-to-end test in the approved plan stays the bar.

## 5. File downloads (API 13)

`com.superlogical.terminal.download_begin` takes a `ref` (`download:` and a UUID the client chooses), a `path`, and optionally `if_none_match`. It answers with the resolved absolute `path`, a `name`, a `size` that is a hint only, `modified_at`, and usually a single-use `ticket` for a `rex.data.bytes.v1` stream. The API help does not say how the stream ends, and no client code has read one. That is why D0 comes first.

### UX

- **Starting one.** A new action, `pane.download`, shown as **Download file…** in the pane menu and in Commands. It has no default key and can be bound like any other action. It opens a dialog, so its remote class is "never": a `client.perform` request cannot start a download, and neither can anything a terminal prints. The pane is captured when the dialog opens, as it is for uploads.
- **The dialog** is titled with the host ("Download from devbox") and has one field, **File on devbox**, which accepts an absolute path, `~/…`, or a path relative to the pane's foreground program's directory. If the pane has a single-line selection, the field starts with it. Below it, **Save in** shows the folder, with **Change** to edit it. Enter starts; Escape closes.
- **Save dialog.** The first version has no system file chooser. Rune has none, and the tiling desktops this client is mostly used on often have no portal backend, so an in-app folder field must exist either way. A **Choose…** button that calls the desktop portal over the existing `zbus` dependency is a later, optional piece (D4).
- **Location.** The XDG download directory (`xdg-user-dir DOWNLOAD`, else `~/Downloads`), or `downloads.directory` from `workspace.json`, which **Change** writes. The file keeps the server's `name` after sanitizing. An existing name is never overwritten: the new file becomes `name (1).ext`, and so on.
- **Large files.** If the size hint is 1 GiB or more, or more than the free space minus 256 MiB, the dialog stays open and says so ("build.img is 3.2 GB. ~/Downloads has 41 GB free."), with **Cancel** focused.
- **Errors before any bytes** (no such file, a folder, no permission, an old server) stay in the dialog, in the server's words.
- **Progress.** While a transfer runs, one toolbar chip, in the form of the pending-sequence chip: `↓ build.log 42%`, or `↓ 3 files`. Clicking it opens a Downloads popover with a row per transfer from this launch: name, host, a state line ("12.4 of 80 MB", "Saved to ~/Downloads", "Cancelled", "Failed: …"). With nothing running there is no chip; Commands → Downloads opens the same popover.
- **Cancel** is on the row. It closes the stream and deletes the partial file.
- **When it finishes:** one information notice, "Saved build.log from devbox to ~/Downloads". A failure is a sticky notice with the reason. Nothing is retried, as with uploads.
- **Opening.** Never automatic, and left out of the first version. A finished row offers only **Show in folder** (the file manager's `ShowItems` call, else `xdg-open` on the folder). An **Open** button was built (an allow list of types checked by name, first bytes and the desktop's own `xdg-mime` answer) and removed: a file's name and content cannot pin which program a desktop runs (the type comes from the name on some desktops and from the content on others, and `xdg-open` falls back to `$BROWSER` or mailcap when it has no default or no display), and the default program for a type is not Rex's to vet. Open a saved file from the file manager.

### Storage and cache policy

- **No cache in the first version.** A download is a file the user asked for, in a folder they can see. The client sends no `if_none_match`, so it should never receive `not_modified`; if it does, it reports an error. Nothing is kept under `$XDG_CACHE_HOME`.
- **Partial files** are written in the destination folder as `.<name>.<uuid>.rex-part`, so the final rename stays on one filesystem. They are created with `O_EXCL` and `O_NOFOLLOW` and mode 0600, and each is recorded in `$XDG_STATE_HOME/rex-linux/downloads.json`. A partial file is deleted on cancel, on failure, at exit, and at the next launch if the app was killed. Only recorded files are deleted; the folder is never swept.
- **Finishing** is a rename that fails if the name exists (`RENAME_NOREPLACE`); the mode becomes 0644 less the umask. The executable bit is never set.
- **If a cache is added later** (for previews, or from a link), it follows the server's rule: store only when the reply has a `version`, validate with `if_none_match`, keep it under `$XDG_CACHE_HOME/rex-linux/downloads/` in 0700 directories with a size cap, and let Settings clear it. That is a separate decision.

### Size limits

| Limit | Value |
| --- | --- |
| Confirmation | A hint of 1 GiB or more, or more than free space less 256 MiB |
| Free-space reserve | Abort when the destination has under 256 MiB left |
| Growth past the hint | Abort at the larger of twice the hint and the hint plus 64 MiB (the server says the stream may differ from the hint; a file being appended to must not run for ever) |
| Stalled stream | 30 seconds without bytes |
| At once | Three per host; a fourth is refused with a notice |
| Memory | Chunks go straight to disk; nothing is buffered whole |

There is no fixed maximum size. The disk and the confirmation are the limits.

### Remote-host safety

- **Nothing a host sends starts a download:** no escape sequence, no `client.perform`, no queued client action. Only this person's action does.
- **The file name is hostile input.** Take the last path component; drop control characters and Unicode direction overrides; turn a leading dot into `_`; trim trailing dots and spaces; replace an empty name, `.` or `..` with `download`; cap it at 255 bytes, keeping the extension. The same cleaned name is used in the dialog, the notices and the popover.
- **The destination is never followed through a symlink and never overwritten.**
- **Never opened, never made executable.** Rex Linux does not open a downloaded file at all (see Opening); the only thing a row can do with a finished file is show it in its folder.
- **The resolved path the server reports** is shown in the confirmation and in the row's tooltip, so a relative path that resolved somewhere unexpected is visible.
- **A download is tied to the host whose pane was captured.** Its control connection makes the request; the bytes arrive on that host's data endpoint, including through an SSH forward.
- **Revision gate:** 13. On an older host (shuvbot today) the dialog says "Rex on shuvbot is too old to send files; update Rex there."

### Pieces

| PR | Contents | Size |
| --- | --- | --- |
| **D0** | `scripts/download-contract.py` against a private API 13 server under `~/.cache/agent-ws/`: how the stream is framed and how it ends, close codes, and the replies for a missing file, a folder, a symlink, a FIFO, an unreadable file, an empty file, a file growing during the transfer, an expired and a reused ticket, and when `version` is absent. Results go into `docs/server-api13.md`. No client change. | S |
| **D1** | `src/download.rs` (names, destination, receiving, limits), `HostCommand::Download` and a cancel, the model's transfer list, the revision gate. | M |
| **D2** | The action, dialog, chip, popover, notices and the `downloads.directory` setting, following `DESIGN.md`, checked in a rendered build. | M |
| **D3** | `scripts/download-acceptance.py` and the README, `PRODUCT.md`, `DESIGN.md` and changelog text. The README claims downloads only after this passes. | S |
| D4 (optional, later) | Portal **Choose…**; Ctrl+click on a `file://` link filling in the dialog. | S each |

### Tests

- **Unit:** the name table (traversal, control characters, direction overrides, dots, long names, empty); unique names; refusing a symlinked destination; partial-file records and cleanup; every row of the limits table with injected free space.
- **Mock server:** the request (`ref` format, no `if_none_match`); a reply with no ticket; an unexpected `not_modified`; a stream shorter and longer than the hint; a close that is not normal; a stall; cancel closes the socket and removes the partial file; a reconnect mid-transfer fails the transfer and sends nothing again; revision 12 never sends the request.
- **Headless UI:** dialog focus and Escape; the selection prefill; the large-file state with Cancel focused; the old-server message; the chip appears and goes; a finished row's actions; nothing is opened without a click; `pane.download` is in the never-remote class and the allowed set is still exactly eight.
- **Live** (private API 13 server, built binary, Xvfb): names with spaces, Unicode and a newline; a leading dash; `~` and a relative path after `cd`; a symlink; a folder and a FIFO refused; an empty file; a 1 GiB sparse file cancelled halfway; a file appended to during the transfer; the same file twice gives `name (1)`; an API 12 server shows the message; one transfer through a real `ssh://` host, as the remote-control acceptance does.

### Risks

| Risk | Handling |
| --- | --- |
| The stream's framing differs from the upload direction. | D0 before any client code. |
| Rex changes the method while it is in alpha. | The revision gate, and D0 rerun on every server bump, as the compatibility notes already do. |
| A crafted name or a race in the destination folder writes somewhere else. | Exclusive, no-follow creation; no-replace rename; the tests above. |
| A large file fills the disk. | Confirmation, the reserve, the growth limit. |
| A chip in the toolbar contradicts "quiet when healthy". | It exists only while a transfer runs, like the pending-sequence chip. `DESIGN.md` gains the rule. |
| Only two of the three servers support it. | The gate's message; shuvbot stays the live API 12 case. |

## 6. Fork harvest

### New commits since the November harvest

The last harvest recorded kitten-lily `master` at `1ba8a1c` and kamleshkc2002 `master` at `da8da40`. Checked on 2026-11-10 (PST):

```sh
gh api repos/kitten-lily/rex-rune/compare/1ba8a1c...bd5a4a7 --jq '.status, .total_commits'
gh api repos/kamleshkc2002/rex-rune/compare/da8da40...0e5d65d --jq '.status, .total_commits'
gh api repos/shuv1337/rex-rune/compare/master...kitten-lily:master --jq '.ahead_by, .behind_by'
gh api repos/shuv1337/rex-rune/compare/master...kamleshkc2002:master --jq '.ahead_by, .behind_by'
```

| Fork | Branch | Then → now | Compare result | New commits of its own |
| --- | --- | --- | --- | --- |
| kitten-lily | `master` | `1ba8a1c` → `bd5a4a7` | ahead by 135 | **One:** the merge `bd5a4a7`, "Sync upstream master" (2026-11-10 9:49 AM PST), whose parents are `1ba8a1c` and our `0e5d65d` (0.5.0 plus a Dependabot merge). The other 134 are our own commits, `ef1d09a` through `0e5d65d`. `git rev-list bd5a4a7 ^origin/master ^1ba8a1c` returns only the merge. |
| kitten-lily | `feature/scrolling-layout-restructure` | `c3e1eaf`, unchanged | | None. |
| kamleshkc2002 | `master` | `da8da40` → `0e5d65d` | ahead by 74; against our `master`: ahead 0, behind 7 | **None.** All 74 are ours (`1067864` through `0e5d65d`). The fork is a fast-forward mirror. |
| kamleshkc2002 | `release/v0.4.0` | `eb5b183`, unchanged | | None. |

| Commit | Verdict |
| --- | --- |
| kitten-lily [`bd5a4a7`](https://github.com/kitten-lily/rex-rune/commit/bd5a4a7e52e466558baadff0341b338eecc2d36e) Sync upstream master | **Skip** as a commit: it is merge bookkeeping. It is still useful as evidence. Against our side of the merge her tree differs in 27 files (5,484 lines added): her scrolling layout, Activity view and fontconfig chain, now resting on Phase 5 navigation, the Phase 6 keymap and 0.5.0's remote control. Last time "conflicts with Phase 5 and 6" was a reason to skip both features; she has now done that integration once, and it can be read. |
| kamleshkc2002, 74 commits | **Skip.** Ours. |

Two forks have appeared since: **paulbkim-dev** has nothing of its own (its `master` is our `0e5d65d`), and **LoricAndre** has three commits on `master`:

| Commit | Subject | Verdict |
| --- | --- | --- |
| `fd4e753` | feat: add compact/comfortable UI densities | **Adapt (H1).** A Compact pane density: no headers, gutters, margins or rounded cards, and a 1 pt separator between panes, close to tmux. Chrome only; placements stay the server's. To settle in the adaptation: Compact hides the per-pane activity word, which `DESIGN.md` relies on to show which pane needs the user. |
| `e0be986` | fix: clippy lint | **Skip.** A restyled `if let` in `tailnet.rs`; our pinned toolchain is clean. |
| `d3a9cbe` | feat: configure pane padding | **Adapt (H1).** Terminal padding of 0 to 32 pt in Settings → Terminal, default 8, clamped, with tests. The same commit adds `/vendor/` to `.gitignore`, which must not be taken (our `vendor/` is tracked), and puts the runtime libraries on `LD_LIBRARY_PATH` in the Nix dev shell, which is worth taking. |

H1 is about 360 lines with tests, as one PR, crediting Loric ANDRE with `Co-authored-by:` and `Based on LoricAndre/rex-rune@<sha>` lines, as the last harvest did.

### The two skipped changes, reconsidered

**Scroll geometry (kitten-lily's scrolling columns): skip again.**

- It is better bounded than the last harvest's summary suggested. It is off by default and chosen per tab; only the top-level side-by-side parts become columns with client-local widths; splits inside a column keep the server's ratios.
- It still means rewriting two written rules, and her fork does rewrite them: "the client never invents layout" becomes "the scrolling view is the declared exception", and "pane geometry never animates" becomes "cards may move as rigid surfaces".
- It is a second layout mode for good. Every later change to navigation, pane focus, zoom, divider drags and the remote-control guards has to be made twice. It adds about 490 lines to `layout.rs`, a large share of 950 in `workspace.rs`, and four nested settings maps keyed by host, session, window and column.
- A tab whose widths differ on each client resizes its terminals every time ownership moves between this client and the Mac or iOS app. Tiled tabs only do that when the windows differ in size.
- Nobody using this client has asked for it. If the owner wants columns that scroll, that is a product decision with its own plan, not a harvest.

**Activity sidebar: adapt, narrowed, as its own feature (H2), last in the order.**

- What changed: her merge shows the view working on top of Phase 5, Phase 6 and remote control, so the integration is no longer unknown; and the owner's setup (three servers, agents in many sessions) is the case it serves. Tab words tell you about the session on screen. The Activity view answers "which pane, on which host, needs me".
- What still does not fit: in her tree `sync_prefetch` asks for every session's view on every connected host whenever "Show activity in tabs" is on, and that is the default. Phase 5 kept background work bounded on purpose.
- The narrowed version: an **Activity** mode for the sidebar, off by default, next to the unchanged hosts → sessions → tabs mode. Sessions are probed across hosts **only while the sidebar is open in Activity mode**, through her paced queue as it is (one request in flight, 25 ms apart, each terminal once per connection, asked again only when a live event outdated the answer). Closing the sidebar or leaving the mode starts no new probes.
- Take: the probe queue (`a6889a0`), the grouped rows (`8294a11`), the mode (`337b4eb`), focusing the pane a row names (`ef9c0b3`, through Phase 5's tracked navigation), the refresh stamps from `64d5983` without its default change, and OSC 7 directories (`1d27f0d`) as row detail only.
- Still skip: session-row and switcher progress (`b554525`, `0e07ce3`, `4d0fcc6`). They need every session probed all the time, which is the part being declined.
- Size: L. Roughly 1,400 lines of source and 1,000 of tests, in three PRs (queue; model and view; navigation and docs), credited to Lily as before. It needs a `DESIGN.md` pass first, because a list of working agents has to stay consistent with "quiet when healthy".

## 7. #20: cross-server relay (`client_call_requested`)

### What it is

`client.action` and `client.key` accept a `servers` list: other servers this client is connected to, each with a `name` and an opaque `id`. A Lua action in host A's Rex config can then call `rex.server("name")`. The server answers by sending this client a `client_call_requested` event, the client makes that one call on the named server, and it reports the outcome with `client.reply` within 10 seconds. Rex Linux sends no `servers` list, and two tests pin that, so no such event can arrive today.

### Why it is a different permission from Remote control

Remote control lets a host change what this window shows. The relay lets **host A's config act on host B with this user's identity there**: create, close, read and type into terminals. And every ordinary key already goes through `client.key`, so once B is listed, any key typed in a pane on A can run Lua that reaches B.

### Sketch

- **Consent is per pair and off by default.** In Settings → Remote Hosts, host A gets "Key bindings on A may call:" with a switch for each other saved host. It is stored on A as a list of endpoints and cleared when either endpoint changes. No pair is ever implied, including this machine's own server.
- **`servers` carries only pairs that are allowed and connected.** `name` is the saved host's name; `id` is a per-launch value the client maps back. The `server` argument, which runs A's action against a session on B, is never sent: using one host's config everywhere is a different feature.
- **A request is accepted only while one of this client's own `client.key` or `client.action` requests is outstanding on that connection.** Anything else is answered `refused`.
- **Method classes, decided in code by an exhaustive match, default deny.** Never relayed: server lifecycle, `client.perform`, `client.reply`, `client.keymap`, data tickets (`data.connect`, `upload_begin`, `download_begin`) and anything that would chain to a third server. The rest sit in two tiers: reads of metadata, and everything that creates, types, reads content or closes. The second tier is a second switch on the pair.
- **Bounds:** one call in flight per source host; a call budget per action; an 8-second client timeout inside the server's 10; replies carry the connection's epoch, as `client.perform` replies do.
- **Visibility:** one notice per action naming both hosts and the number of calls, and a log line per call.
- **Tests:** the mock harness for consent off and on, each class, epoch and timeout; headless tests for the settings and their clearing; a live check between two private servers with a Lua fixture.
- **Size:** M to L, about 500 lines of source and as many of tests, in two PRs (consent and metadata reads; the second tier).

### Recommendation: not now

- No workflow needs it. An agent or a person on A already reaches B with `rex -S B …`, under their own identity, which is the model Rex's skill teaches.
- It would be the widest permission in the client, and its trigger is ordinary typing.
- Rex is in alpha, and this corner has no reference behaviour to follow yet.
- Keep `servers` empty and the two pinning tests. Leave #20 open with this sketch linked. Revisit when someone has a concrete binding they cannot get any other way.

## 8. Review decisions (resolved 2026-11-10)

The questions below are historical context. Answers: **pkg.shuv.dev**, **no AUR**, and **explicit `--trust-repo-keys` only**.

### Original questions

1. **The pacman repository's permanent address.** A Pages site has one custom domain, and `pkg.shuv.dev` is already chosen for APT. *Recommendation:* serve pacman from `https://pkg.shuv.dev/arch/$arch`. It is odd to read and costs nothing. Nothing has shipped at either address yet, so if a neutral name for both is preferred (for example `pkg.shuv.dev`, with APT under `/apt`), say so before U2 starts; afterwards a change means every user edits a file.
2. **Publish `rex-linux-bin` to the AUR as well?** It needs an AUR account of yours and its SSH key as a CI secret, and it lists the package publicly under that account. *Recommendation:* not yet. Ship the pacman repository, keep the `PKGBUILD` AUR-ready in the repository, and publish when someone outside asks for it or once two releases have gone through the pacman repository cleanly.
3. **The install script's fallback to the repository's keys: by itself, or only on request?** It matters only for a saved copy of the script that is older than a key rotation; a copy fetched fresh always has the current keys. *Recommendation:* only on request (`--trust-repo-keys`, or a yes at a terminal prompt), reading the keys from the release's `v*` tag and printing their fingerprints. An automatic fallback would let someone who can alter what GitHub serves pass a saved script's check by serving their own keys, and that check is the only thing the embedded keys are for.
