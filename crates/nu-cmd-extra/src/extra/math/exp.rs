use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathExp;

impl Command for MathExp {
    fn name(&self) -> &str {
        "math exp"
    }

    fn signature(&self) -> Signature {
        Signature::build("math exp")
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
        "Returns e raised to the power of x."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["exponential", "exponentiation", "euler"]
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
        vec![
            Example {
                description: "Get e raised to the power of zero",
                example: "0 | math exp",
                result: Some(Value::test_float(1.0f64)),
            },
            Example {
                description: "Get e (same as 'math e')",
                example: "1 | math exp",
                result: Some(Value::test_float(1.0f64.exp())),
            },
        ]
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

            Value::float(val.exp(), span)
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

        test_examples(MathExp {})
    }
}
