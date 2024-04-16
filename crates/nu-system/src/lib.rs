mod foreground;
#[cfg(any(target_os = "android", target_os = "linux"))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
pub mod os_info;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(unix)]
pub use self::foreground::stdin_fd;
pub use self::foreground::{ForegroundChild, ForegroundGuard};
#[cfg(any(target_os = "android", target_os = "linux"))]
pub use self::linux::*;
#[cfg(target_os = "macos")]
pub use self::macos::*;
#[cfg(target_os = "windows")]
pub use self::windows::*;
