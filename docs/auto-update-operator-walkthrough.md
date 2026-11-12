# Operating the update system

A walkthrough for whoever runs Rex Linux releases and update channels: the owner, or an agent working for them. It follows the [approved auto-update plan](https://github.com/shuv1337/rex-rune/blob/plan/auto-update/docs/plans/auto-update.md) as re-ordered in [the roadmap](plans/roadmap-0.5.2.md), and the scripts and workflow in this repository. [`docs/releases.md`](releases.md) stays the reference for the signing pipeline; this document is the order of work.

Written 2026-11-10 (PST) at `61e0f2a`. **Only signed releases exist today.** Every step that depends on something not yet built is marked with the piece that builds it: "(after PR U1)" and so on, using the roadmap's names:

| Piece | Builds |
| --- | --- |
| U1 | The install script (`install.sh` as a release asset) |
| U2 | The Arch package, the signed pacman repository, the Pages site, the OpenPGP package key |
| U3 | Channel markers and the tarball install receipt |
| U4 | The in-app update check and notice |
| U5 | The APT repository |
| U6 | The tarball self-updater |

Each step is marked **[Agent]** (an agent can do it alone) or **[User]** (only the owner can).

## 1. Who does what

**The owner's part, in full:**

| When | What | Why only the owner |
| --- | --- | --- |
| Every release | One click: approve the `release` environment for the tag's run. | It is the control that guards the signing keys. |
| Once (after PR U2) | Add two DNS records for `pkg.shuv.dev`. | DNS ownership. |
| Once (after PR U2) | Run one script on the offline machine and hand over three files. | The OpenPGP primary key never touches an online machine. |
| Once per machine, optional | `sudo` to move a machine from the user install to the pacman package. | Root on the owner's machines. |
| Twice a year | Confirm both offline keys can still be read. | Only the owner holds them. |
| In an incident | Sign a recovery release with the offline backup key; revoke an OpenPGP subkey. | Same. |

Everything else is an agent's: release pull requests, test runs, tagging once told to release, the pre-approval summary, verification, repository rebuilds, container checks, Pages and environment settings during setup, and the monthly health check.

**Reviewed operating rule (2026-11-10, PST).** Agents drive setup, hosting and releases; only physical/offline actions require the owner. PRs, merges and releases in Sprint 1 are approved. For the signing gate, post the release queue item and wait for the owner's answer; only an explicit `approve` response authorizes the agent to approve that pending deployment through the GitHub API. Never infer approval from silence. Sprint 1 includes roadmap items 1–4 only; APT, Activity, the self-updater and #20 are deferred. No AUR; repository-key fallback requires `--trust-repo-keys`.

## 2. Cutting a release

Built today. Steps 1 to 5 can run any time; step 6 starts the clock.

1. **[Agent] Open the release pull request.** Set the version in `Cargo.toml`, the `rex-linux` entry in `Cargo.lock`, and `nix/package.nix`. Give `CHANGELOG.md` a `## X.Y.Z - YYYY-MM-DD` section; the workflow takes the release notes from it and fails if it is empty. Update the README's install examples.
2. **[Agent] Run the checks.**

   ```sh
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets --all-features -- -D warnings
   cargo test --locked --all-features
   cargo test --manifest-path vendor/rexrs/Cargo.toml --all-features
   python3 scripts/test-release.py          # needs minisign; uses throwaway keys
   actionlint .github/workflows/release.yml # if installed
   ```

3. **[Agent] Merge, then run the workflow by hand.** A manual run builds and tests both architectures and signs with a throwaway key. It never enters the `release` environment and cannot publish.

   ```sh
   gh workflow run release.yml --repo shuv1337/rex-rune --ref master
   # A few seconds later, once the run is listed:
   run=$(gh run list --repo shuv1337/rex-rune --workflow release.yml --event workflow_dispatch --limit 1 --json databaseId --jq '.[0].databaseId')
   gh run watch "$run" --repo shuv1337/rex-rune --exit-status
   ```

4. **[Agent] Check that run's output.** Download the packages and the throwaway-signed metadata into one `dist/` and verify with the throwaway public keys that the artifact carries:

   ```sh
   for name in linux-x86_64 linux-aarch64 test-release-metadata; do
     gh run download "$run" --repo shuv1337/rex-rune --name "$name" --dir dist
   done
   python3 scripts/sign-release.py --tag vX.Y.Z --verify-only --dist dist --keys-dir dist
   ```

5. **[Agent] Check that no older release is waiting for approval.** Approving an older run after a newer release would move "latest" backwards.

   ```sh
   gh run list --repo shuv1337/rex-rune --workflow release.yml --status waiting
   ```

6. **[Agent] Tag**, once the owner has said to release.

   ```sh
   git tag -a vX.Y.Z -m 'Rex Linux X.Y.Z' && git push origin vX.Y.Z
   ```

7. **CI builds and attests.** Both architectures are rebuilt and tested, the `attest` job records provenance for the packages, and `publish` stops at the `release` environment.
8. **[Agent] Post the pre-approval summary to the queue:** the tag and its commit; that the commit is on `master` (`git merge-base --is-ancestor vX.Y.Z origin/master`); what changed in the pipeline since the last release (`git diff vPREV..vX.Y.Z --stat -- .github scripts packaging`); the build and attestation results; the run's URL.
9. **[User] Approve.** Open the run, choose **Review deployments**, tick `release`, **Approve and deploy**. A pending approval lapses after 30 days.
10. **CI signs and publishes.** In `publish`, in this order:
    - writes `SHA256SUMS` and `latest.json`, and saves both as `unsigned-release-metadata`;
    - signs both with the CI minisign key;
    - verifies each signature against `packaging/keys/`, checks the trusted comments (`rex-linux vX.Y.Z latest.json`), and rechecks packages against `SHA256SUMS` against `latest.json`;
    - creates a draft, uploads the assets, publishes, and marks it latest.

    Any failure before the draft publishes nothing. Assets per release:

    | From | Assets |
    | --- | --- |
    | 0.5.0 | 8: two tarballs, two `.deb`s, `SHA256SUMS`, `latest.json`, and a `.minisig` for each of those two |
    | (after PR U1) | 10: plus `install.sh` and `install.sh.minisig` |
    | (after PR U2) | 14: plus two `.pkg.tar.zst` and their `.sig` files |

11. **[Agent] Verify the published release.**

    ```sh
    repo=shuv1337/rex-rune; tag=vX.Y.Z
    gh release view "$tag" --repo "$repo" --json isDraft,isPrerelease,isImmutable,assets \
      --jq '{isDraft,isPrerelease,isImmutable,assets:[.assets[].name]}'
    work=~/.cache/agent-ws/verify-$tag
    git clone --depth 1 --branch "$tag" "https://github.com/$repo" "$work"
    gh release download "$tag" --repo "$repo" --dir "$work/dist"
    cd "$work"
    python3 scripts/sign-release.py --tag "$tag" --verify-only
    (cd dist && sha256sum -c SHA256SUMS)
    for file in dist/*.tar.gz dist/*.deb; do gh attestation verify "$file" --repo "$repo"; done
    curl -fsSL "https://github.com/$repo/releases/latest/download/latest.json" |
      python3 -c 'import json,sys; print(json.load(sys.stdin)["version"])'
    ```

    Expect: not a draft, not a prerelease, immutable, the asset count above, every check passing, and the last line printing the new version.
12. **[Agent] Check each channel that exists:** section 3 step 3 (after PR U1), section 5 step 2 (after PR U2), section 6 step 2 (after PR U5), section 7 (after PR U4).
13. **[Agent] Report** the result in a results view. No routine notification.

**If `publish` fails after creating a draft,** follow "Recovery and rotation" in `docs/releases.md`: inspect the draft, take the packages and `release-metadata` from that same run, and upload only what is missing. Never rebuild into the same tag, and never use `--clobber`.

## 3. The install script (after PR U1)

What users run:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh | sh
```

The same command upgrades. `… | sh -s -- --uninstall` removes the install and keeps preferences. Like every other channel (pacman `rex-linux-bin`, `.deb`, Nix and the tarball), it installs only the `rex-linux` desktop client (`~/.local/bin/rex-linux`), its `.desktop` launcher and icons, and never installs, replaces or shadows the `rex` server.

1. **Publishing is part of the release.** The source is `packaging/install/install.sh`. `publish` signs it and uploads it with the other assets, so it passes the same approval click and is immutable afterwards. There is nothing separate to deploy.
2. **Keeping it in step with the keys.** The script carries both public keys as constants. `scripts/test-release.py` fails if they differ from `packaging/keys/release.pub` and `release-backup.pub`, so a key change that misses the script cannot merge. **[Agent]** after any change under `packaging/keys/`, run that test and `bash scripts/install-script-test.sh`.
3. **[Agent] After each release, run the real command in clean containers**, and on shuvmbp-1 with a temporary home for aarch64:

   ```sh
   url=https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh
   for image in archlinux:base ubuntu:24.04; do
     docker run --rm -e url="$url" "$image" sh -euc '
       if command -v pacman >/dev/null; then pacman -Syu --noconfirm desktop-file-utils
       else apt-get update && apt-get install -y curl ca-certificates openssl desktop-file-utils; fi
       useradd -m tester
       run() { su tester -c "curl --proto =https --tlsv1.2 -fsSL $url | sh -s -- $*"; }
       run                                          # installs
       /home/tester/.local/bin/rex-linux --version
       run                                          # reports: already current
       run --uninstall
       test ! -e /home/tester/.local/bin/rex-linux'
   done
   # aarch64, on shuvmbp-1, in a temporary home so the real install is untouched:
   env -u XDG_BIN_HOME -u XDG_DATA_HOME -u XDG_CACHE_HOME -u XDG_STATE_HOME HOME="$(mktemp -d)" \
     sh -c 'curl --proto =https --tlsv1.2 -fsSL "$0" | sh && "$HOME/.local/bin/rex-linux" --version' "$url"
   ```

   Neither container has `minisign`, so both exercise the OpenSSL route. The binary links only glibc at load time, so `--version` runs in a bare container.
4. **[Agent] Check the published script is the reviewed one:**

   ```sh
   diff <(git show vX.Y.Z:packaging/install/install.sh) <(curl -fsSL "$url") && echo same
   ```

## 4. The package site and key custody (U2)

The site is **https://pkg.shuv.dev**, with pacman under `arch/$arch` and room for
APT later. It uses **rex-rune's Pages**, not a second repository: there was no
existing Pages site to conflict with, and the release job can deploy without a
cross-repository token. There is no AUR publication.

Configured by the agent on 2026-11-10 (PST):

- Pages source: GitHub Actions; custom domain: `pkg.shuv.dev`.
- Cloudflare DNS-only CNAME `pkg.shuv.dev → shuv1337.github.io`.
- Account-level Pages domain verification TXT:
  `_github-pages-challenge-shuv1337.pkg.shuv.dev`. **Keep this record.**
- `github-pages` permits `master` (bootstrap) and `v*` tags; administrator bypass
  is off. The signing `release` environment still permits **only v* tags**,
  requires shuv1337's review, and disallows administrator bypass.
- Public certificate: `packaging/keys/rex-linux-archive-key.asc`, also on the site.
  Primary: **4BC4B85F4AF37FA0D834B0B52B192F3817340BAB**.
  Signing subkey: **55B8D5A1BB0A48B766F0323FB588FF9861CADE75**.
  Subkey expiry: **2029-11-09 12:14 PST**.
- Primary custody: shuvbot, `~/rex-signing-backup/openpgp/gnupg/`, encrypted,
  certify-only, as the working copy. Its separate passphrase is in the owner's
  1Password Personal vault (item "Rex Linux pacman-repo OpenPGP primary
  (4BC4…0BAB)") since 2026-11-10; the login Keychain item
  `rex-linux-openpgp-primary` was deleted after a restore test. That item also
  holds an encrypted secret-key export (primary plus CI subkey) and the
  revocation certificate. It is offline-style storage on a separate online Mac,
  not air-gapped storage. A non-cloud offline backup is still an operator
  responsibility.
- CI has **only the signing subkey**, re-encrypted under a different random
  passphrase, as `PACKAGE_SIGNING_KEY` and `PACKAGE_SIGNING_PASSPHRASE`
  in `release`. The subkey export and passphrase files were uploaded through
  `gh secret set --env release`, then `gshred -u` removed them on shuvbot.
  Neither the primary nor its passphrase went to CI.

### Before the first pacman release

`repo.yml` has a **bootstrap** mode on master. It publishes only the public key
and a landing page saying the repository is not available yet. It has no secrets
and cannot run after any non-draft pacman release exists:

```sh
gh workflow run repo.yml --repo shuv1337/rex-rune --ref master -f bootstrap=true
```

Do not retrofit packages onto already-published immutable releases. The first
release tagged after this merge (planned as **0.5.3**) is the first with pacman
packages, signatures and a database; this implementation task does not cut it.
The `arch-package` job is unconditional, so any tag cut from `master` after the
merge ships them.

Six pipeline guards that are worth knowing: the site build keeps the newest five
eligible releases first and only then checks them, so a damaged old release ages
out of the window instead of blocking rebuilds for ever; `publish` passes
`--require-tag` so the release just published must appear in the site, with a
short retry while the releases API catches up; `publish` and `repo.yml` share one
concurrency group, so two rebuilds never interleave.

Done on 2026-11-10: the primary's passphrase moved out of the auto-unlocked login
Keychain into 1Password, with an encrypted copy of the primary and its revocation
certificate in the same item, apart from shuvbot. Still open, by design, for the
operator before 0.5.3: a **non-cloud offline copy** of the encrypted primary and
its revocation certificate. A withdrawal via `exclude`
applies to one run only; make a withdrawal permanent by marking the release as a
prerelease, which the selection honours (GitHub keeps that flag editable on
immutable releases), and only if that fails by committing an `exclude` default.

**Known scope limit:** signing with two subkeys during a rotation overlap works
only while both have more than 180 days left, because the expiry check applies
to every secret subkey in CI. Rotate at about 2.5 years, with a few months of
overlap, and remove the old subkey from the secret before its final 180 days. A successful fixture test is not proof that
GitHub's stored production secrets have been exercised.

Check the DNS, GitHub's Pages domain health and certificate, then enforce HTTPS:

```sh
dig +short CNAME pkg.shuv.dev
gh api repos/shuv1337/rex-rune/pages/health
gh api repos/shuv1337/rex-rune/pages --jq '{cname, https_enforced, cert: .https_certificate.state}'
gh api -X PUT repos/shuv1337/rex-rune/pages -F https_enforced=true
curl -fsSI https://pkg.shuv.dev/rex-linux-archive-key.asc
```

**Never leave a dangling CNAME.** Before disabling Pages, removing its custom
domain, deleting/renaming/transferring the repository, remove or repoint the
Cloudflare CNAME first and wait out its 300-second TTL. Keep the account-level
verification and its TXT record. A host migration must keep the public key and
repository URL; clients still verify signatures. Do not proxy the CNAME through
Cloudflare while GitHub provisions/renews its certificate.

## 5. The Arch repository (U2)

### One-time setup on each Arch machine, after 0.5.3 is published

The certificate is published to `keyserver.ubuntu.com`. Import it and locally
sign **the full primary fingerprint**, not a short ID:

```sh
sudo pacman-key --init
sudo pacman-key --keyserver hkps://keyserver.ubuntu.com --recv-keys 4BC4B85F4AF37FA0D834B0B52B192F3817340BAB
sudo pacman-key --finger 4BC4B85F4AF37FA0D834B0B52B192F3817340BAB
# Compare the full fingerprint above with the public-key registry in this repo.
sudo pacman-key --lsign-key 4BC4B85F4AF37FA0D834B0B52B192F3817340BAB
```

If the keyserver is unavailable, use the same certificate over HTTPS instead
of `--recv-keys`, then run the fingerprint and local-sign commands above:

```sh
curl --proto '=https' --tlsv1.2 -fsSLo rex-linux-archive-key.asc https://pkg.shuv.dev/rex-linux-archive-key.asc
sudo pacman-key --add rex-linux-archive-key.asc
```

Add this **once** to `/etc/pacman.conf` (keep the literal `$arch`):

```ini
[rex-linux]
SigLevel = Required DatabaseRequired
Server = https://pkg.shuv.dev/arch/$arch
```

Then `sudo pacman -Syu rex-linux-bin`. Both package and database signatures are
mandatory. Future upgrades use `sudo pacman -Syu`, without an AUR helper.

If `~/.local/bin/rex-linux` shadows `/usr/bin/rex-linux`, first uninstall the
user copy with the install script's `--uninstall` (preferences are retained).
After installing, `command -v rex-linux` must print `/usr/bin/rex-linux`;
`pacman -Qi rex-linux-bin` should say `Validated By : Signature`, and
`/usr/share/rex-linux/channel` must contain `channel=pacman`.
A later `mise run install` creates a source install and shadows the package again.

### Publishing and rebuilding

1. `arch-package` runs **real makepkg** in a digest-pinned `archlinux:latest`
   container. It repackages both native tarballs, compares the executable bytes,
   and uses `CARCH=aarch64` for ARM (nothing is compiled or executed for ARM).
   Both packages enter the signed checksums, manifest and attestations.
2. `publish`, behind the existing **single release approval**, signs both
   packages, adds their detached signatures to the draft, then publishes.
   It rebuilds the site from the newest **five immutable, signed, stable releases
   with pacman assets**, numerically ordered. Both architectures are mandatory.
3. Every rebuild verifies the minisign checksum signature and its tag-bound
   comment, checks each package digest and OpenPGP signature, and runs real
   `repo-add --include-sigs`. Only the newest version enters the database;
   older packages remain available for deliberate `pacman -U <URL>` recovery.
   The runner signs both databases. **No private key enters a container.**
4. `pages` deploys the completed artifact. No old site/database state is reused.
   If publication succeeds but the site build fails, do not edit the release;
   rerun the repository workflow on a release tag containing this implementation:

   ```sh
   gh workflow run repo.yml --repo shuv1337/rex-rune --ref v0.5.3
   # Withdraw a version, without modifying an immutable release:
   gh workflow run repo.yml --repo shuv1337/rex-rune --ref v0.5.3 -f exclude=0.5.3
   ```

   A withdrawal requires at least one other eligible version; otherwise the
   rebuild fails and retains the last deployed site. The required reviewer
   approves the signing environment. Never replay an old site artifact after
   a withdrawal; rebuild from the releases with the exclusion instead.
5. CI's `Arch repository tests` workflow (also required by Binary release)
   uses throwaway keys and two fixture versions in an x86_64 Arch container:
   real makepkg for both architectures, signed rebuild, `pacman-key`, install,
   upgrade, altered upstream package rejection, altered package signature refusal,
   and altered database signature refusal. ARM package bytes/metadata are checked;
   this is **not** an aarch64 runtime/QEMU test.

### U3 channel markers and receipt

Official CI builds embed `REX_LINUX_CHANNEL=official`; Nix embeds `nix`;
ordinary builds default to `source`. The same official executable goes into
all three package formats. Pacman and Debian add a marker next to the prefix's
`share/rex-linux/`. Debian does **not** configure APT yet.

The tarball's `install.sh` writes `$XDG_DATA_HOME/rex-linux/install.json`
(default `~/.local/share/rex-linux/install.json`), mode 0600, after successful
installation. Schema 1 is `{"schema":1,"path":"<resolved binary>","version":"X.Y.Z","sha256":"<hex>"}`.
Source installs and uninstall remove it. U4 will consume these markers; no
background update check or self-updater is added here.

## 6. The APT repository (after PR U5)

First-time setup is section 4; nothing more is needed. What users do once (and from the first `.deb` built after U5, installing the `.deb` does it for them):

```sh
sudo curl -fsSLo /etc/apt/sources.list.d/rex-linux.sources https://pkg.shuv.dev/rex-linux.sources
sudo apt update && sudo apt install rex-linux
```

1. **Publishing is part of the release.** The same site build adds `dists/stable/…`, `pool/…`, `InRelease`, `Release.gpg` and `rex-linux.sources`, from the newest five signed releases.
2. **[Agent] After each release, check from a clean Ubuntu container:**

   ```sh
   docker run --rm ubuntu:24.04 bash -euxo pipefail -c '
     apt-get update && apt-get install -y curl ca-certificates
     curl -fsSLo /etc/apt/sources.list.d/rex-linux.sources https://pkg.shuv.dev/rex-linux.sources
     apt-get update 2>&1 | tee /tmp/log
     ! grep -E "^(W|E):" /tmp/log
     apt-get install -y rex-linux
     rex-linux --version
     apt-cache policy rex-linux
     cat /usr/share/rex-linux/channel'
   ```

   Expect no warnings, the new version as the candidate from `https://pkg.shuv.dev`, and `channel=deb`. Repeat with `debian:trixie`.
3. **[Agent] Check the upgrade path** once two versions are in the repository: install the older one (`apt-get install -y rex-linux=<previous>`), then `apt-get upgrade -y`, and confirm the version moved.
4. **Rebuilding without a release** is the same `repo.yml` run as for Arch.

## 7. The in-app notice (after PR U4) and the self-updater (after PR U6)

### What users see

- **Once a day**, about 30 seconds after launch and then every 24 hours, the app reads `latest.json` and its signature from `releases/latest/download/`. Nothing about the user is sent. Source builds never check on their own. The check can be turned off in Settings or with `REX_LINUX_NO_UPDATE_CHECK=1`.
- **One information notice per new version:** "Rex Linux X.Y.Z is available. Settings → Updates has the details."
- **Settings → Updates** shows the installed version, how it was installed, the last check, a **Check now** button, and the action for that install:

  | Install | Action shown |
  | --- | --- |
  | pacman package | `sudo pacman -Syu`, with Copy |
  | `.deb` | The `apt` command with Copy (after PR U5); before that, a link to the `.deb` |
  | Tarball or install script | **Update now**, then **Restart to finish**; **Copy command** remains the fallback |
  | Nix | `nix flake update rex-rune` or `nix profile upgrade rex-linux` |
  | Source build | "Built from source. Update with git." |
  | Anything else | A link to the release |

- A failed or offline check is silent. Only **Check now** and `rex-linux --check-update` report errors.

### Testing before a release is published

The shipped app has no way to point at another address or key, on purpose. Test against a staged release like this:

1. **[Agent]** Take the packages and `test-release-metadata` from the manual workflow run (section 2, steps 3 and 4). They are the staged release: real packages, with metadata signed by a throwaway key whose public half is in the artifact.
2. **[Agent]** Serve that directory on loopback and run the update probe (`src/bin/update_probe.rs`, after PR U4), which calls the same library code with the test key and address. Check that it reports the staged version and the right action for each channel.
3. **[Agent]** Run `cargo test --locked --no-default-features update::installer`. The signed loopback fixture installs a stub `99.0.0` package into a temporary prefix through the real verification, extraction, desktop-file-utils and transaction code. Separate tests cover tampering, downgrade/floor refusals, hostile tar entries and rollback at every replacement. See [self-update validation](self-updates.md). There is no shipped test-key or alternate-download override.
4. **[Agent]** After publishing, on a machine still on the previous version, run `rex-linux --check-update`. It must print the channel, the installed version, the new version and the action.

### Pulling a bad release

Roll forward. Nobody is ever moved down a version automatically.

1. **[Agent] Point "latest" back at the last good release.**

   ```sh
   repo=shuv1337/rex-rune; bad=vX.Y.Z; good=vX.Y.W
   gh release edit "$good" --repo "$repo" --latest
   gh release edit "$bad" --repo "$repo" --prerelease
   curl -fsSL "https://github.com/$repo/releases/latest/download/latest.json" |
     python3 -c 'import json,sys; print(json.load(sys.stdin)["version"])'    # the good version
   ```

   The first command is the one that matters; the latest pointer stays editable under immutable releases. If GitHub refuses the second on an immutable release, carry on. Do not delete assets and do not move the tag.
2. **[Agent] Rebuild the package site without it** (after PR U2): `gh workflow run repo.yml --repo "$repo" --ref "$good" -f exclude="${bad#v}"`. **[User]** approves.
3. **What each channel does now:**
   - The notice and the install script see the good version as latest. Installs already on the bad one are offered nothing and are not downgraded. The script says how to go back by hand (`--version`).
   - pacman and APT stop offering the bad version. Someone who has it can go back with `sudo pacman -U https://pkg.shuv.dev/arch/$(uname -m)/rex-linux-bin-<good>-1-$(uname -m).pkg.tar.zst` or `sudo apt install rex-linux=<good>`.
   - The tarball self-updater never downgrades. It automatically rolls back failed installation transactions, not completed releases. To deliberately return to an older release, run the verified install script with `--version X.Y.Z`; no **Roll back** button or `--rollback` CLI is shipped.
4. **[Agent] Prepare the fixed patch release** through section 2. **[User]** approves it. Publishing it marks it latest again.

## 8. Nix

Docs only: the app never updates a Nix install.

1. **[Agent]** The release pull request already sets `version` in `nix/package.nix`. A Rune bump also needs the `rune-kit` hash there, and a `ghostty-vt` bump needs the Ghostty archive and Zig dependency hashes (see `AGENTS.md`).
2. **How users follow releases** (`docs/nix.md`, section added after PR U3):

   | Installed as | To update |
   | --- | --- |
   | A flake input, `github:shuv1337/rex-rune` | `nix flake update rex-rune`, then rebuild |
   | `nix profile install github:shuv1337/rex-rune` | `nix profile upgrade rex-linux` |
   | Pinned to a tag, `github:shuv1337/rex-rune/vX.Y.Z` | Change the tag |

   The plain reference follows `master`, which can be ahead of the latest release.
3. **[Agent]** On a machine with Nix, after a release: `nix build github:shuv1337/rex-rune/vX.Y.Z#rex-linux && ./result/bin/rex-linux --version`.
4. (after PR U4) A Nix build shows the notice with the commands above and never a button.

## 9. Keys: rotation, revocation and incidents

| Key | Where the secret is | Trusted by |
| --- | --- | --- |
| Minisign CI key, ID `A1FD78C27652816E` | `RELEASE_MINISIGN_KEY` and `RELEASE_MINISIGN_PASSWORD` in the `release` environment | `packaging/keys/release.pub`; the install script (after PR U1); the app (after PR U4) |
| Minisign backup key, ID `368EA1A12E2721F3` | The owner's offline machine. Never on GitHub. | `packaging/keys/release-backup.pub`; the same clients |
| OpenPGP package key (after PR U2) | Primary: offline. Signing subkey: `PACKAGE_SIGNING_KEY` in `release`. | pacman keyrings; `rex-linux.sources` (after PR U5) |

### Planned rotation of the minisign CI key

- **Before PR U4 ships, a rotation costs nothing:** no installed app holds a key. **[Agent]** generates a new encrypted key pair in a private directory outside any checkout, replaces the two environment secrets, replaces `packaging/keys/release.pub`, updates the key ID and fingerprint in `packaging/keys/README.md`, `docs/releases.md` and the README, updates the install script's constant, and runs `python3 scripts/test-release.py`. The next release is signed by the new key. Saved copies of an older install script fail closed, or go through the key fallback.
- **After PR U4, rotate with an overlap,** as `docs/releases.md` says: a release signed by the old key adds the new public key to what clients trust; keep signing with the old key for about a year; then switch the secret. This needs a small pull request first, because `sign-release.py` reads exactly two key files today. Installs older than the bridging release reinstall by hand once.

### The CI minisign key leaks, or might have

A leaked key alone signs nothing useful: the attacker also has to control what gets downloaded. Act anyway.

1. **[Agent] Remove the secrets at once.** Tag releases then fail closed.

   ```sh
   gh secret delete RELEASE_MINISIGN_KEY --repo shuv1337/rex-rune --env release
   gh secret delete RELEASE_MINISIGN_PASSWORD --repo shuv1337/rex-rune --env release
   ```

2. **[Agent] Look for misuse:** every release and its assets (`gh release list`, `gh release view`), the latest pointer, and recent runs of `release.yml` and who approved them.
3. **[Agent] Prepare the recovery release:** a new CI key pair; `packaging/keys/release.pub` replaced, so nothing trusts the leaked key any more; the install script's constant; the app's embedded keys (after PR U4).
4. **[User] Sign that release offline with the backup key,** following "Manual or offline signing" in `docs/releases.md`. Every client already trusts the backup key. That is what it is for.
5. **[User] Generate a new backup key pair on the offline machine** and hand over only its public half. **[Agent]** commits it in the same release, as the approved plan requires.
6. **[Agent] Store the new CI secrets,** publish the recovery release by the manual steps in `docs/releases.md`, and verify it (section 2, step 11).
7. Installs that have not updated since before the leak still trust the old key. Nothing fixes that from here, so say it in the release notes.

### The offline backup key is lost or exposed

**[User]** makes a new one. **[Agent]** ships its public half in the next ordinary release, which the CI key signs. After PR U4 that needs the same overlap as a planned rotation. If both minisign keys are lost, self-update stops and people reinstall by hand. There is no unsigned path.

### The OpenPGP subkey: expiry and rotation (after PR U2)

- The subkey expires three years after it was made (this one on 2029-11-09 PST). From about 2.5 years in, the release job fails when under 180 days remain, so rotation happens early.
- **[User]**, offline: add a new signing subkey with the primary key and export the certificate and the new subkey, with the same script as in section 4.
- **[Agent]:** commit the new certificate, replace the two secrets, and sign the repositories with **both** subkeys during the overlap; drop the old subkey from the secret before it has 180 days left, since the expiry check covers every subkey in CI. Rebuild the site (section 5, step 5).
- **What users must do:** pacman users run the `pacman-key --add` command again; the primary key they signed locally has not changed. APT users get the new certificate with a normal package upgrade, because the `.deb` carries `rex-linux.sources`.

### The OpenPGP subkey leaks (after PR U2)

1. **[Agent]** delete `PACKAGE_SIGNING_KEY` and `PACKAGE_SIGNING_PASSPHRASE` from the environment.
2. **[User]**, offline: revoke the subkey with the primary key, add a new one, export.
3. **[Agent]:** commit the certificate, set the secrets, rebuild the site. **[User]** approves.
4. APT installs that still hold the old certificate see a signature error on `apt update` until they fetch `rex-linux.sources` again; put the one-line command in the release notes. pacman users import the new certificate.
5. As with minisign, the leaked subkey helps only someone who also controls the site or its DNS. Check both (section 10).

### What none of this covers

A takeover of the GitHub account can approve the environment and sign. The defence is hardware two-factor on the account, as the approved plan says.

## 10. Monthly health check

All **[Agent]**, read-only, about ten minutes. Report only what is wrong.

1. **The latest pointer and its signature.**

   ```sh
   repo=shuv1337/rex-rune; d=$(mktemp -d)
   for f in latest.json latest.json.minisig SHA256SUMS SHA256SUMS.minisig; do
     curl -fsSL -o "$d/$f" "https://github.com/$repo/releases/latest/download/$f"
   done
   for f in latest.json SHA256SUMS; do
     minisign -Vm "$d/$f" -p packaging/keys/release.pub || minisign -Vm "$d/$f" -p packaging/keys/release-backup.pub
   done
   python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$d/latest.json"
   gh release list --repo "$repo" --limit 3
   ```

   The version must be the newest release, and the signature should be the CI key's. A backup-key signature outside an incident is itself an incident.
2. **Releases are immutable and complete:** `gh release view <tag> --repo "$repo" --json isImmutable,isDraft,assets` for the newest three; no stray drafts.
3. **The gate is as it was set.**

   ```sh
   gh api repos/$repo/environments/release --jq '{can_admins_bypass, reviewers: [.protection_rules[].reviewers[]?.reviewer.login]}'
   gh api repos/$repo/environments/release/deployment-branch-policies --jq '[.branch_policies[] | {name, type}]'
   gh secret list --repo "$repo" --env release
   gh api repos/$repo/rulesets --jq '[.[] | {name, enforcement}]'
   gh run list --repo "$repo" --workflow release.yml --status waiting
   ```

   Expect: no administrator bypass; reviewer `shuv1337`; only `v*` tags; the secret names from section 9 and no others; both tag rulesets active; nothing waiting.
4. **Recent release runs:** each tag run was approved by the owner, and none ran from a ref other than a `v*` tag.
5. **The install script** (after PR U1): section 3, steps 3 and 4.
6. **The site** (after PR U2):

   ```sh
   gh api repos/$repo/pages --jq '{cname, https_enforced, cert: .https_certificate.state}'
   dig +short CNAME pkg.shuv.dev
   gpg --show-keys packaging/keys/rex-linux-archive-key.asc     # the subkey's expiry: more than 180 days away
   ```

   Then the Arch container check (section 5, step 2) and, after PR U5, the Ubuntu one (section 6, step 2).
7. **The notice** (after PR U4): `rex-linux --check-update` on shuvdev and shuvmbp-1.
8. **Pipeline upkeep:** open Dependabot pull requests for the pinned actions, and the pinned container digests.
9. **[User], twice a year:** confirm that the backup minisign key and the OpenPGP primary key can be read from both offline copies. Nothing is signed.
