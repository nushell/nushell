use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math tan"
    }

    fn signature(&self) -> Signature {
        Signature::build("math tan")
            .switch("degrees", "Use degrees instead of radians", Some('d'))
            .input_output_types(vec![
                (Type::Number, Type::Float),
                (
                    Type::List(Box::new(Type::Number)),
                    Type::List(Box::new(Type::Float)),
                ),
            ])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the tangent of the number."
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
                description: "Apply the tangent to π/4",
                example: "(math pi) / 4 | math tan",
                result: Some(SpannedValue::test_float(1f64)),
            },
            Example {
                description: "Apply the tangent to a list of angles in degrees",
                example: "[-45 0 45] | math tan -d",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_float(-1f64),
                        SpannedValue::test_float(0f64),
                        SpannedValue::test_float(1f64),
                    ],
                    span: Span::test_data(),
                }),
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

            let val = if use_degrees { val.to_radians() } else { val };

            SpannedValue::Float {
                val: val.tan(),
                span,
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
