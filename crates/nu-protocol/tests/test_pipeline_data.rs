use nu_protocol::{IntoPipelineData, Range, Signals, Span, Value, engine::EngineState};
use std::{
    ops::Bound,
    sync::{Arc, atomic::AtomicBool},
};

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

/// `1..1000` as the evaluator hands it to a command, carrying the engine's
/// signals, with the interrupt already set.
fn interrupted_range() -> Value {
    let mut engine_state = EngineState::new();
    engine_state.set_signals(Signals::new(Arc::new(AtomicBool::new(true))));
    let mut range = Value::test_range(Range::new_int(1, None, Bound::Included(1000)));
    range.inject_signals(&engine_state);
    range
}

#[test]
fn interrupted_range_stops_when_iterated_strictly() {
    let pipeline_data = interrupted_range().into_pipeline_data();
    let values = pipeline_data.into_iter_strict(Span::test_data()).unwrap();
    assert_eq!(values.count(), 0);
}

#[test]
fn interrupted_range_stops_when_expanded() {
    let pipeline_data = interrupted_range().into_pipeline_data();
    let value = pipeline_data
        .try_expand_range()
        .unwrap()
        .into_value(Span::test_data())
        .unwrap();
    assert_eq!(value.as_list().unwrap().len(), 0);
}
