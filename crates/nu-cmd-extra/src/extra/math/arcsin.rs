use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math arcsin"
    }

    fn signature(&self) -> Signature {
        Signature::build("math arcsin")
            .switch("degrees", "Return degrees instead of radians", Some('d'))
            .input_output_types(vec![
                (Type::Number, Type::Float),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Float)),
                ),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the arcsine of the number."
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let use_degrees = call.has_flag("degrees");
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, head, use_degrees),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        let pi = std::f64::consts::PI;
        vec![
            Example {
                description: "Get the arcsine of 1",
                example: "1 | math arcsin",
                result: Some(Value::test_float(pi / 2.0)),
            },
            Example {
                description: "Get the arcsine of 1 in degrees",
                example: "1 | math arcsin -d",
                result: Some(Value::test_float(90.0)),
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
                let val = val.asin();
                let val = if use_degrees { val.to_degrees() } else { val };

                Value::Float { val, span }
            } else {
                Value::Error {
                    error: Box::new(ShellError::UnsupportedInput(
                        "'arcsin' undefined for values outside the closed interval [-1, 1].".into(),
                        "value originates from here".into(),
                        head,
                        span,
                    )),
                }
            }
        }
        Value::Error { .. } => value,
        other => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.expect_span(),
            }),
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
