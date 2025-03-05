use nu_test_support::nu;

#[test]
fn generates_chars_of_specified_length() {
    let actual = nu!(r#"
        random chars --length 15 | str stats | get chars.0
        "#);

    let result = actual.out;
    assert_eq!(result, "15");
}

#[test]
fn generates_chars_of_specified_filesize() {
    let actual = nu!(r#"
        random chars --length 1kb | str stats | get bytes.0
        "#);

    let result = actual.out;
    assert_eq!(result, "1000");
}
