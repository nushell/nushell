use chrono::{DateTime, FixedOffset};
use nu_protocol::{Config, Span, Value};
use rstest::rstest;

#[test]
fn test_comparison_nothing() {
    let values = vec![
        Value::test_int(1),
        Value::test_string("string"),
        Value::test_float(1.0),
    ];

    let nothing = Value::Nothing {
        span: Span::test_data(),
    };

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

#[rstest]
#[case(Value::test_string("foo"), "'foo'")]
#[case(Value::test_string("foo bar"), "'foo bar'")]
#[case(
    Value::test_string("contains 'single' quote"),
    "'contains 'single' quote'"
)]
#[case(Value::test_int(10), "10")]
#[case(Value::test_date(DateTime::<FixedOffset>::parse_from_rfc3339("2023-10-11T14:22:33.000111222-05:00").expect("manifest constant")),
     "2023-10-11T14:22:33.000111222-05:00")]
#[case(Value::test_record(vec!("a", "b", "ccc"), vec!(
    Value::test_float(3.14), Value::test_string("contains'single'quote"), Value::test_string("embedded space")
)),
"{a: 3.14, b: 'contains'single'quote', ccc: 'embedded space'}")]
#[case(Value::test_list(vec!(
    Value::test_string("embedded space"), Value::test_string(r#"embedded "quotes" and more"#)
)),
    r#"['embedded space', 'embedded "quotes" and more']"#)]
fn test_into_string_parsable(#[case] val: Value, #[case] expected: &str) {
    let _inval = val.into_string_parsable(",", &Config::default());
    assert_eq!(
        val.into_string_parsable(", ", &Config::default()),
        expected,
        "actual == expected"
    );
}
