use nu_protocol::Range;
use nu_test_support::prelude::*;
use rstest::rstest;
use std::ops::Bound;

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

#[rstest]
#[case(Range::new_int(None, None, Bound::Included(2)))]
#[case(Range::new_int(0, None, Bound::Included(0)))]
#[case(Range::new_int(1, None, Bound::Included(2)))]
#[case(Range::new_int(1, None, Bound::Excluded(2)))]
#[case(Range::new_int(1, None, Bound::Unbounded))]
#[case(Range::new_int(-2, None, Bound::Included(2)))]
#[case(Range::new_int(2, None, Bound::Included(-2)))]
#[case(Range::new_int(1, 3, Bound::Included(9)))]
#[case(Range::new_int(9, 7, Bound::Included(1)))]
#[case(Range::new_float(1.0, None, Bound::Included(2.0)))]
#[case(Range::new_float(1.0, None, Bound::Included(2.0)))]
#[case(Range::new_float(1.0, None, Bound::Included(2.0)))]
#[case(Range::new_float(1.0, None, Bound::Excluded(2.0)))]
#[case(Range::new_float(-2.0, None, Bound::Included(2.0)))]
#[case(Range::new_float(2.0, None, Bound::Included(-2.0)))]
#[case(Range::new_float(-2.0, None, Bound::Excluded(2.0)))]
#[case(Range::new_float(0.1, 0.2, Bound::Included(0.5)))]
#[case(Range::new_float(-0.5, -0.4, Bound::Included(0.0)))]
fn serde_round_trip(#[case] range: Range) {
    let serialized = serde_json::to_string(&range).unwrap();
    let deserialized = serde_json::from_str(&serialized).unwrap();
    assert_eq!(range, deserialized, "{range:?}");
}
