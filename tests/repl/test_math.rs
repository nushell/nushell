use crate::repl::tests::{TestResult, fail_test, run_test};

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
fn bit_shr() -> TestResult {
    run_test("16 bit-shr 1", "8")
}

#[test]
fn bit_shl() -> TestResult {
    run_test("5 bit-shl 1", "10")
}

#[test]
fn bit_shr_overflow() -> TestResult {
    fail_test("16 bit-shr 10000", "exceeds available bits")
}

#[test]
fn bit_shl_overflow() -> TestResult {
    fail_test("5 bit-shl 10000000", "exceeds available bits")
}

#[test]
fn bit_shl_neg_operand() -> TestResult {
    // This would overflow the `u32` in the right hand side to 2
    fail_test(
        "9 bit-shl -9_223_372_036_854_775_806",
        "exceeds available bits",
    )
}

#[test]
fn bit_shr_neg_operand() -> TestResult {
    // This would overflow the `u32` in the right hand side
    fail_test("9 bit-shr -2", "exceeds available bits")
}

#[test]
fn bit_shl_add() -> TestResult {
    run_test("2 bit-shl 1 + 2", "16")
}

#[test]
fn sub_bit_shr() -> TestResult {
    run_test("10 - 2 bit-shr 2", "2")
}

#[test]
fn and() -> TestResult {
    run_test("true and false", "false")
}

#[test]
fn or() -> TestResult {
    run_test("true or false", "true")
}

#[test]
fn xor_1() -> TestResult {
    run_test("false xor true", "true")
}

#[test]
fn xor_2() -> TestResult {
    run_test("true xor true", "false")
}

#[test]
fn bit_xor() -> TestResult {
    run_test("4 bit-xor 4", "0")
}

#[test]
fn bit_xor_add() -> TestResult {
    run_test("4 bit-xor 2 + 2", "0")
}

#[test]
fn bit_and() -> TestResult {
    run_test("2 bit-and 4", "0")
}

#[test]
fn bit_or() -> TestResult {
    run_test("2 bit-or 4", "6")
}

#[test]
fn bit_and_or() -> TestResult {
    run_test("2 bit-or 4 bit-and 1 + 2", "2")
}

#[test]
fn pow() -> TestResult {
    run_test("3 ** 3", "27").unwrap();
    run_test("2 ** 1 ** 2", "2").unwrap();
    run_test("2 ** 3 ** 2 ** 1 ** 5", "512").unwrap();
    run_test("1.42 ** 2 ** 1.01 ** 9", "2.1135450418757156").unwrap();
    run_test("2.571 ** 3.1 ** 2.18", "67804.81966071267").unwrap();
    run_test("2.0 ** 3.0 ** 4.0", "2417851639229258349412352.0")
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
fn not_precedence() -> TestResult {
    run_test("not false and false", "false")
}

#[test]
fn not_precedence2() -> TestResult {
    run_test("(not false) and false", "false")
}

#[test]
fn not_precedence3() -> TestResult {
    run_test("not not true and true", "true")
}

#[test]
fn not_precedence4() -> TestResult {
    run_test("not not true and not not true", "true")
}

#[test]
fn floating_add() -> TestResult {
    run_test("10.1 + 0.8", "10.9")
}

#[test]
fn precedence_of_or_groups() -> TestResult {
    run_test(r#"4 mod 3 == 0 or 5 mod 5 == 0"#, "true")
}

#[test]
fn test_filesize_op() -> TestResult {
    run_test("-5kb + 4.5kb", "-500 B")
}

#[test]
fn test_duration_op() -> TestResult {
    run_test("4min + 20sec", "4min 20sec").unwrap();
    run_test("42sec * 2", "1min 24sec").unwrap();
    run_test("(3min + 14sec) / 2", "1min 37sec").unwrap();
    run_test("(4min + 20sec) mod 69sec", "53sec")
}

#[test]
fn lt() -> TestResult {
    run_test("1 < 3", "true").unwrap();
    run_test("3 < 3", "false").unwrap();
    run_test("3 < 1", "false")
}

// Comparison operators return null if 1 side or both side is null.
// The motivation for this behaviour: JT asked the C# devs and they said this is
// the behaviour they would choose if they were starting from scratch.
#[test]
fn lt_null() -> TestResult {
    run_test("3 < null | to nuon", "null").unwrap();
    run_test("null < 3 | to nuon", "null").unwrap();
    run_test("null < null | to nuon", "null")
}

#[test]
fn lte() -> TestResult {
    run_test("1 <= 3", "true").unwrap();
    run_test("3 <= 3", "true").unwrap();
    run_test("3 <= 1", "false")
}

#[test]
fn lte_null() -> TestResult {
    run_test("3 <= null | to nuon", "null").unwrap();
    run_test("null <= 3 | to nuon", "null").unwrap();
    run_test("null <= null | to nuon", "null")
}

#[test]
fn gt() -> TestResult {
    run_test("1 > 3", "false").unwrap();
    run_test("3 > 3", "false").unwrap();
    run_test("3 > 1", "true")
}

#[test]
fn gt_null() -> TestResult {
    run_test("3 > null | to nuon", "null").unwrap();
    run_test("null > 3 | to nuon", "null").unwrap();
    run_test("null > null | to nuon", "null")
}

#[test]
fn gte() -> TestResult {
    run_test("1 >= 3", "false").unwrap();
    run_test("3 >= 3", "true").unwrap();
    run_test("3 >= 1", "true")
}

#[test]
fn gte_null() -> TestResult {
    run_test("3 >= null | to nuon", "null").unwrap();
    run_test("null >= 3 | to nuon", "null").unwrap();
    run_test("null >= null | to nuon", "null")
}
