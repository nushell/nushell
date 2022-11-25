use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
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
                    // TODO: This variant duplicates the functionality of
                    // `take`. See #6611, #6611, #6893
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter style it would be List<T> ->
                    // List<T>.
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter it would be List<T> -> T.
                    Type::List(Box::new(Type::Any)),
                    Type::Any,
                ),
                (Type::Binary, Type::Binary),
            ])
            .optional(
                "rows",
                SyntaxShape::Int,
                "starting from the front, the number of rows to return",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Return only the first several rows of the input. Counterpart of 'last'. Opposite of 'skip'."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
                description: "Return the first 2 items of a list/table",
                example: "[1 2 3] | first 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the first 2 bytes of a binary value",
                example: "0x[01 23 45] | first 2",
                result: Some(Value::Binary {
                    val: vec![0x01, 0x23],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn first_helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    let rows: Option<i64> = call.opt(engine_state, stack, 0)?;
    // FIXME: for backwards compatibility reasons, if `rows` is not specified we
    // return a single element and otherwise we return a single list. We should probably
    // remove `rows` so that `first` always returns a single element; getting a list of
    // the first N elements is covered by `take`
    let return_single_element = rows.is_none();
    let rows_desired: usize = match rows {
        Some(i) if i < 0 => return Err(ShellError::NeedsPositiveValue(head)),
        Some(x) => x as usize,
        None => 1,
    };

    let ctrlc = engine_state.ctrlc.clone();
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
                if return_single_element {
                    if vals.is_empty() {
                        Err(ShellError::AccessEmptyContent(head))
                    } else {
                        Ok(vals[0].clone().into_pipeline_data())
                    }
                } else {
                    Ok(vals
                        .into_iter()
                        .take(rows_desired)
                        .into_pipeline_data(ctrlc)
                        .set_metadata(metadata))
                }
            }
            Value::Binary { val, span } => {
                let slice: Vec<u8> = val.into_iter().take(rows_desired).collect();
                Ok(PipelineData::Value(
                    Value::Binary { val: slice, span },
                    metadata,
                ))
            }
            Value::Range { val, .. } => {
                if return_single_element {
                    Ok(val.from.into_pipeline_data())
                } else {
                    Ok(val
                        .into_range_iter(ctrlc.clone())?
                        .take(rows_desired)
                        .into_pipeline_data(ctrlc)
                        .set_metadata(metadata))
                }
            }
            _ => Err(input_not_supported_error()),
        },
        PipelineData::ListStream(mut ls, metadata) => {
            if return_single_element {
                if let Some(v) = ls.next() {
                    Ok(v.into_pipeline_data())
                } else {
                    Err(ShellError::AccessEmptyContent(head))
                }
            } else {
                Ok(ls
                    .take(rows_desired)
                    .into_pipeline_data(ctrlc)
                    .set_metadata(metadata))
            }
        }
        _ => Err(input_not_supported_error()),
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
