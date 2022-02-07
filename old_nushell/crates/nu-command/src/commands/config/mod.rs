pub mod clear;
pub mod command;
pub mod get;
pub mod path;
pub mod remove;
pub mod set;
pub mod set_into;

pub use clear::SubCommand as ConfigClear;
pub use command::Command as Config;
pub use get::SubCommand as ConfigGet;
pub use path::SubCommand as ConfigPath;
pub use remove::SubCommand as ConfigRemove;
pub use set::SubCommand as ConfigSet;
pub use set_into::SubCommand as ConfigSetInto;

use nu_errors::ShellError;

pub fn err_no_global_cfg_present() -> ShellError {
    ShellError::untagged_runtime_error("No global config found!")
}
