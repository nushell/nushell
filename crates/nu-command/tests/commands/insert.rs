use nu_test_support::{nu, pipeline};

#[test]
fn insert_the_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | insert dev-dependencies.new_assertions "0.7.0"
            | get dev-dependencies.new_assertions
        "#
    ));

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn insert_the_column_conflict() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | insert dev-dependencies.pretty_assertions "0.7.0"
        "#
    ));

    assert!(actual.err.contains("column already exists"));
}
