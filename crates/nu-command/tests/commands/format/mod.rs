use nu_test_support::{
    fs::Stub::{EmptyFile, FileWithContentToBeTrimmed},
    prelude::*,
};
use rstest::rstest;

mod duration;
mod filesize;

#[test]
fn creates_the_resulting_string_from_the_given_fields() -> Result {
    let code = r#"
        open cargo_sample.toml
        | get package
        | format pattern "{name} has license {license}"
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("nu has license ISC")
}

#[test]
fn format_input_record_output_string() -> Result {
    let code = r#"{name: Downloads} | format pattern "{name}""#;
    test().run(code).expect_value_eq("Downloads")
}

#[test]
fn given_fields_can_be_column_paths() -> Result {
    let code = r#"
        open cargo_sample.toml
        | format pattern "{package.name} is {package.description}"
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("nu is a new type of shell")
}

#[test]
fn cant_use_variables() -> Result {
    let code = r#"
        open cargo_sample.toml
        | format pattern "{$it.package.name} is {$it.package.description}"
    "#;

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_error()?;

    assert_eq!(err.generic_error()?, "Removed functionality");
    Ok(())
}

#[rstest]
#[case::unclosed_column("{package.name")]
#[case::lone_open("{")]
#[case::lone_close("}")]
#[case::trailing_open("hello {")]
#[case::closed_then_open("{name}{")]
fn error_unmatched_brace(#[case] pattern: &str) -> Result {
    // Pattern must be a source literal. `format pattern` derives spans from the
    // argument span vs string length, and panics when the value comes from a variable.
    let err = test()
        .run(format!("{{}} | format pattern '{pattern}'"))
        .expect_error()?;

    let ShellError::DelimiterError { msg, .. } = err else {
        return Err(err.into());
    };

    assert_eq!(msg, "there are unmatched curly braces");
    Ok(())
}

#[test]
fn escaped_open_brace_is_literal() -> Result {
    test().run("{} | format pattern '{{'").expect_value_eq("{")
}

#[test]
fn format_filesize_works() -> Result {
    Playground::setup("format_filesize_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        let code = "
            ls
            | format filesize kB size
            | get size
            | first
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq("0 kB")
    })
}

#[test]
fn format_filesize_works_with_nonempty_files() -> Result {
    Playground::setup(
        "format_filesize_works_with_nonempty_files",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "sample.toml",
                r#"
                    [dependency]
                    name = "nu"
                "#,
            )]);

            let code = "ls sample.toml | format filesize B size | get size | first";
            #[cfg(not(windows))]
            let expected = "25 B";
            #[cfg(windows)]
            let expected = "27 B";

            test().cwd(dirs.test()).run(code).expect_value_eq(expected)
        },
    )
}

#[test]
fn format_filesize_with_invalid_unit() -> Result {
    let code = "1MB | format filesize sec";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit { .. }));
    Ok(())
}

#[test]
fn format_duration_with_invalid_unit() -> Result {
    let code = "1sec | format duration MB";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit { .. }));
    Ok(())
}
