use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math sin"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sin")
            .switch("degrees", "Use degrees instead of radians", Some('d'))
            .input_output_types(vec![(Type::Number, Type::Float)])
            .vectorizes_over_list(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the sine of the number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let use_degrees = call.has_flag("degrees");
        input.map(
            move |value| operate(value, head, use_degrees),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply the sine to π/2",
                example: "(math pi) / 2 | math sin",
                result: Some(Value::test_float(1f64)),
            },
            Example {
                description: "Apply the sine to a list of angles in degrees",
                example: "[0 90 180 270 360] | math sin -d | math round --precision 4",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_float(0f64),
                        Value::test_float(1f64),
                        Value::test_float(0f64),
                        Value::test_float(-1f64),
                        Value::test_float(0f64),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(value: Value, head: Span, use_degrees: bool) -> Value {
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let (val, span) = match numeric {
                Value::Int { val, span } => (val as f64, span),
                Value::Float { val, span } => (val, span),
                _ => unreachable!(),
            };

            let val = if use_degrees { val.to_radians() } else { val };

            Value::Float {
                val: val.sin(),
                span,
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only numerical values are supported, input type: {:?}",
                    other.get_type()
                ),
                other.span().unwrap_or(head),
            ),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
