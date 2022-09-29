use nu_test_support::{nu, pipeline};

#[test]
fn lines() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml -r
            | lines
            | skip while $it != "[dependencies]"
            | skip 1
            | first
            | split column "="
            | get column1.0
            | str trim
        "#
    ));

    assert_eq!(actual.out, "rustyline");
}

#[test]
fn lines_proper_buffering() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open lines_test.txt -r
            | lines
            | str length
            | to json -r
        "#
    ));

    assert_eq!(actual.out, "[8193,3]");
}

#[test]
fn lines_multi_value_split() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample-simple.json
            | get first second
            | lines
            | length
        "#
    ));

    assert_eq!(actual.out, "6");
}
