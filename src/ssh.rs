//! Reaching a Rex server over SSH. An `ssh://[user@]host[:port][/socket]` endpoint forwards the
//! server's Unix socket on that machine to a local one with `ssh -L`, and the client attaches
//! to the local end like any `unix://` endpoint. Only `sshd` is needed on the other side, so a
//! machine with Tailscale SSH, or any SSH key already set up, works without configuring Rex
//! there. Without a `/socket`, the machine is asked where its server listens.
use anyhow::{Context, Result, bail, ensure};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::AsyncReadExt;

pub mod setup;

/// Socket discovery succeeded over SSH, but found no server. Authentication and
/// transport failures deliberately do not carry this opt-in setup signal.
#[derive(Debug)]
pub struct MissingServer(pub String);
impl std::fmt::Display for MissingServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for MissingServer {}

/// Where an `ssh://` endpoint points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// The login name; none lets `ssh` apply its configuration and the local user name.
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    /// The Rex socket's absolute path on that machine, when the user named it; none asks the
    /// machine (see [`RESOLVE`]).
    pub socket: Option<String>,
}

const USAGE: &str = "Use ssh://user@host, with an optional :port and /path/to/server.sock";

impl Target {
    /// `None` for endpoints of other schemes; an error for a malformed `ssh://` one.
    pub fn parse(endpoint: &str) -> Result<Option<Target>> {
        let Some(rest) = endpoint.strip_prefix("ssh://") else {
            return Ok(None);
        };
        ensure!(!rest.contains(char::is_whitespace), "{USAGE}");
        let (rest, socket) = match rest.find('/') {
            Some(slash) => {
                let socket = &rest[slash..];
                ensure!(
                    socket.len() > 1 && !socket.contains(['\'', '"', '$', '`', '\\']),
                    "Use an absolute socket path after the host, such as /run/rex/server.sock"
                );
                (&rest[..slash], Some(socket))
            }
            None => (rest, None),
        };
        let (user, address) = match rest.rsplit_once('@') {
            Some((user, address)) => (Some(user), address),
            None => (None, rest),
        };
        // An IPv6 address is written in brackets, as in a URL.
        let (host, after) = match address.strip_prefix('[') {
            Some(inner) => {
                let (host, after) = inner.rsplit_once(']').with_context(|| USAGE)?;
                (host, after)
            }
            None => match address.rsplit_once(':') {
                Some((host, _)) => (host, &address[host.len()..]),
                None => (address, ""),
            },
        };
        let port = match after.strip_prefix(':') {
            Some(port) => Some(
                port.parse::<u16>()
                    .ok()
                    .filter(|p| *p > 0)
                    .with_context(|| format!("\"{port}\" is not a port number"))?,
            ),
            None => {
                ensure!(after.is_empty(), "{USAGE}");
                None
            }
        };
        ensure!(!host.is_empty() && !host.starts_with('-'), "{USAGE}");
        ensure!(
            user.is_none_or(|u| !u.is_empty() && !u.starts_with('-') && !u.contains(':')),
            "{USAGE}"
        );
        Ok(Some(Target {
            user: user.map(str::to_owned),
            host: host.to_owned(),
            port,
            socket: socket.map(str::to_owned),
        }))
    }

    /// The `ssh://` endpoint of this target.
    pub fn endpoint(&self) -> String {
        let mut out = String::from("ssh://");
        if let Some(user) = &self.user {
            out.push_str(user);
            out.push('@');
        }
        out.push_str(&self.host_in_url());
        if let Some(port) = self.port {
            out.push_str(&format!(":{port}"));
        }
        if let Some(socket) = &self.socket {
            out.push_str(socket);
        }
        out
    }

    fn missing_server_message(&self, user: &str) -> String {
        let install_target = Target {
            socket: None,
            ..self.clone()
        }
        .endpoint()
        .replace('\'', "'\\''");
        format!(
            "No Rex server found on {} for {user}: nothing in {user}'s runtime directory, /run/rex or /var/run/rex. Attachment never starts a server. Choose Start personal server… to review a one-time start, or run rex server install '{install_target}' yourself (Rex preview.14 or newer; asks before changes). If a server already runs elsewhere, name its socket in the SSH endpoint.",
            self.host
        )
    }

    /// The host as a URL writes it: an IPv6 address in brackets.
    fn host_in_url(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        }
    }

    /// `user@host` as `ssh` takes it.
    pub fn destination(&self) -> String {
        match &self.user {
            Some(user) => format!("{user}@{}", self.host),
            None => self.host.clone(),
        }
    }

    /// The options every `ssh` run uses: never prompt, trust a machine's key the first time
    /// (and refuse a changed one), notice a dead link within a minute, and keep clear of a
    /// `ControlMaster` in the user's configuration, which would take the forward over and keep
    /// it after this `ssh` ends, so the next forward of that path never listens. Logging stays
    /// at its default so a server's banner (how Tailscale SSH explains a refused login) is
    /// seen.
    fn options(&self) -> Vec<String> {
        let mut args = vec![
            "-o".into(),
            "ControlMaster=no".into(),
            "-o".into(),
            "ControlPath=none".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=10".into(),
            "-o".into(),
            "StrictHostKeyChecking=accept-new".into(),
            "-o".into(),
            "ServerAliveInterval=15".into(),
            "-o".into(),
            "ServerAliveCountMax=3".into(),
        ];
        if let Some(port) = self.port {
            args.push("-p".into());
            args.push(port.to_string());
        }
        args
    }

    /// The local socket the forward binds: fixed per endpoint within this process, so panes
    /// reconnect through the same path after a new tunnel replaces a dead one, and in a
    /// directory of this process's own, so another instance (or a probe) forwarding the same
    /// endpoint neither takes this path over nor removes it when it closes.
    pub fn local_socket(&self) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        self.endpoint().hash(&mut hasher);
        tunnels_directory()
            .join(format!("ssh-{}", std::process::id()))
            .join(format!("{:016x}.sock", hasher.finish()))
    }
}

/// Where every process's tunnel sockets live.
fn tunnels_directory() -> PathBuf {
    crate::platform::dirs::runtime()
}

/// Remove the socket directories of processes that are gone, which a crash or a kill leaves
/// behind. Only `ssh-<pid>` directories whose process no longer exists are touched.
fn sweep_stale_tunnels(directory: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix("ssh-"))
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != std::process::id() && !crate::platform::process::exists(pid) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Finds the Rex socket on the other machine, trying in order: the path `rex server status`
/// reports, the login user's own server in the runtime directory (Linux) or under
/// Application Support (macOS), then a shared server's `/run/rex/server.sock` or FreeBSD's
/// `/var/run/rex/server.sock`. Prints `sock <path>` for the first that is a socket, else
/// `none <user>`. It is wrapped in `sh -c '…'`, so it holds no single quote.
const RESOLVE: &str = "p=\"\"; for c in \"$(rex server status --autostart=false 2>/dev/null | grep -o \"unix:///[^[:space:]]*\\.sock\" | head -n 1 | sed \"s#^unix://##\")\" \"${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/rex/server.sock\" \"$HOME/Library/Application Support/rex/server.sock\" /run/rex/server.sock /var/run/rex/server.sock; do if [ -n \"$c\" ] && [ -S \"$c\" ]; then p=\"$c\"; break; fi; done; if [ -n \"$p\" ]; then printf \"sock %s\\n\" \"$p\"; else printf \"none %s\\n\" \"$(id -un)\"; fi";

/// The check for a socket the user named: `sock <path>` when it is one, else `missing <path>`.
fn check_script(socket: &str) -> String {
    format!(
        "if [ -S \"{socket}\" ]; then printf \"sock %s\\n\" \"{socket}\"; else printf \"missing %s\\n\" \"{socket}\"; fi"
    )
}

/// A running `ssh -L` forward. Dropping it ends the forward.
pub struct Tunnel {
    child: tokio::process::Child,
    /// Held here because `Child::wait` closes the stdin it still owns, and that would end the
    /// remote `cat` and with it the login the forward lives on.
    _stdin: Option<tokio::process::ChildStdin>,
    stderr: Arc<Mutex<String>>,
    local: PathBuf,
    endpoint: String,
    target: Target,
}

impl Tunnel {
    /// A forward for `endpoint` if it is an `ssh://` one, else `None`.
    pub async fn for_endpoint(endpoint: &str) -> Result<Option<Tunnel>> {
        match Target::parse(endpoint)? {
            Some(target) => Ok(Some(Tunnel::open(&target).await?)),
            None => Ok(None),
        }
    }

    /// Find the Rex socket on `target`, then forward it to a local socket and wait until the
    /// local end accepts connections.
    pub async fn open(target: &Target) -> Result<Tunnel> {
        let remote = resolve(target).await?;
        let local = target.local_socket();
        if let Some(parent) = local.parent() {
            // Every process's folder, then this one's: each ours alone, or no tunnel.
            for directory in [parent.parent().unwrap_or(parent), parent] {
                crate::platform::dirs::private(directory).with_context(|| {
                    format!(
                        "{} is not a private folder for the tunnel",
                        directory.display()
                    )
                })?;
            }
            if let Some(all) = parent.parent() {
                sweep_stale_tunnels(all);
            }
        }
        // Only this process's own earlier forward of the endpoint can be here.
        let _ = tokio::fs::remove_file(&local).await;
        // The remote `cat` ends when its input does, so if this process dies without dropping
        // the tunnel, the closed stdin pipe still ends the forward instead of leaving `ssh`
        // behind.
        let child = tokio::process::Command::new("ssh")
            .args(target.options())
            .args([
                "-T",
                "-o",
                "ExitOnForwardFailure=yes",
                "-o",
                "StreamLocalBindUnlink=yes",
            ])
            .arg("-L")
            .arg(format!("{}:{remote}", local.display()))
            .arg("--")
            .arg(target.destination())
            .arg("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(ssh_error)?;
        let mut tunnel = Tunnel::adopt(child, target, local);
        let started = std::time::Instant::now();
        loop {
            if let Some(status) = tunnel.child.try_wait()? {
                bail!("{}", tunnel.exit_message(status));
            }
            if tokio::net::UnixStream::connect(&tunnel.local).await.is_ok() {
                return Ok(tunnel);
            }
            ensure!(
                started.elapsed() < Duration::from_secs(20),
                "ssh {} did not open the forward in time",
                tunnel.target.destination()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn adopt(mut child: tokio::process::Child, target: &Target, local: PathBuf) -> Tunnel {
        let stderr = Arc::new(Mutex::new(String::new()));
        if let Some(pipe) = child.stderr.take() {
            let sink = stderr.clone();
            tokio::spawn(drain(pipe, sink));
        }
        Tunnel {
            _stdin: child.stdin.take(),
            child,
            stderr,
            endpoint: format!("unix://{}", local.display()),
            local,
            target: target.clone(),
        }
    }

    /// The `unix://` endpoint of the local end.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Resolves once the forward ends, with why.
    pub async fn closed(&mut self) -> anyhow::Error {
        match self.child.wait().await {
            Ok(status) => anyhow::anyhow!("{}", self.exit_message(status)),
            Err(error) => anyhow::Error::from(error).context("waiting for ssh"),
        }
    }

    fn exit_message(&self, status: std::process::ExitStatus) -> String {
        let stderr = self.stderr.lock().expect("ssh stderr");
        match last_line(&stderr) {
            Some(line) => format!("ssh {}: {line}", self.target.destination()),
            None if status.success() => {
                format!("ssh {} closed the forward", self.target.destination())
            }
            None => format!("ssh {} exited ({status})", self.target.destination()),
        }
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        let _ = std::fs::remove_file(&self.local);
        // Gone once the last forward of this process ends; it fails harmlessly otherwise.
        if let Some(parent) = self.local.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}

/// Wait for a tunnel to end; pends forever without one, for `select!` arms.
pub async fn closed(tunnel: &mut Option<Tunnel>) -> anyhow::Error {
    match tunnel {
        Some(tunnel) => tunnel.closed().await,
        None => std::future::pending().await,
    }
}

async fn drain(mut pipe: tokio::process::ChildStderr, sink: Arc<Mutex<String>>) {
    let mut buffer = [0u8; 1024];
    while let Ok(n) = pipe.read(&mut buffer).await {
        if n == 0 {
            break;
        }
        let mut text = sink.lock().expect("ssh stderr");
        text.push_str(&String::from_utf8_lossy(&buffer[..n]));
        if text.len() > 8192 {
            let cut = text.len() - 4096;
            text.drain(..cut);
        }
    }
}

/// The last line `ssh` printed that explains something, without its `ssh: ` prefix. A
/// server's parting words (such as Tailscale SSH refusing a user) beat the generic "Connection
/// closed" that follows them.
fn last_line(stderr: &str) -> Option<String> {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("Warning: Permanently added"))
        .collect();
    let generic = |l: &str| {
        l.starts_with("Connection closed by")
            || l.starts_with("Connection to ")
            || l.starts_with("Transferred:")
            || l.starts_with("Bytes per second:")
    };
    lines
        .iter()
        .rfind(|l| !generic(l))
        .or(lines.last())
        .map(|l| l.strip_prefix("ssh: ").unwrap_or(l).to_owned())
}

fn ssh_error(error: std::io::Error) -> anyhow::Error {
    if error.kind() == std::io::ErrorKind::NotFound {
        anyhow::anyhow!("ssh isn't installed on this machine")
    } else {
        anyhow::Error::from(error).context("run ssh")
    }
}

/// Ask the other machine where its Rex server listens, or whether the socket the user named
/// is there.
async fn resolve(target: &Target) -> Result<String> {
    let script = match &target.socket {
        Some(socket) => check_script(socket),
        None => RESOLVE.to_owned(),
    };
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("ssh")
            .args(target.options())
            .arg("--")
            .arg(target.destination())
            .arg(format!("sh -c '{script}'"))
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .with_context(|| format!("ssh {} did not answer", target.destination()))?
    .map_err(ssh_error)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        bail!(
            "ssh {}: {}",
            target.destination(),
            last_line(&stderr).unwrap_or_else(|| format!("exited ({})", output.status))
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    enum Answer {
        Sock(String),
        None(String),
        Missing(String),
    }
    let answer = stdout.lines().find_map(|l| {
        l.strip_prefix("sock ")
            .map(|p| Answer::Sock(p.to_owned()))
            .or_else(|| {
                l.strip_prefix("none ")
                    .map(|u| Answer::None(u.trim().to_owned()))
            })
            .or_else(|| {
                l.strip_prefix("missing ")
                    .map(|p| Answer::Missing(p.trim().to_owned()))
            })
    });
    match answer {
        Some(Answer::Sock(path)) => Ok(path),
        Some(Answer::None(user)) => Err(MissingServer(target.missing_server_message(&user)).into()),
        Some(Answer::Missing(path)) => bail!(
            "{path} on {} is not a socket. Is Rex running there?",
            target.host
        ),
        None => bail!(
            "ssh {} did not report a Rex socket: {}",
            target.destination(),
            last_line(&stderr).unwrap_or_else(|| stdout.trim().to_owned())
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_parse_and_print() {
        let t = Target::parse("ssh://shuv@box.tail1234.ts.net")
            .unwrap()
            .unwrap();
        assert_eq!(t.user.as_deref(), Some("shuv"));
        assert_eq!(t.host, "box.tail1234.ts.net");
        assert_eq!(t.port, None);
        assert_eq!(t.endpoint(), "ssh://shuv@box.tail1234.ts.net");
        assert_eq!(t.destination(), "shuv@box.tail1234.ts.net");

        let t = Target::parse("ssh://box:2222").unwrap().unwrap();
        assert_eq!(t.user, None);
        assert_eq!(t.port, Some(2222));
        assert_eq!(t.socket, None);
        assert_eq!(t.endpoint(), "ssh://box:2222");
        assert_eq!(t.destination(), "box");

        let t = Target::parse("ssh://ops@box:2222/run/rex/server.sock")
            .unwrap()
            .unwrap();
        assert_eq!(t.user.as_deref(), Some("ops"));
        assert_eq!(t.port, Some(2222));
        assert_eq!(t.socket.as_deref(), Some("/run/rex/server.sock"));
        assert_eq!(t.endpoint(), "ssh://ops@box:2222/run/rex/server.sock");
        assert_eq!(t.destination(), "ops@box");

        let t = Target::parse("ssh://me@[fd7a::1]:22/run/rex/server.sock")
            .unwrap()
            .unwrap();
        assert_eq!(t.host, "fd7a::1");
        assert_eq!(t.port, Some(22));
        assert_eq!(t.endpoint(), "ssh://me@[fd7a::1]:22/run/rex/server.sock");
        assert_eq!(t.destination(), "me@fd7a::1");
        let t = Target::parse("ssh://[fd7a::1]").unwrap().unwrap();
        assert_eq!(t.host, "fd7a::1");
        assert_eq!(t.port, None);

        assert!(Target::parse("https://box").unwrap().is_none());
        assert!(Target::parse("unix:///tmp/x.sock").unwrap().is_none());
        for bad in [
            "ssh://",
            "ssh://@box",
            "ssh://-box",
            "ssh://me@-box",
            "ssh://-user@box",
            "ssh://[-box]",
            "ssh://box:x",
            "ssh://box:0",
            "ssh://box/",
            "ssh://box/run/a b.sock",
            "ssh://box/run/$(x).sock",
            "ssh://a b",
            "ssh://[fd7a::1",
            "ssh://[fd7a::1]x",
        ] {
            assert!(Target::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn missing_server_hint_preserves_ssh_config_user_port_and_ipv6() {
        for endpoint in [
            "ssh://work-alias",
            "ssh://ops@box:2222",
            "ssh://me@[fd7a::1]:22",
        ] {
            let target = Target::parse(endpoint).unwrap().unwrap();
            let message = target.missing_server_message("remote-user");
            assert!(message.contains(&format!("rex server install '{endpoint}'")));
            assert!(message.contains("Attachment never starts a server"));
            assert!(message.contains("asks before changes"));
        }
        let target = Target::parse("ssh://box/path/to/server.sock")
            .unwrap()
            .unwrap();
        assert!(
            target
                .missing_server_message("me")
                .contains("rex server install 'ssh://box'")
        );
    }

    #[test]
    fn the_local_socket_is_fixed_per_endpoint_and_owned_by_this_process() {
        let a = Target::parse("ssh://shuv@box").unwrap().unwrap();
        let b = Target::parse("ssh://shuv@box").unwrap().unwrap();
        let c = Target::parse("ssh://root@box").unwrap().unwrap();
        let d = Target::parse("ssh://shuv@box/run/rex/server.sock")
            .unwrap()
            .unwrap();
        assert_eq!(a.local_socket(), b.local_socket());
        assert_ne!(a.local_socket(), c.local_socket());
        assert_ne!(a.local_socket(), d.local_socket());
        assert!(a.local_socket().to_string_lossy().ends_with(".sock"));
        let own = format!("ssh-{}", std::process::id());
        assert_eq!(
            a.local_socket().parent().unwrap().file_name().unwrap(),
            own.as_str(),
            "another instance forwarding the same endpoint uses another directory"
        );
    }

    #[test]
    fn stale_tunnel_directories_of_gone_processes_are_swept() {
        let root = std::env::temp_dir().join(format!("rex-sweep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // Linux pids never exceed 4194304, so this process is gone.
        let gone = root.join("ssh-4194305");
        let own = root.join(format!("ssh-{}", std::process::id()));
        let other = root.join("notes");
        for d in [&gone, &own, &other] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("x.sock"), b"").unwrap();
        }
        sweep_stale_tunnels(&root);
        assert!(!gone.exists(), "a gone process's directory is removed");
        assert!(own.exists(), "this process's directory stays");
        assert!(other.exists(), "anything else is left alone");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn waiting_for_a_tunnel_to_end_leaves_its_login_running() {
        // Stands in for `ssh … cat`, which ends when its stdin does.
        let child = tokio::process::Command::new("sh")
            .args(["-c", "cat >/dev/null; echo the login ended >&2; exit 3"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let target = Target::parse("ssh://me@box").unwrap().unwrap();
        let local = std::env::temp_dir().join(format!("rex-tunnel-{}.sock", std::process::id()));
        let mut tunnel = Tunnel::adopt(child, &target, local);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), tunnel.closed())
                .await
                .is_err(),
            "the forward lasts as long as this process holds it"
        );
        // This process ending closes the pipe, and that ends the forward.
        tunnel._stdin = None;
        let error = tokio::time::timeout(Duration::from_secs(5), tunnel.closed())
            .await
            .expect("the login ends with its stdin");
        assert!(error.to_string().starts_with("ssh me@box"), "{error}");
    }

    #[test]
    fn ssh_stays_clear_of_a_shared_control_master() {
        let target = Target::parse("ssh://me@box").unwrap().unwrap();
        let options = target.options();
        for option in ["ControlMaster=no", "ControlPath=none"] {
            assert!(options.iter().any(|o| o == option), "{option}: {options:?}");
        }
    }

    #[test]
    fn the_check_script_names_the_socket() {
        let script = check_script("/run/rex/server.sock");
        assert!(script.contains("-S \"/run/rex/server.sock\""));
        assert!(!script.contains('\''), "it is wrapped in sh -c '…'");
    }

    #[test]
    fn the_last_useful_stderr_line_is_reported() {
        assert_eq!(
            last_line(
                "Warning: Permanently added 'box' (ED25519) to the list of known hosts.\nshuv@box: Permission denied (publickey).\n"
            ),
            Some("shuv@box: Permission denied (publickey).".into())
        );
        assert_eq!(last_line("\n  \n"), None);
        assert_eq!(
            last_line("ssh: connect to host box port 22: Connection refused"),
            Some("connect to host box port 22: Connection refused".into())
        );
        assert_eq!(
            last_line(
                "tailscale: tailnet policy does not permit you to SSH as user \"me\"\nConnection closed by 100.1.2.3 port 22\n"
            ),
            Some("tailscale: tailnet policy does not permit you to SSH as user \"me\"".into())
        );
        assert_eq!(
            last_line("Connection closed by 100.1.2.3 port 22\n"),
            Some("Connection closed by 100.1.2.3 port 22".into())
        );
    }

    #[test]
    fn the_resolve_script_has_no_single_quotes_and_knows_the_documented_paths() {
        assert!(!RESOLVE.contains('\''), "it is wrapped in sh -c '…'");
        for path in [
            "rex server status",
            "/rex/server.sock",
            "/run/rex/server.sock",
            "/var/run/rex/server.sock",
            "Application Support/rex/server.sock",
        ] {
            assert!(RESOLVE.contains(path), "{path}");
        }
        // The shell accepts it.
        let status = std::process::Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(RESOLVE)
            .status()
            .unwrap();
        assert!(status.success());
    }
}
