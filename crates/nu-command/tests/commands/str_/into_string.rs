use nu_test_support::playground::{Dirs, Playground};
use nu_test_support::{nu, pipeline};

#[test]
fn from_range() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1..5 | into string | to json
        "#
        )
    );

    assert_eq!(actual.out, "[\"1\",\"2\",\"3\",\"4\",\"5\"]");
}

#[test]
fn from_number() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 5 | into string
        "#
        )
    );

    assert_eq!(actual.out, "5");
}

#[test]
fn from_decimal() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1.5 | into string
        "#
        )
    );

    assert_eq!(actual.out, "1.5");
}

#[test]
fn from_boolean() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo true | into string
        "#
        )
    );

    assert_eq!(actual.out, "true");
}

#[test]
fn from_string() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "one" | into string
        "#
        )
    );

    assert_eq!(actual.out, "one");
}

#[test]
fn from_filename() {
    Playground::setup("from_filename", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.toml",
            r#"
                [dependency]
                name = "nu"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls sample.toml | get name | into string"
        );

        assert_eq!(actual.out, "sample.toml");
    })
}

#[test]
fn from_filesize() {
    Playground::setup("from_filesize", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.toml",
            r#"
                [dependency]
                name = "nu"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls sample.toml | get size | into string"
        );

        assert_eq!(actual.out, "25 B");
    })
}

#[test]
fn from_decimal_correct_trailing_zeros() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 1.23000 | into string -d 3
        "#
    ));

    assert!(actual.out.contains("1.230"));
}

#[test]
fn from_int_decimal_correct_trailing_zeros() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 1.00000 | into string -d 3
        "#
    ));

    assert!(actual.out.contains("1.000"));
}

#[test]
fn from_int_decimal_trim_trailing_zeros() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 1.00000 | into string | format "{$it} flat"
        "#
    ));

    assert!(actual.out.contains("1 flat")); // "1" would match "1.0"
}

#[test]
fn from_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo '[{"name": "foo", "weight": 32.377}, {"name": "bar", "weight": 15.2}]'
        | from json
        | into string weight -d 2
        "#
    ));

    assert!(actual.out.contains("32.38"));
    assert!(actual.out.contains("15.20"));
}
