use crate::repl::tests::{TestResult, run_test};
use nu_test_support::nu;

/// Test closure serialization to ensure that encoding and decoding within the same process and
/// within different processes works as expected.
fn run_closure_test(closure: &str, op: &str, expected: &str) -> TestResult {
    // In-process round-trip
    run_test(
        &format!("{closure} | into record | into closure | {op}"),
        expected,
    )?;

    // Cross-process round-trip
    let encoded = nu!(format!("{closure} | into record | to nuon"));
    assert!(encoded.err.is_empty(), "encoding failed: {}", encoded.err);

    run_test(&format!("{} | into closure | {op}", encoded.out), expected)
}

#[test]
fn closure_round_trip_simple() -> TestResult {
    run_closure_test("{|| 1 + 1 }", "do $in", "2")
}

#[test]
fn closure_round_trip_with_parameter() -> TestResult {
    run_closure_test("{|x| $x * 3 }", "do $in 5", "15")
}

#[test]
fn closure_round_trip_string_result() -> TestResult {
    run_closure_test(r#"{|| "hello" }"#, "do $in", "hello")
}

#[test]
fn closure_round_trip_nested_block() -> TestResult {
    run_closure_test("{ { 1 } }", "do $in | do $in", "1")
}

#[test]
fn closure_round_trip_triple_nested() -> TestResult {
    run_closure_test("{ { { 99 } } }", "do $in | do $in | do $in", "99")
}

#[test]
fn closure_round_trip_captured_closure() -> TestResult {
    run_closure_test("let inner = {|| 42 }; {|| do $inner }", "do $in", "42")
}

#[test]
fn closure_round_trip_captured_value() -> TestResult {
    run_closure_test("let x = 10; {|| $x + 5 }", "do $in", "15")
}

#[test]
fn closure_round_trip_captured_list_value() -> TestResult {
    run_closure_test("let x = [1, 2, 3]; {|| $x | get 1 }", "do $in", "2")
}

#[test]
fn closure_round_trip_captured_record_value() -> TestResult {
    run_closure_test("let x = { foo: 5 }; {|| $x | get foo }", "do $in", "5")
}
