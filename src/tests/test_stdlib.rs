use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn library_loaded() -> TestResult {
    run_test(
        "help std | lines | first 1 | to text",
        "std.nu, `used` to load all standard library components",
    )
}

#[test]
fn prelude_loaded() -> TestResult {
    run_test("std help commands | where name == open | length", "1")
}

#[test]
fn not_loaded() -> TestResult {
    fail_test("log info", "")
}

#[test]
fn use_command() -> TestResult {
    run_test("use std assert; assert true; print 'it works'", "it works")
}
