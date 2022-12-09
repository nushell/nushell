use nu_test_support::{nu, pipeline};

#[test]
fn sets_the_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | upsert dev-dependencies.pretty_assertions "0.7.0"
            | get dev-dependencies.pretty_assertions
        "#
    ));

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn doesnt_convert_record_to_table() {
    let actual = nu!(
        cwd: ".", r#"{a:1} | upsert a 2 | to nuon"#
    );

    assert_eq!(actual.out, "{a: 2}");
}

#[cfg(features = "inc")]
#[test]
fn sets_the_column_from_a_block_run_output() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | upsert dev-dependencies.pretty_assertions { open cargo_sample.toml | get dev-dependencies.pretty_assertions | inc --minor }
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
            {content: null}
            | upsert content { open --raw cargo_sample.toml | lines | first 5 }
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
            {content: null}
            | upsert content (open --raw cargo_sample.toml | lines | first 5)
            | get content.1
            | str contains "nu"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn uses_optional_index_argument_inserting() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[[a]; [7] [6]] | upsert b {|el ind| $ind + 1 + $el.a } | to nuon"#
    ));

    assert_eq!(actual.out, "[[a, b]; [7, 8], [6, 8]]");
}

#[test]
fn uses_optional_index_argument_updating() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[[a]; [7] [6]] | upsert a {|el ind| $ind + 1 + $el.a } | to nuon"#
    ));

    assert_eq!(actual.out, "[[a]; [8], [8]]");
}

#[test]
fn index_does_not_exist() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[1,2,3] | upsert 4 4"#
    ));

    assert!(actual.err.contains("index too large (max: 3)"));
}

#[test]
fn upsert_empty() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[] | upsert 1 1"#
    ));

    assert!(actual.err.contains("index too large (max: 0)"));
}
