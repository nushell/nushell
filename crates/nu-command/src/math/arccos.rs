use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math arccos"
    }

    fn signature(&self) -> Signature {
        Signature::build("math arccos")
            .switch("degrees", "Return degrees instead of radians", Some('d'))
            .input_output_types(vec![(Type::Number, Type::Float)])
            .vectorizes_over_list(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the arccosine of the number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry", "inverse"]
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
                description: "Get the arccosine of 1",
                example: "1 | math arccos",
                result: Some(Value::test_float(0.0f64)),
            },
            Example {
                description: "Get the arccosine of -1 in degrees",
                example: "-1 | math arccos -d",
                result: Some(Value::test_float(180.0)),
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

            if (-1.0..=1.0).contains(&val) {
                let val = val.acos();
                let val = if use_degrees { val.to_degrees() } else { val };

                Value::Float { val, span }
            } else {
                Value::Error {
                    error: ShellError::UnsupportedInput(
                        "'arccos' undefined for values outside the closed interval [-1, 1].".into(),
                        span,
                    ),
                }
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
