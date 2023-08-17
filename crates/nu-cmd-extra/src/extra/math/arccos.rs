use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math arccos"
    }

    fn signature(&self) -> Signature {
        Signature::build("math arccos")
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
        vec![
            Example {
                description: "Get the arccosine of 1",
                example: "1 | math arccos",
                result: Some(SpannedValue::test_float(0.0f64)),
            },
            Example {
                description: "Get the arccosine of -1 in degrees",
                example: "-1 | math arccos -d",
                result: Some(SpannedValue::test_float(180.0)),
            },
        ]
    }
}

fn operate(value: SpannedValue, head: Span, use_degrees: bool) -> SpannedValue {
    match value {
        numeric @ (SpannedValue::Int { .. } | SpannedValue::Float { .. }) => {
            let (val, span) = match numeric {
                SpannedValue::Int { val, span } => (val as f64, span),
                SpannedValue::Float { val, span } => (val, span),
                _ => unreachable!(),
            };

            if (-1.0..=1.0).contains(&val) {
                let val = val.acos();
                let val = if use_degrees { val.to_degrees() } else { val };

                SpannedValue::Float { val, span }
            } else {
                SpannedValue::Error {
                    error: Box::new(ShellError::UnsupportedInput(
                        "'arccos' undefined for values outside the closed interval [-1, 1].".into(),
                        "value originates from here".into(),
                        head,
                        span,
                    )),
                    span,
                }
            }
        }
        SpannedValue::Error { .. } => value,
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            }),
            span: head,
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
