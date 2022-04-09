use nu_protocol::{Span, Value};

#[test]
fn test_comparison_nothing() {
    let values = vec![
        Value::Int {
            val: 1,
            span: Span::test_data(),
        },
        Value::String {
            val: "string".into(),
            span: Span::test_data(),
        },
        Value::Float {
            val: 1.0,
            span: Span::test_data(),
        },
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
