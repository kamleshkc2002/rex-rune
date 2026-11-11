//! Low-cost, privacy-respecting release metadata check, Minisign verification, install-channel
//! detection, and persistent update state. Never downloads binaries or modifies the installation.
use anyhow::{Context, Result, bail, ensure};
use base64::Engine as _;
use ring::signature::{ED25519, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::io::AsRawFd,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REPOSITORY: &str = "https://github.com/shuv1337/rex-rune";
pub const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json";
pub const DEFAULT_SIGNATURE_URL: &str =
    "https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json.minisig";
pub const INSTALL_SCRIPT_COMMAND: &str = "curl --proto '=https' --tlsv1.2 -fsSL https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh | sh";

pub const CI_PUBLIC_KEY: &str = include_str!("../packaging/keys/release.pub");
pub const BACKUP_PUBLIC_KEY: &str = include_str!("../packaging/keys/release-backup.pub");

pub const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
pub const STARTUP_DELAY: Duration = Duration::from_secs(30);
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_SIGNATURE_BYTES: usize = 4 * 1024;
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Strict three-part numeric version (`X.Y.Z`) without leading zeros, prerelease tags, or build metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StableVersion(pub u64, pub u64, pub u64);

impl StableVersion {
    pub fn parse(input: &str) -> Result<Self> {
        let mut parts = input.split('.');
        let major = parse_component(parts.next().context("missing major version")?)?;
        let minor = parse_component(parts.next().context("missing minor version")?)?;
        let patch = parse_component(parts.next().context("missing patch version")?)?;
        ensure!(parts.next().is_none(), "expected X.Y.Z version: {input:?}");
        Ok(Self(major, minor, patch))
    }
}

fn parse_component(part: &str) -> Result<u64> {
    ensure!(!part.is_empty(), "empty version component");
    ensure!(
        part.bytes().all(|b| b.is_ascii_digit()),
        "non-numeric version component: {part:?}"
    );
    ensure!(
        part == "0" || !part.starts_with('0'),
        "leading zero in version component: {part:?}"
    );
    part.parse::<u64>()
        .with_context(|| format!("version component out of range: {part}"))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub schema: u32,
    pub product: String,
    pub version: String,
    pub tag: String,
    pub published: String,
    pub notes_url: String,
    pub update_from: String,
    pub targets: BTreeMap<String, TargetArtifacts>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetArtifacts {
    pub min_glibc: String,
    pub tarball: TarballArtifact,
    pub deb: DebArtifact,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TarballArtifact {
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebArtifact {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

impl ReleaseManifest {
    /// Parse and validate `latest.json` against the signed schema and URL invariants.
    pub fn parse_and_validate(raw: &[u8]) -> Result<Self> {
        ensure!(
            raw.len() <= MAX_MANIFEST_BYTES,
            "manifest exceeds {MAX_MANIFEST_BYTES} bytes"
        );
        let manifest: Self = serde_json::from_slice(raw).context("invalid latest.json")?;
        ensure!(
            manifest.schema == 1,
            "unsupported manifest schema {}",
            manifest.schema
        );
        ensure!(
            manifest.product == "rex-linux",
            "unexpected product {:?}",
            manifest.product
        );
        let version = StableVersion::parse(&manifest.version)?;
        let update_from = StableVersion::parse(&manifest.update_from)?;
        ensure!(
            update_from <= version,
            "update_from {} is newer than version {}",
            manifest.update_from,
            manifest.version
        );
        let expected_tag = format!("v{}", manifest.version);
        ensure!(
            manifest.tag == expected_tag,
            "manifest tag {:?} does not match {:?}",
            manifest.tag,
            expected_tag
        );
        let expected_notes = format!("{REPOSITORY}/releases/tag/{expected_tag}");
        ensure!(
            manifest.notes_url == expected_notes,
            "unexpected notes_url {:?}",
            manifest.notes_url
        );
        ensure!(
            !manifest.published.trim().is_empty()
                && !manifest.published.chars().any(char::is_control),
            "invalid published timestamp"
        );
        ensure!(!manifest.targets.is_empty(), "manifest has no targets");
        for (target, entry) in &manifest.targets {
            ensure!(
                matches!(target.as_str(), "x86_64-linux-gnu" | "aarch64-linux-gnu"),
                "unexpected target {target:?}"
            );
            ensure!(
                entry.tarball.size > 0 && is_hex_sha256(&entry.tarball.sha256),
                "invalid tarball entry for {target}"
            );
            ensure!(
                entry
                    .tarball
                    .url
                    .starts_with(&format!("{REPOSITORY}/releases/download/{expected_tag}/")),
                "unexpected tarball URL for {target}"
            );
            ensure!(
                entry.deb.size > 0
                    && is_hex_sha256(&entry.deb.sha256)
                    && !entry.deb.name.contains('/')
                    && entry.deb.name.ends_with(".deb"),
                "invalid deb entry for {target}"
            );
        }
        Ok(manifest)
    }
}

fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Verify `manifest_bytes` and `minisig_text` against the trusted public keys and parse `latest.json`.
pub fn verify_and_parse_manifest(
    manifest_bytes: &[u8],
    minisig_text: &str,
    public_keys: &[&str],
) -> Result<ReleaseManifest> {
    ensure!(
        !public_keys.is_empty(),
        "no trusted Minisign public keys configured"
    );
    ensure!(
        minisig_text.len() <= MAX_SIGNATURE_BYTES,
        "signature exceeds {MAX_SIGNATURE_BYTES} bytes"
    );
    let parsed_sig = ParsedMinisig::parse(minisig_text)?;
    let prefix = "rex-linux v";
    let suffix = " latest.json";
    let comment_version = parsed_sig
        .trusted_comment
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(suffix))
        .with_context(|| {
            format!(
                "trusted comment {:?} does not match 'rex-linux vX.Y.Z latest.json'",
                parsed_sig.trusted_comment
            )
        })?;
    let _ = StableVersion::parse(comment_version)?;

    let mut verified = false;
    for key_text in public_keys {
        let pubkey = ParsedPublicKey::parse(key_text)?;
        if pubkey.key_id == parsed_sig.key_id && pubkey.verify(manifest_bytes, &parsed_sig).is_ok()
        {
            verified = true;
            break;
        }
    }
    ensure!(
        verified,
        "latest.json signature did not verify against any trusted public key"
    );

    let manifest = ReleaseManifest::parse_and_validate(manifest_bytes)?;
    ensure!(
        manifest.version == comment_version,
        "trusted comment version {comment_version} does not match manifest version {}",
        manifest.version
    );
    Ok(manifest)
}

struct ParsedPublicKey {
    key_id: [u8; 8],
    pubkey: [u8; 32],
}

impl ParsedPublicKey {
    fn parse(text: &str) -> Result<Self> {
        let line = text
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
            .context("Minisign public key is missing base64 payload")?;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(line)
            .context("invalid base64 in Minisign public key")?;
        ensure!(
            raw.len() == 42,
            "Minisign public key must decode to 42 bytes"
        );
        ensure!(&raw[0..2] == b"Ed", "unsupported public key algorithm");
        let mut key_id = [0u8; 8];
        key_id.copy_from_slice(&raw[2..10]);
        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(&raw[10..42]);
        Ok(Self { key_id, pubkey })
    }

    fn verify(&self, message: &[u8], sig: &ParsedMinisig) -> Result<()> {
        let digest = blake2b_512(message);
        let key = UnparsedPublicKey::new(&ED25519, &self.pubkey);
        key.verify(&digest, &sig.signature)
            .map_err(|_| anyhow::anyhow!("Ed25519 signature verification failed"))?;
        let mut global_msg = Vec::with_capacity(64 + sig.trusted_comment.len());
        global_msg.extend_from_slice(&sig.signature);
        global_msg.extend_from_slice(sig.trusted_comment.as_bytes());
        key.verify(&global_msg, &sig.global_signature)
            .map_err(|_| anyhow::anyhow!("Ed25519 global signature verification failed"))?;
        Ok(())
    }
}

struct ParsedMinisig {
    key_id: [u8; 8],
    signature: [u8; 64],
    trusted_comment: String,
    global_signature: [u8; 64],
}

impl ParsedMinisig {
    fn parse(text: &str) -> Result<Self> {
        let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
        ensure!(
            lines.len() == 4,
            "Minisign signature must have exactly 4 lines"
        );
        ensure!(
            lines[0].starts_with("untrusted comment: "),
            "missing untrusted comment header"
        );
        let raw_sig = base64::engine::general_purpose::STANDARD
            .decode(lines[1].trim())
            .context("invalid base64 in signature line")?;
        ensure!(
            raw_sig.len() == 74,
            "signature line must decode to 74 bytes"
        );
        // Minisign prehashed mode is 'ED' (0x45, 0x44); reject legacy unhashed 'Ed'.
        ensure!(
            &raw_sig[0..2] == b"ED",
            "expected prehashed ED Minisign signature"
        );
        let trusted_comment = lines[2]
            .strip_prefix("trusted comment: ")
            .context("missing trusted comment prefix")?
            .to_owned();
        ensure!(
            !trusted_comment.is_empty() && !trusted_comment.chars().any(char::is_control),
            "invalid trusted comment"
        );
        let raw_global = base64::engine::general_purpose::STANDARD
            .decode(lines[3].trim())
            .context("invalid base64 in global signature line")?;
        ensure!(
            raw_global.len() == 64,
            "global signature must decode to 64 bytes"
        );
        let mut key_id = [0u8; 8];
        key_id.copy_from_slice(&raw_sig[2..10]);
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&raw_sig[10..74]);
        let mut global_signature = [0u8; 64];
        global_signature.copy_from_slice(&raw_global);
        Ok(Self {
            key_id,
            signature,
            trusted_comment,
            global_signature,
        })
    }
}

/// RFC 7693 BLAKE2b-512 (unkeyed, 64-byte digest), used by Minisign `ED` prehashed signatures.
pub fn blake2b_512(input: &[u8]) -> [u8; 64] {
    const IV: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    const SIGMA: [[usize; 16]; 12] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    ];

    fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
        v[d] = (v[d] ^ v[a]).rotate_right(32);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = (v[b] ^ v[c]).rotate_right(24);
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
        v[d] = (v[d] ^ v[a]).rotate_right(16);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = (v[b] ^ v[c]).rotate_right(63);
    }

    fn compress(h: &mut [u64; 8], block: &[u8; 128], t: u128, last: bool) {
        let mut m = [0u64; 16];
        for (i, chunk) in block.chunks_exact(8).enumerate() {
            m[i] = u64::from_le_bytes(chunk.try_into().unwrap());
        }
        let mut v = [0u64; 16];
        v[..8].copy_from_slice(h);
        v[8..16].copy_from_slice(&IV);
        v[12] ^= t as u64;
        v[13] ^= (t >> 64) as u64;
        if last {
            v[14] = !v[14];
        }
        for s in &SIGMA {
            g(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
            g(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
            g(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
            g(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
            g(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
            g(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
            g(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
            g(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
        }
        for i in 0..8 {
            h[i] ^= v[i] ^ v[i + 8];
        }
    }

    let mut h = IV;
    h[0] ^= 0x01010000 ^ 64;
    let mut t: u128 = 0;
    if input.is_empty() {
        compress(&mut h, &[0u8; 128], 0, true);
    } else {
        let mut offset = 0;
        while input.len() - offset > 128 {
            let block: &[u8; 128] = input[offset..offset + 128].try_into().unwrap();
            t += 128;
            compress(&mut h, block, t, false);
            offset += 128;
        }
        let remaining = &input[offset..];
        t += remaining.len() as u128;
        let mut last_block = [0u8; 128];
        last_block[..remaining.len()].copy_from_slice(remaining);
        compress(&mut h, &last_block, t, true);
    }
    let mut out = [0u8; 64];
    for (i, word) in h.iter().enumerate() {
        out[i * 8..(i + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// How this `rex-linux` binary was installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallChannel {
    Source,
    Nix,
    Pacman {
        omarchy: bool,
        custom_hint: Option<String>,
    },
    Deb {
        custom_hint: Option<String>,
    },
    OtherPackage {
        channel: String,
        hint: Option<String>,
    },
    Tarball,
    Unmanaged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelGuidance {
    pub summary: String,
    pub detail: String,
    pub command: Option<String>,
}

#[derive(Deserialize)]
struct InstallReceipt {
    schema: u32,
    path: String,
    #[allow(dead_code)]
    version: String,
    sha256: String,
}

impl InstallChannel {
    /// Detect the install channel of the running process.
    pub fn detect() -> Self {
        let exe = std::env::current_exe().ok();
        let data_dir = xdg_data_dir();
        let os_release = fs::read_to_string("/etc/os-release").ok();
        Self::detect_with(
            crate::BUILD_CHANNEL,
            exe.as_deref(),
            &data_dir,
            os_release.as_deref(),
        )
    }

    pub fn detect_with(
        build_channel: &str,
        exe_path: Option<&Path>,
        data_dir: &Path,
        os_release: Option<&str>,
    ) -> Self {
        match build_channel {
            "source" | "" => return Self::Source,
            "nix" => return Self::Nix,
            _ => {}
        }
        let resolved_exe =
            exe_path.and_then(|p| fs::canonicalize(p).ok().or_else(|| Some(p.to_path_buf())));
        if let Some(exe) = &resolved_exe
            && let Some(marker) = find_channel_marker(exe)
            && let Some(channel) = parse_channel_marker(&marker, os_release)
        {
            return channel;
        }
        let receipt_path = data_dir.join("rex-linux/install.json");
        if let Some(exe) = &resolved_exe
            && verify_install_receipt(&receipt_path, exe)
        {
            return Self::Tarball;
        }
        Self::Unmanaged
    }

    pub fn id(&self) -> &str {
        match self {
            Self::Source => "source",
            Self::Nix => "nix",
            Self::Pacman { .. } => "pacman",
            Self::Deb { .. } => "deb",
            Self::OtherPackage { channel, .. } => channel,
            Self::Tarball => "tarball",
            Self::Unmanaged => "unmanaged",
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Source => "Source build".into(),
            Self::Nix => "Nix package".into(),
            Self::Pacman { omarchy: true, .. } => "Arch / Omarchy package (pacman)".into(),
            Self::Pacman { omarchy: false, .. } => "Arch Linux package (pacman)".into(),
            Self::Deb { .. } => "Debian / Ubuntu package (.deb)".into(),
            Self::OtherPackage { channel, .. } => format!("System package ({channel})"),
            Self::Tarball => "Release installer (install.sh)".into(),
            Self::Unmanaged => "Standalone binary".into(),
        }
    }

    /// Whether background update checks are permitted for this channel when enabled in preferences.
    pub fn checks_by_default(&self) -> bool {
        !matches!(self, Self::Source)
    }

    /// Channel-specific update instructions for `version`.
    pub fn guidance(&self, version: &str) -> ChannelGuidance {
        let version = &crate::format_version(version);
        let deb_arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            _ => "amd64",
        };
        match self {
            Self::Pacman {
                omarchy,
                custom_hint,
            } => {
                if let Some(hint) = custom_hint {
                    return ChannelGuidance {
                        summary: format!(
                            "Rex Linux {version} is available. Run `{hint}` to update."
                        ),
                        detail: format!(
                            "Installed via pacman. Run `{hint}` in a terminal to upgrade."
                        ),
                        command: Some(hint.clone()),
                    };
                }
                if *omarchy {
                    ChannelGuidance {
                        summary: format!(
                            "Rex Linux {version} is available. Run `omarchy update` (or `sudo pacman -Syu rex-linux-bin`)."
                        ),
                        detail: "Installed via the signed Arch / Omarchy repository. Run `omarchy update` or `sudo pacman -Syu rex-linux-bin`.".into(),
                        command: Some("omarchy update".into()),
                    }
                } else {
                    ChannelGuidance {
                        summary: format!(
                            "Rex Linux {version} is available. Run `sudo pacman -Syu rex-linux-bin`."
                        ),
                        detail: "Installed via pacman. Run `sudo pacman -Syu rex-linux-bin` (or `omarchy update` on Omarchy).".into(),
                        command: Some("sudo pacman -Syu rex-linux-bin".into()),
                    }
                }
            }
            Self::Tarball => ChannelGuidance {
                summary: format!(
                    "Rex Linux {version} is available. Re-run the verified install.sh command to upgrade."
                ),
                detail: "Installed in ~/.local/bin with install.sh. Run the verified installer command in a terminal to upgrade.".into(),
                command: Some(INSTALL_SCRIPT_COMMAND.into()),
            },
            Self::Deb { custom_hint } => {
                let deb_name = format!("rex-linux_{version}_{deb_arch}.deb");
                let cmd = custom_hint
                    .clone()
                    .unwrap_or_else(|| format!("sudo apt install ./{deb_name}"));
                ChannelGuidance {
                    summary: format!(
                        "Rex Linux {version} is available. Download {deb_name} from GitHub Releases and install with `{cmd}` (or `sudo dpkg -i`)."
                    ),
                    detail: format!(
                        "Installed from a .deb package. Download {deb_name} from GitHub Releases and run `{cmd}` (or `sudo dpkg -i {deb_name}`)."
                    ),
                    command: Some(cmd),
                }
            }
            Self::Nix => ChannelGuidance {
                summary: format!(
                    "Rex Linux {version} is available. Run `nix flake update rex-rune` (or `nix profile upgrade rex-linux`)."
                ),
                detail: "Installed with Nix. Update your flake input with `nix flake update rex-rune` and rebuild, or run `nix profile upgrade rex-linux`.".into(),
                command: Some("nix flake update rex-rune".into()),
            },
            Self::OtherPackage { channel, hint } => {
                let cmd = hint.clone();
                let detail = match &cmd {
                    Some(h) => format!("Installed via {channel}. Run `{h}` to upgrade."),
                    None => format!("Installed via {channel}. Upgrade with your system package manager."),
                };
                ChannelGuidance {
                    summary: format!("Rex Linux {version} is available. {detail}"),
                    detail,
                    command: cmd,
                }
            }
            Self::Source => ChannelGuidance {
                summary: format!(
                    "Rex Linux {version} is available. Pull the latest release tag and rebuild with cargo."
                ),
                detail: "Built from source; automatic update checks are off. Pull the repository and run `cargo build --locked --release --bin rex-linux`.".into(),
                command: Some("git pull && cargo build --locked --release --bin rex-linux".into()),
            },
            Self::Unmanaged => ChannelGuidance {
                summary: format!(
                    "Rex Linux {version} is available on GitHub Releases."
                ),
                detail: "Standalone binary without a package marker or install receipt. Download the latest release from GitHub Releases.".into(),
                command: None,
            },
        }
    }
}

fn find_channel_marker(resolved_exe: &Path) -> Option<PathBuf> {
    let bin_dir = resolved_exe.parent()?;
    let prefix = bin_dir.parent()?;
    let relative = prefix.join("share/rex-linux/channel");
    if relative.is_file() {
        return Some(relative);
    }
    if resolved_exe == Path::new("/usr/bin/rex-linux") {
        let system = PathBuf::from("/usr/share/rex-linux/channel");
        if system.is_file() {
            return Some(system);
        }
    }
    None
}

fn parse_channel_marker(marker_path: &Path, os_release: Option<&str>) -> Option<InstallChannel> {
    let text = fs::read_to_string(marker_path).ok()?;
    let mut channel = None;
    let mut hint = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let val = v.trim();
            if val.chars().any(char::is_control) {
                continue;
            }
            match k.trim() {
                "channel" if !val.is_empty() => channel = Some(val.to_owned()),
                "hint" if !val.is_empty() => hint = Some(val.to_owned()),
                _ => {}
            }
        }
    }
    let channel = channel?;
    let omarchy = is_omarchy(os_release);
    Some(match channel.as_str() {
        "pacman" => {
            let custom_hint = hint.filter(|h| h != "sudo pacman -Syu");
            InstallChannel::Pacman {
                omarchy,
                custom_hint,
            }
        }
        "deb" => {
            let custom_hint = hint.filter(|h| h != "Install the latest .deb from GitHub Releases");
            InstallChannel::Deb { custom_hint }
        }
        "nix" => InstallChannel::Nix,
        _ => InstallChannel::OtherPackage { channel, hint },
    })
}

pub fn is_omarchy(os_release: Option<&str>) -> bool {
    let Some(content) = os_release else {
        return false;
    };
    for line in content.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if (key == "ID" || key == "ID_LIKE")
                && value
                    .split_whitespace()
                    .any(|token| token.eq_ignore_ascii_case("omarchy"))
            {
                return true;
            }
        }
    }
    false
}

fn verify_install_receipt(receipt_path: &Path, resolved_exe: &Path) -> bool {
    let Ok(bytes) = fs::read(receipt_path) else {
        return false;
    };
    let Ok(receipt) = serde_json::from_slice::<InstallReceipt>(&bytes) else {
        return false;
    };
    if receipt.schema != 1 || !is_hex_sha256(&receipt.sha256) {
        return false;
    }
    let receipt_target = Path::new(&receipt.path);
    let resolved_target =
        fs::canonicalize(receipt_target).unwrap_or_else(|_| receipt_target.to_path_buf());
    if resolved_target != resolved_exe {
        return false;
    }
    let Ok(exe_bytes) = fs::read(resolved_exe) else {
        return false;
    };
    let digest = hex_sha256(&exe_bytes);
    digest == receipt.sha256
}

fn hex_sha256(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in hash {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Persistent update check state stored in `$XDG_STATE_HOME/rex-linux/update.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateState {
    pub last_checked: Option<u64>,
    pub latest_version: Option<String>,
    pub highest_verified: Option<String>,
    pub announced_version: Option<String>,
    pub dismissed_version: Option<String>,
    pub notes_url: Option<String>,
    pub published: Option<String>,
    pub unsupported_floor: Option<String>,
}

impl UpdateState {
    pub fn default_path() -> PathBuf {
        xdg_state_dir().join("rex-linux/update.json")
    }

    pub fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    /// Whether a background check is due at `now_epoch_secs`.
    pub fn is_due(&self, now_epoch_secs: u64) -> bool {
        match self.last_checked {
            Some(last) => now_epoch_secs.saturating_sub(last) >= CHECK_INTERVAL_SECS,
            None => true,
        }
    }

    /// Available update if `latest_version` is strictly newer than `current_version`.
    pub fn available_update(&self, current_version: &str) -> Option<AvailableUpdate> {
        let current = StableVersion::parse(current_version).ok()?;
        let latest_str = self.latest_version.as_deref()?;
        let latest = StableVersion::parse(latest_str).ok()?;
        if latest <= current {
            return None;
        }
        let expected_notes_url = format!("{REPOSITORY}/releases/tag/v{latest_str}");
        let notes_url = self
            .notes_url
            .clone()
            .filter(|u| u == &expected_notes_url)
            .unwrap_or(expected_notes_url);
        let unsupported_floor = self
            .unsupported_floor
            .as_deref()
            .and_then(|f| StableVersion::parse(f).ok().map(|fv| (f, fv)))
            .filter(|(_, fv)| current < *fv)
            .map(|(f, _)| f.to_owned());
        Some(AvailableUpdate {
            version: latest_str.to_owned(),
            notes_url,
            published: self.published.clone(),
            dismissed: self.dismissed_version.as_deref() == Some(latest_str),
            announced: self.announced_version.as_deref() == Some(latest_str),
            unsupported_floor,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub notes_url: String,
    pub published: Option<String>,
    pub dismissed: bool,
    pub announced: bool,
    pub unsupported_floor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    UpToDate {
        version: String,
    },
    UpdateAvailable {
        version: String,
        notes_url: String,
        published: String,
    },
    UnsupportedFloor {
        latest: String,
        required_floor: String,
        notes_url: String,
    },
}

/// Configuration for running an update check.
#[derive(Clone, Debug)]
pub struct CheckConfig {
    pub current_version: String,
    pub manifest_url: String,
    pub signature_url: String,
    pub public_keys: Vec<String>,
    pub state_path: PathBuf,
    pub allow_loopback_http: bool,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").to_owned(),
            manifest_url: DEFAULT_MANIFEST_URL.to_owned(),
            signature_url: DEFAULT_SIGNATURE_URL.to_owned(),
            public_keys: vec![CI_PUBLIC_KEY.to_owned(), BACKUP_PUBLIC_KEY.to_owned()],
            state_path: UpdateState::default_path(),
            allow_loopback_http: false,
        }
    }
}

/// Whether `REX_LINUX_NO_UPDATE_CHECK=1` is set in the environment.
pub fn env_disabled() -> bool {
    std::env::var_os("REX_LINUX_NO_UPDATE_CHECK")
        .and_then(|v| v.into_string().ok())
        .is_some_and(|v| matches!(v.trim(), "1" | "true" | "yes"))
}

pub fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Perform a verified release metadata check and update the persistent state file under an advisory lock.
pub fn check_for_updates(config: &CheckConfig, force: bool) -> Result<(CheckOutcome, UpdateState)> {
    let lock_path = config.state_path.with_extension("lock");
    if let Some(parent) = lock_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let _ = fs::create_dir_all(parent);
    }
    let _lock = FileLock::try_acquire(&lock_path);

    let mut state = UpdateState::load(&config.state_path);
    let now = now_epoch_secs();
    if !force && !state.is_due(now) {
        if let Some(avail) = state.available_update(&config.current_version) {
            if let Some(floor) = avail.unsupported_floor {
                return Ok((
                    CheckOutcome::UnsupportedFloor {
                        latest: avail.version,
                        required_floor: floor,
                        notes_url: avail.notes_url,
                    },
                    state,
                ));
            }
            return Ok((
                CheckOutcome::UpdateAvailable {
                    version: avail.version,
                    notes_url: avail.notes_url,
                    published: avail.published.unwrap_or_default(),
                },
                state,
            ));
        }
        return Ok((
            CheckOutcome::UpToDate {
                version: config.current_version.clone(),
            },
            state,
        ));
    }

    let manifest_bytes = fetch_bytes(
        &config.manifest_url,
        MAX_MANIFEST_BYTES,
        config.allow_loopback_http,
    )?;
    let sig_bytes = fetch_bytes(
        &config.signature_url,
        MAX_SIGNATURE_BYTES,
        config.allow_loopback_http,
    )?;
    let sig_text = std::str::from_utf8(&sig_bytes).context("signature is not valid UTF-8")?;
    let key_refs: Vec<&str> = config.public_keys.iter().map(String::as_str).collect();
    let manifest = verify_and_parse_manifest(&manifest_bytes, sig_text, &key_refs)?;

    apply_verified_manifest(&mut state, &config.current_version, &manifest, now).map(|outcome| {
        let _ = state.save(&config.state_path);
        (outcome, state)
    })
}

/// Apply a verified manifest to `state`, enforcing anti-rollback (`highest_verified`) and `update_from`.
pub fn apply_verified_manifest(
    state: &mut UpdateState,
    current_version: &str,
    manifest: &ReleaseManifest,
    now_secs: u64,
) -> Result<CheckOutcome> {
    let current = StableVersion::parse(current_version)?;
    let candidate = StableVersion::parse(&manifest.version)?;
    let floor = StableVersion::parse(&manifest.update_from)?;

    if let Some(highest_str) = &state.highest_verified
        && let Ok(highest) = StableVersion::parse(highest_str)
    {
        ensure!(
            candidate >= highest,
            "refusing rollback manifest {} below previously verified {}",
            manifest.version,
            highest_str
        );
    }

    state.last_checked = Some(now_secs);
    state.highest_verified = Some(manifest.version.clone());

    if candidate <= current {
        state.latest_version = Some(manifest.version.clone());
        state.notes_url = Some(manifest.notes_url.clone());
        state.published = Some(manifest.published.clone());
        state.unsupported_floor = None;
        return Ok(CheckOutcome::UpToDate {
            version: current_version.to_owned(),
        });
    }

    state.latest_version = Some(manifest.version.clone());
    state.notes_url = Some(manifest.notes_url.clone());
    state.published = Some(manifest.published.clone());

    if current < floor {
        state.unsupported_floor = Some(manifest.update_from.clone());
        return Ok(CheckOutcome::UnsupportedFloor {
            latest: manifest.version.clone(),
            required_floor: manifest.update_from.clone(),
            notes_url: manifest.notes_url.clone(),
        });
    }
    state.unsupported_floor = None;

    Ok(CheckOutcome::UpdateAvailable {
        version: manifest.version.clone(),
        notes_url: manifest.notes_url.clone(),
        published: manifest.published.clone(),
    })
}

fn validate_fetch_url(raw_url: &str, allow_loopback_http: bool) -> Result<url::Url> {
    let parsed = url::Url::parse(raw_url).with_context(|| format!("invalid URL {raw_url:?}"))?;
    match parsed.scheme() {
        "https" => Ok(parsed),
        "http" if allow_loopback_http && is_loopback_host(parsed.host_str()) => Ok(parsed),
        scheme => bail!("refusing non-HTTPS update URL scheme {scheme:?}"),
    }
}

fn is_loopback_host(host: Option<&str>) -> bool {
    matches!(host, Some("127.0.0.1" | "localhost" | "[::1]" | "::1"))
}

fn fetch_bytes(raw_url: &str, max_bytes: usize, allow_loopback_http: bool) -> Result<Vec<u8>> {
    let parsed = validate_fetch_url(raw_url, allow_loopback_http)?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .https_only(!allow_loopback_http)
        .max_redirects(5)
        .user_agent("rex-linux")
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get(parsed.as_str())
        .header(
            "Accept",
            "application/octet-stream, application/json, text/plain",
        )
        .call()
        .with_context(|| format!("GET {raw_url} failed"))?;
    let status = response.status().as_u16();
    ensure!(status == 200, "unexpected HTTP status {status}");
    let mut buf = Vec::new();
    let mut reader = response.body_mut().as_reader().take((max_bytes + 1) as u64);
    reader
        .read_to_end(&mut buf)
        .context("failed reading HTTP response body")?;
    ensure!(
        buf.len() <= max_bytes,
        "HTTP response exceeded {max_bytes} bytes"
    );
    Ok(buf)
}

struct FileLock {
    file: File,
}

impl FileLock {
    fn try_acquire(path: &Path) -> Option<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .ok()?;
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc == 0 { Some(Self { file }) } else { None }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

fn xdg_data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
}

pub(crate) fn xdg_state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        })
}

/// Run the `--check-update` CLI flow: print one plain-text summary line and return an exit code
/// (`0` when up to date, `10` when a newer version is available, `1` on verification/fetch error).
pub fn run_cli_check(config: &CheckConfig) -> i32 {
    let channel = InstallChannel::detect();
    match check_for_updates(config, true) {
        Ok((CheckOutcome::UpToDate { version }, _)) => {
            println!("Rex Linux {version} is up to date ({})", channel.id());
            0
        }
        Ok((
            CheckOutcome::UpdateAvailable {
                version, notes_url, ..
            },
            _,
        )) => {
            let guidance = channel.guidance(&version);
            println!(
                "Rex Linux {version} is available (installed: {}, channel: {}). {} Release notes: {notes_url}",
                config.current_version,
                channel.id(),
                guidance.detail
            );
            10
        }
        Ok((
            CheckOutcome::UnsupportedFloor {
                latest,
                required_floor,
                notes_url,
            },
            _,
        )) => {
            println!(
                "Rex Linux {latest} is available (installed: {}, channel: {}), requires manual upgrade via {required_floor}+. Release notes: {notes_url}",
                config.current_version,
                channel.id()
            );
            10
        }
        Err(error) => {
            eprintln!("rex-linux update check failed: {error:#}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    // Deterministic test keypair generated exclusively for unit tests.
    // Secret seed: [7u8; 32], Key ID: 0102030405060708
    fn test_keypair() -> ([u8; 8], ring::signature::Ed25519KeyPair, String) {
        use ring::signature::KeyPair as _;
        let seed = [7u8; 32];
        let pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        let key_id = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut record = Vec::with_capacity(42);
        record.extend_from_slice(b"Ed");
        record.extend_from_slice(&key_id);
        record.extend_from_slice(pair.public_key().as_ref());
        let b64 = base64::engine::general_purpose::STANDARD.encode(&record);
        let pub_text = format!("untrusted comment: minisign public key 0807060504030201\n{b64}\n");
        (key_id, pair, pub_text)
    }

    fn sign_minisig(
        key_id: &[u8; 8],
        pair: &ring::signature::Ed25519KeyPair,
        message: &[u8],
        trusted_comment: &str,
        prehashed: bool,
    ) -> String {
        let sig_bytes = if prehashed {
            let digest = blake2b_512(message);
            pair.sign(&digest)
        } else {
            pair.sign(message)
        };
        let mut sig_record = Vec::with_capacity(74);
        sig_record.extend_from_slice(if prehashed { b"ED" } else { b"Ed" });
        sig_record.extend_from_slice(key_id);
        sig_record.extend_from_slice(sig_bytes.as_ref());
        let mut global_msg = Vec::new();
        global_msg.extend_from_slice(sig_bytes.as_ref());
        global_msg.extend_from_slice(trusted_comment.as_bytes());
        let global_sig = pair.sign(&global_msg);
        format!(
            "untrusted comment: signature from rex test\n{}\ntrusted comment: {trusted_comment}\n{}\n",
            base64::engine::general_purpose::STANDARD.encode(&sig_record),
            base64::engine::general_purpose::STANDARD.encode(global_sig.as_ref()),
        )
    }

    fn sample_manifest_json(version: &str, update_from: &str) -> Vec<u8> {
        let sha = "a".repeat(64);
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "product": "rex-linux",
            "version": version,
            "tag": format!("v{version}"),
            "published": "2026-11-20T10:00:00Z",
            "notes_url": format!("{REPOSITORY}/releases/tag/v{version}"),
            "update_from": update_from,
            "targets": {
                "x86_64-linux-gnu": {
                    "min_glibc": "2.39",
                    "tarball": {
                        "url": format!("{REPOSITORY}/releases/download/v{version}/rex-linux-{version}-x86_64-linux-gnu.tar.gz"),
                        "size": 12345,
                        "sha256": sha
                    },
                    "deb": {
                        "name": format!("rex-linux_{version}_amd64.deb"),
                        "size": 23456,
                        "sha256": sha
                    }
                },
                "aarch64-linux-gnu": {
                    "min_glibc": "2.39",
                    "tarball": {
                        "url": format!("{REPOSITORY}/releases/download/v{version}/rex-linux-{version}-aarch64-linux-gnu.tar.gz"),
                        "size": 12345,
                        "sha256": sha
                    },
                    "deb": {
                        "name": format!("rex-linux_{version}_arm64.deb"),
                        "size": 23456,
                        "sha256": sha
                    }
                }
            }
        }))
        .unwrap()
    }

    #[test]
    fn blake2b_512_matches_rfc_7693_vectors() {
        let empty = blake2b_512(b"");
        let empty_hex: String = empty.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            empty_hex,
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
        );
        let abc = blake2b_512(b"abc");
        let abc_hex: String = abc.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            abc_hex,
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        );
    }

    #[test]
    fn stable_version_comparison_is_numeric_and_rejects_malformed() {
        assert!(StableVersion::parse("0.6.0").unwrap() > StableVersion::parse("0.5.3").unwrap());
        assert!(StableVersion::parse("0.10.0").unwrap() > StableVersion::parse("0.5.9").unwrap());
        assert_eq!(
            StableVersion::parse("0.6.0").unwrap(),
            StableVersion::parse("0.6.0").unwrap()
        );
        for bad in [
            "",
            "v0.6.0",
            "0.6",
            "0.6.0.1",
            "01.6.0",
            "0.06.0",
            "0.6.0-rc.1",
            "0.6.0+build",
            "../0.6.0",
        ] {
            assert!(
                StableVersion::parse(bad).is_err(),
                "accepted invalid version {bad:?}"
            );
        }
    }

    #[test]
    fn embedded_release_keys_parse_cleanly() {
        let ci = ParsedPublicKey::parse(CI_PUBLIC_KEY).unwrap();
        let backup = ParsedPublicKey::parse(BACKUP_PUBLIC_KEY).unwrap();
        assert_ne!(ci.key_id, backup.key_id);
        assert_ne!(ci.pubkey, backup.pubkey);
    }

    #[test]
    fn minisign_verification_accepts_valid_and_rejects_tampering_comment_and_legacy_ed() {
        let (key_id, pair, pub_text) = test_keypair();
        let manifest_bytes = sample_manifest_json("0.6.1", "0.5.0");
        let valid_sig = sign_minisig(
            &key_id,
            &pair,
            &manifest_bytes,
            "rex-linux v0.6.1 latest.json",
            true,
        );

        let parsed =
            verify_and_parse_manifest(&manifest_bytes, &valid_sig, &[CI_PUBLIC_KEY, &pub_text])
                .unwrap();
        assert_eq!(parsed.version, "0.6.1");

        // Tampered manifest fails
        let mut tampered = manifest_bytes.clone();
        tampered.push(b' ');
        assert!(verify_and_parse_manifest(&tampered, &valid_sig, &[&pub_text]).is_err());

        // Wrong trusted comment fails even when cryptographically signed
        let wrong_comment_sig = sign_minisig(
            &key_id,
            &pair,
            &manifest_bytes,
            "rex-linux v0.6.0 latest.json",
            true,
        );
        assert!(
            verify_and_parse_manifest(&manifest_bytes, &wrong_comment_sig, &[&pub_text]).is_err()
        );

        // SHA256SUMS signature replayed as latest.json fails
        let wrong_file_sig = sign_minisig(
            &key_id,
            &pair,
            &manifest_bytes,
            "rex-linux v0.6.1 SHA256SUMS",
            true,
        );
        assert!(verify_and_parse_manifest(&manifest_bytes, &wrong_file_sig, &[&pub_text]).is_err());

        // Non-prehashed 'Ed' signature is rejected
        let legacy_sig = sign_minisig(
            &key_id,
            &pair,
            &manifest_bytes,
            "rex-linux v0.6.1 latest.json",
            false,
        );
        assert!(verify_and_parse_manifest(&manifest_bytes, &legacy_sig, &[&pub_text]).is_err());

        // Untrusted key fails
        assert!(
            verify_and_parse_manifest(
                &manifest_bytes,
                &valid_sig,
                &[CI_PUBLIC_KEY, BACKUP_PUBLIC_KEY]
            )
            .is_err()
        );
    }

    #[test]
    fn anti_rollback_and_update_floor_are_enforced() {
        let mut state = UpdateState::default();
        let m_6_2 =
            ReleaseManifest::parse_and_validate(&sample_manifest_json("0.6.2", "0.5.0")).unwrap();
        let outcome = apply_verified_manifest(&mut state, "0.6.0", &m_6_2, 1000).unwrap();
        assert!(matches!(
            outcome,
            CheckOutcome::UpdateAvailable { ref version, .. } if version == "0.6.2"
        ));
        assert_eq!(state.highest_verified.as_deref(), Some("0.6.2"));

        // Replaying an older signed manifest (0.6.1 < 0.6.2) is rejected.
        let m_6_1 =
            ReleaseManifest::parse_and_validate(&sample_manifest_json("0.6.1", "0.5.0")).unwrap();
        assert!(apply_verified_manifest(&mut state, "0.6.0", &m_6_1, 2000).is_err());

        // Manifest with update_from higher than current version returns UnsupportedFloor.
        let m_7_0 =
            ReleaseManifest::parse_and_validate(&sample_manifest_json("0.7.0", "0.6.5")).unwrap();
        let outcome = apply_verified_manifest(&mut state, "0.6.0", &m_7_0, 3000).unwrap();
        assert_eq!(
            outcome,
            CheckOutcome::UnsupportedFloor {
                latest: "0.7.0".into(),
                required_floor: "0.6.5".into(),
                notes_url: format!("{REPOSITORY}/releases/tag/v0.7.0"),
            }
        );
    }

    #[test]
    fn channel_detection_and_guidance_match_all_install_methods() {
        let temp = std::env::temp_dir().join(format!("rex-channel-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let bin_dir = temp.join("usr/bin");
        let share_dir = temp.join("usr/share/rex-linux");
        let data_dir = temp.join("data");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&share_dir).unwrap();
        fs::create_dir_all(data_dir.join("rex-linux")).unwrap();
        let exe = bin_dir.join("rex-linux");
        fs::write(&exe, b"#!/bin/sh\necho rex\n").unwrap();

        // Source and Nix build channels take priority and never consult markers.
        assert_eq!(
            InstallChannel::detect_with("source", Some(&exe), &data_dir, None),
            InstallChannel::Source
        );
        assert!(!InstallChannel::Source.checks_by_default());
        assert_eq!(
            InstallChannel::detect_with("nix", Some(&exe), &data_dir, None),
            InstallChannel::Nix
        );
        assert!(
            InstallChannel::Nix
                .guidance("0.6.1")
                .summary
                .contains("nix flake update rex-rune")
        );

        // Pacman marker on standard Arch vs Omarchy
        fs::write(
            share_dir.join("channel"),
            "channel=pacman\nhint=sudo pacman -Syu\n",
        )
        .unwrap();
        let arch =
            InstallChannel::detect_with("official", Some(&exe), &data_dir, Some("ID=arch\n"));
        assert_eq!(
            arch,
            InstallChannel::Pacman {
                omarchy: false,
                custom_hint: None
            }
        );
        assert!(
            arch.guidance("0.6.1")
                .summary
                .contains("sudo pacman -Syu rex-linux-bin")
        );
        assert_eq!(
            arch.guidance("0.6.1").command.as_deref(),
            Some("sudo pacman -Syu rex-linux-bin")
        );

        let omarchy = InstallChannel::detect_with(
            "official",
            Some(&exe),
            &data_dir,
            Some("ID=omarchy\nID_LIKE=arch\n"),
        );
        assert_eq!(
            omarchy,
            InstallChannel::Pacman {
                omarchy: true,
                custom_hint: None
            }
        );
        assert!(omarchy.guidance("0.6.1").summary.contains("omarchy update"));
        assert_eq!(
            omarchy.guidance("0.6.1").command.as_deref(),
            Some("omarchy update")
        );

        // Deb marker
        fs::write(
            share_dir.join("channel"),
            "channel=deb\nhint=Install the latest .deb from GitHub Releases\n",
        )
        .unwrap();
        let deb = InstallChannel::detect_with("official", Some(&exe), &data_dir, None);
        assert_eq!(deb, InstallChannel::Deb { custom_hint: None });
        assert!(deb.guidance("0.6.1").summary.contains("sudo apt install"));
        assert!(deb.guidance("0.6.1").summary.contains("dpkg"));

        // Remove package marker and test install.json receipt
        fs::remove_file(share_dir.join("channel")).unwrap();
        assert_eq!(
            InstallChannel::detect_with("official", Some(&exe), &data_dir, None),
            InstallChannel::Unmanaged
        );

        let resolved = fs::canonicalize(&exe).unwrap();
        let digest = hex_sha256(&fs::read(&resolved).unwrap());
        let receipt = serde_json::json!({
            "schema": 1,
            "path": resolved.to_string_lossy(),
            "version": "0.6.0",
            "sha256": digest,
        });
        fs::write(
            data_dir.join("rex-linux/install.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        let tarball = InstallChannel::detect_with("official", Some(&exe), &data_dir, None);
        assert_eq!(tarball, InstallChannel::Tarball);
        assert_eq!(
            tarball.guidance("0.6.1").command.as_deref(),
            Some(INSTALL_SCRIPT_COMMAND)
        );

        // Tampered binary hash invalidates receipt -> Unmanaged
        fs::write(&exe, b"#!/bin/sh\necho modified\n").unwrap();
        assert_eq!(
            InstallChannel::detect_with("official", Some(&exe), &data_dir, None),
            InstallChannel::Unmanaged
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn loopback_server_handles_success_and_failure_modes() {
        let (key_id, pair, pub_text) = test_keypair();
        let manifest_bytes = sample_manifest_json("0.6.1", "0.5.0");
        let valid_sig = sign_minisig(
            &key_id,
            &pair,
            &manifest_bytes,
            "rex-linux v0.6.1 latest.json",
            true,
        );

        let temp = std::env::temp_dir().join(format!("rex-update-http-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let state_path = temp.join("update.json");

        // Helper for one-shot loopback HTTP server
        let serve_twice = |first: (u16, Vec<u8>), second: (u16, Vec<u8>)| -> String {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            std::thread::spawn(move || {
                for (status, body) in [first, second] {
                    let Ok((mut stream, _)) = listener.accept() else {
                        break;
                    };
                    let mut req = [0u8; 1024];
                    let _ = stream.read(&mut req);
                    let header = format!(
                        "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(&body);
                }
            });
            format!("http://127.0.0.1:{}", addr.port())
        };

        // 1. Valid manifest + signature succeeds and persists state.
        let base = serve_twice(
            (200, manifest_bytes.clone()),
            (200, valid_sig.as_bytes().to_vec()),
        );
        let config = CheckConfig {
            current_version: "0.6.0".into(),
            manifest_url: format!("{base}/latest.json"),
            signature_url: format!("{base}/latest.json.minisig"),
            public_keys: vec![pub_text.clone()],
            state_path: state_path.clone(),
            allow_loopback_http: true,
        };
        let (outcome, state) = check_for_updates(&config, true).unwrap();
        assert!(matches!(
            outcome,
            CheckOutcome::UpdateAvailable { ref version, .. } if version == "0.6.1"
        ));
        assert_eq!(state.latest_version.as_deref(), Some("0.6.1"));

        // 2. Non-forced check within 24h reuses cached state without network requests.
        let cached_config = CheckConfig {
            manifest_url: "https://127.0.0.1:1/unreachable".into(),
            signature_url: "https://127.0.0.1:1/unreachable".into(),
            ..config.clone()
        };
        let (cached_outcome, _) = check_for_updates(&cached_config, false).unwrap();
        assert_eq!(cached_outcome, outcome);

        // 3. Rate limit (429), captive portal HTML, and oversized body fail cleanly without corrupting state.
        for (status, body) in [
            (429, b"rate limited".to_vec()),
            (500, b"server error".to_vec()),
            (200, b"<html>captive portal</html>".to_vec()),
            (200, vec![b'a'; MAX_MANIFEST_BYTES + 10]),
        ] {
            let bad_base = serve_twice((status, body), (200, valid_sig.as_bytes().to_vec()));
            let bad_cfg = CheckConfig {
                manifest_url: format!("{bad_base}/latest.json"),
                signature_url: format!("{bad_base}/latest.json.minisig"),
                ..config.clone()
            };
            assert!(check_for_updates(&bad_cfg, true).is_err());
            assert_eq!(
                UpdateState::load(&state_path).latest_version.as_deref(),
                Some("0.6.1")
            );
        }

        // 4. Plain HTTP is refused unless allow_loopback_http is set.
        let http_refused = CheckConfig {
            allow_loopback_http: false,
            ..config
        };
        assert!(check_for_updates(&http_refused, true).is_err());

        let _ = fs::remove_dir_all(&temp);
    }
}
