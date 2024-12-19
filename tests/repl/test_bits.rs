use crate::repl::tests::{fail_test, run_test, TestResult};

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
    run_test(
        "[1 2 3 8 9 10] | bits or 2 | str join '.'",
        "3.2.3.10.11.10",
    )
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
    run_test(
        "[1 2 3 8 9 10] | bits xor 2 | into string | str join '.'",
        "3.0.1.10.11.8",
    )
}

#[test]
fn bits_shift_left() -> TestResult {
    run_test("2 | bits shl 3", "16")
}

#[test]
fn bits_shift_left_negative_operand() -> TestResult {
    fail_test("8 | bits shl -2", "positive value")
}

#[test]
fn bits_shift_left_exceeding1() -> TestResult {
    // We have no type accepting more than 64 bits so guaranteed fail
    fail_test("8 | bits shl 65", "more than the available bits")
}

#[test]
fn bits_shift_left_exceeding2() -> TestResult {
    // Explicitly specifying 2 bytes, but 16 is already the max
    fail_test(
        "8 | bits shl --number-bytes 2 16",
        "more than the available bits",
    )
}

#[test]
fn bits_shift_left_exceeding3() -> TestResult {
    // This is purely down to the current autodetect feature limiting to the smallest integer
    // type thus assuming a u8
    fail_test("8 | bits shl 9", "more than the available bits")
}

#[test]
fn bits_shift_left_negative() -> TestResult {
    run_test("-3 | bits shl 5", "-96")
}

#[test]
fn bits_shift_left_list() -> TestResult {
    run_test(
        "[1 2 7 32 9 10] | bits shl 3 | str join '.'",
        "8.16.56.0.72.80",
    )
}

#[test]
fn bits_shift_left_binary1() -> TestResult {
    run_test(
        "0x[01 30 80] | bits shl 3 | format bits",
        "00001001 10000100 00000000",
    )
}

#[test]
fn bits_shift_left_binary2() -> TestResult {
    // Whole byte case
    run_test(
        "0x[01 30 80] | bits shl 8 | format bits",
        "00110000 10000000 00000000",
    )
}

#[test]
fn bits_shift_left_binary3() -> TestResult {
    // Compared to the int case this is made inclusive of the bit count
    run_test(
        "0x[01 30 80] | bits shl 24 | format bits",
        "00000000 00000000 00000000",
    )
}

#[test]
fn bits_shift_left_binary4() -> TestResult {
    // Shifting by both bytes and bits
    run_test(
        "0x[01 30 80] | bits shl 15 | format bits",
        "01000000 00000000 00000000",
    )
}

#[test]
fn bits_shift_left_binary_exceeding() -> TestResult {
    // Compared to the int case this is made inclusive of the bit count
    fail_test("0x[01 30] | bits shl 17 | format bits", "")
}

#[test]
fn bits_shift_right() -> TestResult {
    run_test("8 | bits shr 2", "2")
}

#[test]
fn bits_shift_right_negative_operand() -> TestResult {
    fail_test("8 | bits shr -2", "positive value")
}

#[test]
fn bits_shift_right_exceeding1() -> TestResult {
    // We have no type accepting more than 64 bits so guaranteed fail
    fail_test("8 | bits shr 65", "more than the available bits")
}

#[test]
fn bits_shift_right_exceeding2() -> TestResult {
    // Explicitly specifying 2 bytes, but 16 is already the max
    fail_test(
        "8 | bits shr --number-bytes 2 16",
        "more than the available bits",
    )
}

#[test]
fn bits_shift_right_exceeding3() -> TestResult {
    // This is purely down to the current autodetect feature limiting to the smallest integer
    // type thus assuming a u8
    fail_test("8 | bits shr 9", "more than the available bits")
}

#[test]
fn bits_shift_right_negative() -> TestResult {
    run_test("-32 | bits shr 2", "-8")
}

#[test]
fn bits_shift_right_list() -> TestResult {
    run_test(
        "[12 98 7 64 900 10] | bits shr 3 | str join '.'",
        "1.12.0.8.112.1",
    )
}

#[test]
fn bits_shift_right_binary1() -> TestResult {
    run_test(
        "0x[01 30 80] | bits shr 3 | format bits",
        "00000000 00100110 00010000",
    )
}

#[test]
fn bits_shift_right_binary2() -> TestResult {
    // Whole byte case
    run_test(
        "0x[01 30 80] | bits shr 8 | format bits",
        "00000000 00000001 00110000",
    )
}

#[test]
fn bits_shift_right_binary3() -> TestResult {
    // Compared to the int case this is made inclusive of the bit count
    run_test(
        "0x[01 30 80] | bits shr 24 | format bits",
        "00000000 00000000 00000000",
    )
}

#[test]
fn bits_shift_right_binary4() -> TestResult {
    // Shifting by both bytes and bits
    run_test(
        "0x[01 30 80] | bits shr 15 | format bits",
        "00000000 00000000 00000010",
    )
}

#[test]
fn bits_shift_right_binary_exceeding() -> TestResult {
    // Compared to the int case this is made inclusive of the bit count
    fail_test(
        "0x[01 30] | bits shr 17 | format bits",
        "available bits (16)",
    )
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
    run_test(
        "[1 2 7 32 9 10] | bits rol 3 | str join '.'",
        "8.16.56.1.72.80",
    )
}

#[test]
fn bits_rotate_left_negative_operand() -> TestResult {
    fail_test("8 | bits rol -2", "positive value")
}
#[test]
fn bits_rotate_left_exceeding1() -> TestResult {
    // We have no type accepting more than 64 bits so guaranteed fail
    fail_test("8 | bits rol 65", "more than the available bits (8)")
}

#[test]
fn bits_rotate_left_exceeding2() -> TestResult {
    // This is purely down to the current autodetect feature limiting to the smallest integer
    // type thus assuming a u8
    fail_test("8 | bits rol 9", "more than the available bits (8)")
}

#[test]
fn bits_rotate_left_binary1() -> TestResult {
    run_test(
        "0x[01 30 80] | bits rol 3 | format bits",
        "00001001 10000100 00000000",
    )
}

#[test]
fn bits_rotate_left_binary2() -> TestResult {
    // Whole byte case
    run_test(
        "0x[01 30 80] | bits rol 8 | format bits",
        "00110000 10000000 00000001",
    )
}

#[test]
fn bits_rotate_left_binary3() -> TestResult {
    // Compared to the int case this is made inclusive of the bit count
    run_test(
        "0x[01 30 80] | bits rol 24 | format bits",
        "00000001 00110000 10000000",
    )
}

#[test]
fn bits_rotate_left_binary4() -> TestResult {
    // Shifting by both bytes and bits
    run_test(
        "0x[01 30 80] | bits rol 15 | format bits",
        "01000000 00000000 10011000",
    )
}

#[test]
fn bits_rotate_right() -> TestResult {
    run_test("2 | bits ror 6", "8")
}

#[test]
fn bits_rotate_right_negative() -> TestResult {
    run_test("-3 | bits ror 4", "-33")
}

#[test]
fn bits_rotate_right_list() -> TestResult {
    run_test(
        "[1 2 7 32 23 10] | bits ror 4 | str join '.'",
        "16.32.112.2.113.160",
    )
}
#[test]
fn bits_rotate_right_negative_operand() -> TestResult {
    fail_test("8 | bits ror -2", "positive value")
}

#[test]
fn bits_rotate_right_exceeding1() -> TestResult {
    // We have no type accepting more than 64 bits so guaranteed fail
    fail_test("8 | bits ror 65", "more than the available bits (8)")
}

#[test]
fn bits_rotate_right_exceeding2() -> TestResult {
    // This is purely down to the current autodetect feature limiting to the smallest integer
    // type thus assuming a u8
    fail_test("8 | bits ror 9", "more than the available bits (8)")
}

#[test]
fn bits_rotate_right_binary1() -> TestResult {
    run_test(
        "0x[01 30 80] | bits ror 3 | format bits",
        "00000000 00100110 00010000",
    )
}

#[test]
fn bits_rotate_right_binary2() -> TestResult {
    // Whole byte case
    run_test(
        "0x[01 30 80] | bits ror 8 | format bits",
        "10000000 00000001 00110000",
    )
}

#[test]
fn bits_rotate_right_binary3() -> TestResult {
    // Compared to the int case this is made inclusive of the bit count
    run_test(
        "0x[01 30 80] | bits ror 24 | format bits",
        "00000001 00110000 10000000",
    )
}

#[test]
fn bits_rotate_right_binary4() -> TestResult {
    // Shifting by both bytes and bits
    run_test(
        "0x[01 30 80] | bits ror 15 | format bits",
        "01100001 00000000 00000010",
    )
}
