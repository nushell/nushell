use crate::repl::tests::{TestResult, fail_test, run_test_std};

#[test]
fn not_loaded() -> TestResult {
    fail_test("log info", "")
}

#[test]
fn use_command() -> TestResult {
    run_test_std("use std/assert; assert true; print 'it works'", "it works")
}
