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
                (Type::Nothing, Type::Int),
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

    fn examples(&self) -> Vec<Example<'_>> {
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
            Example {
                description: "Count the length a null value",
                example: "null | length",
                result: Some(Value::test_int(0)),
            },
        ]
    }
}

fn length_row(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(call.head);
    match input {
        PipelineData::Empty | PipelineData::Value(Value::Nothing { .. }, ..) => {
            Ok(Value::int(0, call.head).into_pipeline_data())
        }
        PipelineData::Value(Value::Binary { val, .. }, ..) => {
            Ok(Value::int(val.len() as i64, call.head).into_pipeline_data())
        }
        PipelineData::Value(Value::List { vals, .. }, ..) => {
            Ok(Value::int(vals.len() as i64, call.head).into_pipeline_data())
        }
        PipelineData::ListStream(stream, ..) => {
            Ok(Value::int(stream.into_iter().count() as i64, call.head).into_pipeline_data())
        }
        PipelineData::ByteStream(stream, ..) if stream.type_().is_binary_coercible() => {
            Ok(Value::int(
                match stream.reader() {
                    Some(r) => r.bytes().count() as i64,
                    None => 0,
                },
                call.head,
            )
            .into_pipeline_data())
        }
        _ => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "list, table, binary, and nothing".into(),
            wrong_type: input.get_type().to_string(),
            dst_span: call.head,
            src_span: span,
        }),
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
