use nu_protocol::SUPPORTED_DURATION_UNITS;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

// Tests happy paths

#[test]
fn into_duration_float() -> Result {
    test()
        .run("1.07min | into duration | into string")
        .expect_value_eq("1min 4sec 200ms")
}

#[test]
fn into_duration_from_record_cell_path() -> Result {
    test()
        .run("{d: '1hr'} | into duration d | get d | into string")
        .expect_value_eq("1hr")
}

#[test]
fn into_duration_from_record() -> Result {
    let code = "
        {week: 10, day: 1, hour: 2, minute: 3, second: 4, millisecond: 5, microsecond: 6, nanosecond: 7, sign: '+'}
        | into duration
        | into record
    ";

    test().run(code).expect_value_eq(test_value!({
        week: 10,
        day: 1,
        hour: 2,
        minute: 3,
        second: 4,
        millisecond: 5,
        microsecond: 6,
        nanosecond: 7,
        sign: "+",
    }))
}

#[test]
fn into_duration_from_record_negative() -> Result {
    let code = "
        {week: 10, day: 1, hour: 2, minute: 3, second: 4, millisecond: 5, microsecond: 6, nanosecond: 7, sign: '-'}
        | into duration
        | into record
    ";

    test().run(code).expect_value_eq(test_value!({
        week: 10,
        day: 1,
        hour: 2,
        minute: 3,
        second: 4,
        millisecond: 5,
        microsecond: 6,
        nanosecond: 7,
        sign: "-",
    }))
}

#[test]
fn into_duration_from_record_defaults() -> Result {
    test()
        .run("{} | into duration | into int")
        .expect_value_eq(0)
}

#[test]
fn into_duration_from_record_round_trip() -> Result {
    test()
        .run("'10wk 1day 2hr 3min 4sec 5ms 6µs 7ns' | into duration | into record | into duration | into string")
        .expect_value_eq("10wk 1day 2hr 3min 4sec 5ms 6µs 7ns")
}

#[test]
fn into_duration_table_column() -> Result {
    let code = "
        [[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']]
        | into duration value
        | update value {|row| $row.value | into string }
    ";

    test().run(code).expect_value_eq(test_table![
        ["value"];
        ["1sec"],
        ["2min"],
        ["3hr"],
        ["4day"],
        ["5wk"],
    ])
}

#[rstest]
#[case::hours_and_minutes("'3:34:00' | into duration", "3hr 34min")]
#[case::millis("'16:59:58.235' | into duration", "16hr 59min 58sec 235ms")]
#[case::tenths("'2:45:31.2' | into duration", "2hr 45min 31sec 200ms")]
#[case::hundredths("'2:45:31.23' | into duration", "2hr 45min 31sec 230ms")]
#[case::four_fraction_digits("'2:45:31.2345' | into duration", "2hr 45min 31sec 234ms 500µs")]
#[case::micros("'16:59:58.235123' | into duration", "16hr 59min 58sec 235ms 123µs")]
#[case::nanos(
    "'16:59:58.235123456' | into duration",
    "16hr 59min 58sec 235ms 123µs 456ns"
)]
fn into_duration_colon_string(#[case] code: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{code} | into string"))
        .expect_value_eq(expected)
}

#[test]
fn into_duration_clock_error_two() -> Result {
    let err = test().run("'3:34' | into duration").expect_shell_error()?;

    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_contains("hh:mm:ss", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}

// Tests error paths

#[test]
fn into_duration_from_record_fails_with_wrong_type() -> Result {
    let err = test()
        .run("{week: '10'} | into duration")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::OnlySupportsThisInputType {
            exp_input_type,
            wrong_type,
            ..
        } if exp_input_type == "int" && wrong_type == "string"
    );
    Ok(())
}

#[test]
fn into_duration_from_record_fails_with_invalid_date_time_values() -> Result {
    let err = test()
        .run("{week: -10} | into duration")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::IncorrectValue { msg, .. } if msg == "number should be positive"
    );
    Ok(())
}

#[test]
fn into_duration_from_record_fails_with_invalid_sign() -> Result {
    let err = test()
        .run("{week: 10, sign: 'x'} | into duration")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::IncorrectValue { msg, .. } if msg == "Invalid sign. Allowed signs are +, -"
    );
    Ok(())
}

// Tests invalid usage

#[rstest]
#[case::invalid_unit("1 | into duration --unit xx")]
#[case::filesize_unit("1 | into duration --unit MB")]
fn into_duration_invalid_unit(#[case] code: &str) -> Result {
    let err = test().run(code).expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::InvalidUnit { supported_units, .. }
            if supported_units == SUPPORTED_DURATION_UNITS.join(", ")
    );
    Ok(())
}

#[test]
fn into_duration_from_record_fails_with_unknown_key() -> Result {
    let err = test()
        .run("{week: 10, unknown: 1} | into duration")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::UnsupportedInput { msg, .. }
            if msg == "Column 'unknown' is not valid for a structured duration. Allowed columns are: week, day, hour, minute, second, millisecond, microsecond, nanosecond, sign"
    );
    Ok(())
}

#[test]
fn into_duration_from_record_incompatible_with_unit_flag() -> Result {
    let code = "
        {week: 10, day: 1, hour: 2, minute: 3, second: 4, sign: '-'}
        | into duration --unit sec
    ";

    let err = test().run(code).expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::IncompatibleParameters {
            left_message,
            right_message,
            ..
        } if left_message == "got a record as input"
            && right_message == "the units should be included in the record"
    );
    Ok(())
}
