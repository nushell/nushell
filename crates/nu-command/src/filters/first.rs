use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct First;

impl Command for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the front, the number of rows to return",
        )
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows."
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
                    span: Span::unknown(),
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
    let mut rows_desired: usize = match rows {
        Some(x) => x as usize,
        None => 1,
    };

    let mut input_peek = input.into_iter().peekable();
    if input_peek.peek().is_some() {
        match input_peek.peek().unwrap().get_type() {
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
                    None => Ok(Value::List {
                        vals: input_peek.into_iter().take(rows_desired).collect(),
                        span: head,
                    }
                    .into_pipeline_data()),
                }
            }
            _ => {
                if rows_desired == 1 {
                    match input_peek.next() {
                        Some(val) => Ok(val.into_pipeline_data()),
                        None => Err(ShellError::AccessBeyondEndOfStream(head)),
                    }
                } else {
                    Ok(Value::List {
                        vals: input_peek.into_iter().take(rows_desired).collect(),
                        span: head,
                    }
                    .into_pipeline_data())
                }
            }
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

        test_examples(First {})
    }
}
