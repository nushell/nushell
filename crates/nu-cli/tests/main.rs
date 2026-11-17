#![allow(clippy::unwrap_used)]

mod commands;
mod completions;
mod highlight_use;
mod last_result;
mod validate_line;

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;
