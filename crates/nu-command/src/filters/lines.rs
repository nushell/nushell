use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, RawStream, ShellError,
    Signature, Span, SpannedValue, Type,
};
use once_cell::sync::Lazy;
// regex can be replaced with fancy-regex once it supports `split()`
// https://github.com/fancy-regex/fancy-regex/issues/104
use regex::Regex;

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
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let skip_empty = call.has_flag("skip-empty");

        // match \r\n or \n
        static LINE_BREAK_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\r\n|\n").expect("unable to compile regex"));
        match input {
            #[allow(clippy::needless_collect)]
            // Collect is needed because the string may not live long enough for
            // the Rc structure to continue using it. If split could take ownership
            // of the split values, then this wouldn't be needed
            PipelineData::Value(SpannedValue::String { val, span }, ..) => {
                let mut lines = LINE_BREAK_REGEX
                    .split(&val)
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
                        Some(SpannedValue::string(s, span))
                    }
                });

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::ListStream(stream, ..) => {
                let iter = stream
                    .into_iter()
                    .filter_map(move |value| {
                        if let SpannedValue::String { val, span } = value {
                            let mut lines = LINE_BREAK_REGEX
                                .split(&val)
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
                                    .map(move |x| SpannedValue::String { val: x, span }),
                            )
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
                    SpannedValue::Error { error, .. } => Err(*error),
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
                    Err(err) => SpannedValue::Error {
                        error: Box::new(err),
                        span: head,
                    },
                })
                .into_pipeline_data(ctrlc)),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split multi-line string into lines",
            example: r#"$"two\nlines" | lines"#,
            result: Some(SpannedValue::List {
                vals: vec![
                    SpannedValue::test_string("two"),
                    SpannedValue::test_string("lines"),
                ],
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
    type Item = Result<SpannedValue, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        static LINE_BREAK_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\r\n|\n").expect("unable to compile regex"));

        loop {
            if !self.queue.is_empty() {
                let s = self.queue.remove(0usize);

                if self.skip_empty && s.trim().is_empty() {
                    continue;
                }

                return Some(Ok(SpannedValue::String {
                    val: s,
                    span: self.span,
                }));
            } else {
                // inner is complete, feed out remaining state
                if self.inner_complete {
                    if !self.incomplete_line.is_empty() {
                        let r = Some(Ok(SpannedValue::string(
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
                            match v {
                                // TODO: Value::Binary support required?
                                SpannedValue::String { val, span } => {
                                    self.span = span;

                                    let mut lines = LINE_BREAK_REGEX
                                        .split(&val)
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
                                // Propagate errors by explicitly matching them before the final case.
                                SpannedValue::Error { error, .. } => return Some(Err(*error)),
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
