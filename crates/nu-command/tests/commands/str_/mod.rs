mod into_string;
mod join;

use nu_test_support::{fs::Stub::FileWithContent, prelude::*};

#[test]
fn trims() -> Result {
    Playground::setup("str_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu "
                "#,
        )]);

        let code = "open sample.toml | str trim dependency.name | get dependency.name";
        test().cwd(dirs.test()).run(code).expect_value_eq("nu")
    })
}

#[test]
fn error_trim_multiple_chars() -> Result {
    let code = r#"
    echo "does it work now?!" | str trim --char "?!"
    "#;

    let err = test().run(code).expect_shell_error()?;
    assert_contains("char", err.to_string());
    Ok(())
}

#[test]
fn capitalizes() -> Result {
    Playground::setup("str_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu"
                "#,
        )]);

        let code = "open sample.toml | str capitalize dependency.name | get dependency.name";
        test().cwd(dirs.test()).run(code).expect_value_eq("Nu")
    })
}

#[test]
fn downcases() -> Result {
    Playground::setup("str_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "LIGHT"
                "#,
        )]);

        let code = "open sample.toml | str downcase dependency.name | get dependency.name";
        test().cwd(dirs.test()).run(code).expect_value_eq("light")
    })
}

#[test]
fn non_ascii_downcase() -> Result {
    let code = "'ὈΔΥΣΣΕΎΣ' | str downcase";
    test().run(code).expect_value_eq("ὀδυσσεύς")
}

#[test]
fn upcases() -> Result {
    Playground::setup("str_test_4", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    name = "nushell"
                "#,
        )]);

        let code = "open sample.toml | str upcase package.name | get package.name";
        test().cwd(dirs.test()).run(code).expect_value_eq("NUSHELL")
    })
}

#[test]
fn non_ascii_upcase() -> Result {
    let code = "'ὀδυσσεύς' | str upcase";
    test().run(code).expect_value_eq("ὈΔΥΣΣΕΎΣ")
}

#[test]
// #[ignore = "Playgrounds are not supported in nu-cmd-extra"]
fn camelcases() -> Result {
    Playground::setup("str_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [dependency]
                name = "THIS_IS_A_TEST"
            "#,
        )]);

        let code = "open sample.toml | str camel-case dependency.name | get dependency.name";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("thisIsATest")
    })
}

#[test]
fn converts_to_int() -> Result {
    let code = r#"
        echo '[{number_as_string: "1"}]'
        | from json
        | into int number_as_string
        | rename number
        | where number == 1
        | get number.0

    "#;

    test().run(code).expect_value_eq(1)
}

#[test]
fn converts_to_float() -> Result {
    let code = r#"
        echo "3.1, 0.0415"
        | split row ","
        | into float
        | math sum
    "#;

    #[expect(clippy::approx_constant)]
    test().run(code).expect_value_eq(3.1415)
}

#[test]
fn find_and_replaces() -> Result {
    Playground::setup("str_test_6", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let code = r#"
             open sample.toml
             | str replace KATZ "5289" fortune.teller.phone
             | get fortune.teller.phone
         "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("1-800-5289")
    })
}

#[test]
fn find_and_replaces_without_passing_field() -> Result {
    Playground::setup("str_test_7", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let code = r#"
             open sample.toml
             | get fortune.teller.phone
             | str replace KATZ "5289"
         "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("1-800-5289")
    })
}

#[test]
fn regex_error_in_pattern() -> Result {
    Playground::setup("str_test_8", |dirs, _sandbox| {
        let code = r#"
             'source string'
             | str replace -r 'source \Ufoo' "destination"
         "#;

        let err = test().cwd(dirs.test()).run(code).expect_shell_error()?;
        assert_contains("Incorrect value", err.to_string());
        Ok(())
    })
}

#[test]
fn find_and_replaces_with_closure() -> Result {
    let code = "
         'source string'
         | str replace 'str' { str upcase }
     ";

    test().run(code).expect_value_eq("source STRing")
}

#[test]
fn find_and_replaces_regex_with_closure() -> Result {
    let code = r#"
         'source string'
         | str replace -r 's(..)ing' {|capture|
           $"($capture) from ($in)"
         }
     "#;

    test().run(code).expect_value_eq("source tr from string")
}

#[test]
fn find_and_replaces_closure_error() -> Result {
    let code = "
         'source string'
         | str replace 'str' { 1 / 0 }
     ";

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn find_and_replaces_regex_closure_error() -> Result {
    let code = "
         'source string'
         | str replace -r 'str' { 1 / 0 }
     ";

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn find_and_replaces_closure_type_mismatch() -> Result {
    let code = "
         'source string'
         | str replace 'str' { 42 }
     ";

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::RuntimeTypeMismatch { .. }));
    Ok(())
}

#[test]
fn find_and_replaces_regex_closure_type_mismatch() -> Result {
    let code = "
         'source string'
         | str replace -r 'str' { 42 }
     ";

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::RuntimeTypeMismatch { .. }));
    Ok(())
}

#[test]
fn substrings_the_input() -> Result {
    Playground::setup("str_test_8", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-ROBALINO"
                 "#,
        )]);

        let code = "
             open sample.toml
             | str substring 6..14 fortune.teller.phone
             | get fortune.teller.phone
         ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("ROBALINO")
    })
}

#[test]
fn substring_empty_if_start_index_is_greater_than_end_index() -> Result {
    Playground::setup("str_test_9", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-ROBALINO"
                 "#,
        )]);

        let code = "
             open sample.toml
             | str substring 6..4 fortune.teller.phone
             | get fortune.teller.phone
         ";

        test().cwd(dirs.test()).run(code).expect_value_eq("")
    })
}

#[test]
fn substrings_the_input_and_returns_the_string_if_end_index_exceeds_length() -> Result {
    Playground::setup("str_test_10", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let code = "
             open sample.toml
             | str substring 0..999 package.name
             | get package.name
         ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("nu-arepas")
    })
}

#[test]
fn substrings_the_input_and_returns_blank_if_start_index_exceeds_length() -> Result {
    Playground::setup("str_test_11", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let code = "
             open sample.toml
             | str substring 50..999 package.name
             | get package.name
         ";

        test().cwd(dirs.test()).run(code).expect_value_eq("")
    })
}

#[test]
fn substrings_the_input_and_treats_start_index_as_zero_if_blank_start_index_given() -> Result {
    Playground::setup("str_test_12", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let code = "
             open sample.toml
             | str substring ..1 package.name
             | get package.name
         ";

        test().cwd(dirs.test()).run(code).expect_value_eq("nu")
    })
}

#[test]
fn substrings_the_input_and_treats_end_index_as_length_if_blank_end_index_given() -> Result {
    Playground::setup("str_test_13", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nu-arepas"
                 "#,
        )]);

        let code = "
             open sample.toml
             | str substring 3.. package.name
             | get package.name
         ";

        test().cwd(dirs.test()).run(code).expect_value_eq("arepas")
    })
}

#[test]
fn substring_by_negative_index() -> Result {
    Playground::setup("str_test_13", |dirs, _| {
        let mut tester = test().cwd(dirs.test());
        tester
            .run("'apples' | str substring 0..-1")
            .expect_value_eq("apples")?;
        tester
            .run("'apples' | str substring 0..<-1")
            .expect_value_eq("apple")
    })
}

#[test]
fn substring_of_empty_string() -> Result {
    let code = "'' | str substring ..0";
    test().run(code).expect_value_eq("")
}

#[test]
fn substring_drops_content_type() -> Result {
    let code = format!(
        "open {} | str substring 0..2 | metadata | get content_type? | describe",
        file!(),
    );
    test().run(code).expect_value_eq("nothing")
}

#[test]
fn str_reverse() -> Result {
    let code = r#"
        echo "nushell" | str reverse
        "#;

    let outcome: String = test().run(code)?;
    assert_contains("llehsun", outcome);
    Ok(())
}

#[test]
fn test_redirection_trim() -> Result {
    let code = "
        let x = (nu --testbin cococo niceone); $x | str trim | str length
        ";

    test().add_nu_to_path().run(code).expect_value_eq(7)
}
