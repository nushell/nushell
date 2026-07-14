use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn grid_errors_with_few_columns() -> Result {
    let error = test()
        .run("[1 2 3 4 5] | grid --width 5")
        .expect_shell_error()?
        .generic_error()?;

    assert_eq!(&error, "Couldn't fit grid into 5 columns");
    Ok(())
}

#[rstest]
#[case::empty_list("[]", "")]
#[case::list_of_string("[a b c]", "a │ b │ c\n")]
#[case::list_of_various_data_types(
    "[{a: b, c: d} a bc 1 23 true false null {key: value} [list]]",
    "{a: b, c: d} │ a │ bc │ 1 │ 23 │ true │ false │  │ {key: value} │ [list]\n"
)]
fn test_output_without_column_name(#[case] code: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{code} | grid --width 100"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::table_with_name("[Darren Juhan Piep] | wrap name", "Darren │ Juhan │ Piep\n")]
fn test_output_with_column_name(#[case] code: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{code} | grid name --width 100"))
        .expect_value_eq(expected)
}
