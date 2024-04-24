use nu_engine::command_prelude::*;
use nu_protocol::RawStream;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct Lines;

impl Command for Lines {
    fn name(&self) -> &str {
        "lines"
    }

    fn usage(&self) -> &str {
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
        let ctrlc = engine_state.ctrlc.clone();
        let skip_empty = call.has_flag(engine_state, stack, "skip-empty")?;

        let span = input.span().unwrap_or(call.head);
        match input {
            PipelineData::Value(Value::String { val, .. }, ..) => {
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
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::ListStream(stream, metadata) => {
                let iter = stream
                    .into_iter()
                    .filter_map(move |value| {
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
                    .flatten();

                Ok(iter
                    .into_pipeline_data(engine_state.ctrlc.clone())
                    .set_metadata(metadata))
            }
            PipelineData::Value(val, ..) => {
                match val {
                    // Propagate existing errors
                    Value::Error { error, .. } => Err(*error),
                    _ => Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "string or raw data".into(),
                        wrong_type: val.get_type().to_string(),
                        dst_span: head,
                        src_span: val.span(),
                    }),
                }
            }
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                metadata,
                ..
            } => Ok(RawStreamLinesAdapter::new(stream, head, skip_empty)
                .map(move |x| x.unwrap_or_else(|err| Value::error(err, head)))
                .into_pipeline_data(ctrlc)
                .set_metadata(metadata)),
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

#[derive(Debug)]
struct RawStreamLinesAdapter {
    inner: RawStream,
    inner_complete: bool,
    skip_empty: bool,
    span: Span,
    incomplete_line: String,
    queue: VecDeque<String>,
}

impl Iterator for RawStreamLinesAdapter {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(s) = self.queue.pop_front() {
                if self.skip_empty && s.trim().is_empty() {
                    continue;
                }
                return Some(Ok(Value::string(s, self.span)));
            } else {
                // inner is complete, feed out remaining state
                if self.inner_complete {
                    return if self.incomplete_line.is_empty() {
                        None
                    } else {
                        Some(Ok(Value::string(
                            std::mem::take(&mut self.incomplete_line),
                            self.span,
                        )))
                    };
                }

                // pull more data from inner
                if let Some(result) = self.inner.next() {
                    match result {
                        Ok(v) => {
                            let span = v.span();
                            match v {
                                // TODO: Value::Binary support required?
                                Value::String { val, .. } => {
                                    self.span = span;

                                    let mut lines = val.lines();

                                    // handle incomplete line from previous
                                    if !self.incomplete_line.is_empty() {
                                        if let Some(first) = lines.next() {
                                            self.incomplete_line.push_str(first);
                                            self.queue.push_back(std::mem::take(
                                                &mut self.incomplete_line,
                                            ));
                                        }
                                    }

                                    // save completed lines
                                    self.queue.extend(lines.map(String::from));

                                    if !val.ends_with('\n') {
                                        // incomplete line, save for next time
                                        // if `val` and `incomplete_line` were empty,
                                        // then pop will return none
                                        if let Some(s) = self.queue.pop_back() {
                                            self.incomplete_line = s;
                                        }
                                    }
                                }
                                // Propagate errors by explicitly matching them before the final case.
                                Value::Error { error, .. } => return Some(Err(*error)),
                                other => {
                                    return Some(Err(ShellError::OnlySupportsThisInputType {
                                        exp_input_type: "string".into(),
                                        wrong_type: other.get_type().to_string(),
                                        dst_span: self.span,
                                        src_span: other.span(),
                                    }));
                                }
                            }
                        }
                        Err(err) => return Some(Err(err)),
                    }
                } else {
                    self.inner_complete = true;
                }
            }
        }
    }
}

impl RawStreamLinesAdapter {
    pub fn new(inner: RawStream, span: Span, skip_empty: bool) -> Self {
        Self {
            inner,
            span,
            skip_empty,
            incomplete_line: String::new(),
            queue: VecDeque::new(),
            inner_complete: false,
        }
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
