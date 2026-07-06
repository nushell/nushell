use nu_test_support::prelude::*;

#[test]
fn byte_stream_into_bits() -> Result {
    test()
        .run("[0x[01] 0x[02 03]] | bytes collect | format bits")
        .expect_value_eq("00000001 00000010 00000011")
}

#[test]
fn byte_stream_into_bits_is_stream() -> Result {
    test()
        .run("[0x[01] 0x[02 03]] | bytes collect | format bits | describe")
        .expect_value_eq("string (stream)")
}
