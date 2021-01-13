#![allow(clippy::module_inception)]

#[cfg(feature = "rustyline-support")]
pub(crate) mod completer;
#[cfg(feature = "rustyline-support")]
pub(crate) mod helper;

#[cfg(feature = "rustyline-support")]
pub(crate) use helper::Helper;
