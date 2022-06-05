use nu_protocol::{Span, Value};

#[test]
fn test_comparison_nothing() {
    let values = vec![
        Value::Int(1),
        Value::String("string".into()),
        Value::Float(1.0),
    ];

    let nothing = Value::Nothing;

    for value in values {
        assert!(matches!(
            value.eq(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool(false))
        ));

        assert!(matches!(
            value.ne(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool(true))
        ));

        assert!(matches!(
            nothing.eq(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool(false))
        ));

        assert!(matches!(
            nothing.ne(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool(true))
        ));
    }
}
