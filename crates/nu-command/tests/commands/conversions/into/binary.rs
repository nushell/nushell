use nu_test_support::nu;

#[test]
fn sets_stream_from_internal_command_as_binary() {
    let result = nu!("seq 1 10 | to text | into binary | describe");
    assert_eq!("binary (stream)", result.out);
}

#[test]
fn sets_stream_from_external_command_as_binary() {
    let result = nu!("^nu --testbin cococo | into binary | describe");
    assert_eq!("binary (stream)", result.out);
}
