//! Where a Rex server on this machine listens.
use std::path::PathBuf;

/// The endpoint of this user's own server: `REX_SERVER` when set, else the socket the Rex
/// server makes by default on this platform.
///
/// | Platform | Socket |
/// | --- | --- |
/// | Linux and other Unix | `$XDG_RUNTIME_DIR/rex/server.sock`, else `/run/user/<uid>/rex/server.sock` |
/// | macOS | `~/Library/Application Support/rex/server.sock` |
pub fn default_endpoint() -> String {
    std::env::var("REX_SERVER").unwrap_or_else(|_| format!("unix://{}", default_socket().display()))
}

/// The socket of this user's own server, whatever `REX_SERVER` says: the one a probe that
/// creates sessions must never be pointed at.
pub fn default_socket() -> PathBuf {
    let folder = if cfg!(target_os = "macos") {
        super::dirs::home().join("Library/Application Support")
    } else {
        super::dirs::xdg_runtime()
            .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", super::process::user_id())))
    };
    folder.join("rex/server.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_socket_is_where_this_platforms_server_listens() {
        let socket = default_socket();
        assert!(socket.ends_with("rex/server.sock"));
        if cfg!(target_os = "macos") {
            assert_eq!(
                socket,
                crate::platform::dirs::home().join("Library/Application Support/rex/server.sock")
            );
        } else if std::env::var_os("XDG_RUNTIME_DIR").is_none() {
            assert!(socket.starts_with("/run/user"));
        }
    }
}
