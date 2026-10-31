use chrono::DateTime;
use nu_protocol::casing::Casing;
use nu_protocol::{ast::PathMember, record};

use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

// Tests happy paths

const TEST_DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S%z";

#[rstest]
#[case::string(
    "{
        name: 'foo',
        start_time: '2021-02-27T14:55:40+01:00',
        stop_time: '2021-02-27T15:55:40+01:00',
    }",
    _,
    test_record! {
        "name" => "foo",
        "start_time" => "2021-02-27T14:55:40+01:00",
        "stop_time" => "2021-02-27T15:55:40+01:00",
    },
)]
#[case::record(
    "{
        name: 'foo',
        start_time: {
            year: 2021,
            month: 2,
            day: 27,
            hour: 14,
            minute: 55,
            second: 40,
            timezone: '+01:00',
        },
        stop_time: {
            year: 2021,
            month: 2,
            day: 27,
            hour: 15,
            minute: 55,
            second: 40,
            timezone: '+01:00',
        },
    }",
    _,
    test_record! {
        "name" => "foo",
        "start_time" => "2021-02-27T14:55:40+01:00",
        "stop_time" => "2021-02-27T15:55:40+01:00",
    },
)]
#[case::duration(
    "{
        name: 'foo',
        start_time: 1_614_434_140_000ms,
        stop_time: 1_614_437_740sec,
    }",
    _,
    test_record! {
        "name" => "foo",
        "start_time" => "2021-02-27T13:55:40+00:00",
        "stop_time" => "2021-02-27T14:55:40+00:00",
    },
)]
#[case::duration_negative(
    "{
        name: 'foo',
        start_time: -2day,
        stop_time: -1day,
    }",
    _,
    test_record! {
        "name" => "foo",
        "start_time" => "1969-12-30T00:00:00+00:00",
        "stop_time" => "1969-12-31T00:00:00+00:00",
    },
)]
#[case::duration_with_timezone(
    "{
        name: 'foo',
        start_time: 1_614_434_140_000ms,
        stop_time: 1_614_437_740sec,
    }",
    "--timezone utc",
    test_record! {
        "name" => "foo",
        "start_time" => "2021-02-27T13:55:40+00:00",
        "stop_time" => "2021-02-27T14:55:40+00:00",
    },
)]
#[case::duration_with_timezone(
    "{
        name: 'foo',
        start_time: 1_614_434_140_000ms,
        stop_time: 1_614_437_740sec,
    }",
    "--timezone America/New_York",
    test_record! {
        "name" => "foo",
        "start_time" => "2021-02-27T08:55:40-05:00",
        "stop_time" => "2021-02-27T09:55:40-05:00",
    },
)]
#[case::duration_with_offset(
    "{
        name: 'foo',
        start_time: 1_614_434_140_000ms,
        stop_time: 1_614_437_740sec,
    }",
    "--offset -6",
    test_record! {
        "name" => "foo",
        "start_time" => "2021-02-27T07:55:40-06:00",
        "stop_time" => "2021-02-27T08:55:40-06:00",
    },
)]
fn into_datetime_cell_path(
    #[case] input: &str,
    #[case(opt)] args: Option<&str>,
    #[case] mut expected: Value,
) -> Result {
    for col in ["start_time", "stop_time"] {
        expected.update_cell_path(
            &[PathMember::test_string(col, false, Casing::Sensitive)],
            Box::new(|val| {
                Value::test_date(
                    DateTime::parse_from_str(val.as_str().unwrap(), TEST_DATETIME_FMT).unwrap(),
                )
            }),
        )?;
    }

    let code = format!(
        "$in | from nuon | into datetime start_time stop_time {}",
        args.unwrap_or("")
    );

    let actual = test().run_with_data(code, input);

    actual.clone().expect_value_eq(expected.clone())?;

    let actual = actual?;

    for col in ["start_time", "stop_time"] {
        let actual_date = actual.get_data_by_key(col).unwrap().as_date()?;
        let expected_date = expected.get_data_by_key(col).unwrap().as_date()?;

        assert_eq!(expected_date.timezone(), actual_date.timezone());
        assert_eq!(expected_date.offset(), actual_date.offset());
    }

    Ok(())
}

#[test]
fn into_datetime_from_record() -> Result {
    test().run(
        "{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5, millisecond: 6, microsecond: 7, nanosecond: 8, timezone: '+01:00'} | into datetime | into record",
    )
    .expect_value_eq(
        record! {
            "year" => Value::test_int(2023),
            "month" => Value::test_int(1),
            "day" => Value::test_int(2),
            "hour" => Value::test_int(3),
            "minute" => Value::test_int(4),
            "second" => Value::test_int(5),
            "millisecond" => Value::test_int(6),
            "microsecond" => Value::test_int(7),
            "nanosecond" => Value::test_int(8),
            "timezone" => Value::test_string("+01:00"),
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

#[rstest]
#[case::winter(1_704_067_200_000_000_000, "2024-01-01 01:00 +01:00")]
#[case::summer(1_719_792_000_000_000_000, "2024-07-01 02:00 +02:00")]
fn named_timezone_converts_nanosecond_timestamp(
    #[case] timestamp: i64,
    #[case] expected: &str,
) -> Result {
    let code = "
        $in
        | into datetime --timezone Europe/Berlin
        | format date '%Y-%m-%d %H:%M %:z'
    ";

    test()
        .run_with_data(code, timestamp)
        .expect_value_eq(expected)
}

#[rstest]
#[case::winter("2024-01-01 00:00 +00:00", "2024-01-01 01:00 +01:00")]
#[case::summer("2024-07-01 00:00 +00:00", "2024-07-01 02:00 +02:00")]
fn named_timezone_converts_formatted_input_with_offset(
    #[case] input: &str,
    #[case] expected: &str,
) -> Result {
    let code = "
        $in
        | into datetime --format '%Y-%m-%d %H:%M %z' --timezone Europe/Berlin
        | format date '%Y-%m-%d %H:%M %:z'
    ";

    test().run_with_data(code, input).expect_value_eq(expected)
}

#[rstest]
#[case::winter("2024-01-01 12:00", "2024-01-01 12:00 +01:00")]
#[case::summer("2024-07-01 12:00", "2024-07-01 12:00 +02:00")]
fn named_timezone_interprets_formatted_wall_clock(
    #[case] input: &str,
    #[case] expected: &str,
) -> Result {
    let code = "
        $in
        | into datetime --format '%Y-%m-%d %H:%M' --timezone eUrOpE/bErLiN
        | format date '%Y-%m-%d %H:%M %:z'
    ";

    test().run_with_data(code, input).expect_value_eq(expected)
}

#[test]
fn named_timezone_interprets_formatted_date_only() -> Result {
    let code = "
        '2024-07-01'
        | into datetime --format '%Y-%m-%d' --timezone Europe/Berlin
        | format date '%Y-%m-%d %H:%M %:z'
    ";

    test().run(code).expect_value_eq("2024-07-01 00:00 +02:00")
}

#[test]
fn named_timezone_interprets_formatted_time_only() -> Result {
    let code = "
        '12:30'
        | into datetime --format '%H:%M' --timezone Asia/Kolkata
        | format date '%H:%M %:z'
    ";

    test().run(code).expect_value_eq("12:30 +05:30")
}

#[rstest]
#[case::nonexistent("2024-03-31 02:30")]
#[case::ambiguous("2024-10-27 02:30")]
fn named_timezone_rejects_invalid_wall_clock(#[case] input: &str) -> Result {
    let code = "$in | into datetime --format '%Y-%m-%d %H:%M' --timezone Europe/Berlin";
    let err = test().run_with_data(code, input).expect_shell_error()?;
    assert_matches!(err, ShellError::DatetimeParseError { .. });
    Ok(())
}

#[test]
fn list_flag_produces_available_format_entries() -> Result {
    let len: i64 = test().run("into datetime --list | length")?;
    assert_ne!(len, 0);
    Ok(())
}

#[test]
fn into_datetime_list_of_strings() -> Result {
    let code =
        r#"["2023-03-30 10:10:07 -05:00", "2023-05-05 13:43:49 -05:00"] | into datetime | length"#;
    let actual: i64 = test().run(code)?;
    assert_eq!(actual, 2);
    Ok(())
}

#[test]
fn into_datetime_list_of_dates_is_noop_keeps_length() -> Result {
    let code = r#"["2023-03-30 10:10:07 -05:00", "2023-05-05 13:43:49 -05:00"] | into datetime | into datetime | length"#;
    let actual: i64 = test().run(code)?;
    assert_eq!(actual, 2);
    Ok(())
}

#[test]
fn into_datetime_from_any_typed_pipeline_is_datetime() -> Result {
    // `http get` is typed `any`. A catch-all `(any, table)` pair on `into datetime`
    // (used for `--list`) must not make this infer as `table`.
    let code = r#"
        def f []: nothing -> any { "2020-01-01" }
        let d: datetime = (f | into datetime)
        $d | describe
    "#;
    test().run(code).expect_value_eq("datetime")
}
