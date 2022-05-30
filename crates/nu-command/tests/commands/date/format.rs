use nu_test_support::{nu, pipeline};

#[test]
fn formatter_not_valid() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        date format '%N'
        "#
        )
    );

    assert!(actual.err.contains("invalid format"));
}
