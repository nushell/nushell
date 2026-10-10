use nu_test_support::prelude::*;

#[test]
fn duplicate_key_keeps_first_position_and_last_value() -> Result {
    // `to nuon` keeps the field order, which comparing records ignores.
    let code = "
        {a: 1, b: 2, c: 3}
        | record each {|key, value| if $key == c { {a: 30} } else { {$key: $value} } }
        | to nuon
    ";

    test().run(code).expect_value_eq("{a: 30, b: 2}")
}

#[test]
fn returned_record_adds_every_field() -> Result {
    test()
        .run("{a: 1} | record each {|key, value| {$key: $value, $'($key)_copy': $value}}")
        .expect_value_eq(test_value!({a: 1, a_copy: 1}))
}

#[test]
fn null_and_empty_record_drop_the_field() -> Result {
    test()
        .run("{a: 1, b: 2} | record each {|key, value| if $key == a { {} } }")
        .expect_value_eq(test_value!({}))
}

#[test]
fn pair_key_is_coerced_to_string() -> Result {
    test()
        .run("{a: 1} | record each {|key, value| [$value $key]}")
        .expect_value_eq(test_value!({"1": "a"}))
}

#[test]
fn input_is_empty() -> Result {
    test()
        .run("{a: 1} | record each {|key, value| {$key: $in}}")
        .expect_value_eq(test_value!({a: ()}))
}

#[test]
fn empty_record_stays_empty() -> Result {
    test()
        .run("{} | record each {|key, value| {x: 1}}")
        .expect_value_eq(test_value!({}))
}

#[test]
fn list_of_wrong_length_errors() -> Result {
    test()
        .run("{a: 1} | record each {|key, value| [$key $value 3]}")
        .expect_error_code_eq("nu::shell::incorrect_value")
}

#[test]
fn other_result_type_errors() -> Result {
    test()
        .run("{a: 1} | record each {|key, value| $value}")
        .expect_error_code_eq("nu::shell::type_mismatch")
}

#[test]
fn streams_a_table() -> Result {
    // `1..` never ends, so this only finishes if `record each` streams.
    let code =
        "1.. | each {|i| {a: $i}} | record each {|key, value| {$key: ($value * 10)}} | first 2";

    test()
        .run(code)
        .expect_value_eq(test_value!([{a: 10}, {a: 20}]))
}

#[test]
fn failed_row_does_not_shift_arguments_of_the_next_row() -> Result {
    // The first row fails while binding `value`. The second row must still bind its
    // own key and value instead of reusing the first row's key.
    let code = "
        let pair = {|key, value: string| [$key $value]}
        [{a: 1} {b: y}] | record each $pair | skip 1
    ";

    test().run(code).expect_value_eq(test_value!([{b: "y"}]))
}
