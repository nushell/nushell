use crate::repl::tests::{TestResult, fail_test, run_test};
use rstest::rstest;

#[test]
fn int_in_inc_range() -> TestResult {
    run_test(r#"1 in -4..9.42"#, "true")
}

#[test]
fn int_in_dec_range() -> TestResult {
    run_test(r#"1 in 9..-4.42"#, "true")
}

#[test]
fn int_in_exclusive_range() -> TestResult {
    run_test(r#"3 in 0..<3"#, "false")
}

#[test]
fn float_in_inc_range() -> TestResult {
    run_test(r#"1.58 in -4.42..9"#, "true")
}

#[test]
fn float_in_dec_range() -> TestResult {
    run_test(r#"1.42 in 9.42..-4.42"#, "true")
}

#[test]
fn non_number_in_range() -> TestResult {
    fail_test(r#"'a' in 1..3"#, "nu::parser::operator_incompatible_types")
}

#[test]
fn float_not_in_inc_range() -> TestResult {
    run_test(r#"1.4 not-in 2..9.42"#, "true")
}

#[test]
fn range_and_reduction() -> TestResult {
    run_test(r#"1..6..36 | math sum"#, "148")
}

#[test]
fn zip_ranges() -> TestResult {
    run_test(r#"1..3 | zip 4..6 | get 2.1"#, "6")
}

#[test]
fn int_in_stepped_range() -> TestResult {
    run_test(r#"7 in 1..3..15"#, "true")?;
    run_test(r#"7 in 1..3..=15"#, "true")
}

#[test]
fn int_in_unbounded_stepped_range() -> TestResult {
    run_test(r#"1000001 in 1..3.."#, "true")
}

#[test]
fn int_not_in_unbounded_stepped_range() -> TestResult {
    run_test(r#"2 in 1..3.."#, "false")
}

#[test]
fn float_in_stepped_range() -> TestResult {
    run_test(r#"5.5 in 1..1.5..10"#, "true")
}

#[test]
fn float_in_unbounded_stepped_range() -> TestResult {
    run_test(r#"100.5 in 1..1.5.."#, "true")
}

#[test]
fn float_not_in_unbounded_stepped_range() -> TestResult {
    run_test(r#"2.1 in 1.2..3.."#, "false")
}

#[rstest]
#[case("1..=3..", "expected number")]
#[case("..=3..=15", "expected number")]
#[case("..=(..", "expected closing )")]
#[case("..=()..", "expected at least one range bound")]
#[case("..=..", "expected at least one range bound")]
#[test]
fn bad_range_syntax(#[case] input: &str, #[case] expect: &str) -> TestResult {
    fail_test(&format!("def foo [r: range] {{}}; foo {input}"), expect)
}
