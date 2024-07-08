use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math log"
    }

    fn signature(&self) -> Signature {
        Signature::build("math log")
            .required(
                "base",
                SyntaxShape::Number,
                "Base for which the logarithm should be computed.",
            )
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
        "Returns the logarithm for an arbitrary base."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["base", "exponent", "inverse", "euler"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let base: Spanned<f64> = call.req(engine_state, stack, 0)?;

        if base.item <= 0.0f64 {
            return Err(ShellError::UnsupportedInput {
                msg: "Base has to be greater 0".into(),
                input: "value originates from here".into(),
                msg_span: head,
                input_span: base.span,
            });
        }
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        let base = base.item;
        input.map(
            move |value| operate(value, head, base),
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the logarithm of 100 to the base 10",
                example: "100 | math log 10",
                result: Some(Value::test_float(2.0f64)),
            },
            Example {
                example: "[16 8 4] | math log 2",
                description: "Get the log2 of a list of values",
                result: Some(Value::list(
                    vec![
                        Value::test_float(4.0),
                        Value::test_float(3.0),
                        Value::test_float(2.0),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn operate(value: Value, head: Span, base: f64) -> Value {
    let span = value.span();
    match value {
        numeric @ (Value::Int { .. } | Value::Float { .. }) => {
            let (val, span) = match numeric {
                Value::Int { val, .. } => (val as f64, span),
                Value::Float { val, .. } => (val, span),
                _ => unreachable!(),
            };

            if val <= 0.0 {
                return Value::error(
                    ShellError::UnsupportedInput {
                        msg: "'math log' undefined for values outside the open interval (0, Inf)."
                            .into(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span: span,
                    },
                    span,
                );
            }
            // Specialize for better precision/performance
            let val = if base == 10.0 {
                val.log10()
            } else if base == 2.0 {
                val.log2()
            } else {
                val.log(base)
            };

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

        test_examples(SubCommand {})
    }
}
