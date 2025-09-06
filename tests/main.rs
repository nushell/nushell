extern crate nu_test_support;

mod const_;
mod eval;
mod hooks;
mod integration;
mod modules;
mod overlays;
mod parsing;
mod path;
#[cfg(feature = "plugin")]
mod plugin_persistence;
#[cfg(feature = "plugin")]
mod plugins;
mod repl;
mod scope;
mod shell;
