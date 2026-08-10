#![allow(clippy::unwrap_used)]

mod commands;
mod completions;
mod last_result;

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;
