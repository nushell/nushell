use nu_test_support::nu;

#[test]
fn source_file_relative_to_file() {
    let actual = nu!(cwd: "tests/eval", r#"
        {x: 1, x: 2}
        "#);

    assert!(actual.err.contains("redefined"));
}
