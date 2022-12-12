use nu_protocol::{Span, Value};

#[test]
fn test_comparison_nothing() {
    let values = vec![
        Value::int(1, Span::test_data()),
        Value::string("string", Span::test_data()),
        Value::float(1.0, Span::test_data()),
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
