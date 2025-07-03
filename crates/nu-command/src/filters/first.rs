use nu_engine::command_prelude::*;
use nu_protocol::{Signals, shell_error::io::IoError};
use std::io::Read;

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
                (Type::Binary, Type::Int),
                (Type::Range, Type::Any),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Return the first element of the input. For multiple rows, use `take`."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        first_helper(engine_state, stack, call, input)
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
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Return the first item of a range",
                example: "1..3 | first",
                result: Some(Value::test_int(1)),
            },
        ]
    }
}

fn first_helper(
    _engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;

    if call.has_positional_args(stack, 0) {
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "The 'first' command no longer takes an argument. Use 'take' to get multiple elements.".into(),
            span: call.head,
        });
    }

    match input {
        PipelineData::Value(val, _) => {
            let span = val.span();
            match val {
                Value::List { mut vals, .. } => {
                    if let Some(val) = vals.first_mut() {
                        Ok(std::mem::take(val).into_pipeline_data())
                    } else {
                        Err(ShellError::AccessEmptyContent { span: head })
                    }
                }
                Value::Binary { val, .. } => {
                    if let Some(&byte) = val.first() {
                        Ok(Value::int(byte.into(), span).into_pipeline_data())
                    } else {
                        Err(ShellError::AccessEmptyContent { span: head })
                    }
                }
                Value::Range { val, .. } => {
                    let mut iter = val.into_range_iter(span, Signals::empty());
                    if let Some(v) = iter.next() {
                        Ok(v.into_pipeline_data())
                    } else {
                        Err(ShellError::AccessEmptyContent { span: head })
                    }
                }
                // Propagate errors by explicitly matching them before the final case.
                Value::Error { error, .. } => Err(*error),
                other => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "list, binary or range".into(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: other.span(),
                }),
            }
        }
        PipelineData::ListStream(stream, _metadata) => {
            if let Some(v) = stream.into_iter().next() {
                Ok(v.into_pipeline_data())
            } else {
                Err(ShellError::AccessEmptyContent { span: head })
            }
        }
        PipelineData::ByteStream(stream, _metadata) => {
            if stream.type_().is_binary_coercible() {
                let span = stream.span();
                if let Some(mut reader) = stream.reader() {
                    let mut byte = [0u8];
                    if reader
                        .read(&mut byte)
                        .map_err(|err| IoError::new(err, span, None))?
                        > 0
                    {
                        Ok(Value::int(byte[0] as i64, head).into_pipeline_data())
                    } else {
                        Err(ShellError::AccessEmptyContent { span: head })
                    }
                } else {
                    Ok(PipelineData::Empty)
                }
            } else {
                Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "list, binary or range".into(),
                    wrong_type: stream.type_().describe().into(),
                    dst_span: head,
                    src_span: stream.span(),
                })
            }
        }
        PipelineData::Empty => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "list, binary or range".into(),
            wrong_type: "null".into(),
            dst_span: call.head,
            src_span: call.head,
        }),
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
