use nu_test_support::{nu, pipeline};

#[test]
fn sets_the_column_from_a_block_run_output() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | insert dev-dependencies.newdep "1"
            | get dev-dependencies.newdep
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn sets_the_column_from_a_block_full_stream_output() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            wrap _
            | insert content { open --raw cargo_sample.toml | lines | first 5 }
            | get content.1
            | str contains "nu"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn sets_the_column_from_an_invocation() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            wrap content
            | insert content $(open --raw cargo_sample.toml | lines | first 5)
            | get content.1
            | str contains "nu"
        "#
    ));

    assert_eq!(actual.out, "true");
}
