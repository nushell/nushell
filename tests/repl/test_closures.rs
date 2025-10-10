use crate::repl::tests::{TestResult, fail_test, run_test};

#[test]
fn do_no_argument() -> TestResult {
    let input = "do { 42 }";
    let expected = "42";
    run_test(input, expected)
}

#[test]
fn do_one_argument() -> TestResult {
    let input = "do {|x| $x } 42";
    let expected = "42";
    run_test(input, expected)
}

#[test]
fn do_missing_argument() -> TestResult {
    let input = "do {|x| $x}";
    let expected = "nu::shell::missing_parameter";
    fail_test(input, expected)
}

#[test]
fn do_typed_argument() -> TestResult {
    let input = "do {|x: int| $x} 42";
    let expected = "42";
    run_test(input, expected)
}

#[test]
fn do_type_mismatch() -> TestResult {
    let input = "do {|x: int| $x} 4.2";
    let expected = "nu::shell::cant_convert";
    fail_test(input, expected)
}

#[test]
fn do_optional_argument() -> TestResult {
    let input = "do {|x?| $x | describe}";
    let expected = "nothing";
    run_test(input, expected)
}

#[test]
fn do_variable_argument() -> TestResult {
    let input = "do {|...rest| $rest} 1 2 | to nuon";
    let expected = "[1, 2]";
    run_test(input, expected)
}

#[test]
fn default() -> TestResult {
    let input = "null | default { 42 }";
    let expected = "42";
    run_test(input, expected)
}

#[test]
fn default_optional_argument() -> TestResult {
    let input = "{} | default {|x?| if $x == null { 'no x' } else { $x } } foo | to nuon";
    let expected = "{foo: \"no x\"}";
    run_test(input, expected)
}
