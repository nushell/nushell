use nu_protocol::Range;
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case("1..2")]
fn engine_eq_from_str(#[case] input: &str) -> Result {
    let mut tester = test();
    let () = tester.run("def range [r: range] { $r }")?;
    let via_engine: Range = tester.run(format!("range {input}"))?;
    let via_from_str: Range = input.parse().expect("range parses");
    assert_eq!(via_engine, via_from_str);
    Ok(())
}