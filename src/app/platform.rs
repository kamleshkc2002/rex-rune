//! What differs between operating systems, for the app: desktop services that exist on
//! one platform and not another. The library's platform code (folders, processes, the
//! local endpoint) is `rex_rune::platform`.
//!
//! A `cfg` for an operating system belongs in here, not at a call site. A service that a
//! platform does not have says so through its interface, so the app can leave the feature
//! out instead of offering something that does nothing.
pub mod open;
pub mod selection;
