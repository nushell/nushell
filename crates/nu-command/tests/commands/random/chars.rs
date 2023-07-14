use nu_test_support::nu;

#[test]
fn generates_chars_of_specified_length() {
    let actual = nu!(r#"
        random chars -l 15 | size | get chars
        "#);

    let result = actual.out;
    assert_eq!(result, "15");
}
