# Update now for pacman installs: design note

Status: **proposal, nothing built.** Written 2026-11-12 (PST) against `20f64b4`, and
revised the same day after an independent review of the security argument.

## The ask

Settings → Updates offers **Update now** for install.sh installs under `~/.local`
([how that works](../self-updates.md)). For a pacman install it shows instructions
instead: `omarchy update` on Omarchy, `sudo pacman -Syu rex-linux-bin` elsewhere, with
**Copy command**. The owner wants pacman installs to update without pasting a sudo
command, and both install methods used daily (shuvdev runs the install.sh build,
shuvmbp the pacman package from `pkg.shuv.dev`).

This note compares the ways to get there and recommends one. It builds nothing.

## What exists today

- **The package.** `rex-linux-bin`, built from `packaging/arch/PKGBUILD.in`, installs
  `/usr/bin/rex-linux`, the desktop entry, icons, licenses and
  `/usr/share/rex-linux/channel` (`channel=pacman`). It has no install scriptlet.
- **The repository.** `https://pkg.shuv.dev/arch/$arch`, a signed pacman repository.
  The database names the newest version; the newest five releases' packages and their
  detached OpenPGP signatures stay in the directory, so an exact version can be
  fetched by URL. A machine trusts it by importing and locally signing the Rex
  archive key and adding a `[rex-linux]` section with `SigLevel = Required
  DatabaseRequired` (the [operator walkthrough](../auto-update-operator-walkthrough.md)
  has the steps).
- **The signed manifest.** Every release publishes `latest.json` with a Minisign
  signature. The client embeds the two release public keys and already verifies the
  manifest to decide that an update exists. For each architecture the manifest carries
  a `pacman` entry beside `tarball` and `deb`: the package's file name, exact size and
  SHA-256. The client's `TargetArtifacts` type does not read that entry yet.
- **The client's channel detection.** `InstallChannel::Pacman { omarchy, custom_hint }`
  in `src/update.rs`, from the channel file and `/etc/os-release`.
- **Two signing roots.** Minisign (a CI key and an offline backup key) signs the
  manifest, the checksums and install.sh. OpenPGP (an offline primary and a CI
  subkey) signs the packages and the repository database. The CI halves of both sit
  in the same protected `release` environment, behind one approval per release.

Checked on shuvdev (Omarchy, pacman 7.1, polkit 128), read-only. The author checked
the first five; the reviews checked the hook and the control-group file ownership:

- The live 0.7.5 manifest has the `pacman` entry
  (`rex-linux-bin-0.7.5-1-x86_64.pkg.tar.zst`, 22,135,553 bytes), and the site serves
  that file at that size with a 119-byte `.sig` beside it. Downloaded as an ordinary
  user, the file's SHA-256 matches the manifest's, and `gpgv` with only
  `packaging/keys/rex-linux-archive-key.asc` as its keyring reports a good signature
  by the CI subkey `55B8…DE75` (and a bad one for other bytes).
- `pkexec`, `gpgv`, `curl` and `systemd-run` are installed and `polkitd` runs.
  Omarchy's shell carries its own polkit agent (`/usr/share/omarchy/shell/plugins/polkit`).
  That it is registered in a running desktop session, and what its dialog shows for a
  custom action, was **not** confirmed.
- `pacman-conf LocalFileSigLevel` is `PackageOptional`, so `pacman -U` of a local file
  with no signature beside it installs.
- pacman's keyring on this machine holds 188 public keys: Arch's packagers and the
  keys of the other repositories configured here.
- `omarchy update` authorises with sudo once, takes a snapshot, refreshes keyrings,
  runs a full `pacman -Syu` and migrations, then updates AUR packages with yay. It
  runs pacman through `systemd-run --scope` so that pacman is not in the desktop
  session's control group.
- In the user's part of the control-group tree, `cgroup.kill` and `cgroup.freeze` are
  owned by the user.
- Omarchy installs a pacman hook (`00-omarchy-update-guard`) that stops a direct
  `pacman -S…u…` and points at `omarchy update`. It lets `pacman -U` through. This
  means the alternative the client prints today on Omarchy, `sudo pacman -Syu
  rex-linux-bin`, is refused there; that text should be corrected whatever is decided
  here.

## Why this needs root at all

install.sh installs live in files the user owns, so the client can replace them itself
after verifying the signed manifest. A pacman install lives in `/usr`, owned by root and
recorded in pacman's database. Changing it means running pacman as root. The whole
question is how the unprivileged client is allowed to cause that, and what else that
permission could be used for.

## Options

### (a) A polkit action and a small root helper in the package

The package adds three files:

- `/usr/lib/rex-linux/rex-linux-update`: a small program owned by root, **not** setuid.
- `/usr/share/polkit-1/actions/dev.shuv.rex-linux.update.policy`: a polkit action
  bound to that exact path (`org.freedesktop.policykit.exec.path`), with a fixed
  message that names Rex Linux and does not echo the caller's arguments.
- `/usr/share/rex-linux/archive-key.gpg`: the Rex package-signing certificate
  (`packaging/keys/rex-linux-archive-key.asc`), for the helper alone.

Update now runs `pkexec --disable-internal-agent /usr/lib/rex-linux/rex-linux-update
<version>`. polkit decides whether to allow it. With the action's default
(`auth_admin`), the desktop's polkit agent shows its own password dialog, the same one
the person sees for other system changes; there is no command to paste and no
terminal. On Arch an administrator is a member of `wheel`, so the person types their
own password. The default is `auth_admin` and not `auth_admin_keep`: "keep" would let
the same process run the helper again for a few minutes without asking, and one update
needs one run.

What the helper does, in full:

1. Takes one argument, a version, and accepts only `digits.digits.digits`.
2. Replaces itself with `systemd-run --scope --unit rex-linux-update <helper>
   <version>`. `systemd-run --scope` cannot move a running process; it starts the
   command in a new system scope, and because the helper `exec`s it, the process is
   the same one and `pkexec`'s caller still gets its exit status. The fixed scope name
   means a second run cannot start while one exists. Before this point the caller can
   still kill or freeze the helper, and nothing has been done yet. See "Not being
   stopped half-way" below.
3. Sends its own and pacman's output to the journal, not to the caller's pipes, so
   nothing depends on the client still being there. Takes a lock under `/run/lock`,
   opened without following symlinks.
4. Works out the architecture itself (`uname -m`), never from the caller.
5. Reads the installed version from pacman and refuses unless the requested version
   is strictly newer (`vercmp`). Refuses if `rex-linux-bin` is listed in pacman's
   `IgnorePkg`: an administrator pinned it.
6. Downloads
   `https://pkg.shuv.dev/arch/<arch>/rex-linux-bin-<version>-1-<arch>.pkg.tar.zst` and
   its `.sig` into a directory made with `mktemp -d` under a parent only root can
   write (never a fixed name in `/tmp`). curl runs with `-q` first (so root's
   `.curlrc` is not read), `--proto =https`, `--fail`, no redirects, `--max-filesize`
   and a byte count of its own, and `--max-time`.
7. Verifies the signature with `gpgv --keyring <absolute path>` against the shipped
   Rex certificate **only**, and reads `--status-fd`, not the exit code: `gpgv` exits
   0 for a signature by an expired or revoked key. It requires exactly one signature,
   with `GOODSIG` and a `VALIDSIG` naming the Rex primary (`4BC4…0BAB`) and a signing
   subkey of it, and refuses on `EXPKEYSIG`, `REVKEYSIG`, `BADSIG` or `ERRSIG`. This
   is the gate. It does not consult pacman's keyring.
8. Reads the package's own metadata from the verified file
   (`pacman -U --print-format '%n %v %a'`) and refuses unless it is exactly
   `rex-linux-bin`, `<version>-1` and this architecture, with nothing else added or
   removed. A signature covers a file's bytes, not its name, so without this a validly
   signed *older* Rex package served under the newer file name would pass step 7 and
   pacman would downgrade to it. It also keeps a one-package update from pulling in
   dependencies from stale databases.
9. Runs `pacman -U --noconfirm` on that file, without its `.sig` beside it, and
   removes the directory.

It takes no file path, no URL, no option and no environment from the caller (`pkexec`
already replaces the environment with a minimal one), and it does not sync databases,
so it cannot leave the system partly upgraded. Its own exit codes avoid 126 and 127.
`pkexec` exits 126 when the person dismissed the dialog, and 127 for everything else
it cannot do (not authorised, no agent, any error), so the client can tell "you
cancelled" from "the update failed", but cannot tell a missing agent from a refusal
by the code alone.

The client may also pass the package's SHA-256 from its verified manifest. The helper
would treat it only as a further restriction (refuse on a mismatch): it can narrow
what is installed and never widen it.

The URL assumes the package release number is 1, as `PKGBUILD.in` fixes it and
`scripts/build-repo-site.py` names the files. A rebuilt package (`-2`) would need the
helper to take the release number too, validated the same way.

Two variants:

- **Promptless.** A polkit *rule* that answers yes for this one action when the caller
  is in an active local session and in `wheel`. The package can ship the rule as an
  example that an administrator copies into `/etc/polkit-1/rules.d/` once, or install
  it enabled under `/usr/share/polkit-1/rules.d/`. See "The owner's calls".
- **A systemd unit instead of pkexec.** The package ships
  `rex-linux-update@.service`, and Update now starts `rex-linux-update@<version>`.
  systemd asks polkit itself (password dialog by default; a rule can match the unit
  name for the promptless form). The unit gives a clean environment, its own control
  group and journal logging without the helper arranging them. Against it: the dialog
  carries systemd's generic "start unit" wording, not Rex's, and the client learns
  the outcome by asking systemd afterwards instead of from an exit code. A hybrid keeps
  Rex's wording: `pkexec` runs a helper that only validates the version and then
  starts the unit and waits for it, which removes the short window before step 2
  altogether. How `systemctl start --wait` reports the result was not checked. The
  helper's steps 4 to 9 are the same in every form, so this choice can be made at
  build time.

### (b) An AUR `rex-linux-bin` package, for yay

Publish the same `PKGBUILD` to the AUR. People who use yay would install and update
Rex with it, and on Omarchy `omarchy update` already updates AUR packages.

It does not meet the ask. yay calls sudo by default, in a terminal, so the client
still cannot update itself: the best it could do is open a terminal running yay. The
signed repository already gets the same result from `omarchy update`. What the AUR
adds is discovery and a first install with no key import, not an Update now button.

It has costs of its own. A `PKGBUILD` on the AUR is unsigned, so its integrity is the
AUR account's; publishing needs that account and an SSH key in CI, a new surface
outside GitHub. The AUR officially targets x86_64.

The earlier roadmap review (`docs/plans/roadmap-0.5.2.md`, section 8) recorded the
answer as "no AUR". The recommendation behind it was "not yet": publish when someone
outside asks, or once two releases have gone through the pacman repository cleanly.
More than two have. So the AUR can fairly be reconsidered on its own merits; it is
just not a way to get this feature.

### (c) Keep the instructions

Nothing new runs as root. On Omarchy, `omarchy update` is one command the owner runs
anyway and it updates Rex with the rest of the system. The cost is exactly the
complaint: a paste, a terminal, and a full system update when only Rex was wanted.

### (d) "Update in terminal"

Not in the ask, but cheap: a button that opens the desktop's terminal
(`xdg-terminal-exec`, which Omarchy provides) already running `omarchy update`, or
`sudo pacman -Syu` on plain Arch. It removes the paste and adds no root code. On
Omarchy it keeps the snapshot, keyring refresh and migrations that `omarchy update`
does. It still needs a terminal and a sudo password, and it is a whole-system update.

### Rejected in a line each

- **A sudoers drop-in for the helper.** It can be made as narrow as the polkit action.
  polkit is preferred only because the desktop owns the dialog and no terminal is
  involved; a `NOPASSWD` entry is the same standing grant as the promptless rule.
- **A pacman hook or timer that updates Rex by itself.** That is unattended root
  software installation nobody asked for.
- **Making the package's files user-writable.** It would defeat the package.

## Security model for (a)

The client runs as the user and talks to servers, parses terminal output and loads
files, so assume it can be compromised. The question is what an attacker who controls
the client, as that user, gains from the new files.

**With the default action (password dialog).** Nothing without the person's password.
The attacker can make the dialog appear, at a moment that looks legitimate, and a
person who types their password authorises exactly one run of the helper. The risk
that remains is the general one of teaching people to type a password into a dialog an
application raised.

**With the promptless rule.** The attacker can run the helper whenever they like, and
the helper's checks are the whole boundary. What they can cause:

- An upgrade of `rex-linux-bin` to a newer version that the site serves and the Rex
  key signed. Normally that is only the latest, which is the intended effect. The
  site keeps five releases, so up to four of them can be newer than the installed
  one, and a release withdrawn for a fault could be chosen while the site still
  serves it; rebuilding the site with `repo.yml`'s `exclude` input removes it.
- Nothing older: step 5 refuses an older request, and step 8 refuses a file that is not
  the version requested.
- Nothing signed by any other key: step 7 refuses.
- No other package and no arbitrary file: steps 1, 4 and 8, and the fixed URL.
- Nuisance: a download's worth of traffic and time holding the helper's lock.

Under either form, the agent's dialog may show the command line, which is the
caller's argument before the helper has validated it. The action's own message does
not echo it; what Omarchy's agent displays has to be looked at.

**Why the helper checks the signature itself.** `pacman -U <url>` verifies a package
against pacman's whole keyring. That is every key the machine trusts (188 on shuvdev),
not Rex's: pacman has no per-repository key pinning for `-U`. Leaving the check to
pacman would mean that someone who controls `pkg.shuv.dev`, or the connection to it,
needs *any* trusted packager's key to have a package installed as root on request.
Checking with `gpgv` against the one shipped certificate closes that. It also means
the package is downloaded once, into a place the user cannot write, and that exact
file is what is checked, examined and installed.

**What `--noconfirm` would answer.** As pacman's source was last read (not re-checked
here): removing a conflicting package defaults to no, so a conflict fails; importing
an unknown key defaults to yes. The helper gives pacman the file without a signature
beside it, so pacman has no key to ask about. On a machine that requires signatures
for local files, pacman refuses and the client falls back to the instructions.

**Code that runs as root during the install.** pacman's hooks fire on every
transaction. On shuvdev that includes Omarchy's update guard (which lets `-U` through)
and the snapshot lock, plus the usual desktop-database and icon-cache refreshes. They
are the same hooks a hand-run pacman triggers. The package has no install scriptlet
today; one added later would run as root on update, as it would under `pacman -Syu`,
and should be reviewed with that in mind.

**Not being stopped half-way.** A program started through `pkexec` stays in the
caller's control group, which the user's own service manager controls. So the user,
a compromised client, or simply closing Rex could kill pacman in the middle of
extracting files, or freeze it while it holds pacman's database lock. This is reasoned
from how control groups work and from Omarchy's own guard against the same thing, not
reproduced here. Steps 2 and 3 are the answer: the helper restarts itself inside a
system scope before doing anything, as `omarchy update` does for pacman, and writes to
the journal so it does not depend on the client still being there. The client's **Restart to finish** must wait
for the helper to exit.

**What the helper does not defend against.** Someone who holds the Rex OpenPGP package
key. They can already publish a package that `pacman -Syu` installs.

**Revocation, an accepted gap unless decided otherwise.** The shipped certificate is
a fixed file. If the CI signing subkey were compromised and revoked, pacman's keyring
can learn that (`pacman-key --refresh-keys`) and `pacman -Syu` would then refuse its
packages, but the helper would go on accepting them until a package with a corrected
certificate was installed. The stricter rule, which costs one more check and no key
import, is to also require that the signing subkey is currently valid in pacman's
keyring: the intersection of "Rex's key" and "a key this machine still trusts". The
build should take the stricter rule unless it proves impractical.

**Excluded because unsafe:**

- The client downloading the package and passing a path. A file in a directory the
  user can write could be swapped between the check and the install. (A download by
  the helper into a root-only directory is fine, and is the design.)
- Passing a URL, repository name, pacman option or configuration file through.
- Running `pacman -Sy rex-linux-bin`. Refreshing the databases and installing one
  package is a partial upgrade, which Arch does not support.
- A setuid helper.

## What the Minisign-verified manifest gives

- **The client's side, unprivileged.** That an update exists, its version, and the
  package's exact name, size and hash for this architecture, all signed by keys built
  into the client. The client can show these before asking for authorisation, pass the
  verified version to the helper, and refuse to start if the manifest does not list a
  pacman package for this machine. Afterwards it can confirm the installed version is
  the one it asked for. It also stops a stale database or a tampered site from
  deciding *which* version is offered.
- **Not the privileged step.** The helper cannot take the client's word for the hash,
  because the client is the thing assumed compromised. The install is gated by the
  helper's own OpenPGP check.
- **Should the helper verify the manifest too?** It would then need both a Rex
  OpenPGP signature and a Rex Minisign signature. With the signer pinned as above,
  the gain is small: the CI halves of both keys live in the same release environment,
  so whoever has one probably has the other. The cost is an HTTP fetch and a JSON and
  Minisign parser in a root process. The recommendation is not to, but it is a
  custody question and is listed for the owner.

## Omarchy fit

- A one-package `pacman -U` of a third-party package whose dependencies are already
  satisfied does not conflict with `omarchy update`, which afterwards finds
  `rex-linux-bin` current. Step 8 refuses when dependencies are not satisfied.
- It does bypass what `omarchy update` wraps around pacman: the snapshot, the keyring
  refresh and the migrations. For one application package that looks harmless, and
  Omarchy's own guard allows `-U`, but it is Omarchy's stated preference that updates
  go through its command. Listed for the owner.
- A polkit agent must be registered for the password dialog. Omarchy's shell has one;
  that must be confirmed on shuvmbp before building. `--disable-internal-agent` makes
  `pkexec` fail at once when there is none, and the client then shows the
  instructions instead of appearing to hang.
- The first release that carries the helper has to be installed the old way. After
  that, every pacman install has it.
- The shipped certificate's signing subkey expires in 2029. Step 7 refuses an expired
  key, so a new subkey has to reach machines in a package signed by the old one
  before then; otherwise Update now stops verifying and the instructions take over.

## Recommendation

**Build (a) with the default password dialog and the helper checking the signature
itself. Keep (c) as the fallback everywhere. Do not do (b) for this.**

- (a) is the only option that removes both the pasted command and the terminal. With
  the default action its worst case under a compromised client is a dialog the person
  must still approve, for an install they would have made anyway.
- (c)'s text stays for every case (a) cannot serve: no polkit agent, a package from
  before the helper, a failed or cancelled authorisation, any refusal from the helper.
- (d) is a reasonable small step if the owner wants the paste gone sooner or prefers
  that Omarchy machines always go through `omarchy update`. It does not replace (a).
- (b) answers a different question (how people first find and install Rex on Arch).

Outline of the build, if approved, as small PRs in this order:

1. Correct today's Omarchy text so it no longer offers a command Omarchy refuses.
2. The helper, policy and certificate in the package, with the helper tested as root
   in the existing Arch container (`scripts/test-arch-repo.py`): each refusal above, a
   real signed upgrade, a package signed by another key that pacman trusts, a
   tampered package, an older signed package served under the newer name, signatures
   by an expired and by a revoked subkey, an older version, an unknown version, a
   pinned package, a package that would pull in a dependency. `pkaction` confirms the action is registered. No client change.
3. The client: read the manifest's `pacman` entry; offer Update now on a pacman install
   only when the helper and `pkexec` exist; run it; show progress, failure reasons and
   Restart to finish; fall back to the instructions otherwise. Headless tests with a
   fake helper; nothing in the test suite runs `pkexec`.
4. Acceptance on shuvmbp (pacman) and shuvdev (install.sh), both by hand, since both
   need a real desktop session. This includes closing Rex during an update.

## The owner's calls

These are not engineering judgments:

1. **Promptless or not.** Whether the package should also make this work with no
   password dialog, and if so whether that rule is installed enabled or shipped as a
   file an administrator copies into place. It is a standing grant of a narrow root
   action to anything running as the user. The recommendation is the password dialog
   by default and the rule as an opt-in file.
2. **The AUR.** Publishing `rex-linux-bin` there puts a public name under an account
   of the owner's and adds a publishing key to CI. This feature does not need it; the
   earlier "not yet" conditions have since been met, so it is open on its own merits.
3. **What the helper trusts.** The proposal pins the Rex OpenPGP certificate shipped in
   the package. The alternatives are pacman's whole keyring (argued against above) or
   additionally requiring the Minisign-signed manifest (small gain, more root code).
4. **Bypassing `omarchy update` on Omarchy.** Whether a one-package update outside
   Omarchy's snapshot and migrations is acceptable there, or whether Omarchy machines
   should get (d) instead.

## Not verified

- That Omarchy's polkit agent is registered in a desktop session, and what its dialog
  shows for a custom action.
- The control-group behaviour under `pkexec` (the file ownership was checked; killing
  a root process that way was not reproduced), and that restarting under
  `systemd-run --scope` fully removes it.
- The exact pacman invocation for step 8; pacman's `--noconfirm` defaults, its
  downgrade behaviour with `-U`, and whether `-U` honours `IgnorePkg`, against the
  current source.
- `gpgv`'s status lines for this certificate. That `gpgv` exits 0 for expired and
  revoked keys was reproduced by the reviewer with throwaway keys and is in its
  manual.
- The helper's download and `gpgv` check run as root in a system scope (the same
  check passed as an ordinary user for 0.7.5 x86_64 only).
- Arch Linux ARM, and Arch without Omarchy.
