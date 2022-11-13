use nu_test_support::{nu, pipeline};

#[test]
fn fails_when_first_arg_is_multiple_chars() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        seq char aa z
        "#
    ));

    assert!(actual.err.contains("should be 1 character long"));
}

#[test]
fn fails_when_second_arg_is_multiple_chars() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        seq char a zz
        "#
    ));

    assert!(actual.err.contains("should be 1 character long"));
}
