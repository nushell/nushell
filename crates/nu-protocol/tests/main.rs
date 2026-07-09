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
use rstest::rstest;

#[rstest]
#[case::abc("abc")]
#[case::hello("hello")]
#[case::world("world")]
#[case::foo("foo")]
#[case::bar("bar")]
#[case::baz("baz")]
#[case::test("test")]
#[case::rust("rust")]
#[case::nu("nu")]
#[case::plugin("plugin")]
#[case::example("example")]
#[case::echo("echo")]
#[case::input("input")]
#[case::output("output")]
#[case::string("string")]
#[case::data("data")]
#[case::case_one("case one")]
#[case::case_two("case two")]
#[case::lowercase("lowercase")]
#[case::uppercase("UPPERCASE")]
#[case::mixedcase("MixedCase")]
#[case::numbers("12345")]
#[case::symbols("abc_123")]
#[case::short("x")]
#[case::longer("this is a longer string")]
#[nu_test_support::test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn needs_plugin(#[case] input: &str) -> Result {
    let echo: String = test().run_with_data("example echo", input)?;
    assert_eq!(echo, input);
    Ok(())
}
