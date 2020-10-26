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
            | first 1
            | split column "="
            | get Column1
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
            | to json
        "#
    ));

    assert_eq!(actual.out, "[8194,4]");
}

#[test]
fn lines_multi_value_split() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample-simple.json
            | get first second
            | lines
            | count
        "#
    ));

    assert_eq!(actual.out, "6");
}
