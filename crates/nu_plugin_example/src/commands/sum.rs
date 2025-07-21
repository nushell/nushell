use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type, Value};

use crate::ExamplePlugin;

/// `<list> | example sum`
pub struct Sum;

impl PluginCommand for Sum {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example sum"
    }

    fn description(&self) -> &str {
        "Example stream consumer for a list of values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::List(Type::Int.into()), Type::Int),
                (Type::List(Type::Float.into()), Type::Float),
            ])
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "example seq 1 5 | example sum",
            description: "sum values from 1 to 5",
            result: Some(Value::test_int(15)),
        }]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let mut acc = IntOrFloat::Int(0);
        for value in input {
            if let Ok(n) = value.as_int() {
                acc.add_i64(n);
            } else if let Ok(n) = value.as_float() {
                acc.add_f64(n);
            } else {
                return Err(LabeledError::new("Sum only accepts ints and floats")
                    .with_label(format!("found {} in input", value.get_type()), value.span())
                    .with_label("can't be used here", call.head));
            }
        }
        Ok(PipelineData::value(acc.to_value(call.head), None))
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
            IntOrFloat::Int(v) => {
                *v += n;
            }
            IntOrFloat::Float(v) => {
                *v += n as f64;
            }
        }
    }

    pub(crate) fn add_f64(&mut self, n: f64) {
        match self {
            IntOrFloat::Int(v) => {
                *self = IntOrFloat::Float(*v as f64 + n);
            }
            IntOrFloat::Float(v) => {
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

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&Sum)
}
