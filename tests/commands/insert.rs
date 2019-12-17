use nu_test_support::{nu, pipeline};

#[test]
fn insert_plugin() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | insert dev-dependencies.newdep "1"
            | get dev-dependencies.newdep
            | echo $it
        "#
    ));

    assert_eq!(actual, "1");
}
