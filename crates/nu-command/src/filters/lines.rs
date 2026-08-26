use nu_engine::command_prelude::*;
use nu_protocol::Signals;

#[derive(Clone)]
pub struct Lines;

impl Command for Lines {
    fn name(&self) -> &str {
        "lines"
    }

    fn description(&self) -> &str {
        "Converts input to lines."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("lines")
            .input_output_types(vec![(Type::Any, Type::List(Box::new(Type::String)))])
            .switch("skip-empty", "Skip empty lines.", Some('s'))
            .switch("strict", "Validate UTF-8 strictly.", None)
            .category(Category::Filters)
    }
    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let skip_empty = call.has_flag(engine_state, stack, "skip-empty")?;
        let strict = call.has_flag(engine_state, stack, "strict")?;

        match input {
            PipelineData::Value(value, ..) => match value {
                Value::String { val, .. } => {
                    let lines = ByteStream::read_string(val, head, Signals::empty())
                        .lines()
                        .expect(".lines() always succeeds for ByteStreamSource::Read");
                    // source is a UTF-8 String, so strict mode should always produce valid UTF-8 strings
                    let lines = lines.strict(true);

                    Ok(lines_to_pipeline_data(
                        lines,
                        skip_empty,
                        head,
                        engine_state.signals().clone(),
                    ))
                }
                // Propagate existing errors
                Value::Error { error, .. } => Err(*error),
                value => Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string or byte stream".into(),
                    wrong_type: value.get_type().to_string(),
                    dst_span: head,
                    src_span: value.span(),
                }),
            },
            PipelineData::Empty => Ok(PipelineData::empty()),
            PipelineData::ListStream(stream, metadata) => {
                let stream = stream.modify(|iter| {
                    iter.filter_map(move |value| {
                        let span = value.span();
                        if let Value::String { val, .. } = value {
                            Some(
                                val.lines()
                                    .filter_map(|s| {
                                        if skip_empty && s.trim().is_empty() {
                                            None
                                        } else {
                                            Some(Value::string(s, span))
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        } else {
                            None
                        }
                    })
                    .flatten()
                });

                Ok(PipelineData::list_stream(stream, metadata))
            }
            PipelineData::ByteStream(stream, ..) => {
                if let Some(lines) = stream.lines().map(|l| l.strict(strict)) {
                    Ok(lines_to_pipeline_data(
                        lines,
                        skip_empty,
                        head,
                        engine_state.signals().clone(),
                    ))
                } else {
                    Ok(PipelineData::empty())
                }
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Split multi-line string into lines",
                example: r#"$"two\nlines" | lines"#,
                result: Some(Value::list(
                    vec![Value::test_string("two"), Value::test_string("lines")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Skip empty lines",
                example: r#""foo\n\nbar" | lines --skip-empty"#,
                result: Some(Value::list(
                    vec![Value::test_string("foo"), Value::test_string("bar")],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn lines_to_pipeline_data(
    lines: impl Iterator<Item = Result<String, ShellError>> + Send + 'static,
    skip_empty: bool,
    span: Span,
    signals: Signals,
) -> PipelineData {
    lines
        .filter_map(move |line| match line {
            Ok(line) if skip_empty && line.trim().is_empty() => None,
            Ok(line) => Some(Value::string(line, span)),
            Err(err) => Some(Value::error(err, span)),
        })
        .into_pipeline_data(span, signals)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Lines)
    }
}
