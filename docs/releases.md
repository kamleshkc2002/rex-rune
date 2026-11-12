# Publishing a binary release

`.github/workflows/release.yml` builds natively on Ubuntu 24.04 for x86-64 and ARM64. Both architectures must pass formatting, Clippy, application tests, SDK tests, packaging and install/uninstall checks before publication. The toolchains come from `mise.toml`. No client auto-update code is included in the signed-release pipeline.

## Release sequence

Complete the [owner setup](#owner-setup-before-the-first-signed-release) before the first signed tag. The owner approved CI custody: signing runs in `publish`, inside the protected `release` environment with shuv1337 as required reviewer. The isolated signing step can be replaced with [manual signing](#manual-or-offline-signing).

1. Update the version in `Cargo.toml`, the `rex-rune` entry in `Cargo.lock`, and `nix/package.nix`; update `CHANGELOG.md` and installation examples as needed.
2. Commit and push to `master`. Run **Binary release** manually in GitHub Actions to test without publishing. The `release-tools` job tests the manifest and signs/verifies fixture metadata using encrypted throwaway keys, deleted at test exit. The separate `manual-signing-test` job gathers real build artifacts, generates metadata, and signs/verifies it with an encrypted throwaway minisign key pair in `$RUNNER_TEMP`, then deletes the key pair. It has only `contents: read`, no environment, and no production secrets. **`publish` never runs on `workflow_dispatch`, even when run on a tag**. Manual runs do not create attestations or request environment approval.
3. After checking the results, create and push an annotated tag matching the package version, for example `git tag -a v0.5.0 -m 'Rex Rune 0.5.0'` and `git push origin v0.5.0`. Only stable `vX.Y.Z` versions are supported by the manifest generator.
4. The tag workflow rebuilds all eight packages (two `rex-rune` tarballs, two [bridge tarballs](#the-rex-linux-bridge), two `.deb`s, two pacman packages), then the separate `attest` job downloads and attests them using `actions/attest-build-provenance`. `publish` requires successful attestation. Inspect the triggering commit and workflow, then approve the `release` environment as its required reviewer. Builds have no signing secrets; `publish` alone receives them.
5. `publish` gathers both architectures, writes `SHA256SUMS`, generates `latest.json`, and uploads both as the separate `unsigned-release-metadata` artifact **before signing**, so they remain available if signing fails. It also copies `packaging/install/install.sh` byte for byte into `dist/`, signs all three files (the two metadata files and installer), verifies each signature and exact trusted comment against the repository's public keys and candidate tag, and rechecks packages ↔ `SHA256SUMS` ↔ `latest.json` consistency. **A tag run fails closed** if either secret, either public-key file, or a valid signature is missing. Nothing is published on failure.
6. Only after verification does the job create a draft and upload **all sixteen assets** (eight packages, two detached OpenPGP pacman signatures, `SHA256SUMS`, `SHA256SUMS.minisig`, `latest.json`, `latest.json.minisig`, `install.sh`, `install.sh.minisig`), then publish and mark latest. The same approved job rebuilds and signs the package repository; a separate Pages job deploys its public artifact. Check the assets and attestations. Never move published tags, replace assets, or retrofit signatures onto old releases.

The manifest is a release asset at:

- `https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json`
- `https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json.minisig`

Its schema is `1`, product is `rex-linux` ([why](#the-rex-linux-bridge)), and both `x86_64-linux-gnu` and `aarch64-linux-gnu` targets are mandatory. Each target has `min_glibc`, two version-specific tarball URLs (`archive` for current clients, `tarball` for the 0.7.x updater), Debian and pacman filenames, and sizes and SHA-256 digests for each package. The generator checks all eight package hashes against `SHA256SUMS`; missing, extra, duplicate, malformed or mismatched entries fail. Output is stable for fixed inputs. `published` records metadata generation time (before publication), using an RFC 3339 Pacific Time offset (PST/PDT); `--published` allows a fixed timestamp for reproducibility. There is **no expiry**.

`update_from` is currently `0.4.2`, the current compatible tarball layout, not a claim that that version already has an updater. Raise the workflow's explicit floor only when an installation-layout change requires a manual reinstall. `min_glibc` defaults to `2.39`; change `--min-glibc` with any future build baseline change. Trusted signature comments name the product id, tag and file, for example `rex-linux v0.8.0 latest.json`.

## The rex-linux bridge

The client was `rex-linux` through 0.7.x. Four things keep that name so those installs can reach a renamed release, and each has a cost if it changes:

| Kept | Where | Who depends on it |
| --- | --- | --- |
| Product id `rex-linux` | `latest.json`'s `product`; the trusted comment of all three signatures | The updater in every 0.7.x client refuses any other. Current clients and `install.sh` check the same id, so there is one signing namespace |
| Bridge tarballs `rex-linux-<version>-<target>.tar.gz` | Release assets; `tarball` in `latest.json` | The 0.7.x updater's **Update now**. `scripts/test-bridge.py` holds them to its rules |
| The `rex-linux` command | A link in the pacman package, the `.deb`, the Nix package and upgraded `install.sh` installs; `--version` answers `rex-linux X.Y.Z` through it | The 0.7.x updater, which runs `bin/rex-linux --version` before installing; users' scripts and key bindings |
| Repository `[rex-linux]`, `rex-linux.db`, `rex-linux-archive-key.asc` | pkg.shuv.dev | Every `pacman.conf` that already has the repository. The package inside is `rex-rune-bin`, which replaces `rex-linux-bin` |

[Self-update behavior](self-updates.md#updating-from-rex-linux-07x) describes what the client does after the old updater has installed it.

Two things in installed 0.7.x clients cannot be met and are answered in words. Its update notice for a `.deb` install names `rex-linux_<version>_<arch>.deb`, which does not exist: the release notes' first paragraph says to take `rex-rune_*.deb`. And a copy of its `install.sh` that someone kept picks the bridge tarball by name and runs the `install.sh` inside it: that script installs nothing and names the current installer.

**Retiring the bridge.** Keep it for as long as 0.7.x installs should update in place. To end it, in one release: raise `--update-from` in `release.yml` to the first renamed version, so a 0.7.x client reports that a manual upgrade is required and does not download; stop building the bridge tarballs and point `tarball` at the `rex-rune` archive; and remove the `rex-linux` links, `src/legacy.rs` and `migrate_legacy_install`. The product id and the repository name cost nothing to keep, and changing either cuts off installed clients, so leave them unless a second manifest or repository is published beside them first.

`latest.json` cannot gain targets other than the two Linux ones while 0.7.x clients read it: their updater rejects a manifest that names any other target. Releases for another platform need their own manifest.

## Owner setup before the first signed release

**CI custody and both public trust roots are configured.** The owner approved agent generation of the encrypted CI key and random passphrase, uploaded only as `RELEASE_MINISIGN_KEY` and `RELEASE_MINISIGN_PASSWORD` environment secrets in `release`. Private setup files were shredded after upload. The environment requires shuv1337's approval, disables administrator bypass, allows self-review for the single maintainer, and permits only `v*` tags. Active tag rulesets allow admin-only creation and prohibit updates/deletion without bypass; immutable releases are enabled.

The committed CI key is `packaging/keys/release.pub`:

- Minisign key ID: `A1FD78C27652816E`.
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `9e779605311a46a5287d6c9b25eb2083fb17447454239c7d66f1ecebe336d0c6`.

The committed offline recovery key is `packaging/keys/release-backup.pub`:

- Minisign key ID: `368EA1A12E2721F3`.
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `d9fe424ca8b20725e11d6039c39445d38a33023a8d366c69e0612a1c3528de76`.

Both complete public values are in the [public-key registry](../packaging/keys/README.md).
Only the backup public half was supplied; its private key and passphrase remain
on the owner's separate backup machine and must never be uploaded to CI.
Keep recoverable offline copies of the backup secret and passphrase.

**Both required public-key files are present; a tag release no longer fails
for a missing public key.** It still fails closed on missing secrets or invalid
signatures. A passing manual test uses throwaway keys, never enters `release`,
and does not validate the stored production secrets. Do not regenerate either
configured key just to repeat setup.

Do not paste a private key or password into chat, a PR, the repository, or a command-line argument. The backup private key must never reach GitHub. The OpenPGP package key and Pages now serve Arch at `https://pkg.shuv.dev`; APT remains later work. The package secrets are `PACKAGE_SIGNING_KEY` and `PACKAGE_SIGNING_PASSPHRASE`, not `APT_SIGNING_KEY`. See the [operator walkthrough](auto-update-operator-walkthrough.md#4-the-package-site-and-key-custody-u2) for custody, fingerprints, DNS verification, and the no-dangling-CNAME teardown order.

Setup reference for a fresh repository or deliberate rotation (do not overwrite the existing CI key):

1. **Two password-protected minisign key pairs.** On a trusted machine, outside any checkout, run the following yourself. Minisign prompts for passwords. Create the backup pair on your offline machine instead; bring back only its public half. Keep the backup secret and its password offline in two secure places.

   ```sh
   umask 077
   # On the trusted signing machine, in a private directory outside the repo:
   minisign -G -p release.pub -s rex-linux-release.key
   # On the offline backup machine:
   minisign -G -p release-backup.pub -s rex-linux-release-backup.key
   ```

2. **Commit only the two public files**, as `packaging/keys/release.pub` and `packaging/keys/release-backup.pub`, in a reviewed change. Record both public-key values, minisign key IDs and SHA-256 fingerprints of the decoded 42-byte minisign public-key records in the public-key README, replacing the pending-setup wording; keep the README and this document's key IDs and fingerprints in sync. Never commit a throwaway test key as a trust root. The signing helper requires both public files and accepts signatures from either; the later client PR will embed both.

   ```sh
   # From the checkout, after copying only public files from the machines above:
   install -m 644 /path/to/public/release.pub packaging/keys/release.pub
   install -m 644 /path/to/public/release-backup.pub packaging/keys/release-backup.pub
   git add packaging/keys/release.pub packaging/keys/release-backup.pub
   ```

3. **Create and protect the GitHub environment `release` before adding secrets** at <https://github.com/shuv1337/rex-rune/settings/environments>. **GitHub automatically creates an unprotected `release` environment the first time a run references it if it does not already exist.** Do not rely on that automatic creation: explicitly create it, configure its protections, and verify them before adding any secrets. Add **shuv1337 as required reviewer**, disable administrator bypass of protection rules, and allow only `v*` tags under **Selected branches and tags** (no branches). Leave “Prevent self-review” off for the single-maintainer workflow, or designate a second human reviewer if enabling it. Verify the protections before supplying secrets; a workflow's `environment:` declaration alone does not configure them.

4. **Create these two environment secrets**, not repository-wide secrets. The key secret is the complete password-encrypted minisign secret-key file, not base64 wrapped. Supply the password interactively when `gh` prompts; never put it into shell history. Do not upload the offline key.

   ```sh
   gh secret set RELEASE_MINISIGN_KEY --repo shuv1337/rex-rune --env release < /secure/path/rex-linux-release.key
   gh secret set RELEASE_MINISIGN_PASSWORD --repo shuv1337/rex-rune --env release
   # Lists names, not secret values:
   gh secret list --repo shuv1337/rex-rune --env release
   ```

5. **Create an active tag ruleset for `v*`** at <https://github.com/shuv1337/rex-rune/settings/rules>. Restrict tag creation to the maintainer (repository-admin bypass for creation), and restrict updates and deletion. Never use that bypass to move/delete a release tag. This controls who can start a release; it does not replace reviewing the commit before granting the signing environment access.

6. **Enable immutable releases** in the repository's Settings → General → Releases before the first signed release. Drafts remain editable; publication locks their assets and tag. The latest-release pointer remains editable. Existing releases are not retroactively signed or made immutable by this workflow.

7. Run the local tests below and, when ready, a manual **Binary release** run. Inspect its `test-release-metadata` artifact and verify it with the included throwaway public keys (`--keys-dir` pointing to those keys). These test keys are not release trust roots. Confirm the environment protections in Settings before pushing any release tag; manual runs intentionally do not enter that environment. Attestation permissions (`id-token: write`, `attestations: write`) are scoped only to the tag-push `attest` job; `build` has only `contents: read`. No extra signing secret is needed for provenance.

## Arch repository and install provenance

`arch-package` runs `scripts/package-arch.py` after both native builds. It renders
`packaging/arch/PKGBUILD.in` with both real tarball hashes, invokes real `makepkg`
in a digest-pinned Arch container, and compares executable bytes. No signing
secret is present. The package is `rex-rune-bin`, which provides, conflicts with
and replaces `rex-linux-bin`, its name before the rename; no AUR push is configured.

Inside the protected `publish` job, `sign-packages.py` imports only the signing
subkey into a temporary GnuPG home. It refuses a secret primary, an uncommitted
key/subkey, and a subkey with under 180 days until expiry. Packages and detached
signatures become immutable release assets. `build-repo-site.py` then rebuilds
from the newest five signed immutable stable releases, verifies signed checksums
and package signatures, embeds package signatures with real `repo-add`, and signs
both databases. Only the newest release enters the database, so once that is
`rex-rune-bin` every `pacman -Syu` offers the replacement; older package files of
either name stay downloadable. No secret is mounted into Docker. Pages deployment has only
`pages: write` and `id-token: write`, not signing secrets.

For a failed site deploy or withdrawal, run `repo.yml` on a tag containing the
workflow, with optional `exclude=X.Y.Z`; approval is still required. It never
modifies releases. Before the first pacman release, its explicit `bootstrap`
mode on master serves only the certificate and a not-yet-available landing page.
The walkthrough has the exact pacman setup and hosting health checks.

Official builds embed `REX_RUNE_CHANNEL=official`, Nix builds `nix`, and source
builds default to `source`. Every install method (pacman `rex-rune-bin`, `.deb`,
Nix, tarball or `install.sh`) installs only the `rex-rune` desktop client, its
`.desktop` launcher and icons, and never installs, replaces or shadows the `rex`
server. Packages carry a `share/rex-rune/channel` marker and a `rex-linux` link
to the client.
Tarball installation writes the schema-1 path/version/SHA-256 receipt described
in the walkthrough; source installation and uninstall invalidate it. The client
reads these markers and receipts to show channel-specific upgrade guidance in
the in-app update notice and `Settings → Updates`. Verified install.sh installs
under `~/.local` can use **Update now**; all other channels retain package-manager
or manual guidance. See [self-update behavior and validation](self-updates.md).

## Local checks and packaging

```sh
# Python 3.11+ and minisign required; no production credentials or GitHub access.
python3 scripts/test-release.py
# Disposable container matrix: real 0.5.1 assets, tampering, both verifiers,
# OpenSSL 3.0, three shells, old-glibc/musl refusals. Requires Docker.
bash scripts/install-script-test.sh
python3 scripts/test-install-receipt.py # desktop-file-utils required
# After packaging: the bridge tarball against the 0.7.x updater's rules.
python3 scripts/test-bridge.py dist/rex-linux-*.tar.gz --binary target/release/rex-rune
# The 0.7.5 updater installing this build, then the first start (a few minutes; see the script).
bash scripts/bridge-e2e.sh
python3 scripts/test-arch-repo.py       # Docker, gpg, minisign, zstd; throwaway keys only
# If installed (also runs ShellCheck when available):
actionlint .github/workflows/release.yml
```

To package locally after a native `cargo build --locked --release --bin rex-rune`, run `python3 scripts/package-release.py` (tarball) or add `--deb` (also requires `dpkg-dev`). Outputs go into ignored `dist/`. Packages include source commit/toolchain information, desktop integration, dependency license declarations, available license texts, font licenses, and icon provenance. Icon redistribution is authorized by the project owner; it is not covered by the project's MIT license.

The standalone installer is separately signed with trusted comment
`rex-linux vX.Y.Z install.sh`, attested alongside the packages, and excluded from
`SHA256SUMS` (which lists exactly the eight packages). The `install-script` job
must pass before either manual signing or publication; it never sees secrets.
`scripts/test-release.py` pins both embedded keys to `packaging/keys/` and tests
installer tampering along with the two metadata signatures.

The compatibility baseline is the build distribution, not just CPU architecture: current official binaries require glibc 2.39+. Build on an older supported distribution if changing this baseline; do not describe newer-host builds as compatible with older glibc. Runtime-loaded graphics/windowing dependencies are listed explicitly in the Debian package in addition to linked dependencies resolved by `dpkg-shlibdeps`.

## Manual or offline signing

For the offline alternative in plan question 6 (key custody), keep building, attesting and generating metadata in CI, but sign offline and publish manually as below. This PR does not automate an offline handoff. With CI signing secrets absent, a tag run saves `unsigned-release-metadata` and then fails closed at signing. Never publish unsigned files while waiting for signatures, and never use a manual test run's throwaway signatures for a release.

1. Select the **tag-push** run for the exact candidate tag/commit. Check that both builds, `release-tools`, and `attest` succeeded, and that `publish` saved unsigned metadata. Ensure the run has finished or been cancelled before taking over so it cannot publish independently. In a clean checkout of that reviewed tag, download its artifacts into an empty `dist/` (replace `RUN_ID`):

   ```sh
   repo=shuv1337/rex-rune
   tag=v0.5.0
   run_id=RUN_ID
   gh run view "$run_id" --repo "$repo" --json event,headSha,headBranch,status,jobs
   # Match headSha to this checked-out tag and confirm event is push.
   git rev-parse "$tag^{commit}"
   mkdir dist
   gh run download "$run_id" --repo "$repo" --name linux-x86_64 --dir dist
   gh run download "$run_id" --repo "$repo" --name linux-aarch64 --dir dist
   gh run download "$run_id" --repo "$repo" --name linux-pacman --dir dist
   gh run download "$run_id" --repo "$repo" --name unsigned-release-metadata --dir dist
   (cd dist && sha256sum -c SHA256SUMS)
   for file in dist/*.tar.gz dist/*.deb dist/*.pkg.tar.zst; do
     gh attestation verify "$file" --repo "$repo"
   done
   ```

2. Transfer all eight packages plus `latest.json` and `SHA256SUMS` to `dist/` in a trusted offline checkout containing the reviewed signing/manifest scripts and both committed public keys. Confirm the tag/commit and trusted public keys independently. On that offline signing machine, minisign prompts for the key password; use the active or offline backup private key, never a throwaway key:

   ```sh
   tag=v0.5.0
   cp packaging/install/install.sh dist/install.sh
   for file in latest.json SHA256SUMS install.sh; do
     minisign -S -s /secure/path/rex-linux-release-backup.key -m "dist/$file" \
       -t "rex-linux $tag $file"
   done
   # Same metadata verification gate as CI; requires all eight packages but no secrets.
   python3 scripts/sign-release.py --tag "$tag" --verify-only
   ```

   Also sign each pacman package with the offline OpenPGP signing subkey:

   ```sh
   for file in dist/*.pkg.tar.zst; do
     gpg --local-user '55B8D5A1BB0A48B766F0323FB588FF9861CADE75!' --detach-sign "$file"
     gpg --verify "$file.sig" "$file"
   done
   ```

3. Transfer **only the three `.minisig` files and the two `.pkg.tar.zst.sig` files** back to the online checkout's `dist/`. Do not regenerate or reformat signed metadata. Rerun the gate there; it checks both signatures, exact tag/file trusted comments, and the complete packages ↔ `SHA256SUMS` ↔ `latest.json` relationship:

   ```sh
   python3 scripts/sign-release.py --tag "$tag" --verify-only
   ```

4. Prepare and review `RELEASE-NOTES.md` from this version's changelog and the workflow's standing installation notes. Create a draft against the existing remote tag, upload **all sixteen assets**, inspect the draft, then publish:

   ```sh
   gh release create "$tag" --repo "$repo" --draft --verify-tag \
     --title "Rex Rune ${tag#v}" --notes-file RELEASE-NOTES.md
   gh release upload "$tag" --repo "$repo" \
     dist/*.tar.gz dist/*.deb dist/*.pkg.tar.zst dist/*.pkg.tar.zst.sig \
     dist/SHA256SUMS dist/SHA256SUMS.minisig \
     dist/latest.json dist/latest.json.minisig dist/install.sh dist/install.sh.minisig
   gh release view "$tag" --repo "$repo" --json isDraft,assets
   # Only after confirming exactly the sixteen verified assets:
   gh release edit "$tag" --repo "$repo" --draft=false --latest
   ```

Do not use `--clobber`; if a draft already exists, follow recovery below rather than creating another. [README verification instructions](../README.md#verify-a-download) cover manual downloads and `gh attestation verify`.

## Recovery and rotation

If a publish job fails after creating a draft, inspect the draft before retrying; the workflow intentionally refuses to overwrite an existing release and never uses `--clobber`. Recover by downloading packages and the `release-metadata` artifact from **that exact workflow run**, checking all eight hashes (`sha256sum -c SHA256SUMS`), all three minisign signatures (`scripts/sign-release.py --tag vX.Y.Z --verify-only`) and both OpenPGP package signatures, and uploading only missing assets to the draft. Publish only after all sixteen match. Do not rebuild and mix artifacts from unrelated commits or re-sign a different manifest under the same published tag.

**Pulling a bad release:** publish a fixed patch version. Meanwhile mark the bad release as a prerelease so it is not offered as latest, and confirm the latest pointer serves the previous good manifest. Do not delete its assets or move the tag. No automatic downgrade is implied. Once the APT repository exists, rebuild it without the bad release as well.

**Planned minisign rotation:** once the updater embeds keys, ship a release signed by the old key that trusts the new key too. Keep signing with the old key for a long overlap (a year), then switch the CI secret and committed active public key. Installs older than the bridging release must reinstall manually. Coordinate future client trust-root changes with the CI verification keys; do not simply replace a key embedded in installed clients.

**Suspected compromise:** delete the leaked GitHub environment secret. Sign the next release manually with the offline backup key (never upload that key to CI); that release must remove the compromised client trust root and introduce a newly generated backup. Create a replacement CI key and password through the reviewed setup above. Keeping the backup public key in the app from its first updater release makes recovery possible. Losing both private keys requires manual reinstalls; there is no unsigned-update escape hatch.

APT signing and OpenPGP rotation are intentionally deferred to the APT PR.
