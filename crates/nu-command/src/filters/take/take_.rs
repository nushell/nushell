use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Take;

impl Command for Take {
    fn name(&self) -> &str {
        "take"
    }

    fn signature(&self) -> Signature {
        Signature::build("take")
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .required(
                "n",
                SyntaxShape::Int,
                "starting from the front, the number of elements to return",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Take only the first n elements."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["first", "slice", "head"]
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
                example: "[1 2 3] | take 1",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the first 2 items of a list/table",
                example: "[1 2 3] | take 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the first two rows of a table",
                example: "echo [[editions]; [2015] [2018] [2021]] | take 2",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(vec!["editions"], vec![Value::test_int(2015)]),
                        Value::test_record(vec!["editions"], vec![Value::test_int(2018)]),
                    ],
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
    let mut rows_desired: usize = call.req(engine_state, stack, 0)?;

    let ctrlc = engine_state.ctrlc.clone();
    let metadata = input.metadata();

    let mut input_peek = input.into_iter().peekable();
    if input_peek.peek().is_some() {
        match input_peek
            .peek()
            .ok_or_else(|| {
                ShellError::GenericError(
                    "Error in first".into(),
                    "unable to pick on next value".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .get_type()
        {
            Type::Binary => {
                match &mut input_peek.next() {
                    Some(v) => match &v {
                        Value::Binary { val, .. } => {
                            let bytes = val;
                            if bytes.len() >= rows_desired {
                                // We only want to see a certain amount of the binary
                                // so let's grab those parts
                                let output_bytes = bytes[0..rows_desired].to_vec();
                                Ok(Value::Binary {
                                    val: output_bytes,
                                    span: head,
                                }
                                .into_pipeline_data())
                            } else {
                                // if we want more rows that the current chunk size (8192)
                                // we must gradually get bigger chunks while testing
                                // if it's within the requested rows_desired size
                                let mut bigger: Vec<u8> = vec![];
                                bigger.extend(bytes);
                                while bigger.len() < rows_desired {
                                    match input_peek.next() {
                                        Some(Value::Binary { val, .. }) => bigger.extend(val),
                                        _ => {
                                            // We're at the end of our data so let's break out of this loop
                                            // and set the rows_desired to the size of our data
                                            rows_desired = bigger.len();
                                            break;
                                        }
                                    }
                                }
                                let output_bytes = bigger[0..rows_desired].to_vec();
                                Ok(Value::Binary {
                                    val: output_bytes,
                                    span: head,
                                }
                                .into_pipeline_data())
                            }
                        }

                        _ => todo!(),
                    },
                    None => Ok(input_peek
                        .into_iter()
                        .take(rows_desired)
                        .into_pipeline_data(ctrlc)
                        .set_metadata(metadata)),
                }
            }
            _ => Ok(input_peek
                .into_iter()
                .take(rows_desired)
                .into_pipeline_data(ctrlc)
                .set_metadata(metadata)),
        }
    } else {
        Err(ShellError::UnsupportedInput(
            String::from("Cannot perform into string on empty input"),
            head,
        ))
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Take {})
    }
}
