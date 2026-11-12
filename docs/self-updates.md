# In-app install.sh updates

Settings → Updates offers **Update now** for the install.sh/tarball channel.
**Copy command** remains a manual fallback. Progress and failures stay on that
page; a successful install offers **Restart to finish**. Restart saves the
workspace, closes the window, and waits for host shutdown (up to two seconds) so
in-flight downloads can remove partial files before re-exec. Original launch
arguments, including `--config`, `--font`, `--font-size` and `--window-size`, are
preserved, except `--new`: session creation is never replayed. Rex server sessions
keep running. If re-exec fails after the window closes, Rex sends a critical desktop
notification through the session notification service (and logs any delivery failure
to stderr); reopen Rex or repair with install.sh.

## Eligibility and trust

- Only a release install whose schema-1 receipt matches the running binary and
  version can update itself. Source, pacman (`rex-linux-bin`), deb, Nix and other
  package channels keep their existing instructions. No sudo or root operations.
- The binary, launcher, three icons and receipt must be user-writable regular
  files in existing directories under `~/.local`, on one filesystem. Custom
  `XDG_BIN_HOME` / `XDG_DATA_HOME` paths are supported within that boundary.
  Symlinked install files, missing directories and read-only installs are refused
  with a reason. Use the manual installer to repair an invalid/missing receipt.
- Clicking Update now fetches and verifies the manifest again, using only the two
  embedded Minisign keys. It must still name the release shown when clicked, be
  strictly newer than the running/receipt version, satisfy `update_from`, and not
  be older than the highest previously verified release. If latest changed, use
  Check now and retry. Cached update-state JSON is never an artifact trust source.
  The signed manifest has **no expiry**: a valid older manifest at or above the
  local high-water mark is accepted. The high-water mark prevents rollback below
  an observed release, not freezing updates at an already observed release.
- The archive URL comes only from that verified manifest. Initial and redirected
  requests must be HTTPS. Connection timeout is 20 seconds; each transport read
  has a 30-second idle timeout, with a one-hour overall cap including redirects.
  Timeouts are reported as “timed out”. Downloads must match the signed size
  exactly (also capped at 256 MiB) and SHA-256 before extraction or execution.
  The signed target's minimum glibc version is checked before downloading.
- Extraction is into a private directory beside the installed binary. Paths must
  stay below the version/architecture package root; absolute paths, `..`, `.`,
  duplicate paths, all symlinks/hardlinks and special files are refused. Expanded
  payload is capped at 1 GiB and 20,000 entries. No archive installer script runs.
  The verified new binary must report the expected `--version` within 10 seconds;
  otherwise it is killed and the existing installation is left untouched.

## Files and rollback

The file layout matches `scripts/install-desktop.sh --copy --receipt-version`:

- `$XDG_BIN_HOME/rex-linux` (default `~/.local/bin/rex-linux`), mode 0755;
- `$XDG_DATA_HOME/applications/dev.shuv.rex-linux.desktop`;
- `$XDG_DATA_HOME/icons/hicolor/{32,128,256}x{32,128,256}/apps/dev.shuv.rex-linux.png`;
- `$XDG_DATA_HOME/rex-linux/install.json`, mode 0600, containing schema, canonical
  installed path, version and binary SHA-256. Data defaults to `~/.local/share`.

`desktop-file-install` and `desktop-file-validate` are required, just as for the
shell installer. Launcher Exec/TryExec rewriting uses the same quoting. Desktop
and icon caches are refreshed best-effort after committing the payload.

All files are prepared before replacement. A lock prevents simultaneous in-app
updates. A journal and hard-linked backups are synced before replacing files via
same-filesystem atomic renames; the old binary stays backed up until the whole
installation, including its receipt, commits. The journal records each prepared
file's device/inode identity, size and nanosecond modification time. Recovery
requires all of these to match, so inode reuse alone cannot identify a later repair
as this transaction's file. Recovery restores only destinations still belonging
to this transaction, skipping files already restored or not yet replaced. If any
destination belongs to a later installer/repair, the entire journal is discarded
without changing any destination, and Rex shows an informational notice that the
obsolete journal and backups were discarded. Files absent before are removed only
if they still have this transaction's full file identity. Preferences and the Rex server are never
touched.

Individual files are atomically replaced, not the entire multi-directory tree in
one filesystem operation. An interrupted transaction is recovered before the UI
starts on the next launch, using the journal's original install paths even if the
launch's XDG environment changed. Recovery re-execs the restored binary before
any workspace actions run. Only when a transaction journal exists does recovery
wait up to two seconds for the install lock. Without a journal, housekeeping tries
the lock once and silently skips a busy lock, an unwritable directory or a root
launch. If journal recovery fails or the lock remains busy, Rex logs the error to stderr and
**continues opening**, with a persistent (dismissible) UI notice. A failed recovery
retains `.rex-update-transaction` next to the binary for another attempt. Committed
journals and abandoned `.rex-update-*` / `.rex-update-cleanup-*` directories are
swept best-effort under the same lock, after recovery, including when no transaction
was started. Sweep or post-rollback cleanup failures never suppress a successful
rollback's required re-exec.

### Manual recovery

First close Rex windows and allow any updater to finish; do not run recovery as
root. A transient lock/storage failure can be retried by reopening Rex. Persistent
errors (a legacy journal without full file identity records, mismatched binary paths, or data
folders symlinked outside `~/.local`) require manual repair. There is no rollback
button or `--rollback` command.

Keep the journal and its `old-*` backups until a complete manual installation has
been verified. Do **not** copy individual backups over files from a later repair.
For a default install, retire the journal under the install lock before repairing:

```sh
bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
(
  flock -w 10 9 || exit 1
  if [ -d "$bin/.rex-update-transaction" ]; then
    mv "$bin/.rex-update-transaction" "$bin/.rex-recovery-backup-$(date +%s)"
  fi
  curl --proto '=https' --tlsv1.2 -fsSL \
    https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh | sh
) 9>"$bin/.rex-update.lock"
"$bin/rex-linux" --version
```

Use the original installation's `XDG_BIN_HOME` and `XDG_DATA_HOME` if customized
(the journal's `binary` and `data` fields record them). Check that the launcher
opens and the expected version is installed before deleting the retained
`.rex-recovery-backup-*` directory. If repair fails, retain it for diagnosis.
Directories outside the supported boundary can still be managed by the manual
installer, but remain ineligible for in-app updates. Deliberately installing an
older release remains the shell installer's explicit `--version` operation.

## Automatic installation

**Install updates automatically** defaults **off**, including existing settings
files. When enabled (`updates.install_automatically: true`), a successful scheduled
or manual check can install an eligible release in the background and offer
restart. It never restarts without a click. Automatic checks must also be on;
`REX_LINUX_NO_UPDATE_CHECK=1` disables automatic installation too. The daily check
cadence is unchanged. An installation error stays visible for manual retry; it
does not create a rapid background retry loop. Checks and installation cannot run
concurrently in one workspace. Across processes, the update-state lock covers only
load/apply/save, not network requests; each fetch reloads the latest state before
applying its manifest. Dismissal and announcement saves change only their notice
fields under that same lock, preserving the verified-version high-water mark.

## Validation

Requires the normal Rust/Zig toolchains and `desktop-file-utils`:

```sh
cargo test --locked --no-default-features update::installer
cargo test --locked --all-features updater_
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

`signed_fixture_end_to_end` serves a manifest, Minisign signature and tarball from
loopback using a deterministic **test-only** key, then exercises the real hash,
extraction, desktop integration, replacement and receipt code against a temporary
prefix. Only the artifact transport is substituted to map the signed production
URL to the local fixture. It executes the installed fixture binary and checks its
version, permissions and receipt. The shipped app has no alternate-key or HTTP
artifact option. Additional tests cover tampered/unknown signatures, size/hash
failures, downgrade/floor/changed-release refusals, unsafe tar paths and links,
read-only files, rollback at every replacement, partial rollback failures, lock
contention, stale-stage cleanup, actual-executable recovery, and a real
`install-desktop.sh` repair after interruption. The signed fixture includes a PAX
long path, matching Python tarfile releases; archive tests exercise size bombs
and more than 20,000 entries. Cases requiring non-root installation are skipped
when running tests as root.

Real-font screenshots can be captured by running this test alone:

```sh
mkdir -p /path/to/captures
REX_TEST_SNAP_DIR=/path/to/captures cargo test --locked --all-features \
  updater_settings_render_install_states -- --test-threads=1
# Add REX_UPDATE_MINIMUM=1 for the 640×480 layout.
```
