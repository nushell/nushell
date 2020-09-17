#![allow(clippy::module_inception)]

#[cfg(feature = "rustyline-support")]
pub(crate) mod completer;
pub(crate) mod filesystem_shell;
pub(crate) mod help_shell;
#[cfg(feature = "rustyline-support")]
pub(crate) mod helper;
pub(crate) mod painter;
pub(crate) mod palette;
pub(crate) mod shell;
pub(crate) mod shell_manager;
pub(crate) mod value_shell;

#[cfg(feature = "rustyline-support")]
pub(crate) use helper::Helper;
