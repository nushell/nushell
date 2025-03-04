#![doc = include_str!("../README.md")]
mod exit_status;
mod foreground;
mod util;

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(any(target_os = "android", target_os = "linux"))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
mod netbsd;
pub mod os_info;
#[cfg(target_os = "windows")]
mod windows;

pub use self::exit_status::ExitStatus;
#[cfg(unix)]
pub use self::foreground::stdin_fd;
pub use self::foreground::{
    ForegroundChild, ForegroundGuard, ForegroundWaitStatus, UnfreezeHandle,
};

pub use self::util::*;

#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;
#[cfg(any(target_os = "android", target_os = "linux"))]
pub use self::linux::*;
#[cfg(target_os = "macos")]
pub use self::macos::*;
#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
pub use self::netbsd::*;
#[cfg(target_os = "windows")]
pub use self::windows::*;
