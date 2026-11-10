# Release public keys

Only public keys belong here. Keep secret keys and passwords outside the checkout.
Never upload the offline backup secret or its passphrase to CI.

## CI signing key — `release.pub`

- Minisign key ID: `A1FD78C27652816E`
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `9e779605311a46a5287d6c9b25eb2083fb17447454239c7d66f1ecebe336d0c6`

```text
untrusted comment: minisign public key A1FD78C27652816E
RWRugVJ2wnj9oVdIvBhQFK1/BDvL1KkA4aH1HB2Q0uA30OBHXIvTlHwL
```

The encrypted CI private key and random passphrase are environment secrets in
GitHub's protected `release` environment, not repository-wide secrets. Each tag
release requires approval by shuv1337; administrator bypass is disabled.

## Offline recovery key — `release-backup.pub`

- Minisign key ID: `368EA1A12E2721F3`
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `d9fe424ca8b20725e11d6039c39445d38a33023a8d366c69e0612a1c3528de76`

```text
untrusted comment: minisign public key 368EA1A12E2721F3
RWTzIScuoaGONtqf+4m48HAGuwNgUxvd/AFZareIgMwCGCKVjbTjcmJe
```

Only this public half was supplied for the setup PR. Its private key and passphrase
remain on the owner's separate backup machine, never in this checkout or GitHub.
Keep recoverable offline copies; never send the backup secret or its passphrase.
Since 2026-11-10 its password is in the owner's 1Password Personal vault, not the
shuvbot login Keychain, with an encrypted copy of the key attached to the same item.

**Both required public-key files are present.** A tag release no longer fails
for a missing public key, but still fails closed on missing secrets or invalid
signatures. Manual workflow tests use throwaway keys and do not enter the
`release` environment; they cannot prove production-secret availability. The
future updater will embed both public keys; no client verification is added
by this change.

## Package repository — `rex-linux-archive-key.asc`

- Primary (Ed25519, certify-only, no expiry):
  `4BC4B85F4AF37FA0D834B0B52B192F3817340BAB`
- CI signing subkey (Ed25519):
  `55B8D5A1BB0A48B766F0323FB588FF9861CADE75`
- Subkey expires **2029-11-09 12:14 PST**; signing refuses under 180 days remaining.
- Public download: <https://pkg.shuv.dev/rex-linux-archive-key.asc>.

Generated on shuvbot on 2026-11-10. The encrypted primary and revocation certificate
stay in `~/rex-signing-backup/openpgp/gnupg/` as the working copy. Since 2026-11-10
its independent passphrase is in the owner's **1Password Personal vault** (item
"Rex Linux pacman-repo OpenPGP primary (4BC4…0BAB)"), not the auto-unlocked login
Keychain; the former Keychain item `rex-linux-openpgp-primary` was deleted after a
restore test. The same item holds an encrypted `--export-secret-keys` export
(primary plus CI subkey) and the revocation certificate, so an encrypted copy now
exists apart from shuvbot. This is **offline-style custody on a separate online Mac**,
not an air-gapped machine. A non-cloud offline backup is still not established.
Only an exported secret **subkey**, with a stub primary and a separately generated
passphrase, was uploaded to `PACKAGE_SIGNING_KEY` / `PACKAGE_SIGNING_PASSPHRASE`
in the protected `release` environment. The export was tested with a fresh agent
and then shredded with `gshred -u` on shuvbot. SSD/APFS snapshots can defeat physical
overwrite guarantees; neither exporting nor deleting promises forensic erasure.
The primary's passphrase was never uploaded to GitHub.

See [release setup](../../docs/releases.md) for verification and recovery.
