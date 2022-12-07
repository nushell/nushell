use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math arctanh"
    }

    fn signature(&self) -> Signature {
        Signature::build("math arctanh")
            .input_output_types(vec![(Type::Number, Type::Float)])
            .vectorizes_over_list(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the inverse of the hyperbolic tangent function."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry", "inverse", "hyperbolic"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        input.map(
            move |value| operate(value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the arctanh of 1",
            example: "1 | math arctanh",
            result: Some(Value::test_float(f64::INFINITY)),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let (val, span) = match numeric {
                Value::Int { val, span } => (val as f64, span),
                Value::Float { val, span } => (val, span),
                _ => unreachable!(),
            };

            if (-1.0..=1.0).contains(&val) {
                let val = val.atanh();

                Value::Float { val, span }
            } else {
                Value::Error {
                    error: ShellError::UnsupportedInput(
                        "'arctanh' undefined for values outside the open interval (-1, 1).".into(),
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
