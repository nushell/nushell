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
        "Converts input to lines"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("lines")
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .switch("skip-empty", "skip empty lines", Some('s'))
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let skip_empty = call.has_flag("skip-empty");
        match input {
            #[allow(clippy::needless_collect)]
            // Collect is needed because the string may not live long enough for
            // the Rc structure to continue using it. If split could take ownership
            // of the split values, then this wouldn't be needed
            PipelineData::Value(Value::String { val, span }, ..) => {
                let split_char = if val.contains("\r\n") { "\r\n" } else { "\n" };

                let mut lines = val
                    .split(split_char)
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

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
            PipelineData::ListStream(stream, ..) => {
                let mut split_char = "\n";

                let iter = stream
                    .into_iter()
                    .filter_map(move |value| {
                        if let Value::String { val, span } = value {
                            if split_char != "\r\n" && val.contains("\r\n") {
                                split_char = "\r\n";
                            }

                            let mut lines = val
                                .split(split_char)
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

                            Some(
                                lines
                                    .into_iter()
                                    .map(move |x| Value::String { val: x, span }),
                            )
                        } else {
                            None
                        }
                    })
                    .flatten();

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Value(val, ..) => Err(ShellError::UnsupportedInput(
                format!("Not supported input: {}", val.as_string()?),
                head,
            )),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::new(head)),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(RawStreamLinesAdapter::new(stream, head, skip_empty)
                .into_iter()
                .enumerate()
                .map(move |(_idx, x)| match x {
                    Ok(x) => x,
                    Err(err) => Value::Error { error: err },
                })
                .into_pipeline_data(ctrlc)),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split multi-line string into lines",
            example: r#"echo $"two\nlines" | lines"#,
            result: Some(Value::List {
                vals: vec![Value::test_string("two"), Value::test_string("lines")],
                span: Span::test_data(),
            }),
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

                return Some(Ok(Value::String {
                    val: s,
                    span: self.span,
                }));
            } else {
                // inner is complete, feed out remaining state
                if self.inner_complete {
                    if !self.incomplete_line.is_empty() {
                        let r = Some(Ok(Value::String {
                            val: self.incomplete_line.to_string(),
                            span: self.span,
                        }));
                        self.incomplete_line = String::new();
                        return r;
                    }

                    return None;
                }

                // pull more data from inner
                if let Some(result) = self.inner.next() {
                    match result {
                        Ok(v) => {
                            match v {
                                Value::String { val, span } => {
                                    self.span = span;

                                    let split_char =
                                        if val.contains("\r\n") { "\r\n" } else { "\n" };

                                    let mut lines = val
                                        .split(split_char)
                                        .map(|s| s.to_string())
                                        .collect::<Vec<_>>();

                                    // handle incomplete line from previous
                                    if !self.incomplete_line.is_empty() {
                                        if let Some(first) = lines.first() {
                                            let new_incomplete_line =
                                                self.incomplete_line.to_string() + first.as_str();
                                            lines.splice(0..1, vec![new_incomplete_line]);
                                            self.incomplete_line = String::new();
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
                                    self.queue.append(&mut lines);
                                }
                                // TODO: Value::Binary support required?
                                _ => {
                                    return Some(Err(ShellError::UnsupportedInput(
                                        "Unsupport type from raw stream".to_string(),
                                        self.span,
                                    )))
                                }
                            }
                        }
                        Err(_) => todo!(),
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
