use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn bits_and() -> TestResult {
    run_test("2 | bits and 4", "0")
}

#[test]
fn bits_and_negative() -> TestResult {
    run_test("-3 | bits and 5", "5")
}

#[test]
fn bits_and_list() -> TestResult {
    run_test("[1 2 3 8 9 10] | bits and 2 | str join '.'", "0.2.2.0.0.2")
}

#[test]
fn bits_or() -> TestResult {
    run_test("2 | bits or 3", "3")
}

#[test]
fn bits_or_negative() -> TestResult {
    run_test("-3 | bits or 5", "-3")
}

#[test]
fn bits_or_list() -> TestResult {
    run_test("[1 2 3 8 9 10] | bits or 2 | str join '.'", "3.2.3.10.11.10")
}

#[test]
fn bits_xor() -> TestResult {
    run_test("2 | bits xor 3", "1")
}

#[test]
fn bits_xor_negative() -> TestResult {
    run_test("-3 | bits xor 5", "-8")
}

#[test]
fn bits_xor_list() -> TestResult {
    run_test("[1 2 3 8 9 10] | bits xor 2 | str join '.'", "3.0.1.10.11.8")
}

#[test]
fn bits_shift_left() -> TestResult {
    run_test("2 | bits shl 3", "16")
}

#[test]
fn bits_shift_left_negative() -> TestResult {
    run_test("-3 | bits shl 5", "-96")
}

#[test]
fn bits_shift_left_list() -> TestResult {
    run_test("[1 2 7 32 9 10] | bits shl 3 | str join '.'", "8.16.56.256.72.80")
}

#[test]
fn bits_shift_right() -> TestResult {
    run_test("8 | bits shr 2", "2")
}

#[test]
fn bits_shift_right_negative() -> TestResult {
    run_test("-32 | bits shr 2", "-8")
}

#[test]
fn bits_shift_right_list() -> TestResult {
    run_test("[12 98 7 64 900 10] | bits shr 3 | str join '.'", "1.12.0.8.112.1")
}

#[test]
fn bits_rotate_left() -> TestResult {
    run_test("2 | bits rol 3", "16")
}

#[test]
fn bits_rotate_left_negative() -> TestResult {
    run_test("-3 | bits rol 5", "-65")
}

#[test]
fn bits_rotate_left_list() -> TestResult {
    run_test("[1 2 7 32 9 10] | bits rol 3 | str join '.'", "8.16.56.256.72.80")
}

#[test]
fn bits_rotate_right() -> TestResult {
    run_test("2 | bits ror 62", "8")
}

#[test]
fn bits_rotate_right_negative() -> TestResult {
    run_test("-3 | bits ror 60", "-33")
}

#[test]
fn bits_rotate_right_list() -> TestResult {
    run_test("[1 2 7 32 23 10] | bits ror 60 | str join '.'", "16.32.112.512.368.160")
}
