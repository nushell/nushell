#![doc = include_str!("../README.md")]
pub mod extra;
pub use extra::*;

#[cfg(test)]
#[macro_use]
extern crate nu_test_support;

#[cfg(test)]
use nu_test_support::harness::main;
