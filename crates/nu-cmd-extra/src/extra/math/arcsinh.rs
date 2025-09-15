use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathArcSinH;

impl Command for MathArcSinH {
    fn name(&self) -> &str {
        "math arcsinh"
    }

    fn signature(&self) -> Signature {
        Signature::build("math arcsinh")
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
        "Returns the inverse of the hyperbolic sine function."
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
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(move |value| operate(value, head), engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Get the arcsinh of 0",
            example: "0 | math arcsinh",
            result: Some(Value::test_float(0.0f64)),
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

            let val = val.asinh();

            Value::float(val, span)
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

        test_examples(MathArcSinH {})
    }
}
