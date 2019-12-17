use nu_test_support::{nu, pipeline};

#[test]
fn creates_a_new_table_with_the_new_row_given() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | edit dev-dependencies.pretty_assertions "7"
            | get dev-dependencies.pretty_assertions
            | echo $it
        "#
    ));

    assert_eq!(actual, "7");
}
