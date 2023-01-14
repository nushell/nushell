use crate::tests::{fail_test, run_test, TestResult};

// Tests for $nothing / null / Value::Nothing
#[test]
fn nothing_fails_string() -> TestResult {
    fail_test("$nothing.foo", "doesn't support cell paths")
}

#[test]
fn nothing_fails_int() -> TestResult {
    fail_test("$nothing.3", "doesn't support cell paths")
}

// Tests for records
#[test]
fn record_single_field_success() -> TestResult {
    run_test("{foo: 'bar'}.foo == 'bar'", "true")
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
fn nested_record_field_success() -> TestResult {
    run_test("{foo: {bar: 'baz'} }.foo.bar == 'baz'", "true")
}

#[test]
fn nested_record_field_failure() -> TestResult {
    fail_test("{foo: {bar: 'baz'} }.foo.asdf", "")
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

// test that accessing a nonexistent row fails
#[test]
fn list_row_access_failure() -> TestResult {
    fail_test("[{foo: 'bar'}, {foo: 'baz'}].2", "")
}

// regression test for an old bug
#[test]
fn do_not_delve_too_deep_in_nested_lists() -> TestResult {
    fail_test("[[{foo: bar}]].foo", "cannot find column")
}
