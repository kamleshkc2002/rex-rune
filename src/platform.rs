//! What differs between operating systems, for the library: where files live, how another
//! process is identified, where a local Rex server listens, and the few system calls that
//! have no portable form.
//!
//! A `cfg` for an operating system belongs in here, not at a call site. Each function has
//! one meaning on every platform; where a platform cannot do something, the function says
//! so in its result instead of compiling to something that fails quietly. Nothing here
//! depends on the UI, so the probes build without it. The app's own platform code (the
//! primary selection, window chrome) is in `src/app/platform`.
/// Whether releases are published for this platform: packages, an installer and signed
/// updates. Elsewhere the client is always a source build. (`update::installer` itself is
/// compiled only where this is true; a module cannot be chosen by a constant.)
pub const HAS_RELEASES: bool = cfg!(target_os = "linux");

pub mod dirs;
pub mod fs;
pub mod local;
pub mod net;
pub mod process;
