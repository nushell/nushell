use nu_protocol::Range;
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case("..2")]
#[case("0..0")]
#[case("1..2")]
#[case("1..<2")]
#[case("1..")]
#[case("-2..2")]
#[case("2..-2")]
#[case("1..3..9")]
#[case("9..7..1")]
#[case("1.0..2.0")]
#[case("1..2.0")]
#[case("1.0..2")]
#[case("1.0..<2.0")]
#[case("-2.0..2.0")]
#[case("2.0..-2.0")]
#[case("-2.0..<2")]
#[case("0.1..0.2..0.5")]
#[case("-0.5..-0.4..0.0")]
fn engine_eq_from_str(#[case] input: &str) -> Result {
    let mut tester = test();
    let via_engine: Range = tester.run(input)?;
    let via_from_str: Range = input.parse().expect("range parses");
    assert_eq!(via_engine, via_from_str, "{input}");
    Ok(())
}
