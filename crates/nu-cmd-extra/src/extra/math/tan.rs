use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathTan;

impl Command for MathTan {
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

    fn description(&self) -> &str {
        "Returns the tangent of the number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let use_degrees = call.has_flag(engine_state, stack, "degrees")?;
        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| operate(value, head, use_degrees),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply the tangent to π/4",
                example: "3.141592 / 4 | math tan | math round --precision 4",
                result: Some(Value::test_float(1f64)),
            },
            Example {
                description: "Apply the tangent to a list of angles in degrees",
                example: "[-45 0 45] | math tan --degrees",
                result: Some(Value::list(
                    vec![
                        Value::test_float(-1f64),
                        Value::test_float(0f64),
                        Value::test_float(1f64),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn operate(value: Value, head: Span, use_degrees: bool) -> Value {
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let span = numeric.span();
            let (val, span) = match numeric {
                Value::Int { val, .. } => (val as f64, span),
                Value::Float { val, .. } => (val, span),
                _ => unreachable!(),
            };

            let val = if use_degrees { val.to_radians() } else { val };

            Value::float(val.tan(), span)
        }
        Value::Error { .. } => value,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "numeric".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathTan {})
    }
}
