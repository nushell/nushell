extern crate nu_test_support;

mod const_;
mod hooks;
mod modules;
mod overlays;
mod parsing;
mod path;
#[cfg(feature = "plugin")]
mod plugins;
mod scope;
mod shell;
