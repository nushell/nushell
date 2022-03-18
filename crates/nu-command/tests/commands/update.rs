use nu_test_support::{nu, pipeline};

#[test]
fn sets_the_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | update dev-dependencies.pretty_assertions "0.7.0"
            | get dev-dependencies.pretty_assertions
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
        "#
    ));

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn sets_the_column_from_a_block_full_stream_output() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            wrap content
            | update content { open --raw cargo_sample.toml | lines | first 5 }
            | get content.1
            | str contains "nu"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn sets_the_column_from_a_subexpression() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            wrap content
            | update content (open --raw cargo_sample.toml | lines | first 5)
            | get content.1
            | str contains "nu"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn upsert_column_missing() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | update dev-dependencies.new_assertions "0.7.0"
        "#
    ));

    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn update_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [1, 2, 3] | update 1 abc | to json -r
        "#
    ));

    assert_eq!(actual.out, r#"[1,"abc",3]"#);
}

#[test]
fn update_past_end_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [1, 2, 3] | update 5 abc | to json -r
        "#
    ));

    assert!(actual.err.contains("too large"));
}
