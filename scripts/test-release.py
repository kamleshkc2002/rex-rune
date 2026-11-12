#!/usr/bin/env python3
"""Local release tests. Requires minisign; creates/deletes throwaway keys only.

Run: python3 scripts/test-release.py
No GitHub access, real signing keys, packages, or servers are used.
"""

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("manifest", SCRIPTS / "release-manifest.py")
manifest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(manifest)
VERSION = "0.8.0"
PUBLISHED = "2026-11-20T10:00:00-08:00"
TARGETS = {"x86_64-linux-gnu": "amd64", "aarch64-linux-gnu": "arm64"}


def fixtures(dist):
    packages = {}
    for target, arch in TARGETS.items():
        cpu = target.split("-linux-")[0]
        for name in (f"rex-rune-{VERSION}-{target}.tar.gz", f"rex-linux-{VERSION}-{target}.tar.gz",
                     f"rex-rune_{VERSION}_{arch}.deb", f"rex-rune-bin-{VERSION}-1-{cpu}.pkg.tar.zst"):
            packages[name] = (name + "\nfixture package\n").encode()
            (dist / name).write_bytes(packages[name])
    (dist / "SHA256SUMS").write_text("".join(
        f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in sorted(packages.items())
    ))
    (dist / "install.sh").write_bytes((SCRIPTS.parent / "packaging/install/install.sh").read_bytes())
    return packages


class InstallerKeyTests(unittest.TestCase):
    def test_embedded_keys_match_registry(self):
        script = (SCRIPTS.parent / "packaging/install/install.sh").read_text()
        for constant, name in (("CI_KEY", "release.pub"), ("BACKUP_KEY", "release-backup.pub")):
            key = (SCRIPTS.parent / "packaging/keys" / name).read_text().splitlines()[1]
            self.assertEqual(script.count(f"{constant}=\'{key}\'"), 1)


class ManifestTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rex-manifest-test-")
        self.addCleanup(self.temp.cleanup)
        self.dist = Path(self.temp.name)
        self.packages = fixtures(self.dist)

    def generate(self, **kwargs):
        return manifest.generate(self.dist, **{
            "version": VERSION, "update_from": "0.4.2", "published": PUBLISHED, **kwargs,
        })

    def test_schema_and_both_architectures(self):
        value = self.generate()
        self.assertEqual(set(value), {"schema", "product", "version", "tag", "published", "notes_url", "update_from", "targets"})
        self.assertEqual(value["schema"], 1)
        # The id every published 0.7.x updater requires; it did not change with the rename.
        self.assertEqual(value["product"], "rex-linux")
        self.assertEqual(value["version"], VERSION)
        self.assertEqual(value["tag"], f"v{VERSION}")
        self.assertEqual(value["published"], PUBLISHED)
        self.assertEqual(value["update_from"], "0.4.2")
        self.assertEqual(value["notes_url"], f"{manifest.RELEASE_URL}/tag/v{VERSION}")
        self.assertEqual(set(value["targets"]), set(TARGETS))
        for target, arch in TARGETS.items():
            entry = value["targets"][target]
            self.assertEqual(set(entry), {"min_glibc", "tarball", "archive", "deb", "pacman"})
            self.assertEqual(set(entry["tarball"]), {"url", "size", "sha256"})
            self.assertEqual(set(entry["archive"]), {"url", "size", "sha256"})
            self.assertEqual(set(entry["deb"]), {"name", "size", "sha256"})
            self.assertEqual(set(entry["pacman"]), {"name", "size", "sha256"})
            self.assertEqual(entry["min_glibc"], "2.39")
            cpu = target.split("-linux-")[0]
            # `tarball` is what the 0.7.x updater downloads: the bridge under the old name.
            for kind, name in (("tarball", f"rex-linux-{VERSION}-{target}.tar.gz"),
                               ("archive", f"rex-rune-{VERSION}-{target}.tar.gz"),
                               ("deb", f"rex-rune_{VERSION}_{arch}.deb"),
                               ("pacman", f"rex-rune-bin-{VERSION}-1-{cpu}.pkg.tar.zst")):
                package = entry[kind]
                self.assertEqual(package["size"], len(self.packages[name]))
                self.assertEqual(package["sha256"], hashlib.sha256(self.packages[name]).hexdigest())
                if kind in ("tarball", "archive"):
                    self.assertEqual(package["url"], f"{manifest.RELEASE_URL}/download/v{VERSION}/{name}")
                else:
                    self.assertEqual(package["name"], name)

    def test_stable_cli_output(self):
        command = [sys.executable, str(SCRIPTS / "release-manifest.py"), "--dist", str(self.dist),
                   "--version", VERSION, "--update-from", "0.4.2", "--published", PUBLISHED]
        subprocess.run(command, check=True, capture_output=True)
        first = (self.dist / "latest.json").read_bytes()
        subprocess.run(command, check=True, capture_output=True)
        self.assertEqual(first, (self.dist / "latest.json").read_bytes())
        self.assertEqual(json.loads(first), self.generate())

    def test_bad_versions_and_floor(self):
        for field in ("version", "update_from"):
            for value in ("v0.8.0", "0.8", "01.8.0", "0.8.0-rc.1", "0.8.0+build", "../0.8.0"):
                with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                    self.generate(**{field: value})
        with self.assertRaises(ValueError):
            self.generate(update_from="0.9.0")
        self.assertEqual(self.generate(update_from=VERSION)["update_from"], VERSION)
        # Numeric ordering, not lexicographic: 0.10 is newer than 0.8.
        with self.assertRaises(ValueError):
            self.generate(update_from="0.10.0")

    def test_bad_timestamp_and_glibc(self):
        for value in ("yesterday", "2026-11-20T10:00:00", "2026-02-30T10:00:00Z"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.generate(published=value)
        with self.assertRaises(ValueError):
            self.generate(min_glibc="invalid")

    def test_checksums_reject_missing_extra_duplicate_and_malformed_entries(self):
        path = self.dist / "SHA256SUMS"
        original = path.read_text()
        for content in (original.split("\n", 1)[1], original + "0" * 64 + "  extra.deb\n",
                        original + original.splitlines()[0] + "\n", "not a checksum\n"):
            with self.subTest(content=content), self.assertRaises(ValueError):
                path.write_text(content)
                self.generate()

    def test_changed_and_missing_package(self):
        path = self.dist / next(iter(self.packages))
        path.write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "mismatch"):
            self.generate()
        path.unlink()
        with self.assertRaises(FileNotFoundError):
            self.generate()

    def test_empty_package(self):
        name = next(iter(self.packages))
        (self.dist / name).write_bytes(b"")
        sums = self.dist / "SHA256SUMS"
        sums.write_text(sums.read_text().replace(hashlib.sha256(self.packages[name]).hexdigest(), hashlib.sha256(b"").hexdigest()))
        with self.assertRaisesRegex(ValueError, "empty package"):
            self.generate()


class SigningTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="rex-sign-test-")
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name)
        cls.password = "throwaway-test-password-not-a-secret"
        # Use encrypted keys to exercise the same password input as CI.
        for name in ("release", "release-backup"):
            subprocess.run(
                ["minisign", "-G", "-p", str(cls.root / f"{name}.pub"), "-s", str(cls.root / f"{name}.key")],
                input=f"{cls.password}\n{cls.password}\n", text=True, check=True, capture_output=True,
            )

    def setUp(self):
        self.temp_dist = tempfile.TemporaryDirectory(dir=self.root)
        self.addCleanup(self.temp_dist.cleanup)
        self.dist = Path(self.temp_dist.name)
        fixtures(self.dist)
        (self.dist / "latest.json").write_text(json.dumps(manifest.generate(self.dist, VERSION, "0.4.2", PUBLISHED)))
        self.env = {k: v for k, v in os.environ.items() if k not in ("RELEASE_MINISIGN_KEY", "RELEASE_MINISIGN_PASSWORD")}
        self.env.update(TMPDIR=str(self.dist), RELEASE_MINISIGN_KEY=(self.root / "release.key").read_text(),
                        RELEASE_MINISIGN_PASSWORD=self.password)

    def run_sign(self, *args):
        result = subprocess.run(
            [sys.executable, str(SCRIPTS / "sign-release.py"), "--dist", str(self.dist),
             "--keys-dir", str(self.root), "--tag", f"v{VERSION}", *args],
            env=self.env, text=True, capture_output=True,
        )
        self.assertEqual(list(self.dist.glob("rex-release-sign-*")), [], "temporary secret key was not removed")
        self.assertNotIn(self.password, result.stdout + result.stderr)
        return result

    def test_sign_verify_and_trusted_comments_with_either_key(self):
        for key in ("release", "release-backup"):
            with self.subTest(key=key):
                self.env["RELEASE_MINISIGN_KEY"] = (self.root / f"{key}.key").read_text()
                result = self.run_sign()
                self.assertEqual(result.returncode, 0, result.stderr)
                for name in ("latest.json", "SHA256SUMS", "install.sh"):
                    # The signing product id is the pre-rename name, as published clients check.
                    self.assertIn(f"trusted comment: rex-linux v{VERSION} {name}", (self.dist / f"{name}.minisig").read_text())
                self.assertEqual(self.run_sign("--verify-only").returncode, 0)

    def test_verify_only_needs_no_secrets_and_rejects_missing_signature(self):
        self.assertEqual(self.run_sign().returncode, 0)
        del self.env["RELEASE_MINISIGN_KEY"]
        del self.env["RELEASE_MINISIGN_PASSWORD"]
        self.assertEqual(self.run_sign("--verify-only").returncode, 0)
        (self.dist / "latest.json.minisig").unlink()
        self.assertNotEqual(self.run_sign("--verify-only").returncode, 0)

    def test_tampering_either_file_fails(self):
        self.assertEqual(self.run_sign().returncode, 0)
        for name in ("latest.json", "SHA256SUMS", "install.sh"):
            path = self.dist / name
            original = path.read_bytes()
            path.write_bytes(original + b"tampered")
            self.assertNotEqual(self.run_sign("--verify-only").returncode, 0)
            path.write_bytes(original)

    def test_wrong_trusted_comment_fails(self):
        for name in ("latest.json", "SHA256SUMS", "install.sh"):
            with self.subTest(name=name):
                self.assertEqual(self.run_sign().returncode, 0)
                # A cryptographically valid signature must still bind the exact tag/file.
                subprocess.run(
                    ["minisign", "-S", "-s", str(self.root / "release.key"),
                     "-m", str(self.dist / name), "-t", f"rex-linux v0.4.2 {name}", "-q"],
                    input=self.password + "\n", text=True, check=True, capture_output=True,
                )
                result = self.run_sign("--verify-only")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("trusted comment must be exactly", result.stderr)

    def test_package_changed_after_signing_fails(self):
        self.assertEqual(self.run_sign().returncode, 0)
        package = next(self.dist.glob("*.tar.gz"))
        package.write_bytes(package.read_bytes() + b"changed after signing")
        result = self.run_sign("--verify-only")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA256SUMS mismatch", result.stderr)

    def test_signed_manifest_inconsistent_with_packages_fails(self):
        path = self.dist / "latest.json"
        value = json.loads(path.read_text())
        value["targets"]["x86_64-linux-gnu"]["tarball"]["size"] += 1
        path.write_text(json.dumps(value))
        # Signatures are valid, but neither signing nor verify-only may accept this.
        for args in ((), ("--verify-only",)):
            with self.subTest(args=args):
                result = self.run_sign(*args)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("latest.json does not match", result.stderr)

    def test_wrong_key_fails(self):
        # Signing key is valid but neither trusted public key matches it.
        with tempfile.TemporaryDirectory(dir=self.root) as temp:
            keys = Path(temp)
            for name in ("release.pub", "release-backup.pub"):
                (keys / name).write_bytes((self.root / "release-backup.pub").read_bytes())
            result = self.run_sign("--keys-dir", str(keys))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("did not verify", result.stderr)

    def test_wrong_password_and_missing_public_key_fail(self):
        self.env["RELEASE_MINISIGN_PASSWORD"] = "wrong-password"
        self.assertNotEqual(self.run_sign().returncode, 0)
        result = self.run_sign("--keys-dir", str(self.dist))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing public key", result.stderr)

    def test_absent_secrets_fail_closed(self):
        original = self.env.copy()
        for missing in (("RELEASE_MINISIGN_KEY",), ("RELEASE_MINISIGN_PASSWORD",),
                        ("RELEASE_MINISIGN_KEY", "RELEASE_MINISIGN_PASSWORD")):
            self.env = {k: v for k, v in original.items() if k not in missing}
            result = self.run_sign()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("refusing unsigned release", result.stderr)
            self.assertEqual(list(self.dist.glob("*.minisig")), [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
