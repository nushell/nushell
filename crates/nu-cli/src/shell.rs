#![allow(clippy::module_inception)]

pub(crate) mod completer;
pub(crate) mod filesystem_shell;
pub(crate) mod help_shell;
pub(crate) mod helper;
pub(crate) mod palette;
pub(crate) mod shell;
pub(crate) mod shell_manager;
pub(crate) mod value_shell;

pub(crate) use helper::Helper;
