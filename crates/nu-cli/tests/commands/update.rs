use nu_test_support::{nu, pipeline};

#[test]
fn sets_the_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | update dev-dependencies.pretty_assertions "0.7.0"
            | get dev-dependencies.pretty_assertions
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "0.7.0");
}

#[cfg(features = "inc")]
#[test]
fn sets_the_column_from_a_block_run_output() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | update dev-dependencies.pretty_assertions { open cargo_sample.toml | get dev-dependencies.pretty_assertions | inc --minor }
            | get dev-dependencies.pretty_assertions
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "0.7.0");
}
