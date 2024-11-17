use std::io::Read;

use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Length;

impl Command for Length {
    fn name(&self) -> &str {
        "length"
    }

    fn description(&self) -> &str {
        "Count the number of items in an input list, rows in a table, or bytes in binary data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("length")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Int),
                (Type::Binary, Type::Int),
            ])
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["count", "size", "wc"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        length_row(call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Count the number of items in a list",
                example: "[1 2 3 4 5] | length",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Count the number of rows in a table",
                example: "[{a:1 b:2}, {a:2 b:3}] | length",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Count the number of bytes in binary data",
                example: "0x[01 02] | length",
                result: Some(Value::test_int(2)),
            },
        ]
    }
}

fn length_row(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(call.head);
    match input {
        PipelineData::Value(Value::Nothing { .. }, ..) => {
            Ok(Value::int(0, call.head).into_pipeline_data())
        }
        // I added this here because input_output_type() wasn't catching a record
        // being sent in as input from echo. e.g. "echo {a:1 b:2} | length"
        PipelineData::Value(Value::Record { .. }, ..) => {
            Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "list, and table".into(),
                wrong_type: "record".into(),
                dst_span: call.head,
                src_span: span,
            })
        }
        PipelineData::Value(Value::Binary { val, .. }, ..) => {
            Ok(Value::int(val.len() as i64, call.head).into_pipeline_data())
        }
        PipelineData::ByteStream(stream, _) if stream.type_().is_binary_coercible() => {
            Ok(Value::int(
                match stream.reader() {
                    Some(r) => r.bytes().count() as i64,
                    None => 0,
                },
                call.head,
            )
            .into_pipeline_data())
        }
        _ => {
            let mut count: i64 = 0;
            // Check for and propagate errors
            for value in input.into_iter() {
                if let Value::Error { error, .. } = value {
                    return Err(*error);
                }
                count += 1
            }
            Ok(Value::int(count, call.head).into_pipeline_data())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Length {})
    }
}
