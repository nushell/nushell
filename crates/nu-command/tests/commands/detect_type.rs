use nu_protocol::{DataSource, IntoPipelineData, PipelineMetadata, Span, Value};
use nu_test_support::prelude::*;
use rstest::rstest;

enum ExpectTo {
    Keep,
    Drop,
}

#[rstest]
#[case("42")]
#[case("-42")]
#[case("3.14")]
#[case("6.02e23")]
#[case("inf")]
#[case("NaN")]
#[case("1_000")]
#[case("1,000")]
#[case("true")]
#[case("FALSE")]
#[case("+42")]
#[case("5ns")]
#[case("4hr 3min")]
#[case("2022-01-01")]
#[case("2022-01-01T00:00:00Z")]
fn whitespace_padded_values_remain_strings(
    #[case] input: &str,
    #[values(' ', '\t', '\n', char::from(0xa0))] whitespace: char,
    #[values("leading", "trailing", "surrounding")] position: &str,
    #[values(false, true)] prefer_filesize: bool,
) -> Result {
    let padded = match position {
        "leading" => format!("{whitespace}{input}"),
        "trailing" => format!("{input}{whitespace}"),
        "surrounding" => format!("{whitespace}{input}{whitespace}"),
        _ => unreachable!("test cases specify a padding position"),
    };
    let command = if prefer_filesize {
        "detect type --prefer-filesize"
    } else {
        "detect type"
    };
    test()
        .run_with_data(command, padded.clone())
        .expect_value_eq(padded)
}

#[rstest]
#[case::arabic_indic_integer("٤٢")]
#[case::fullwidth_integer("４２")]
#[case::arabic_indic_float("٤.٢")]
#[case::fullwidth_float("４.２")]
#[case::unicode_delimited_integer("１,０００")]
#[case::unicode_boolean("falſe")]
#[case::integer_overflow("99999999999999999999")]
#[case::positive_boundary_overflow("9223372036854775808")]
#[case::negative_boundary_overflow("-9223372036854775809")]
#[case::comma_overflow("999,999,999,999,999,999,999")]
#[case::underscore_overflow("999_999_999_999_999_999_999")]
#[case::mixed_separators("1,000_000")]
#[case::reversed_mixed_separators("1_000,000")]
#[case::invalid_ymd("2022-01-00")]
#[case::invalid_dmy("00/01/2022")]
#[case::invalid_mdy("01/00/2022")]
#[case::invalid_timestamp("2022-01-00T00:00:00Z")]
#[case::invalid_duration("5unknown")]
fn unconvertible_values_remain_strings(
    #[case] input: &str,
    #[values(false, true)] prefer_filesize: bool,
    #[values(false, true)] prefer_dmy: bool,
) -> Result {
    let command =
        format!("detect type --prefer-filesize={prefer_filesize} --prefer-dmy={prefer_dmy}");
    test().run_with_data(&command, input).expect_value_eq(input)
}

#[rstest]
#[case("42", Value::test_int(42))]
#[case("+42", Value::test_int(42))]
#[case("-42", Value::test_int(-42))]
#[case("9223372036854775807", Value::test_int(i64::MAX))]
#[case("-9223372036854775808", Value::test_int(i64::MIN))]
#[case("1,000,000", Value::test_int(1_000_000))]
#[case("1_000_000", Value::test_int(1_000_000))]
#[case("+1,000", Value::test_int(1_000))]
#[case("+1_000", Value::test_int(1_000))]
#[case("-1,000", Value::test_int(-1_000))]
#[case("-1_000", Value::test_int(-1_000))]
#[case("+3.5", Value::test_float(3.5))]
#[case("-3.5", Value::test_float(-3.5))]
#[case(".5", Value::test_float(0.5))]
#[case("3.", Value::test_float(3.0))]
#[case("6.02e23", Value::test_float(6.02e23))]
#[case("TrUe", Value::test_bool(true))]
#[case("FaLsE", Value::test_bool(false))]
#[case("5ns", Value::test_duration(5))]
#[case("4hr 3min", Value::test_duration(14_580_000_000_000))]
fn valid_values_convert(#[case] input: &str, #[case] expected: Value) -> Result {
    test()
        .run_with_data("detect type", input)
        .expect_value_eq(expected)
}

#[test]
fn rfc3339_date_keeps_offset_and_nanoseconds() -> Result {
    use chrono::{FixedOffset, TimeZone, Timelike};
    let expected = FixedOffset::east_opt(5 * 3600 + 30 * 60)
        .and_then(|offset| offset.with_ymd_and_hms(2025, 4, 18, 4, 4, 32).single())
        .and_then(|date| date.with_nanosecond(123_456_789))
        .expect("valid date");
    test()
        .run_with_data("detect type", "2025-04-18T04:04:32.123456789+05:30")
        .expect_value_eq(Value::test_date(expected))
}

#[rstest]
#[case("+1024")]
#[case("+1,024")]
#[case("+1_024")]
fn signed_integers_convert_to_filesizes(#[case] input: &str) -> Result {
    test()
        .run_with_data("detect type --prefer-filesize", input)
        .expect_value_eq(Value::test_filesize(1024))
}

#[rstest]
#[case("1", ExpectTo::Drop)]
#[case("01/02/2026", ExpectTo::Drop)]
#[case("true", ExpectTo::Drop)]
#[case("4hr 3min", ExpectTo::Drop)]
#[case("truee", ExpectTo::Keep)]
#[case("test", ExpectTo::Keep)]
#[case("", ExpectTo::Keep)]
#[case(" ", ExpectTo::Keep)]
#[case(" 42", ExpectTo::Keep)]
#[case(" 3.14", ExpectTo::Keep)]
#[case(" 1_000", ExpectTo::Keep)]
#[case(" true", ExpectTo::Keep)]
#[case(" 5ns", ExpectTo::Keep)]
#[case("5ns ", ExpectTo::Keep)]
#[case("٤٢", ExpectTo::Keep)]
#[case("falſe", ExpectTo::Keep)]
#[case("99999999999999999999", ExpectTo::Keep)]
#[case("999,999,999,999,999,999,999", ExpectTo::Keep)]
#[case("2022-01-00", ExpectTo::Keep)]
#[case("1,000_000", ExpectTo::Keep)]
#[case("+42", ExpectTo::Drop)]
fn content_type_metadata(#[case] input: &str, #[case] expect_to: ExpectTo) -> Result {
    let in_meta = Some(
        PipelineMetadata::default()
            .with_data_source(DataSource::FilePath("test.txt".into()))
            .with_content_type(Some("text/plain".into())),
    );

    let data = input
        .into_value(Span::test_data())
        .into_pipeline_data_with_metadata(in_meta.clone());

    let out_meta = test()
        .run_raw_with_data("detect type", data)?
        .body
        .take_metadata();

    let expected = match expect_to {
        ExpectTo::Keep => in_meta,
        ExpectTo::Drop => in_meta.map(|m| m.with_content_type(None)),
    };

    assert_eq!(expected, out_meta);

    Ok(())
}
