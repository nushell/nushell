use nu_test_support::nu;

#[test]
fn byte_stream_into_bits() {
    let result = nu!("[0x[01] 0x[02 03]] | bytes collect | format bits");
    assert_eq!("00000001 00000010 00000011", result.out);
}

#[test]
fn byte_stream_into_bits_is_stream() {
    let result = nu!("[0x[01] 0x[02 03]] | bytes collect | format bits | describe");
    assert_eq!("string (stream)", result.out);
}
