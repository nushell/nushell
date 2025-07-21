use nu_engine::command_prelude::*;

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
            .switch("skip-empty", "skip empty lines", Some('s'))
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

        let span = input.span().unwrap_or(call.head);
        match input {
            PipelineData::Value(value, ..) => match value {
                Value::String { val, .. } => {
                    let lines = if skip_empty {
                        val.lines()
                            .filter_map(|s| {
                                if s.trim().is_empty() {
                                    None
                                } else {
                                    Some(Value::string(s, span))
                                }
                            })
                            .collect()
                    } else {
                        val.lines().map(|s| Value::string(s, span)).collect()
                    };

                    Ok(Value::list(lines, span).into_pipeline_data())
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
                if let Some(lines) = stream.lines() {
                    Ok(lines
                        .map(move |line| match line {
                            Ok(line) => Value::string(line, head),
                            Err(err) => Value::error(err, head),
                        })
                        .into_pipeline_data(head, engine_state.signals().clone()))
                } else {
                    Ok(PipelineData::empty())
                }
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split multi-line string into lines",
            example: r#"$"two\nlines" | lines"#,
            result: Some(Value::list(
                vec![Value::test_string("two"), Value::test_string("lines")],
                Span::test_data(),
            )),
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Lines {})
    }
}
