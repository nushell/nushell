use nu_test_support::{nu, pipeline};

#[test]
fn binary_skip() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_data.ods --raw |
            skip 2 |
            take 2 |
            into int
        "#
    ));

    assert_eq!(actual.out, "772");
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!(cwd: ".", pipeline("1 | skip 2"));

    assert!(actual.err.contains("only_supports_this_input_type"));
}
