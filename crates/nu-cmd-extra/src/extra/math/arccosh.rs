use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math arccosh"
    }

    fn signature(&self) -> Signature {
        Signature::build("math arccosh")
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
        "Returns the inverse of the hyperbolic cosine function."
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, head),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the arccosh of 1",
            example: "1 | math arccosh",
            result: Some(SpannedValue::test_float(0.0f64)),
        }]
    }
}

fn operate(value: SpannedValue, head: Span) -> SpannedValue {
    match value {
        numeric @ (SpannedValue::Int { .. } | SpannedValue::Float { .. }) => {
            let (val, span) = match numeric {
                SpannedValue::Int { val, span } => (val as f64, span),
                SpannedValue::Float { val, span } => (val, span),
                _ => unreachable!(),
            };

            if (1.0..).contains(&val) {
                let val = val.acosh();

                SpannedValue::Float { val, span }
            } else {
                SpannedValue::Error {
                    error: Box::new(ShellError::UnsupportedInput(
                        "'arccosh' undefined for values below 1.".into(),
                        "value originates from here".into(),
                        head,
                        span,
                    )),
                }
            }
        }
        SpannedValue::Error { .. } => value,
        other => SpannedValue::Error {
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
