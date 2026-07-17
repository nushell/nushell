use nu_protocol::{
    Config, DurationFormat, Span, Value,
    engine::{EngineState, Stack},
};
use rstest::rstest;

#[test]
fn test_comparison_nothing() {
    let values = vec![
        Value::test_int(1),
        Value::test_string("string"),
        Value::test_float(1.0),
    ];

    let nothing = Value::nothing(Span::test_data());

    for value in values {
        assert!(matches!(
            value.eq(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool { val: false, .. })
        ));

        assert!(matches!(
            value.ne(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));

        assert!(matches!(
            nothing.eq(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool { val: false, .. })
        ));

        assert!(matches!(
            nothing.ne(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));
    }
}

#[test]
fn test_float_equality_comparison() {
    let values = vec![
        (
            Value::test_float(0.30000000000000004),
            Value::test_float(0.3),
        ),
        (
            Value::test_float(0.1),
            #[allow(clippy::excessive_precision)]
            Value::test_float(0.10000000000000001),
        ),
        (
            Value::test_float(1.0000000000000002),
            Value::test_float(1.0),
        ),
        (
            Value::test_float(2.220446049250313e-16),
            Value::test_float(0.0),
        ),
        (Value::test_float(1e-16), Value::test_float(0.0)),
        (
            #[allow(clippy::eq_op)]
            Value::test_float((1e16 + 1.0) - 1e16),
            Value::test_float(0.0),
        ),
        (
            Value::test_float(10000000000000000.0),
            Value::test_float(10000000000000002.0),
        ),
        (
            Value::test_float(9007199254740992.0),
            Value::test_float(9007199254740993.0),
        ),
        (
            Value::test_float(4503599627370496.0),
            Value::test_float(4503599627370497.0),
        ),
        (
            Value::test_float(1.7976931348623157e308),
            Value::test_float(1.7976931348623155e308),
        ),
        (
            Value::test_float(1.0),
            Value::test_float(0.9999999999999999),
        ),
        (
            #[allow(clippy::approx_constant)]
            Value::test_float(3.141592653589793),
            Value::test_float(3.1415926535897927),
        ),
    ];

    for value in values {
        assert!(matches!(
            value.0.eq(Span::test_data(), &value.1, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));

        assert!(matches!(
            value.1.eq(Span::test_data(), &value.0, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));
    }
}

#[rstest]
#[case(365 * 24 * 3600 * 1_000_000_000, "52wk 1day")]
#[case( (((((((7 + 2) * 24 + 3) * 60 + 4) * 60) + 5) * 1000 + 6) * 1000 + 7) * 1000 + 8,
"1wk 2day 3hr 4min 5sec 6ms 7µs 8ns")]
fn test_duration_to_string(#[case] in_ns: i64, #[case] expected: &str) {
    let dur = Value::test_duration(in_ns);
    assert_eq!(
        expected,
        dur.to_expanded_string("", &Config::default()),
        "expected != observed"
    );
}

// 365 days = 52wk 1day with default (wk) max unit
const ONE_YEAR_NS: i64 = 365 * 24 * 3600 * 1_000_000_000;
// 1wk 2day 3hr 4min 5sec 6ms 7µs 8ns
const MIXED_DURATION_NS: i64 =
    ((((((7 + 2) * 24 + 3) * 60 + 4) * 60 + 5) * 1000 + 6) * 1000 + 7) * 1000 + 8;

fn config_with_duration_format(format: DurationFormat) -> Config {
    Config {
        duration_format: format,
        ..Default::default()
    }
}

#[rstest]
#[case::day_max(ONE_YEAR_NS, DurationFormat::Day, "365day")]
#[case::hr_max(ONE_YEAR_NS, DurationFormat::Hour, "8760hr")]
#[case::min_max(ONE_YEAR_NS, DurationFormat::Minute, "525600min")]
#[case::sec_max(ONE_YEAR_NS, DurationFormat::Second, "31536000sec")]
#[case::ms_max(3_000_000_000, DurationFormat::Millisecond, "3000ms")]
#[case::mixed_day_max(
    MIXED_DURATION_NS,
    DurationFormat::Day,
    "9day 3hr 4min 5sec 6ms 7µs 8ns"
)]
#[case::mixed_hr_max(MIXED_DURATION_NS, DurationFormat::Hour, "219hr 4min 5sec 6ms 7µs 8ns")]
#[case::mixed_min_max(MIXED_DURATION_NS, DurationFormat::Minute, "13144min 5sec 6ms 7µs 8ns")]
#[case::mixed_sec_max(MIXED_DURATION_NS, DurationFormat::Second, "788645sec 6ms 7µs 8ns")]
#[case::mixed_ms_max(MIXED_DURATION_NS, DurationFormat::Millisecond, "788645006ms 7µs 8ns")]
#[case::mixed_us_max(MIXED_DURATION_NS, DurationFormat::Microsecond, "788645006007µs 8ns")]
#[case::mixed_ns_max(MIXED_DURATION_NS, DurationFormat::Nanosecond, "788645006007008ns")]
#[case::wk_max_unchanged(ONE_YEAR_NS, DurationFormat::Week, "52wk 1day")]
fn test_duration_format_config(
    #[case] in_ns: i64,
    #[case] max_unit: DurationFormat,
    #[case] expected: &str,
) {
    let dur = Value::test_duration(in_ns);
    let config = config_with_duration_format(max_unit);
    assert_eq!(
        expected,
        dur.to_expanded_string("", &config),
        "duration_format={max_unit:?} for {in_ns}ns"
    );
}

#[test]
fn test_case_insensitive_env_var() {
    let mut engine_state = EngineState::new();
    let stack = Stack::new();

    for (name, value) in std::env::vars() {
        engine_state.add_env_var(name, Value::test_string(value));
    }

    let path_lower = engine_state.get_env_var("path");
    let path_upper = engine_state.get_env_var("PATH");
    let path_mixed = engine_state.get_env_var("PaTh");

    assert_eq!(path_lower, path_upper);
    assert_eq!(path_lower, path_mixed);

    let stack_path_lower = stack.get_env_var(&engine_state, "path");
    let stack_path_upper = stack.get_env_var(&engine_state, "PATH");
    let stack_path_mixed = stack.get_env_var(&engine_state, "PaTh");

    assert_eq!(stack_path_lower, stack_path_upper);
    assert_eq!(stack_path_lower, stack_path_mixed);
}
