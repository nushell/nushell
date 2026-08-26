use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::record_single_field("{foo: 'bar'}.foo == 'bar'")]
#[case::record_single_field_optional("{foo: 'bar'}.foo? == 'bar'")]
#[case::nested_record_field("{foo: {bar: 'baz'} }.foo.bar == 'baz'")]
#[case::record_with_nested_list("{foo: [{bar: 'baz'}]}.foo.0.bar == 'baz'")]
#[case::list_single_field_by_column("[{foo: 'bar'}].foo.0 == 'bar'")]
#[case::list_single_field_by_row("[{foo: 'bar'}].0.foo == 'bar'")]
fn cell_path_bool_successes(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(true)
}

#[rstest]
#[case::get_works_with_cell_path("{foo: 'bar'} | get foo?", "bar")]
#[case::jagged_list_optional_access_first("[{foo: 'bar'}, {}].foo?.0", "bar")]
#[case::jagged_list_optional_access_second("[{}, {foo: 'bar'}].foo?.1", "bar")]
#[case::cell_path_type("$.a.b | describe", "cell-path")]
fn cell_path_string_successes(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::cell_path_literals("let cell_path = $.a.b; {a: {b: 3}} | get $cell_path", 3)]
fn cell_path_int_successes(#[case] code: &str, #[case] expected: i64) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::empty_record_optional_field("{}.foo? == null")]
#[case::get_works_with_cell_path_missing_data("({foo: 'bar'} | get foobar?) == null")]
#[case::record_single_field_optional("{foo: 'bar'}.foobar? == null")]
#[case::record_single_field_optional_short_circuits("{foo: 'bar'}.foobar?.baz == null")]
#[case::record_multiple_optional_fields("{foo: 'bar'}.foobar?.baz? == null")]
#[case::nested_record_field_optional("{foo: {bar: 'baz'} }.foo.asdf? == null")]
#[case::jagged_list_optional_access_first_missing("[{}, {foo: 'bar'}].foo?.0 == null")]
#[case::jagged_list_optional_access_second_missing("[{foo: 'bar'}, {}].foo?.1 == null")]
#[case::list_row_optional_access_first("[{foo: 'bar'}, {foo: 'baz'}].2? == null")]
#[case::list_row_optional_access_second("[{foo: 'bar'}, {foo: 'baz'}].3? == null")]
#[case::deeply_nested_cell_path_short_circuits(
    "{foo: [{bar: 'baz'}]}.foo.3?.bar.asdfdafg.234.foobar == null"
)]
fn cell_path_null_successes(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(true)
}

#[rstest]
#[case::nothing_fails_string("let nil = null; $nil.foo", "IncompatiblePathAccess")]
#[case::nothing_fails_int("let nil = null; $nil.3", "IncompatiblePathAccess")]
#[case::record_single_field("{foo: 'bar'}.foobar", "")]
#[case::record_int("{foo: 'bar'}.3", "")]
#[case::nested_record_field("{foo: {bar: 'baz'} }.foo.asdf", "")]
#[case::record_with_nested_list_int("{foo: [{bar: 'baz'}]}.foo.3.bar", "")]
#[case::record_with_nested_list_column("{foo: [{bar: 'baz'}]}.foo.0.asdf", "")]
#[case::list_single_field("[{foo: 'bar'}].asdf", "")]
#[case::jagged_list_access_first("[{foo: 'bar'}, {}].foo", "CantFindColumn")]
#[case::jagged_list_access_second("[{}, {foo: 'bar'}].foo", "CantFindColumn")]
#[case::list_row_access("[{foo: 'bar'}, {foo: 'baz'}].2", "")]
#[case::do_not_delve_too_deep_in_nested_lists("[[{foo: bar}]].foo", "CantFindColumn")]
fn cell_path_failures(#[case] code: &str, #[case] expected: &str) -> Result {
    let error = test().run(code).expect_shell_error()?;

    if !expected.is_empty() {
        assert_contains(expected, format!("{error:?}"));
    }

    Ok(())
}

#[rstest]
#[case::list_negative_row_access_reports_clear_error(
    "[{foo: 'bar'}, {foo: 'baz'}].-1",
    "negative index is not supported"
)]
fn cell_path_parse_failures(#[case] code: &str, #[case] expected: &str) -> Result {
    let error = test().run(code).expect_parse_error()?;
    assert_contains(expected, format!("{error:?}"));
    Ok(())
}
