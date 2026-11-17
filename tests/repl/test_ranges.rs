use crate::repl::tests::{TestResult, fail_test, run_test};
use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn int_in_inc_range() -> TestResult {
    run_test("1 in -4..9.42", "true")
}

#[test]
fn int_in_dec_range() -> TestResult {
    run_test("1 in 9..-4.42", "true")
}

#[test]
fn int_in_exclusive_range() -> TestResult {
    run_test("3 in 0..<3", "false")
}

#[test]
fn float_in_inc_range() -> TestResult {
    run_test("1.58 in -4.42..9", "true")
}

#[test]
fn float_in_dec_range() -> TestResult {
    run_test("1.42 in 9.42..-4.42", "true")
}

#[test]
fn non_number_in_range() -> TestResult {
    fail_test("'a' in 1..3", "nu::parser::operator_incompatible_types")
}

#[test]
fn float_not_in_inc_range() -> TestResult {
    run_test("1.4 not-in 2..9.42", "true")
}

#[test]
fn range_and_reduction() -> TestResult {
    run_test("1..6..36 | math sum", "148")
}

#[test]
fn zip_ranges() -> TestResult {
    run_test("1..3 | zip 4..6 | get 2.1", "6")
}

#[test]
fn int_in_stepped_range() -> TestResult {
    run_test("7 in 1..3..15", "true")?;
    run_test("7 in 1..3..=15", "true")
}

#[test]
fn int_in_unbounded_stepped_range() -> TestResult {
    run_test("1000001 in 1..3..", "true")
}

#[test]
fn int_not_in_unbounded_stepped_range() -> TestResult {
    run_test("2 in 1..3..", "false")
}

#[test]
fn float_in_stepped_range() -> TestResult {
    run_test("5.5 in 1..1.5..10", "true")
}

#[test]
fn float_in_unbounded_stepped_range() -> TestResult {
    run_test("100.5 in 1..1.5..", "true")
}

#[test]
fn float_not_in_unbounded_stepped_range() -> TestResult {
    run_test("2.1 in 1.2..3..", "false")
}

/// A float range includes its end with `..` and excludes it with `..<`, like an int range.
#[rstest]
#[case::inclusive_end("2.0 in 1.0..2.0", true)]
#[case::exclusive_end("2.0 in 1.0..<2.0", false)]
#[case::int_in_float_range("2 in 1.0..2.0", true)]
#[case::float_in_int_range("2.0 in 1..2", true)]
#[case::float_past_end("2.5 in 1.0..2.0", false)]
#[case::decreasing_inclusive_end("0.0 in 2.0..1.0..0.0", true)]
#[case::decreasing_exclusive_end("0.0 in 2.0..1.0..<0.0", false)]
#[case::match_pattern("match 2.0 { 1..2 => true, _ => false }", true)]
fn float_range_end_bound(#[case] code: &str, #[case] expected: bool) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case("1..=3..", "expected number")]
#[case("..=3..=15", "expected number")]
#[case("..=(..", "expected `)`")]
#[case("..=()..", "expected at least one range bound")]
#[case("..=..", "expected at least one range bound")]
#[test]
fn bad_range_syntax(#[case] input: &str, #[case] expect: &str) -> TestResult {
    fail_test(&format!("def foo [r: range] {{}}; foo {input}"), expect)
}
