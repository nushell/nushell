use nu_test_support::nu;

#[test]
fn generates_bytes_of_specified_length() {
    let actual = nu!(r#"
        random binary 16 | bytes length
        "#);

    let result = actual.out;
    assert_eq!(result, "16");
}

#[test]
fn generates_bytes_of_specified_filesize() {
    let actual = nu!(r#"
        random binary 1kb | bytes length
        "#);

    let result = actual.out;
    assert_eq!(result, "1000");
}
