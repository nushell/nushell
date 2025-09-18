use nu_test_support::nu;

// Tests happy paths

#[test]
fn into_datetime_from_record_cell_path() {
    let actual = nu!(r#"{d: '2021'} | into datetime d"#);

    assert!(actual.out.contains("years ago"));
}

#[test]
fn into_datetime_from_record() {
    let actual = nu!(
        r#"{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5, millisecond: 6, microsecond: 7, nanosecond: 8, timezone: '+01:00'} | into datetime | into record"#
    );
    let expected = nu!(
        r#"{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5, millisecond: 6, microsecond: 7, nanosecond: 8, timezone: '+01:00'}"#
    );

    assert_eq!(expected.out, actual.out);
}

#[test]
fn into_datetime_from_record_very_old() {
    let actual = nu!(r#"{year: -100, timezone: '+02:00'} | into datetime | into record"#);
    let expected = nu!(
        r#"{year: -100, month: 1, day: 1, hour: 0, minute: 0, second: 0, millisecond: 0, microsecond: 0, nanosecond: 0, timezone: '+02:00'}"#
    );

    assert_eq!(expected.out, actual.out);
}

#[test]
fn into_datetime_from_record_defaults() {
    let actual = nu!(r#"{year: 2025, timezone: '+02:00'} | into datetime | into record"#);
    let expected = nu!(
        r#"{year: 2025, month: 1, day: 1, hour: 0, minute: 0, second: 0, millisecond: 0, microsecond: 0, nanosecond: 0, timezone: '+02:00'}"#
    );

    assert_eq!(expected.out, actual.out);
}

#[test]
fn into_datetime_from_record_round_trip() {
    let actual = nu!(
        r#"(1743348798 | into datetime | into record | into datetime | into int) == 1743348798"#
    );

    assert!(actual.out.contains("true"));
}

#[test]
fn into_datetime_table_column() {
    let actual = nu!(r#"[[date]; ["2022-01-01"] ["2023-01-01"]] | into datetime date"#);

    assert!(actual.out.contains(" ago"));
}

// Tests error paths

#[test]
fn into_datetime_from_record_fails_with_wrong_type() {
    let actual = nu!(r#"{year: '2023'} | into datetime"#);

    assert!(
        actual
            .err
            .contains("nu::shell::only_supports_this_input_type")
    );
}

#[test]
fn into_datetime_from_record_fails_with_invalid_date_time_values() {
    let actual = nu!(r#"{year: 2023, month: 13} | into datetime"#);

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}

#[test]
fn into_datetime_from_record_fails_with_invalid_timezone() {
    let actual = nu!(r#"{year: 2023, timezone: '+100:00'} | into datetime"#);

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}

// Tests invalid usage

#[test]
fn into_datetime_from_record_fails_with_unknown_key() {
    let actual = nu!(r#"{year: 2023, unknown: 1} | into datetime"#);

    assert!(actual.err.contains("nu::shell::unsupported_input"));
}

#[test]
fn into_datetime_from_record_incompatible_with_format_flag() {
    let actual = nu!(
        r#"{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5} | into datetime --format ''"#
    );

    assert!(actual.err.contains("nu::shell::incompatible_parameters"));
}

#[test]
fn into_datetime_from_record_incompatible_with_timezone_flag() {
    let actual = nu!(
        r#"{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5} | into datetime --timezone UTC"#
    );

    assert!(actual.err.contains("nu::shell::incompatible_parameters"));
}

#[test]
fn into_datetime_from_record_incompatible_with_offset_flag() {
    let actual = nu!(
        r#"{year: 2023, month: 1, day: 2, hour: 3, minute: 4, second: 5} | into datetime --offset 1"#
    );

    assert!(actual.err.contains("nu::shell::incompatible_parameters"));
}

#[test]
fn test_j_q_format_specifiers_into_datetime() {
    let actual = nu!(r#"
        "20211022_200012" | into datetime --format '%J_%Q'
        "#);

    // Check for the date components - the exact output format may vary
    assert!(actual.out.contains("22 Oct 2021") || actual.out.contains("2021-10-22"));
    assert!(actual.out.contains("20:00:12"));
}

#[test]
fn test_j_q_format_specifiers_round_trip() {
    let actual = nu!(r#"
        "2021-10-22 20:00:12 +01:00" | format date '%J_%Q' | into datetime --format '%J_%Q' | format date '%J_%Q'
        "#);

    assert_eq!(actual.out, "20211022_200012");
}

#[test]
fn test_j_format_specifier_date_only() {
    let actual = nu!(r#"
        "20211022" | into datetime --format '%J'
        "#);

    // Check for the date components - time should default to midnight
    assert!(actual.out.contains("22 Oct 2021") || actual.out.contains("2021-10-22"));
    assert!(actual.out.contains("00:00:00"));
}

#[test]
fn test_q_format_specifier_time_only() {
    let actual = nu!(r#"
        "200012" | into datetime --format '%Q'
        "#);

    // Check for the time components - should parse as time with default date
    assert!(actual.out.contains("20:00:12"));
}
