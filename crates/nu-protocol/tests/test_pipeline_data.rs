use nu_protocol::{IntoPipelineData, Span, Value};

#[test]
fn test_convert_pipeline_data_to_value() {
    // Setup PipelineData
    let value_val = 10;
    let value = Value::int(value_val, Span::new(1, 3));
    let pipeline_data = value.into_pipeline_data();

    // Test that conversion into Value is correct
    let new_span = Span::new(5, 6);
    let converted_value = pipeline_data.into_value(new_span);

    assert_eq!(converted_value, Ok(Value::int(value_val, new_span)));
}
