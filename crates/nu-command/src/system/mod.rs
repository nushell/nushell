mod complete;
mod exec;
mod nu_check;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "macos",
    target_os = "windows"
))]
mod ps;
#[cfg(windows)]
mod registry;
#[cfg(windows)]
mod registry_query;
mod run_external;
mod sys;
mod uname;
mod which_;

pub use complete::Complete;
pub use exec::Exec;
pub use nu_check::NuCheck;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "macos",
    target_os = "windows"
))]
pub use ps::Ps;
#[cfg(windows)]
pub use registry::Registry;
#[cfg(windows)]
pub use registry_query::RegistryQuery;
pub use run_external::{External, command_not_found, eval_external_arguments, which};
pub use sys::*;
pub use uname::UName;
pub use which_::Which;
