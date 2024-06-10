mod cpu;
mod disks;
mod host;
mod mem;
mod net;
mod sys_;
mod temp;
mod users;

pub use cpu::SysCpu;
pub use disks::SysDisks;
pub use host::SysHost;
pub use mem::SysMem;
pub use net::SysNet;
pub use sys_::Sys;
pub use temp::SysTemp;
pub use users::SysUsers;

fn trim_cstyle_null(s: impl AsRef<str>) -> String {
    s.as_ref().trim_matches('\0').into()
}
