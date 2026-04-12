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
#[case::list_of_string("[a b c] | grid", "a │ b │ c\n")]
#[case::list_of_various_data_types(
    "[a bc 1 23 true false null {key: value} [list]] | grid",
    "a │ bc │ 1 │ 23 │ true │ false │  │ {key: value} │ [list]\n"
)]
#[case::record_with_no_name("{test: no_name} | grid", "")]
#[case::record_with_name("{name: test} | grid", "test\n")]
#[case::list_whose_first_element_is_not_record(
    "[a 33 {name: in_the_middle} null {record: without_name} [ijkl {name: e}]] | grid",
    "a │ 33 │ {name: in_the_middle} │  │ {record: without_name} │ [ijkl, {name: e}]\n"
)]
#[case::list_whose_first_element_is_record_with_name_columns(
    "[{name: test} a 33 {name: in_the_middle} null {record: without_name} [ijkl {name: e}]] | grid",
    "test │ a │ 33 │ in_the_middle │  │  │ [ijkl, {name: e}]\n"
)]
#[case::list_whose_first_element_is_record_without_name_columns(
    "[{test: name} a 33 {name: in_the_middle} null {record: without_name} [ijkl {name: e}]] | grid",
    ""
)]
fn test_output(#[case] code: &str, #[case] expected: &str) -> Result {
    let output: String = test().run(code)?;
    assert_eq!(&output, expected);
    Ok(())
}
