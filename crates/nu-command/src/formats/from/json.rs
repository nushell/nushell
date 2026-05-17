use std::io::{BufRead, Cursor};

use nu_engine::command_prelude::*;
use nu_protocol::{
    ListStream, Signals,
    shell_error::{generic::GenericError, io::IoError},
};

#[derive(Clone)]
pub struct FromJson;

impl Command for FromJson {
    fn name(&self) -> &str {
        "from json"
    }

    fn description(&self) -> &str {
        "Convert JSON text into structured data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from json")
            .input_output_types(vec![(Type::String, Type::Any)])
            .switch("objects", "Treat each line as a separate value.", Some('o'))
            .switch(
                "strict",
                "Follow the json specification exactly.",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"'{ "a": 1 }' | from json"#,
                description: "Converts json formatted string to table.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                example: r#"'{ "a": 1, "b": [1, 2] }' | from json"#,
                description: "Converts json formatted string to table.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                })),
            },
            Example {
                example: r#"'{ "a": 1, "b": 2 }' | from json -s"#,
                description: "Parse json strictly which will error on comments and trailing commas.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                })),
            },
            Example {
                example: r#"'{ "a": 1 }
{ "b": 2 }' | from json --objects"#,
                description: "Parse a stream of line-delimited JSON values.",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {"a" => Value::test_int(1)}),
                    Value::test_record(record! {"b" => Value::test_int(2)}),
                ])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let strict = call.has_flag(engine_state, stack, "strict")?;
        let metadata = input.take_metadata().map(|md| md.with_content_type(None));

        // TODO: turn this into a structured underline of the nu_json error
        if call.has_flag(engine_state, stack, "objects")? {
            // Return a stream of JSON values, one for each non-empty line
            match input {
                PipelineData::Value(Value::String { val, .. }, ..) => {
                    Ok(PipelineData::list_stream(
                        read_json_lines(
                            Cursor::new(val),
                            span,
                            strict,
                            engine_state.signals().clone(),
                        ),
                        metadata,
                    ))
                }
                PipelineData::ByteStream(stream, ..)
                    if stream.type_() != ByteStreamType::Binary =>
                {
                    if let Some(reader) = stream.reader() {
                        Ok(PipelineData::list_stream(
                            read_json_lines(reader, span, strict, Signals::empty()),
                            metadata,
                        ))
                    } else {
                        Ok(PipelineData::empty())
                    }
                }
                _ => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string".into(),
                    wrong_type: input.get_type().to_string(),
                    dst_span: call.head,
                    src_span: input.span().unwrap_or(call.head),
                }),
            }
        } else {
            // Return a single JSON value
            let (string_input, span, ..) = input.collect_string_strict(span)?;

            if string_input.is_empty() {
                return Ok(Value::nothing(span).into_pipeline_data());
            }

            Ok(try_str_to_value(&string_input, span, strict)?
                .into_pipeline_data_with_metadata(metadata))
        }
    }
}

/// Create a stream of values from a reader that produces line-delimited JSON
fn read_json_lines(
    input: impl BufRead + Send + 'static,
    span: Span,
    strict: bool,
    signals: Signals,
) -> ListStream {
    let iter = input
        .lines()
        .filter(|line| line.as_ref().is_ok_and(|line| !line.trim().is_empty()) || line.is_err())
        .map(move |line| {
            let line = line.map_err(|err| IoError::new(err, span, None))?;
            try_str_to_value(&line, span, strict)
        })
        .map(move |result| result.unwrap_or_else(|err| Value::error(err, span)));

    ListStream::new(iter, span, signals)
}

pub fn try_str_to_value(input: &str, span: Span, strict: bool) -> Result<Value, ShellError> {
    match strict {
        true => try_str_to_value_impl(
            input,
            span,
            |s| serde_json::from_str(s),
            |err| err.is_syntax().then_some((err.line(), err.column())),
        ),
        false => try_str_to_value_impl(input, span, nu_json::from_str, |err| match err {
            nu_json::Error::Syntax(_, row, col) => Some((*row, *col)),
            _ => None,
        }),
    }
}

#[inline]
fn try_str_to_value_impl<E: std::error::Error>(
    input: &str,
    span: Span,
    parser: impl Fn(&str) -> Result<nu_json::Value, E>,
    on_syntax_err: impl Fn(&E) -> Option<(usize, usize)>,
) -> Result<Value, ShellError> {
    match parser(input) {
        Ok(value) => Ok(value.into_value(span)),
        Err(err) => match on_syntax_err(&err) {
            Some((row, col)) => {
                let label = err.to_string();
                let label_span = Span::from_row_column(row, col, input);
                Err(ShellError::Generic(
                    GenericError::new(
                        "Error while parsing JSON text",
                        "error parsing JSON text",
                        span,
                    )
                    .with_inner([ShellError::OutsideSpannedLabeledError {
                        src: input.into(),
                        error: "Error while parsing JSON text".into(),
                        msg: label,
                        span: label_span,
                    }]),
                ))
            }
            None => Err(ShellError::CantConvert {
                to_type: format!("structured json data ({err})"),
                from_type: "string".into(),
                span,
                help: None,
            }),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromJson)
    }
}
