use nu_test_support::prelude::*;

#[test]
fn keeps_fields_in_input_order() -> Result {
    // `to nuon` keeps the field order, which comparing records ignores.
    test()
        .run("{c: 3, a: 1, b: 2} | record where {|key, value| $value != 1} | to nuon")
        .expect_value_eq("{c: 3, b: 2}")
}

#[test]
fn single_parameter_closure_receives_key() -> Result {
    test()
        .run("{a: 1, b: 2} | record where {|key| $key == b}")
        .expect_value_eq(test_value!({b: 2}))
}

#[test]
fn input_is_empty_without_flags() -> Result {
    test()
        .run("{a: 1} | record where { $in == null }")
        .expect_value_eq(test_value!({a: 1}))
}

#[test]
fn keys_flag_passes_key_as_argument_and_input() -> Result {
    // Without `--keys`, `$in` is empty and this keeps nothing.
    test()
        .run("{a: 1, b: 2} | record where --keys {|key| $key == b and $in == b}")
        .expect_value_eq(test_value!({b: 2}))
}

#[test]
fn values_flag_passes_value_as_argument() -> Result {
    test()
        .run("{a: 1, b: 2} | record where --values {|value| $value == 1}")
        .expect_value_eq(test_value!({a: 1}))
}

#[test]
fn non_bool_result_drops_field() -> Result {
    test()
        .run("{a: 1, b: 2} | record where {|key, value| $value}")
        .expect_value_eq(test_value!({}))
}

#[test]
fn keys_and_values_flags_conflict() -> Result {
    test()
        .run("{a: 1} | record where --keys --values { true }")
        .expect_error_code_eq("nu::shell::incompatible_parameters")
}

#[test]
fn rejects_non_record_input() -> Result {
    test()
        .run_with_data("record where { true }", test_value!(1))
        .expect_error_code_eq("nu::shell::only_supports_this_input_type")
}

#[test]
#[deps(TESTBIN_IECHO)]
fn rejects_byte_stream_without_reading_it() -> Result {
    // `iecho` never stops writing, so this only finishes if the stream is rejected
    // unread. An external's stream has no known type, so the engine lets it through.
    test()
        .run("iecho x | record where { true }")
        .expect_error_code_eq("nu::shell::only_supports_this_input_type")
}

#[test]
fn streams_a_table() -> Result {
    // `1..` never ends, so this only finishes if `record where` streams.
    let code = "1.. | each {|i| {a: $i, b: 0}} | record where {|key, value| $value != 0} | first 2";

    test()
        .run(code)
        .expect_value_eq(test_value!([{a: 1}, {a: 2}]))
}

#[test]
fn keeps_metadata_of_a_table() -> Result {
    let code = "
        [{a: 1}] | metadata set --content-type text/x-test
        | record where { true } | metadata | get content_type
    ";

    test().run(code).expect_value_eq("text/x-test")
}

#[test]
fn non_record_row_becomes_an_error_in_its_place() -> Result {
    test()
        .run("[{a: 1} 2 {a: 3}] | record where { true } | get 1")
        .expect_error_code_eq("nu::shell::only_supports_this_input_type")
}

#[test]
fn rows_after_a_bad_row_are_still_processed() -> Result {
    test()
        .run("[{a: 1} 2 {a: 3}] | record where { true } | last")
        .expect_value_eq(test_value!({a: 3}))
}

#[test]
fn missing_parameter_in_a_row_does_not_shift_the_next_row() -> Result {
    // `--keys` passes one argument, so `v` is missing for every row. The first
    // row's failed call must not leave `k` bound, or the second row's key would
    // fill `v` and the call would succeed.
    test()
        .run("[{a: 1} {b: 2}] | record where --keys {|k, v| true} | get 1")
        .expect_error_code_eq("nu::shell::missing_parameter")
}

#[test]
fn closure_error_is_returned() -> Result {
    let err = test()
        .run("{a: 1} | record where {|key, value| error make {msg: boom}}")
        .expect_labeled_error()?;
    assert_eq!(err.msg, "boom");
    Ok(())
}
