use nu_test_support::prelude::*;

#[test]
fn value_is_argument_and_input() -> Result {
    test()
        .run("{a: 1, b: 2} | record apply {a: {|x| $x + $in}}")
        .expect_value_eq(test_value!({a: 2, b: 2}))
}

#[test]
fn transforms_can_be_stored_and_reused() -> Result {
    let code = "
        let transforms = {a: {into int}}
        [{a: '1', b: x} {a: '2', b: y}] | each { record apply $transforms }
    ";

    test().run(code).expect_value_eq(test_value!([
        {a: 1, b: "x"},
        {a: 2, b: "y"}
    ]))
}

#[test]
fn missing_fields_are_skipped_at_every_level() -> Result {
    test()
        .run("{a: {b: 1}} | record apply {a: {b: {$in + 1}, c: {$in}}, d: {$in}}")
        .expect_value_eq(test_value!({a: {b: 2}}))
}

#[test]
fn strict_reports_missing_field() -> Result {
    test()
        .run("{a: {b: 1}} | record apply --strict {a: {c: {$in}}}")
        .expect_error_code_eq("nu::shell::column_not_found")
}

#[test]
fn record_transform_on_non_record_field_errors() -> Result {
    test()
        .run("{a: 1} | record apply {a: {b: {$in}}}")
        .expect_error_code_eq("nu::shell::type_mismatch")
}

#[test]
fn plain_value_transform_errors_even_for_missing_field() -> Result {
    test()
        .run("{a: 1} | record apply {b: 5}")
        .expect_error_code_eq("nu::shell::type_mismatch")
}

#[test]
fn nested_plain_value_transform_errors_even_for_missing_field() -> Result {
    test()
        .run("{x: 1} | record apply {a: {b: 5}}")
        .expect_error_code_eq("nu::shell::type_mismatch")
}

#[test]
fn transforms_are_checked_before_any_closure_runs() -> Result {
    // If the closure for `a` ran first, its error would be returned instead.
    test()
        .run("{a: 1} | record apply {a: {error make {msg: ran}}, b: 5}")
        .expect_error_code_eq("nu::shell::type_mismatch")
}

#[test]
fn error_in_field_is_returned_for_nested_transform() -> Result {
    let code = "
        {a: 1} | update cells { error make {msg: boom} }
        | record apply {a: {b: {$in}}}
    ";

    let err = test().run(code).expect_labeled_error()?;
    assert_eq!(err.msg, "boom");
    Ok(())
}

#[test]
fn closure_error_is_returned() -> Result {
    let err = test()
        .run("{a: 1} | record apply {a: {error make {msg: boom}}}")
        .expect_labeled_error()?;
    assert_eq!(err.msg, "boom");
    Ok(())
}

#[test]
fn streams_a_table() -> Result {
    // `1..` never ends, so this only finishes if `record apply` streams.
    let code = "1.. | each {|i| {a: ($i | into string)}} | record apply {a: {into int}} | first 2";

    test()
        .run(code)
        .expect_value_eq(test_value!([{a: 1}, {a: 2}]))
}
