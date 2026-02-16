mod commands;
mod format_conversions;
mod sort_utils;
mod string;

use nu_experimental::EXAMPLE;
use rstest::rstest;

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;

#[rstest]
#[experimental_options(EXAMPLE = true)]
#[case::is_true(true)]
#[experimental_options(EXAMPLE = false)]
#[case::is_false(false)]
fn example_is(#[case] is: bool) {
    assert_eq!(EXAMPLE.get(), is);
}
