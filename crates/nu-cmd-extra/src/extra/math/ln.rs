use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathLn;

impl Command for MathLn {
    fn name(&self) -> &str {
        "math ln"
    }

    fn signature(&self) -> Signature {
        Signature::build("math ln")
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
        "Returns the natural logarithm. Base: (math e)."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["natural", "logarithm", "inverse", "euler"]
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
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(move |value| operate(value, head), engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Get the natural logarithm of e",
            example: "2.7182818 | math ln | math round --precision 4",
            result: Some(Value::test_float(1.0f64)),
        }]
    }
}

fn operate(value: Value, head: Span) -> Value {
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let span = numeric.span();
            let (val, span) = match numeric {
                Value::Int { val, .. } => (val as f64, span),
                Value::Float { val, .. } => (val, span),
                _ => unreachable!(),
            };

            if val > 0.0 {
                let val = val.ln();

                Value::float(val, span)
            } else {
                Value::error(
                    ShellError::UnsupportedInput {
                        msg: "'ln' undefined for values outside the open interval (0, Inf).".into(),
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

        test_examples(MathLn {})
    }
}
