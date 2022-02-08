use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn int_in_inc_range() -> TestResult {
    run_test(r#"1 in -4..9.42"#, "true")
}

#[test]
fn int_in_dec_range() -> TestResult {
    run_test(r#"1 in 9.42..-4"#, "true")
}

#[test]
fn int_in_exclusive_range() -> TestResult {
    run_test(r#"3 in 0..<3"#, "false")
}

#[test]
fn non_number_in_range() -> TestResult {
    fail_test(r#"'a' in 1..3"#, "mismatched for operation")
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
