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

// These tests are mostly temporary to ensure the behaviour don't change after a refactor.
// For example, `grid` command should only return strings as it is defined in its command signature
// and there are some other inconsistencies.

#[rstest]
#[case::empty_list("[]", "")]
#[case::empty_record("{}", "")]
#[case::list_of_string("[a b c]", "a │ b │ c\n")]
#[case::list_of_various_data_types(
    "[a bc 1 23 true false null {key: value} [list]]",
    "a │ bc │ 1 │ 23 │ true │ false │  │ {key: value} │ [list]\n"
)]
#[case::record_with_no_name("{test: no_name}", "")]
#[case::record_with_name("{name: test}", "test\n")]
#[case::list_whose_first_element_is_not_record(
    "[a 33 {name: in_the_middle} null {record: without_name} [ijkl {name: e}]]",
    "a │ 33 │ {name: in_the_middle} │  │ {record: without_name} │ [ijkl, {name: e}]\n"
)]
#[case::list_whose_first_element_is_record_with_name_columns(
    "[{name: test} a 33 {name: in_the_middle} null {record: without_name} [ijkl {name: e}]]",
    "test │ a │ 33 │ in_the_middle │  │  │ [ijkl, {name: e}]\n"
)]
#[case::list_whose_first_element_is_record_without_name_columns(
    "[{test: name} a 33 {name: in_the_middle} null {record: without_name} [ijkl {name: e}]]",
    ""
)]
fn test_output_without_column_name(#[case] code: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{code} | grid --width 100"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::record_with_name("{name: test}", "test\n")]
#[case::table_with_name("[Darren Juhan Piep] | wrap name", "Darren │ Juhan │ Piep\n")]
fn test_output_with_column_name(#[case] code: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{code} | grid name --width 100"))
        .expect_value_eq(expected)
}
