#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "endian")]
pub mod endian;
#[cfg(feature = "merge")]
pub mod merge;
#[cfg(feature = "yaml")]
pub mod yaml;

#[cfg(test)]
#[allow(unused_imports)]
#[macro_use]
extern crate nu_test_support;

#[cfg(test)]
use nu_test_support::harness::main;
