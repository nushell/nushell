use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct First;

impl Command for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first")
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
        "Return only the first element of the input. Counterpart of 'last'."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
                    if let Some(first) = vals.first() {
                        Ok(first.clone().into_pipeline_data())
                    } else {
                        Err(ShellError::AccessEmptyContent(head))
                    }
                }
                Value::Binary { val, span } => {
                    if let Some(first) = val.first() {
                        Ok(PipelineData::Value(
                            Value::Binary {
                                val: vec![*first],
                                span,
                            },
                            metadata,
                        ))
                    } else {
                        Err(ShellError::AccessEmptyContent(head))
                    }
                }
                Value::Range { val, .. } => Ok(val.from.into_pipeline_data()),
                _ => Err(input_not_supported_error()),
            },
            PipelineData::ListStream(mut ls, ..) => {
                if let Some(v) = ls.next() {
                    Ok(v.into_pipeline_data())
                } else {
                    Err(ShellError::AccessEmptyContent(head))
                }
            }
            _ => Err(input_not_supported_error()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first item of a list/table",
                example: "[1 2 3] | first",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Return the first byte of a binary value",
                example: "0x[01 23 45] | first",
                result: Some(Value::Binary {
                    val: vec![0x01],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the first number of a range",
                example: "3..5 | first",
                result: Some(Value::test_int(3)),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(First {})
    }
}
