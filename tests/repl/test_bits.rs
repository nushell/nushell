use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::and("2 | bits and 4", 0)]
#[case::and_negative("-3 | bits and 5", 5)]
#[case::or("2 | bits or 3", 3)]
#[case::or_negative("-3 | bits or 5", -3)]
#[case::xor("2 | bits xor 3", 1)]
#[case::xor_negative("-3 | bits xor 5", -8)]
#[case::shift_left("2 | bits shl 3", 16)]
#[case::shift_left_defaults_to_eight_bytes("1 | bits shl 20", 1_048_576)]
#[case::shift_left_negative("-3 | bits shl 5", -96)]
#[case::shift_right("8 | bits shr 2", 2)]
#[case::shift_right_defaults_to_eight_bytes("8 | bits shr 9", 0)]
#[case::shift_right_negative("-32 | bits shr 2", -8)]
#[case::rotate_left("2 | bits rol 3", 16)]
#[case::rotate_left_negative("-3 | bits rol 5", -65)]
#[case::rotate_right("2 | bits ror 6", 8)]
#[case::rotate_right_negative("-3 | bits ror 4", -33)]
fn bits_integer(#[case] code: &str, #[case] expected: i64) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::and("[1 2 3 8 9 10] | bits and 2", [0, 2, 2, 0, 0, 2])]
#[case::or("[1 2 3 8 9 10] | bits or 2", [3, 2, 3, 10, 11, 10])]
#[case::xor("[1 2 3 8 9 10] | bits xor 2", [3, 0, 1, 10, 11, 8])]
#[case::shift_left("[1 2 7 32 9 10] | bits shl 3", [8, 16, 56, 256, 72, 80])]
#[case::shift_right("[12 98 7 64 900 10] | bits shr 3", [1, 12, 0, 8, 112, 1])]
#[case::rotate_left("[1 2 7 32 9 10] | bits rol 3", [8, 16, 56, 1, 72, 80])]
#[case::rotate_right("[1 2 7 32 23 10] | bits ror 4", [16, 32, 112, 2, 113, 160])]
fn bits_list(#[case] code: &str, #[case] expected: [i64; 6]) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::shift_left(
    "0x[01 30 80] | bits shl 3 | format bits",
    "00001001 10000100 00000000"
)]
#[case::shift_left_whole_byte(
    "0x[01 30 80] | bits shl 8 | format bits",
    "00110000 10000000 00000000"
)]
#[case::shift_left_all_bits(
    "0x[01 30 80] | bits shl 24 | format bits",
    "00000000 00000000 00000000"
)]
#[case::shift_left_bytes_and_bits(
    "0x[01 30 80] | bits shl 15 | format bits",
    "01000000 00000000 00000000"
)]
#[case::shift_right(
    "0x[01 30 80] | bits shr 3 | format bits",
    "00000000 00100110 00010000"
)]
#[case::shift_right_whole_byte(
    "0x[01 30 80] | bits shr 8 | format bits",
    "00000000 00000001 00110000"
)]
#[case::shift_right_all_bits(
    "0x[01 30 80] | bits shr 24 | format bits",
    "00000000 00000000 00000000"
)]
#[case::shift_right_bytes_and_bits(
    "0x[01 30 80] | bits shr 15 | format bits",
    "00000000 00000000 00000010"
)]
#[case::rotate_left(
    "0x[01 30 80] | bits rol 3 | format bits",
    "00001001 10000100 00000000"
)]
#[case::rotate_left_whole_byte(
    "0x[01 30 80] | bits rol 8 | format bits",
    "00110000 10000000 00000001"
)]
#[case::rotate_left_all_bits(
    "0x[01 30 80] | bits rol 24 | format bits",
    "00000001 00110000 10000000"
)]
#[case::rotate_left_bytes_and_bits(
    "0x[01 30 80] | bits rol 15 | format bits",
    "01000000 00000000 10011000"
)]
#[case::rotate_right(
    "0x[01 30 80] | bits ror 3 | format bits",
    "00000000 00100110 00010000"
)]
#[case::rotate_right_whole_byte(
    "0x[01 30 80] | bits ror 8 | format bits",
    "10000000 00000001 00110000"
)]
#[case::rotate_right_all_bits(
    "0x[01 30 80] | bits ror 24 | format bits",
    "00000001 00110000 10000000"
)]
#[case::rotate_right_bytes_and_bits(
    "0x[01 30 80] | bits ror 15 | format bits",
    "01100001 00000000 00000010"
)]
fn bits_binary(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::shift_left_negative_operand("8 | bits shl -2", "NeedsPositiveValue")]
#[case::shift_left_exceeding("8 | bits shl 65", "more than the available bits")]
#[case::shift_left_exceeding_number_bytes(
    "8 | bits shl --number-bytes 2 16",
    "more than the available bits"
)]
#[case::shift_left_binary_exceeding("0x[01 30] | bits shl 17 | format bits", "")]
#[case::shift_right_negative_operand("8 | bits shr -2", "NeedsPositiveValue")]
#[case::shift_right_exceeding("8 | bits shr 65", "more than the available bits")]
#[case::shift_right_exceeding_number_bytes(
    "8 | bits shr --number-bytes 2 16",
    "more than the available bits"
)]
#[case::shift_right_binary_exceeding(
    "0x[01 30] | bits shr 17 | format bits",
    "available bits (16)"
)]
#[case::rotate_left_negative_operand("8 | bits rol -2", "NeedsPositiveValue")]
#[case::rotate_left_exceeding("8 | bits rol 65", "more than the available bits (8)")]
#[case::rotate_left_autodetect_exceeding("8 | bits rol 9", "more than the available bits (8)")]
#[case::rotate_right_negative_operand("8 | bits ror -2", "NeedsPositiveValue")]
#[case::rotate_right_exceeding("8 | bits ror 65", "more than the available bits (8)")]
#[case::rotate_right_autodetect_exceeding("8 | bits ror 9", "more than the available bits (8)")]
fn bits_failures(#[case] code: &str, #[case] expected: &str) -> Result {
    let error = test().run(code).expect_shell_error()?;

    if !expected.is_empty() {
        assert_contains(expected, format!("{error:?}"));
    }

    Ok(())
}
