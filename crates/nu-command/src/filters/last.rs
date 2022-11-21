use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct Last;

impl Command for Last {
    fn name(&self) -> &str {
        "last"
    }

    fn signature(&self) -> Signature {
        Signature::build("last")
            .input_output_types(vec![
                (
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter it would be List<T> -> T.
                    Type::List(Box::new(Type::Any)),
                    Type::Any,
                ),
                (Type::Binary, Type::Binary),
                (Type::Range, Type::Int),
            ])
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Return only the last element of the input. Counterpart of 'first'."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1,2,3] | last",
                description: "Get the last item",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: "Return the last byte of a binary value",
                example: "0x[01 23 45] | last",
                result: Some(Value::Binary {
                    val: vec![0x45],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the last number of a range",
                example: "3..5 | last",
                result: Some(Value::test_int(5)),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.metadata();
        let input_span = input.span();
        let input_not_supported_error = || -> ShellError {
            // can't always get a span for input, so try our best and fall back on the span for the `first` call if needed
            if let Some(span) = input_span {
                ShellError::UnsupportedInput("first does not support this input type".into(), span)
            } else {
                ShellError::UnsupportedInput(
                    "first was given an unsupported input type".into(),
                    call.span(),
                )
            }
        };

        match input {
            PipelineData::Value(val, _) => match val {
                Value::List { vals, .. } => {
                    if let Some(last) = vals.last() {
                        Ok(last.clone().into_pipeline_data())
                    } else {
                        Err(ShellError::AccessEmptyContent(head))
                    }
                }
                Value::Binary { val, span } => {
                    if let Some(last) = val.last() {
                        Ok(PipelineData::Value(
                            Value::Binary {
                                val: vec![*last],
                                span,
                            },
                            metadata,
                        ))
                    } else {
                        Err(ShellError::AccessEmptyContent(head))
                    }
                }
                Value::Range { val, .. } => Ok(val.to.into_pipeline_data()),
                _ => Err(input_not_supported_error()),
            },
            PipelineData::ListStream(ls, ..) => {
                if let Some(last) = ls.last() {
                    Ok(last.into_pipeline_data())
                } else {
                    Err(ShellError::AccessEmptyContent(head))
                }
            }
            _ => Err(input_not_supported_error()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Last {})
    }
}
