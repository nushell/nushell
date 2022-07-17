extern crate nu_test_support;

mod hooks;
mod nu_repl;
mod overlays;
mod parsing;
mod path;
#[cfg(feature = "plugin")]
mod plugins;
mod scope;
mod shell;
