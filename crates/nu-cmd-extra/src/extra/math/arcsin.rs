use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathArcSin;

impl Command for MathArcSin {
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

    fn description(&self) -> &str {
        "Returns the arcsine of the number."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry", "inverse"]
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
        let pi = std::f64::consts::PI;
        vec![
            Example {
                description: "Get the arcsine of 1",
                example: "1 | math arcsin",
                result: Some(Value::test_float(pi / 2.0)),
            },
            Example {
                description: "Get the arcsine of 1 in degrees",
                example: "1 | math arcsin --degrees",
                result: Some(Value::test_float(90.0)),
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

            if (-1.0..=1.0).contains(&val) {
                let val = val.asin();
                let val = if use_degrees { val.to_degrees() } else { val };

                Value::float(val, span)
            } else {
                Value::error(
                    ShellError::UnsupportedInput {
                        msg: "'arcsin' undefined for values outside the closed interval [-1, 1]."
                            .into(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span: span,
                    },
                    span,
                )
            }
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

        test_examples(MathArcSin {})
    }
}
