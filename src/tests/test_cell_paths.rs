use crate::tests::{fail_test, run_test, TestResult};

// tests for $nothing / null / Value::Nothing
#[test]
fn nothing_passes_optional() -> TestResult {
    run_test("$nothing?.foo | to nuon", "null")
}

#[test]
fn nothing_fails_string() -> TestResult {
    fail_test("$nothing.foo", "doesn't support cell paths")
}

#[test]
fn nothing_fails_int() -> TestResult {
    fail_test("$nothing.3", "Can't access")
}

// tests for records
#[test]
fn record_single_field_success() -> TestResult {
    run_test("{foo: 'bar'}.foo", "bar")
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
    run_test("{foo: 'bar'}?.foobar  | to nuon", "null")
}

#[test]
fn nested_record_field_success() -> TestResult {
    run_test("{foo: {bar: 'baz'} }.foo.bar", "baz")
}

#[test]
fn nested_record_field_failure() -> TestResult {
    fail_test("{foo: {bar: 'baz'} }.foo.asdf", "")
}

#[test]
fn nested_record_field_optional() -> TestResult {
    run_test("{foo: {bar: 'baz'} }.foo?.asdf  | to nuon", "null")
}

#[test]
fn record_with_nested_list_success() -> TestResult {
    run_test("{foo: [{bar: 'baz'}]}.foo.0.bar", "baz")
}

#[test]
fn record_with_nested_list_int_failure() -> TestResult {
    fail_test("{foo: [{bar: 'baz'}]}.foo.3.bar", "")
}

#[test]
fn record_with_nested_list_column_failure() -> TestResult {
    fail_test("{foo: [{bar: 'baz'}]}.foo.0.asdf", "")
}

#[test]
fn deeply_nested_optional_cell_path() -> TestResult {
    run_test(
        "{foo: [{bar: 'baz'}]}.foo?.3?.bar?.asdfdafg?.234?.foobar  | to nuon",
        "null",
    )
}

// tests for lists
#[test]
fn list_single_field_success() -> TestResult {
    run_test("[{foo: 'bar'}].foo.0", "bar")?;
    // test field access both ways
    run_test("[{foo: 'bar'}].0.foo", "bar")
}

#[test]
fn list_single_field_failure() -> TestResult {
    fail_test("[{foo: 'bar'}].asdf", "")
}

#[test]
fn list_single_field_optional() -> TestResult {
    run_test("[{foo: 'bar'}]?.asdf.0  | to nuon", "null")?;
    run_test("[{foo: 'bar'}].0?.asdf  | to nuon", "null")
}

// test the scenario where some list records are missing columns
#[test]
fn jagged_list_access_fails() -> TestResult {
    fail_test("[{foo: 'bar'}, {}].foo", "")?;
    fail_test("[{}, {foo: 'bar'}].foo", "")
}

// test the scenario where some list records are missing columns
#[test]
fn jagged_list_optional_access_succeeds() -> TestResult {
    run_test("[{foo: 'bar'}, {}]?.foo.0", "bar")?;
    run_test("[{foo: 'bar'}, {}]?.foo.1  | to nuon", "null")
}

// test that accessing a nonexistent row fails
#[test]
fn list_row_access_failure() -> TestResult {
    fail_test("[{foo: 'bar'}, {foo: 'baz'}].2", "")
}

#[test]
fn list_row_optional_access_succeeds() -> TestResult {
    run_test("[{foo: 'bar'}, {foo: 'baz'}]?.2  | to nuon", "null")
}

// tests for ListStreams, currently unused
// TODO: re-enable these tests once we have a way to do streaming cell path access
// I'm using a hack to create ListStreams for testing: pipe a list into `each`

#[ignore = "cell path access doesn't handle ListStreams properly yet"]
#[test]
fn list_stream_single_field_success() -> TestResult {
    run_test(
        "[{foo: 'bar'} {foo: 'baz'}] | each {|i| $i } | get foo.0",
        "bar",
    )?;
    run_test(
        "[{foo: 'bar'} {foo: 'baz'}] | each {|i| $i } | get 0.foo",
        "bar",
    )?;
    run_test(
        "[{foo: 'bar'} {foo: 'baz'}] | each {|i| $i } | get 1.foo",
        "baz",
    )?;
    run_test(
        "[{foo: 'bar'} {foo: 'baz'}] | each {|i| $i } | get foo.1",
        "baz",
    )
}

#[ignore = "cell path access doesn't handle ListStreams properly yet"]
#[test]
fn list_stream_single_field_failure() -> TestResult {
    fail_test(
        "[{foo: 'bar'} {foo: 'baz'}] | each {|i| $i } | get asdf",
        "",
    )
}

#[ignore = "cell path access doesn't handle ListStreams properly yet"]
#[test]
fn list_stream_single_field_optional() -> TestResult {
    run_test(
        "[{foo: 'bar'} {foo: 'baz'}] | each {|i| $i } | get ?.asdf.0 | to nuon",
        "null",
    )?;
    run_test(
        "[{foo: 'bar'} {foo: 'baz'}] | each {|i| $i } | get 0?.asdf | to nuon",
        "null",
    )
}

#[ignore = "cell path access doesn't handle ListStreams properly yet"]
#[test]
fn jagged_liststream_access_fails() -> TestResult {
    fail_test("[{foo: 'bar'} {}] | each {|i| $i } | get foo", "")?;
    fail_test("[{} {foo: 'bar'}] | each {|i| $i } | get foo", "")
}

#[ignore = "cell path access doesn't handle ListStreams properly yet"]
#[test]
fn jagged_liststream_optional_access_succeeds() -> TestResult {
    run_test("[{} {foo: 'bar'}] | each {|i| $i } | get ?.foo.1", "bar")?;
    run_test("[{} {foo: 'bar'}] | each {|i| $i } | get 1?.foo", "bar")?;
    run_test(
        "[{} {foo: 'bar'}] | each {|i| $i } | get 0?.foo | to nuon",
        "null",
    )?;
    run_test(
        "[{} {foo: 'bar'}] | each {|i| $i } | get ?.foo.0 | to nuon",
        "null",
    )
}

// Tests for cell paths as used by `get`
// Any cell path access (ex: `$foo.a`) can be rewritten with `get` (ex: `$foo | get a`)
// This is mostly a straightforward translation, but it gets a little interesting because the first dot in a cell path is optional.
// For example, `$foo?.a` can translate to `$foo | get ?.a` OR `$foo | get ?a`
// `$foo.a` can translate to `$foo | get a` (the usual) or `$foo | get .a` (uncommon, but allowed)

#[test]
fn simple_get() -> TestResult {
    run_test("{foo: 'bar'} | get foo", "bar")
}

#[test]
fn get_with_prefixes() -> TestResult {
    run_test("{foo: 'bar'} | get .foo", "bar")?;
    run_test("{foo: 'bar'} | get ?foo", "bar")?;
    run_test("{foo: 'bar'} | get ?.foo", "bar")?;

    run_test("{} | get ?foo | to nuon", "null")?;
    run_test("{} | get ?.foo | to nuon", "null")
}
