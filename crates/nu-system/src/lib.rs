#[cfg(any(target_os = "android", target_os = "linux"))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
mod foreground;

#[cfg(any(target_os = "android", target_os = "linux"))]
pub use self::linux::*;
#[cfg(target_os = "macos")]
pub use self::macos::*;
#[cfg(target_os = "windows")]
pub use self::windows::*;
pub use self::foreground::external_process_setup;
