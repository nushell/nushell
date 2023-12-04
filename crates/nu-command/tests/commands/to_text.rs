use nu_test_support::nu;

#[test]
fn list_to_text() {
    let actual = nu!(r#"["foo" "bar" "baz"] | to text"#);

    // these actually have newlines between them in the real world but nu! strips newlines, grr
    assert_eq!(actual.out, "foobarbaz");
}

// the output should be the same when `to text` gets a ListStream instead of a Value::List
#[test]
fn list_stream_to_text() {
    // use `each` to convert the list to a ListStream
    let actual = nu!(r#"["foo" "bar" "baz"] | each {|i| $i} | to text"#);

    // these actually have newlines between them in the real world but nu! strips newlines, grr
    assert_eq!(actual.out, "foobarbaz");
}
