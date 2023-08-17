use nu_protocol::{Config, Span, SpannedValue};
use rstest::rstest;

#[test]
fn test_comparison_nothing() {
    let values = vec![
        SpannedValue::test_int(1),
        SpannedValue::test_string("string"),
        SpannedValue::test_float(1.0),
    ];

    let nothing = SpannedValue::Nothing {
        span: Span::test_data(),
    };

    for value in values {
        assert!(matches!(
            value.eq(Span::test_data(), &nothing, Span::test_data()),
            Ok(SpannedValue::Bool { val: false, .. })
        ));

        assert!(matches!(
            value.ne(Span::test_data(), &nothing, Span::test_data()),
            Ok(SpannedValue::Bool { val: true, .. })
        ));

        assert!(matches!(
            nothing.eq(Span::test_data(), &value, Span::test_data()),
            Ok(SpannedValue::Bool { val: false, .. })
        ));

        assert!(matches!(
            nothing.ne(Span::test_data(), &value, Span::test_data()),
            Ok(SpannedValue::Bool { val: true, .. })
        ));
    }
}

#[rstest]
#[case(365 * 24 * 3600 * 1_000_000_000, "52wk 1day")]
#[case( (((((((7 + 2) * 24 + 3) * 60 + 4) * 60) + 5) * 1000 + 6) * 1000 + 7) * 1000 + 8,
"1wk 2day 3hr 4min 5sec 6ms 7µs 8ns")]
fn test_duration_to_string(#[case] in_ns: i64, #[case] expected: &str) {
    let dur = SpannedValue::test_duration(in_ns);
    assert_eq!(
        expected,
        dur.into_string("", &Config::default()),
        "expected != observed"
    );
}
