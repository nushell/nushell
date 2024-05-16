use crate::repl::tests::{fail_test, run_test, TestResult};

// Tests for null / null / Value::Nothing
#[test]
fn nothing_fails_string() -> TestResult {
    fail_test("let nil = null; $nil.foo", "doesn't support cell paths")
}

#[test]
fn nothing_fails_int() -> TestResult {
    fail_test("let nil = null; $nil.3", "doesn't support cell paths")
}

// Tests for records
#[test]
fn record_single_field_success() -> TestResult {
    run_test("{foo: 'bar'}.foo == 'bar'", "true")
}

#[test]
fn record_single_field_optional_success() -> TestResult {
    run_test("{foo: 'bar'}.foo? == 'bar'", "true")
}

#[test]
fn get_works_with_cell_path_success() -> TestResult {
    run_test("{foo: 'bar'} | get foo?", "bar")
}

#[test]
fn get_works_with_cell_path_missing_data() -> TestResult {
    run_test("{foo: 'bar'} | get foobar? | to nuon", "null")
}

#[test]
fn record_single_field_failure() -> TestResult {
    fail_test("{foo: 'bar'}.foobar", "")
}

#[test]
fn record_int_failure() -> TestResult {
    fail_test("{foo: 'bar'}.3", "")
}

#[test]
fn record_single_field_optional() -> TestResult {
    run_test("{foo: 'bar'}.foobar?  | to nuon", "null")
}

#[test]
fn record_single_field_optional_short_circuits() -> TestResult {
    // Check that we return null as soon as the `.foobar?` access
    // fails instead of erroring on the `.baz` access
    run_test("{foo: 'bar'}.foobar?.baz  | to nuon", "null")
}

#[test]
fn record_multiple_optional_fields() -> TestResult {
    run_test("{foo: 'bar'}.foobar?.baz? | to nuon", "null")
}

#[test]
fn nested_record_field_success() -> TestResult {
    run_test("{foo: {bar: 'baz'} }.foo.bar == 'baz'", "true")
}

#[test]
fn nested_record_field_failure() -> TestResult {
    fail_test("{foo: {bar: 'baz'} }.foo.asdf", "")
}

#[test]
fn nested_record_field_optional() -> TestResult {
    run_test("{foo: {bar: 'baz'} }.foo.asdf?  | to nuon", "null")
}

#[test]
fn record_with_nested_list_success() -> TestResult {
    run_test("{foo: [{bar: 'baz'}]}.foo.0.bar == 'baz'", "true")
}

#[test]
fn record_with_nested_list_int_failure() -> TestResult {
    fail_test("{foo: [{bar: 'baz'}]}.foo.3.bar", "")
}

#[test]
fn record_with_nested_list_column_failure() -> TestResult {
    fail_test("{foo: [{bar: 'baz'}]}.foo.0.asdf", "")
}

// Tests for lists
#[test]
fn list_single_field_success() -> TestResult {
    run_test("[{foo: 'bar'}].foo.0 == 'bar'", "true")?;
    // test field access both ways
    run_test("[{foo: 'bar'}].0.foo == 'bar'", "true")
}

#[test]
fn list_single_field_failure() -> TestResult {
    fail_test("[{foo: 'bar'}].asdf", "")
}

// Test the scenario where the requested column is not present in all rows
#[test]
fn jagged_list_access_fails() -> TestResult {
    fail_test("[{foo: 'bar'}, {}].foo", "cannot find column")?;
    fail_test("[{}, {foo: 'bar'}].foo", "cannot find column")
}

#[test]
fn jagged_list_optional_access_succeeds() -> TestResult {
    run_test("[{foo: 'bar'}, {}].foo?.0", "bar")?;
    run_test("[{foo: 'bar'}, {}].foo?.1  | to nuon", "null")?;

    run_test("[{}, {foo: 'bar'}].foo?.0 | to nuon", "null")?;
    run_test("[{}, {foo: 'bar'}].foo?.1", "bar")
}

// test that accessing a nonexistent row fails
#[test]
fn list_row_access_failure() -> TestResult {
    fail_test("[{foo: 'bar'}, {foo: 'baz'}].2", "")
}

#[test]
fn list_row_optional_access_succeeds() -> TestResult {
    run_test("[{foo: 'bar'}, {foo: 'baz'}].2? | to nuon", "null")?;
    run_test("[{foo: 'bar'}, {foo: 'baz'}].3? | to nuon", "null")
}

// regression test for an old bug
#[test]
fn do_not_delve_too_deep_in_nested_lists() -> TestResult {
    fail_test("[[{foo: bar}]].foo", "cannot find column")
}

#[test]
fn cell_path_literals() -> TestResult {
    run_test("let cell_path = $.a.b; {a: {b: 3}} | get $cell_path", "3")
}

// Test whether cell path access short-circuits properly
#[test]
fn deeply_nested_cell_path_short_circuits() -> TestResult {
    run_test(
        "{foo: [{bar: 'baz'}]}.foo.3?.bar.asdfdafg.234.foobar  | to nuon",
        "null",
    )
}

#[test]
fn cell_path_type() -> TestResult {
    run_test("$.a.b | describe", "cell-path")
}
