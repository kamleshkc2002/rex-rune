//! Socket options that exist under different names, or not at all.
use std::{io, time::Duration};

/// `<netinet/tcp.h>` on macOS: the time after which retransmissions stop and the connection
/// is dropped, in seconds. The `libc` crate does not define it.
#[cfg(target_os = "macos")]
const TCP_RXT_CONNDROPTIME: libc::c_int = 0x80;

/// Give up on data the peer has not acknowledged after `limit`, so a connection whose peer
/// vanished while this side was writing fails instead of retransmitting for many minutes.
///
/// Linux calls this `TCP_USER_TIMEOUT` and macOS `TCP_RXT_CONNDROPTIME`. Elsewhere nothing
/// is set and keepalive alone finds a silent peer, once the connection is idle.
pub fn set_unacknowledged_limit(tcp: &tokio::net::TcpStream, limit: Duration) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        socket2::SockRef::from(tcp).set_tcp_user_timeout(Some(limit))
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        let seconds = libc::c_int::try_from(limit.as_secs()).unwrap_or(libc::c_int::MAX);
        let result = unsafe {
            libc::setsockopt(
                tcp.as_raw_fd(),
                libc::IPPROTO_TCP,
                TCP_RXT_CONNDROPTIME,
                (&raw const seconds).cast(),
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (tcp, limit);
        Ok(())
    }
}

/// What [`set_unacknowledged_limit`] left on the socket, or `None` where it sets nothing.
pub fn unacknowledged_limit(tcp: &tokio::net::TcpStream) -> io::Result<Option<Duration>> {
    #[cfg(target_os = "linux")]
    {
        socket2::SockRef::from(tcp).tcp_user_timeout()
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        let mut seconds: libc::c_int = 0;
        let mut size = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        let result = unsafe {
            libc::getsockopt(
                tcp.as_raw_fd(),
                libc::IPPROTO_TCP,
                TCP_RXT_CONNDROPTIME,
                (&raw mut seconds).cast(),
                &raw mut size,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(u64::try_from(seconds)
            .ok()
            .filter(|seconds| *seconds > 0)
            .map(Duration::from_secs))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = tcp;
        Ok(None)
    }
}
