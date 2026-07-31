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
#[case::unclosed_brace("name -> {package.name", "Unclosed delimiter", "{")]
#[case::not_opened_brace("package.name} <- name", "Unbalanced { and }", "}")]
#[case::missing_column(
    "version -> {package.version}",
    "column 'version' is missing in one or more values",
    "package.version"
)]
fn format_string_error(
    #[case] format_str: &str,
    #[case] label_msg: &str,
    #[case] label_text: &str,
) -> Result {
    let input = test_value!({ package: { name: "dummy" } });

    let mut tester = test();
    let () = tester.run_with_data("let format_str = $in", format_str)?;

    let err = tester
        .run_with_data("format pattern $format_str", input)
        .expect_labeled_error()?;

    let inner_err = err.inner.iter().next().expect("at least one inner error");
    let ShellError::OutsideSourceNoUrl { msg, labels, .. } = inner_err else {
        panic!("Expected `ShellError::OutsideSourceNoUrl`, got {inner_err:?}");
    };
    assert_eq!(msg, "Format string has errors.");

    let [label] = labels.as_slice() else {
        panic!("Expected only one label, got {labels:?}")
    };
    assert_eq!(label.label().unwrap(), label_msg);
    let (start, end) = (label.offset(), label.offset() + label.len());
    assert_eq!(&format_str[start..end], label_text);

    Ok(())
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
