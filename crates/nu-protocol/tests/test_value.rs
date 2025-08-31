use nu_protocol::{
    Config, Span, Value,
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

#[test]
fn test_case_insensitive_env_var() {
    let mut engine_state = EngineState::new();
    let stack = Stack::new();

    for (name, value) in std::env::vars() {
        engine_state.add_env_var(name, Value::test_string(value));
    }

    let path_lower = engine_state.get_env_var_insensitive("path");
    let path_upper = engine_state.get_env_var_insensitive("PATH");
    let path_mixed = engine_state.get_env_var_insensitive("PaTh");

    assert_eq!(path_lower, path_upper);
    assert_eq!(path_lower, path_mixed);

    let stack_path_lower = stack.get_env_var_insensitive(&engine_state, "path");
    let stack_path_upper = stack.get_env_var_insensitive(&engine_state, "PATH");
    let stack_path_mixed = stack.get_env_var_insensitive(&engine_state, "PaTh");

    assert_eq!(stack_path_lower, stack_path_upper);
    assert_eq!(stack_path_lower, stack_path_mixed);
}
