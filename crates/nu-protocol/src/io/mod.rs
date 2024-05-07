mod copy;
mod lines;

pub use copy::*;
pub use lines::*;

#[cfg(unix)]
use std::os::fd::OwnedFd;
#[cfg(windows)]
use std::os::windows::io::OwnedHandle;

#[cfg(unix)]
pub(crate) fn convert_file<T: From<OwnedFd>>(file: impl Into<OwnedFd>) -> T {
    file.into().into()
}

#[cfg(windows)]
pub(crate) fn convert_file<T: From<OwnedHandle>>(file: impl Into<OwnedHandle>) -> T {
    file.into().into()
}
