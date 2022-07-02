use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn add_simple() -> TestResult {
    run_test("3 + 4", "7")
}

#[test]
fn add_simple2() -> TestResult {
    run_test("3 + 4 + 9", "16")
}

#[test]
fn broken_math() -> TestResult {
    fail_test("3 + ", "incomplete")
}

#[test]
fn modulo1() -> TestResult {
    run_test("5 mod 2", "1")
}

#[test]
fn modulo2() -> TestResult {
    run_test("5.25 mod 2", "1.25")
}

#[test]
fn bshr() -> TestResult {
    run_test("16 bshr 1", "8")
}

#[test]
fn bshl() -> TestResult {
    run_test("5 bshl 1", "10")
}

#[test]
fn bshl_add() -> TestResult {
    run_test("2 bshl 1 + 2", "16")
}

#[test]
fn sub_bshr() -> TestResult {
    run_test("10 - 2 bshr 2", "2")
}

#[test]
fn and() -> TestResult {
    run_test("true && false", "false")
}

#[test]
fn or() -> TestResult {
    run_test("true || false", "true")
}

#[test]
fn band() -> TestResult {
    run_test("2 band 4", "0")
}

#[test]
fn bor() -> TestResult {
    run_test("2 bor 4", "6")
}

#[test]
fn bit_and_or() -> TestResult {
    run_test("2 bor 4 band 1 + 2", "2")
}

#[test]
fn pow() -> TestResult {
    run_test("3 ** 3", "27")
}

#[test]
fn contains() -> TestResult {
    run_test("'testme' =~ 'test'", "true")
}

#[test]
fn not_contains() -> TestResult {
    run_test("'testme' !~ 'test'", "false")
}

#[test]
fn floating_add() -> TestResult {
    run_test("10.1 + 0.8", "10.9")
}

#[test]
fn precedence_of_or_groups() -> TestResult {
    run_test(r#"4 mod 3 == 0 || 5 mod 5 == 0"#, "true")
}
