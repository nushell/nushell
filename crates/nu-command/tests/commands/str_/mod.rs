mod into_string;
mod join;

use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn trims() {
    Playground::setup("str_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu "
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str trim dependency.name | get dependency.name"
        );

        assert_eq!(actual.out, "nu");
    })
}

#[test]
fn error_trim_multiple_chars() {
    let actual = nu!(r#"
    echo "does it work now?!" | str trim --char "?!"
    "#);

    assert!(actual.err.contains("char"));
}

#[test]
fn capitalizes() {
    Playground::setup("str_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str capitalize dependency.name | get dependency.name"
        );

        assert_eq!(actual.out, "Nu");
    })
}

#[test]
fn downcases() {
    Playground::setup("str_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "LIGHT"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str downcase dependency.name | get dependency.name"
        );

        assert_eq!(actual.out, "light");
    })
}

#[test]
fn non_ascii_downcase() {
    let actual = nu!("'ὈΔΥΣΣΕΎΣ' | str downcase");

    assert_eq!(actual.out, "ὀδυσσεύς");
}

#[test]
fn upcases() {
    Playground::setup("str_test_4", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    name = "nushell"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str upcase package.name | get package.name"
        );

        assert_eq!(actual.out, "NUSHELL");
    })
}

#[test]
fn non_ascii_upcase() {
    let actual = nu!("'ὀδυσσεύς' | str upcase");

    assert_eq!(actual.out, "ὈΔΥΣΣΕΎΣ");
}

#[test]
#[ignore = "Playgrounds are not supported in nu-cmd-extra"]
fn camelcases() {
    Playground::setup("str_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "THIS_IS_A_TEST"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str camel-case dependency.name | get dependency.name"
        );

        assert_eq!(actual.out, "thisIsATest");
    })
}

#[test]
fn converts_to_int() {
    let actual = nu!(r#"
        echo '[{number_as_string: "1"}]'
        | from json
        | into int number_as_string
        | rename number
        | where number == 1
        | get number.0
    
    "#);

    assert_eq!(actual.out, "1");
}

#[test]
fn converts_to_float() {
    let actual = nu!(r#"
        echo "3.1, 0.0415"
        | split row ","
        | into float
        | math sum
    "#);

    assert_eq!(actual.out, "3.1415");
}

#[test]
fn find_and_replaces() {
    Playground::setup("str_test_6", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | str replace KATZ "5289" fortune.teller.phone
             | get fortune.teller.phone
         "#);

        assert_eq!(actual.out, "1-800-5289");
    })
}

#[test]
fn find_and_replaces_without_passing_field() {
    Playground::setup("str_test_7", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | get fortune.teller.phone
             | str replace KATZ "5289"
         "#);

        assert_eq!(actual.out, "1-800-5289");
    })
}

#[test]
fn regex_error_in_pattern() {
    Playground::setup("str_test_8", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), r#"
             'source string'
             | str replace -r 'source \Ufoo' "destination"
         "#);

        let err = actual.err;
        let expecting_str = "Incorrect value";
        assert!(
            err.contains(expecting_str),
            "Error should contain '{expecting_str}', but was: {err}"
        );
    })
}

#[test]
fn substrings_the_input() {
    Playground::setup("str_test_8", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-ROBALINO"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | str substring 6..14 fortune.teller.phone
             | get fortune.teller.phone
         "#);

        assert_eq!(actual.out, "ROBALINO");
    })
}

#[test]
fn substring_empty_if_start_index_is_greater_than_end_index() {
    Playground::setup("str_test_9", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-ROBALINO"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | str substring 6..4 fortune.teller.phone
             | get fortune.teller.phone
         "#);
        assert_eq!(actual.out, "")
    })
}

#[test]
fn substrings_the_input_and_returns_the_string_if_end_index_exceeds_length() {
    Playground::setup("str_test_10", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | str substring 0..999 package.name
             | get package.name
         "#);

        assert_eq!(actual.out, "nu-arepas");
    })
}

#[test]
fn substrings_the_input_and_returns_blank_if_start_index_exceeds_length() {
    Playground::setup("str_test_11", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | str substring 50..999 package.name
             | get package.name
         "#);

        assert_eq!(actual.out, "");
    })
}

#[test]
fn substrings_the_input_and_treats_start_index_as_zero_if_blank_start_index_given() {
    Playground::setup("str_test_12", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | str substring ..1 package.name
             | get package.name
         "#);

        assert_eq!(actual.out, "nu");
    })
}

#[test]
fn substrings_the_input_and_treats_end_index_as_length_if_blank_end_index_given() {
    Playground::setup("str_test_13", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
             open sample.toml
             | str substring 3.. package.name
             | get package.name
         "#);

        assert_eq!(actual.out, "arepas");
    })
}

#[test]
fn substring_by_negative_index() {
    Playground::setup("str_test_13", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), "'apples' | str substring 0..-1",
        );
        assert_eq!(actual.out, "apples");

        let actual = nu!(
            cwd: dirs.test(), "'apples' | str substring 0..<-1",
        );
        assert_eq!(actual.out, "apple");
    })
}

#[test]
fn substring_of_empty_string() {
    let actual = nu!("'' | str substring ..0");
    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "");
}

#[test]
fn substring_drops_content_type() {
    let actual = nu!(format!(
        "open {} | str substring 0..2 | metadata | get content_type? | describe",
        file!(),
    ));
    assert_eq!(actual.out, "nothing");
}

#[test]
fn str_reverse() {
    let actual = nu!(r#"
        echo "nushell" | str reverse
        "#);

    assert!(actual.out.contains("llehsun"));
}

#[test]
fn test_redirection_trim() {
    let actual = nu!(r#"
        let x = (nu --testbin cococo niceone); $x | str trim | str length
        "#);

    assert_eq!(actual.out, "7");
}
