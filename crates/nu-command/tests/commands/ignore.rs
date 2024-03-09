use nu_test_support::nu;

#[test]
fn ignore_still_causes_stream_to_be_consumed_fully() {
    let result = nu!("[foo bar] | each { |val| print $val; $val } | ignore");
    assert_eq!("foobar", result.out);
}
