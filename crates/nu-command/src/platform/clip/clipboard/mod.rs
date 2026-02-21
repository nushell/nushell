mod arboard_provider;
mod error_mapper;
pub mod provider;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

// On other platforms, the clipboard is either a dummy or implemented in `provider.rs`.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub(crate) mod dummy;
