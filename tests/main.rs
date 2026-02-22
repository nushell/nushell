#![allow(non_snake_case)]

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

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;
