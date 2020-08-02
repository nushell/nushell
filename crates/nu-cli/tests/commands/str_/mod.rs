mod collect;

use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn trims() {
    Playground::setup("str_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu "
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str trim dependency.name | get dependency.name | echo $it"
        );

        assert_eq!(actual.out, "nu");
    })
}

#[test]
fn error_trim_multiple_chars() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 'does it work now?!' | str trim -c '?!'
        "#
        )
    );

    assert!(actual.err.contains("char"));
}

#[test]
fn capitalizes() {
    Playground::setup("str_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str capitalize dependency.name | get dependency.name | echo $it"
        );

        assert_eq!(actual.out, "Nu");
    })
}

#[test]
fn downcases() {
    Playground::setup("str_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "LIGHT"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str downcase dependency.name | get dependency.name | echo $it"
        );

        assert_eq!(actual.out, "light");
    })
}

#[test]
fn upcases() {
    Playground::setup("str_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    name = "nushell"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str upcase package.name | get package.name | echo $it"
        );

        assert_eq!(actual.out, "NUSHELL");
    })
}

#[test]
fn converts_to_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '{number_as_string: "1"}'
            | from json
            | str to-int number_as_string
            | rename number
            | where number == 1
            | get number
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn converts_to_decimal() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo "3.1, 0.0415"
            | split row ","
            | str to-decimal
            | math sum
        "#
    ));

    assert_eq!(actual.out, "3.1415");
}

#[test]
fn sets() {
    Playground::setup("str_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nushell"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str set wykittenshell package.name
                 | get package.name
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "wykittenshell");
    })
}

#[test]
fn find_and_replaces() {
    Playground::setup("str_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str find-replace KATZ "5289" fortune.teller.phone
                 | get fortune.teller.phone
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "1-800-5289");
    })
}

#[test]
fn find_and_replaces_without_passing_field() {
    Playground::setup("str_test_7", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | get fortune.teller.phone
                 | str find-replace KATZ "5289"
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "1-800-5289");
    })
}

#[test]
fn substrings_the_input() {
    Playground::setup("str_test_8", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-ROBALINO"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str substring 6,14 fortune.teller.phone
                 | get fortune.teller.phone
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "ROBALINO");
    })
}

#[test]
fn substring_errors_if_start_index_is_greater_than_end_index() {
    Playground::setup("str_test_9", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-ROBALINO"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str substring 6,5 fortune.teller.phone
                 | echo $it
             "#
        ));

        assert!(actual
            .err
            .contains("End must be greater than or equal to Start"))
    })
}

#[test]
fn substrings_the_input_and_returns_the_string_if_end_index_exceeds_length() {
    Playground::setup("str_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str substring 0,999 package.name
                 | get package.name
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "nu-arepas");
    })
}

#[test]
fn substrings_the_input_and_returns_blank_if_start_index_exceeds_length() {
    Playground::setup("str_test_11", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str substring 50,999 package.name
                 | get package.name
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "");
    })
}

#[test]
fn substrings_the_input_and_treats_start_index_as_zero_if_blank_start_index_given() {
    Playground::setup("str_test_12", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str substring ,2 package.name
                 | get package.name
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "nu");
    })
}

#[test]
fn substrings_the_input_and_treats_end_index_as_length_if_blank_end_index_given() {
    Playground::setup("str_test_13", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str substring 3, package.name
                 | get package.name
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "arepas");
    })
}

#[test]
fn from_decimal_correct_trailing_zeros() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 1.23000 | str from -d 3
        "#
    ));

    assert!(actual.out.contains("1.230"));
}

#[test]
fn from_int_decimal_correct_trailing_zeros() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 1.00000 | str from -d 3
        "#
    ));

    assert!(actual.out.contains("1.000"));
}

#[test]
fn from_int_decimal_trim_trailing_zeros() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 1.00000 | str from | format "{$it} flat"
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
        | str from weight -d 2
        "#
    ));

    assert!(actual.out.contains("32.38"));
    assert!(actual.out.contains("15.20"));
}

#[test]
fn str_reverse() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "nushell" | str reverse
        "#
    ));

    assert!(actual.out.contains("llehsun"));
}
