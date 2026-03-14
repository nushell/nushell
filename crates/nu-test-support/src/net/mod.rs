// The `gatekeeper` crate right now only supports unix.
// So we only expose this proxy for unix right now.

#[cfg(unix)]
pub mod proxy;

#[cfg(unix)]
pub use gatekeeper::Address;
