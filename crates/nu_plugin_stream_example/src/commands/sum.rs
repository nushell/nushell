use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, PluginCommand};
use nu_protocol::{Category, PipelineData, PluginExample, PluginSignature, Span, Type, Value};

use crate::StreamExample;

/// `<list> | stream_example sum`
pub struct Sum;

impl PluginCommand for Sum {
    type Plugin = StreamExample;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("stream_example sum")
            .usage("Example stream consumer for a list of values")
            .search_terms(vec!["example".into()])
            .input_output_types(vec![
                (Type::List(Type::Int.into()), Type::Int),
                (Type::List(Type::Float.into()), Type::Float),
            ])
            .plugin_examples(vec![PluginExample {
                example: "seq 1 5 | stream_example sum".into(),
                description: "sum values from 1 to 5".into(),
                result: Some(Value::test_int(15)),
            }])
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &StreamExample,
        _engine: &EngineInterface,
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
}

/// Accumulates numbers into either an int or a float. Changes type to float on the first
/// float received.
#[derive(Clone, Copy)]
enum IntOrFloat {
    Int(i64),
    Float(f64),
}

impl IntOrFloat {
    pub(crate) fn add_i64(&mut self, n: i64) {
        match self {
            IntOrFloat::Int(ref mut v) => {
                *v += n;
            }
            IntOrFloat::Float(ref mut v) => {
                *v += n as f64;
            }
        }
    }

    pub(crate) fn add_f64(&mut self, n: f64) {
        match self {
            IntOrFloat::Int(v) => {
                *self = IntOrFloat::Float(*v as f64 + n);
            }
            IntOrFloat::Float(ref mut v) => {
                *v += n;
            }
        }
    }

    pub(crate) fn to_value(self, span: Span) -> Value {
        match self {
            IntOrFloat::Int(v) => Value::int(v, span),
            IntOrFloat::Float(v) => Value::float(v, span),
        }
    }
}
