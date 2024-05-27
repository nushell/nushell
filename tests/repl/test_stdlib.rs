use crate::repl::tests::{fail_test, run_test_std, TestResult};

#[test]
fn library_loaded() -> TestResult {
    run_test_std("scope modules | where name == 'std' | length", "1")
}

#[test]
fn prelude_loaded() -> TestResult {
    run_test_std("shells | length", "1")
}

#[test]
fn not_loaded() -> TestResult {
    fail_test("log info", "")
}

#[test]
fn use_command() -> TestResult {
    run_test_std("use std assert; assert true; print 'it works'", "it works")
}
