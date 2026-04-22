use nu_test_support::prelude::*;

// Tests happy paths

#[test]
fn into_datetime_from_record_cell_path() -> Result {
    test()
        .run("{d: '2021'} | into datetime d | get d | into record | get year")
        .expect_value_eq(2021)
}

#[test]
fn into_datetime_from_record() -> Result {
    test().run(
        "{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5, millisecond: 6, microsecond: 7, nanosecond: 8, timezone: '+01:00'} | into datetime | into record",
    )
    .expect_value_eq(
        nu_protocol::record! {
            "year" => nu_protocol::Value::test_int(2023),
            "month" => nu_protocol::Value::test_int(1),
            "day" => nu_protocol::Value::test_int(2),
            "hour" => nu_protocol::Value::test_int(3),
            "minute" => nu_protocol::Value::test_int(4),
            "second" => nu_protocol::Value::test_int(5),
            "millisecond" => nu_protocol::Value::test_int(6),
            "microsecond" => nu_protocol::Value::test_int(7),
            "nanosecond" => nu_protocol::Value::test_int(8),
            "timezone" => nu_protocol::Value::test_string("+01:00"),
        },
    )
}

#[test]
fn into_datetime_from_record_very_old() -> Result {
    test()
        .run("{year: -100, timezone: '+02:00'} | into datetime | into record")
        .expect_value_eq(nu_protocol::record! {
            "year" => nu_protocol::Value::test_int(-100),
            "month" => nu_protocol::Value::test_int(1),
            "day" => nu_protocol::Value::test_int(1),
            "hour" => nu_protocol::Value::test_int(0),
            "minute" => nu_protocol::Value::test_int(0),
            "second" => nu_protocol::Value::test_int(0),
            "millisecond" => nu_protocol::Value::test_int(0),
            "microsecond" => nu_protocol::Value::test_int(0),
            "nanosecond" => nu_protocol::Value::test_int(0),
            "timezone" => nu_protocol::Value::test_string("+02:00"),
        })
}

#[test]
fn into_datetime_from_record_defaults() -> Result {
    test()
        .run("{year: 2025, timezone: '+02:00'} | into datetime | into record")
        .expect_value_eq(nu_protocol::record! {
            "year" => nu_protocol::Value::test_int(2025),
            "month" => nu_protocol::Value::test_int(1),
            "day" => nu_protocol::Value::test_int(1),
            "hour" => nu_protocol::Value::test_int(0),
            "minute" => nu_protocol::Value::test_int(0),
            "second" => nu_protocol::Value::test_int(0),
            "millisecond" => nu_protocol::Value::test_int(0),
            "microsecond" => nu_protocol::Value::test_int(0),
            "nanosecond" => nu_protocol::Value::test_int(0),
            "timezone" => nu_protocol::Value::test_string("+02:00"),
        })
}

#[test]
fn into_datetime_from_record_round_trip() -> Result {
    test()
        .run("(1743348798 | into datetime | into record | into datetime | into int) == 1743348798")
        .expect_value_eq(true)
}

#[test]
fn into_datetime_table_column() -> Result {
    let dt: String =
        test().run(r#"[[date]; ["2022-01-01"] ["2023-01-01"]] | into datetime date | to text"#)?;
    assert_contains("ago", dt);
    Ok(())
}

// Tests error paths

#[test]
fn into_datetime_from_record_fails_with_wrong_type() -> Result {
    let err = test()
        .run("{year: '2023'} | into datetime")
        .expect_shell_error()?;

    match err {
        ShellError::OnlySupportsThisInputType {
            exp_input_type,
            wrong_type,
            ..
        } => {
            assert_eq!(exp_input_type, "int");
            assert_eq!(wrong_type, "string");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn into_datetime_from_record_fails_with_invalid_date_time_values() -> Result {
    let err = test()
        .run("{year: 2023, month: 13} | into datetime")
        .expect_shell_error()?;

    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_eq!(
                msg,
                "one of more values are incorrect and do not represent valid date"
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn into_datetime_from_record_fails_with_invalid_timezone() -> Result {
    let err = test()
        .run("{year: 2023, timezone: '+100:00'} | into datetime")
        .expect_shell_error()?;

    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_eq!(msg, "invalid timezone");
            Ok(())
        }
        err => Err(err.into()),
    }
}

// Tests invalid usage

#[test]
fn into_datetime_from_record_fails_with_unknown_key() -> Result {
    let err = test()
        .run("{year: 2023, unknown: 1} | into datetime")
        .expect_shell_error()?;

    match err {
        ShellError::UnsupportedInput { msg, .. } => {
            assert_eq!(
                msg,
                "Column 'unknown' is not valid for a structured datetime. Allowed columns are: year, month, day, hour, minute, second, millisecond, microsecond, nanosecond, timezone"
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn into_datetime_from_record_incompatible_with_format_flag() -> Result {
    let err = test()
        .run(
            "{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5} | into datetime --format ''",
        )
        .expect_shell_error()?;

    match err {
        ShellError::IncompatibleParameters {
            left_message,
            right_message,
            ..
        } => {
            assert_eq!(left_message, "got a record as input");
            assert_eq!(right_message, "cannot be used with records");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn into_datetime_from_record_incompatible_with_timezone_flag() -> Result {
    let err = test()
        .run(
            "{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5} | into datetime --timezone UTC",
        )
        .expect_shell_error()?;

    match err {
        ShellError::IncompatibleParameters {
            left_message,
            right_message,
            ..
        } => {
            assert_eq!(left_message, "got a record as input");
            assert_eq!(
                right_message,
                "the timezone should be included in the record"
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn into_datetime_from_record_incompatible_with_offset_flag() -> Result {
    let err = test()
        .run(
            "{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5} | into datetime --offset 1",
        )
        .expect_shell_error()?;

    match err {
        ShellError::IncompatibleParameters {
            left_message,
            right_message,
            ..
        } => {
            assert_eq!(left_message, "got a record as input");
            assert_eq!(
                right_message,
                "the timezone should be included in the record"
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn test_j_q_format_specifiers_into_datetime() -> Result {
    test()
        .run(r#""20211022_200012" | into datetime --format '%J_%Q' | format date '%J_%Q'"#)
        .expect_value_eq("20211022_200012")
}

#[test]
fn test_j_q_format_specifiers_round_trip() -> Result {
    test()
        .run(
            r#""2021-10-22 20:00:12 +01:00" | format date '%J_%Q' | into datetime --format '%J_%Q' | format date '%J_%Q'"#,
        )
        .expect_value_eq("20211022_200012")
}

#[test]
fn test_j_format_specifier_date_only() -> Result {
    test()
        .run(r#""20211022" | into datetime --format '%J' | format date '%J_%Q'"#)
        .expect_value_eq("20211022_000000")
}

#[test]
fn test_q_format_specifier_time_only() -> Result {
    // Check for the time components - should parse as time with default date
    let dt: String = test().run(r#""200012" | into datetime --format '%Q' | to nuon"#)?;
    assert_contains("20:00:12", dt);
    Ok(())
}

#[test]
fn formatted_input_applies_timezone_flag_as_wall_clock() -> Result {
    test()
        .run(r#""2026-03-21_00:25" | into datetime --format '%F_%R' --timezone utc | into record | get timezone"#)
        .expect_value_eq("+00:00")
}

#[test]
fn formatted_input_applies_short_timezone_flag_as_wall_clock() -> Result {
    test()
        .run(r#""2026-03-21_00:25" | into datetime -f '%F_%R' -z u | into record | get timezone"#)
        .expect_value_eq("+00:00")
}

#[test]
fn formatted_input_applies_offset_flag_as_wall_clock() -> Result {
    test()
        .run(r#""2026-03-21_00:25" | into datetime --format '%F_%R' --offset 2 | into record | get timezone"#)
        .expect_value_eq("+02:00")
}

#[test]
fn formatted_input_applies_short_offset_flag_as_wall_clock() -> Result {
    test()
        .run(r#""2026-03-21_00:25" | into datetime -f '%F_%R' -o 2 | into record | get timezone"#)
        .expect_value_eq("+02:00")
}

#[test]
fn formatted_input_offset_takes_precedence_over_timezone() -> Result {
    test()
        .run(r#""2026-03-21_00:25" | into datetime --format '%F_%R' --timezone utc --offset 3 | into record | get timezone"#)
        .expect_value_eq("+03:00")
}

#[test]
fn formatted_input_rejects_invalid_timezone_flag() -> Result {
    let err = test()
        .run(r#""2026-03-21_00:25" | into datetime --format '%F_%R' --timezone invalid"#)
        .expect_shell_error()?;

    match err {
        ShellError::TypeMismatch { err_message, .. } => {
            assert_eq!(err_message, "Invalid timezone or offset");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn formatted_input_rejects_invalid_offset_flag() -> Result {
    let err = test()
        .run(r#""2026-03-21_00:25" | into datetime --format '%F_%R' --offset 15"#)
        .expect_shell_error()?;

    match err {
        ShellError::TypeMismatch { err_message, .. } => {
            assert_eq!(err_message, "Invalid timezone or offset");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn list_flag_produces_available_format_entries() -> Result {
    let len: i64 = test().run("into datetime --list | length")?;
    assert_ne!(len, 0);
    Ok(())
}
