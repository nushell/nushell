use nu_test_support::prelude::*;
use rstest::rstest;

mod duration;
mod filesize;

#[rstest]
#[case::simple(test_value!({name: "Downloads"}), "{name}", "Downloads")]
#[case::multiple(test_value!({name: "nu", license: "ISC"}), "{name} has license {license}", "nu has license ISC")]
#[case::nested_columns(
    test_value!({package: {name: "nu", description: "a new type of shell"}}),
    "{package.name} is {package.description}",
    "nu is a new type of shell",
)]
fn record_input(
    #[case] input: impl IntoValue,
    #[case] format_str: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let mut tester = test();
    let () = tester.run_with_data("let format_str = $in", format_str)?;
    tester
        .run_with_data("format pattern $format_str", input)
        .expect_value_eq(expected)
}

#[test]
fn cant_use_variables() -> Result {
    let err = test()
        .run_with_data(
            r#"format pattern "{$it.package.name} is {$it.package.description}""#,
            test_value!({package: {name: "nu", description: "a new type of shell"}}),
        )
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
