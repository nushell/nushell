use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, RawStream, ShellError,
    Signature, Span, Type, Value,
};

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
                let mut lines = val.lines().map(String::from).collect::<Vec<_>>();

                // if the last one is empty, remove it, as it was just
                // a newline at the end of the input we got
                if let Some(last) = lines.last() {
                    if last.is_empty() {
                        lines.pop();
                    }
                }

                let iter = lines.into_iter().filter_map(move |s| {
                    if skip_empty && s.trim().is_empty() {
                        None
                    } else {
                        Some(Value::string(s, span))
                    }
                });

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::ListStream(stream, ..) => {
                let iter = stream
                    .into_iter()
                    .filter_map(move |value| {
                        let span = value.span();
                        if let Value::String { val, .. } = value {
                            let mut lines = val
                                .lines()
                                .filter_map(|s| {
                                    if skip_empty && s.trim().is_empty() {
                                        None
                                    } else {
                                        Some(s.to_string())
                                    }
                                })
                                .collect::<Vec<_>>();

                            // if the last one is empty, remove it, as it was just
                            // a newline at the end of the input we got
                            if let Some(last) = lines.last() {
                                if last.is_empty() {
                                    lines.pop();
                                }
                            }

                            Some(lines.into_iter().map(move |x| Value::string(x, span)))
                        } else {
                            None
                        }
                    })
                    .flatten();

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
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
                ..
            } => Ok(RawStreamLinesAdapter::new(stream, head, skip_empty)
                .enumerate()
                .map(move |(_idx, x)| match x {
                    Ok(x) => x,
                    Err(err) => Value::error(err, head),
                })
                .into_pipeline_data(ctrlc)),
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
    queue: Vec<String>,
}

impl Iterator for RawStreamLinesAdapter {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if !self.queue.is_empty() {
                let s = self.queue.remove(0usize);

                if self.skip_empty && s.trim().is_empty() {
                    continue;
                }

                return Some(Ok(Value::string(s, self.span)));
            } else {
                // inner is complete, feed out remaining state
                if self.inner_complete {
                    if !self.incomplete_line.is_empty() {
                        let r = Some(Ok(Value::string(
                            self.incomplete_line.to_string(),
                            self.span,
                        )));
                        self.incomplete_line = String::new();
                        return r;
                    }

                    return None;
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

                                    let mut lines =
                                        val.lines().map(String::from).collect::<Vec<_>>();

                                    // handle incomplete line from previous
                                    if !self.incomplete_line.is_empty() {
                                        if let Some(first) = lines.first_mut() {
                                            let incomplete_line =
                                                std::mem::take(&mut self.incomplete_line);
                                            let append_first =
                                                std::mem::replace(first, incomplete_line);
                                            first.push_str(&append_first);
                                        }
                                    }

                                    // store incomplete line from current
                                    if let Some(last) = lines.last() {
                                        if last.is_empty() {
                                            // we ended on a line ending
                                            lines.pop();
                                        } else {
                                            // incomplete line, save for next time
                                            if let Some(s) = lines.pop() {
                                                self.incomplete_line = s;
                                            }
                                        }
                                    }

                                    // save completed lines
                                    self.queue.extend(lines);
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
            queue: Vec::<String>::new(),
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
