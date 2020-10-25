use nu_test_support::{nu, pipeline};

#[test]
fn invalid_column_reject_error() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            ls | reject dog
        "#
    ));

    assert_eq!(actual.out, "");
}
