use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn from_range() {
    let actual = nu!(r#"
        echo 1..5 | into string | to json -r
        "#);

    assert_eq!(actual.out, "[\"1\",\"2\",\"3\",\"4\",\"5\"]");
}

#[test]
fn from_number() {
    let actual = nu!(r#"
        echo 5 | into string
        "#);

    assert_eq!(actual.out, "5");
}

#[test]
fn from_float() {
    let actual = nu!(r#"
        echo 1.5 | into string
        "#);

    assert_eq!(actual.out, "1.5");
}

#[test]
fn from_boolean() {
    let actual = nu!(r#"
        echo true | into string
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn from_cell_path() {
    let actual = nu!(r#"
        $.test | into string
        "#);

    assert_eq!(actual.out, "$.test");
}

#[test]
fn from_string() {
    let actual = nu!(r#"
        echo "one" | into string
        "#);

    assert_eq!(actual.out, "one");
}

#[test]
fn from_filename() {
    Playground::setup("from_filename", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.toml",
            r#"
                [dependency]
                name = "nu"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls sample.toml | get name | into string | get 0"
        );

        assert_eq!(actual.out, "sample.toml");
    })
}

#[test]
fn from_filesize() {
    Playground::setup("from_filesize", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.toml",
            r#"
                [dependency]
                name = "nu"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls sample.toml | get size | into string | get 0"
        );

        let expected = if cfg!(windows) { "27 B" } else { "25 B" };

        assert_eq!(actual.out, expected);
    })
}

#[test]
fn from_float_correct_trailing_zeros() {
    let actual = nu!(r#"
        1.23000 | into string -d 3
        "#);

    assert!(actual.out.contains("1.230"));
}

#[test]
fn from_int_float_correct_trailing_zeros() {
    let actual = nu!(r#"
        1.00000 | into string -d 3
        "#);

    assert!(actual.out.contains("1.000"));
}

#[test]
fn from_int_float_trim_trailing_zeros() {
    let actual = nu!(r#"
        1.00000 | into string | $"($in) flat"
        "#);

    assert!(actual.out.contains("1 flat")); // "1" would match "1.0"
}

#[test]
fn from_table() {
    let actual = nu!(r#"
    echo '[{"name": "foo", "weight": 32.377}, {"name": "bar", "weight": 15.2}]'
    | from json
    | into string weight -d 2
    "#);

    assert!(actual.out.contains("32.38"));
    assert!(actual.out.contains("15.20"));
}

#[test]
fn from_nothing() {
    let actual = nu!(r#"
        null | into string
        "#);

    assert_eq!(actual.out, "");
}

#[test]
fn int_into_string() {
    let actual = nu!(r#"
        10 | into string
        "#);

    assert_eq!(actual.out, "10");
}

#[test]
fn int_into_string_decimals_0() {
    let actual = nu!(locale: "en_US.UTF-8", r#"
    10 | into string --decimals 0
    "#);

    assert_eq!(actual.out, "10");
}

#[test]
fn int_into_string_decimals_1() {
    let actual = nu!(locale: "en_US.UTF-8", r#"
    10 | into string --decimals 1
    "#);

    assert_eq!(actual.out, "10.0");
}

#[test]
fn int_into_string_decimals_10() {
    let actual = nu!(locale: "en_US.UTF-8", r#"
    10 | into string --decimals 10
    "#);

    assert_eq!(actual.out, "10.0000000000");
}

#[test]
fn int_into_string_decimals_respects_system_locale_de() {
    // Set locale to `de_DE`, which uses `,` (comma) as decimal separator
    let actual = nu!(locale: "de_DE.UTF-8", r#"
    10 | into string --decimals 1
    "#);

    assert_eq!(actual.out, "10,0");
}

#[test]
fn int_into_string_decimals_respects_system_locale_en() {
    // Set locale to `en_US`, which uses `.` (period) as decimal separator
    let actual = nu!(locale: "en_US.UTF-8", r#"
    10 | into string --decimals 1
    "#);

    assert_eq!(actual.out, "10.0");
}
