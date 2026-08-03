use nu_test_support::prelude::*;
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
