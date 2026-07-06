#![allow(clippy::unwrap_used)]

mod cell_path;
mod into_config;
mod pipeline;
mod range;
mod test_config;
mod test_pipeline_data;
mod test_signature;
mod test_value;

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;

use nu_test_support::prelude::*;
use rstest::*;

#[rstest]
#[case::no_need()]
#[deps(NU)]
#[case::needs_nu()]
#[deps(NU_PLUGIN_EXAMPLE)]
#[case::needs_plugin()]
#[nu_test_support::test]
fn showcase() {}