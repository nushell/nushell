use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, Span, Type, Value,
};

use crate::Example;

/// `<list> | example sum`
pub struct Sum;

impl PluginCommand for Sum {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("example sum")
            .usage("Example stream consumer for a list of values")
            .search_terms(vec!["example".into()])
            .input_output_types(vec![
                (Type::List(Type::Int.into()), Type::Int),
                (Type::List(Type::Float.into()), Type::Float),
            ])
            .plugin_examples(vec![PluginExample {
                example: "seq 1 5 | example sum".into(),
                description: "sum values from 1 to 5".into(),
                result: Some(Value::test_int(15)),
            }])
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &Example,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let mut acc = IntOrFloat::Int(0);
        for value in input {
            if let Ok(n) = value.as_i64() {
                acc.add_i64(n);
            } else if let Ok(n) = value.as_f64() {
                acc.add_f64(n);
            } else {
                return Err(LabeledError::new("Sum only accepts ints and floats")
                    .with_label(format!("found {} in input", value.get_type()), value.span())
                    .with_label("can't be used here", call.head));
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
