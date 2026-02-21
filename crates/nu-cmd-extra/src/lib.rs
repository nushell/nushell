#![doc = include_str!("../README.md")]
mod example_test;
pub mod extra;
pub use extra::*;

#[cfg(test)]
pub use example_test::{test_examples, test_examples_with_commands};

#[cfg(test)]
#[macro_use]
extern crate nu_test_support;

#[cfg(test)]
use nu_test_support::harness::main;
