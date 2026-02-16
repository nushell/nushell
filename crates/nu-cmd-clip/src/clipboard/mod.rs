mod arboard_provider;
pub mod clipboard;
mod error_mapper;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[cfg(target_os = "macos")]
pub(crate) mod mac_os;

#[cfg(target_os = "windows")]
pub(crate) mod windows;
