use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError};
use nu_protocol::{IntoInterruptiblePipelineData, ListStream, PipelineData, RawStream, Value};

pub struct Example;

mod int_or_float;
use self::int_or_float::IntOrFloat;

impl Example {
    pub fn seq(
        &self,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let first: i64 = call.req(0)?;
        let last: i64 = call.req(1)?;
        let span = call.head;
        let iter = (first..=last).map(move |number| Value::int(number, span));
        let list_stream = ListStream::from_stream(iter, None);
        Ok(PipelineData::ListStream(list_stream, None))
    }

    pub fn sum(
        &self,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let mut acc = IntOrFloat::Int(0);
        let span = input.span();
        for value in input {
            if let Ok(n) = value.as_i64() {
                acc.add_i64(n);
            } else if let Ok(n) = value.as_f64() {
                acc.add_f64(n);
            } else {
                return Err(LabeledError {
                    label: "Stream only accepts ints and floats".into(),
                    msg: format!("found {}", value.get_type()),
                    span,
                });
            }
        }
        Ok(PipelineData::Value(acc.to_value(call.head), None))
    }

    pub fn collect_external(
        &self,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let stream = input.into_iter().map(|value| {
            value
                .as_str()
                .map(|str| str.as_bytes())
                .or_else(|_| value.as_binary())
                .map(|bin| bin.to_vec())
        });
        Ok(PipelineData::ExternalStream {
            stdout: Some(RawStream::new(Box::new(stream), None, call.head, None)),
            stderr: None,
            exit_code: None,
            span: call.head,
            metadata: None,
            trim_end_newline: false,
        })
    }

    pub fn for_each(
        &self,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let closure = call.req(0)?;
        let config = engine.get_config()?;
        for value in input {
            let result = engine.eval_closure(&closure, vec![value.clone()], Some(value))?;
            eprintln!("{}", result.to_expanded_string(", ", &config));
        }
        Ok(PipelineData::Empty)
    }

    pub fn generate(
        &self,
        engine: &EngineInterface,
        call: &EvaluatedCall,
    ) -> Result<PipelineData, LabeledError> {
        let engine = engine.clone();
        let call = call.clone();
        let initial: Value = call.req(0)?;
        let closure = call.req(1)?;

        let mut next = (!initial.is_nothing()).then_some(initial);

        Ok(std::iter::from_fn(move || {
            next.take()
                .and_then(|value| {
                    engine
                        .eval_closure(&closure, vec![value.clone()], Some(value))
                        .and_then(|record| {
                            if record.is_nothing() {
                                Ok(None)
                            } else {
                                let record = record.as_record()?;
                                next = record.get("next").cloned();
                                Ok(record.get("out").cloned())
                            }
                        })
                        .transpose()
                })
                .map(|result| result.unwrap_or_else(|err| Value::error(err, call.head)))
        })
        .into_pipeline_data(None))
    }
}
