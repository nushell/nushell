use fancy_regex::Regex;
use nu_engine::command_prelude::*;
use nu_protocol::ListStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
pub struct Parse;

impl Command for Parse {
    fn name(&self) -> &str {
        "parse"
    }

    fn usage(&self) -> &str {
        "Parse columns from string data using a simple pattern."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match", "regex", "str extract"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("parse")
            .required("pattern", SyntaxShape::String, "The pattern to match.")
            .input_output_types(vec![
                (Type::String, Type::table()),
                (Type::List(Box::new(Type::Any)), Type::table()),
            ])
            .switch("regex", "use full regex syntax for patterns", Some('r'))
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<Example> {
        let result = Value::test_list(vec![Value::test_record(record! {
            "foo" => Value::test_string("hi"),
            "bar" => Value::test_string("there"),
        })]);

        vec![
            Example {
                description: "Parse a string into two named columns",
                example: "\"hi there\" | parse \"{foo} {bar}\"",
                result: Some(result.clone()),
            },
            Example {
                description: "Parse a string using regex pattern",
                example: "\"hi there\" | parse --regex '(?P<foo>\\w+) (?P<bar>\\w+)'",
                result: Some(result),
            },
            Example {
                description: "Parse a string using fancy-regex named capture group pattern",
                example: "\"foo bar.\" | parse --regex '\\s*(?<name>\\w+)(?=\\.)'",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "name" => Value::test_string("bar"),
                    })],
                )),
            },
            Example {
                description: "Parse a string using fancy-regex capture group pattern",
                example: "\"foo! bar.\" | parse --regex '(\\w+)(?=\\.)|(\\w+)(?=!)'",
                result: Some(Value::test_list(
                    vec![
                        Value::test_record(record! {
                            "capture0" => Value::test_string(""),
                            "capture1" => Value::test_string("foo"),
                        }),
                        Value::test_record(record! {
                            "capture0" => Value::test_string("bar"),
                            "capture1" => Value::test_string(""),
                        }),
                    ],
                )),
            },
            Example {
                description: "Parse a string using fancy-regex look behind pattern",
                example:
                    "\" @another(foo bar)   \" | parse --regex '\\s*(?<=[() ])(@\\w+)(\\([^)]*\\))?\\s*'",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "capture0" => Value::test_string("@another"),
                        "capture1" => Value::test_string("(foo bar)"),
                    })],
                )),
            },
            Example {
                description: "Parse a string using fancy-regex look ahead atomic group pattern",
                example: "\"abcd\" | parse --regex '^a(bc(?=d)|b)cd$'",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "capture0" => Value::test_string("b"),
                    })],
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
    let regex: bool = call.has_flag(engine_state, stack, "regex")?;
    let ctrlc = engine_state.ctrlc.clone();

    let pattern_item = pattern.item;
    let pattern_span = pattern.span;

    let item_to_parse = if regex {
        pattern_item
    } else {
        build_regex(&pattern_item, pattern_span)?
    };

    let regex_pattern = Regex::new(&item_to_parse).map_err(|e| ShellError::GenericError {
        error: "Error with regular expression".into(),
        msg: e.to_string(),
        span: Some(pattern_span),
        help: None,
        inner: vec![],
    })?;

    let columns = column_names(&regex_pattern);

    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(..) => {
            let mut parsed: Vec<Value> = Vec::new();

            for v in input {
                let v_span = v.span();
                match v.coerce_into_string() {
                    Ok(s) => {
                        let results = regex_pattern.captures_iter(&s);

                        for c in results {
                            let captures = match c {
                                Ok(c) => c,
                                Err(e) => {
                                    return Err(ShellError::GenericError {
                                        error: "Error with regular expression captures".into(),
                                        msg: e.to_string(),
                                        span: None,
                                        help: None,
                                        inner: vec![],
                                    })
                                }
                            };

                            let record = columns
                                .iter()
                                .zip(captures.iter().skip(1))
                                .map(|(column_name, cap)| {
                                    let cap_string = cap.map(|v| v.as_str()).unwrap_or("");
                                    (column_name.clone(), Value::string(cap_string, v_span))
                                })
                                .collect();

                            parsed.push(Value::record(record, head));
                        }
                    }
                    Err(_) => {
                        return Err(ShellError::PipelineMismatch {
                            exp_input_type: "string".into(),
                            dst_span: head,
                            src_span: v_span,
                        })
                    }
                }
            }

            Ok(PipelineData::ListStream(
                ListStream::from_stream(parsed.into_iter(), ctrlc),
                None,
            ))
        }
        PipelineData::ListStream(stream, ..) => Ok(PipelineData::ListStream(
            ListStream::from_stream(
                ParseStreamer {
                    span: head,
                    excess: Vec::new(),
                    regex: regex_pattern,
                    columns,
                    stream: stream.stream,
                    ctrlc: ctrlc.clone(),
                },
                ctrlc,
            ),
            None,
        )),

        PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::Empty),

        PipelineData::ExternalStream {
            stdout: Some(stream),
            ..
        } => Ok(PipelineData::ListStream(
            ListStream::from_stream(
                ParseStreamerExternal {
                    span: head,
                    excess: Vec::new(),
                    regex: regex_pattern,
                    columns,
                    stream: stream.stream,
                },
                ctrlc,
            ),
            None,
        )),
    }
}

fn build_regex(input: &str, span: Span) -> Result<String, ShellError> {
    let mut output = "(?s)\\A".to_string();

    //let mut loop_input = input;
    let mut loop_input = input.chars().peekable();
    loop {
        let mut before = String::new();
        while let Some(c) = loop_input.next() {
            if c == '{' {
                // If '{{', still creating a plaintext parse command, but just for a single '{' char
                if loop_input.peek() == Some(&'{') {
                    let _ = loop_input.next();
                } else {
                    break;
                }
            }
            before.push(c);
        }

        if !before.is_empty() {
            output.push_str(&fancy_regex::escape(&before));
        }

        // Look for column as we're now at one
        let mut column = String::new();
        while let Some(c) = loop_input.next() {
            if c == '}' {
                break;
            }
            column.push(c);

            if loop_input.peek().is_none() {
                return Err(ShellError::DelimiterError {
                    msg: "Found opening `{` without an associated closing `}`".to_owned(),
                    span,
                });
            }
        }

        if !column.is_empty() {
            output.push_str("(?P<");
            output.push_str(&column);
            output.push_str(">.*?)");
        }

        if before.is_empty() && column.is_empty() {
            break;
        }
    }

    output.push_str("\\z");
    Ok(output)
}

fn column_names(regex: &Regex) -> Vec<String> {
    regex
        .capture_names()
        .enumerate()
        .skip(1)
        .map(|(i, name)| {
            name.map(String::from)
                .unwrap_or_else(|| format!("capture{}", i - 1))
        })
        .collect()
}

pub struct ParseStreamer {
    span: Span,
    excess: Vec<Value>,
    regex: Regex,
    columns: Vec<String>,
    stream: Box<dyn Iterator<Item = Value> + Send + 'static>,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl Iterator for ParseStreamer {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        if !self.excess.is_empty() {
            return Some(self.excess.remove(0));
        }

        loop {
            if let Some(ctrlc) = &self.ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    break None;
                }
            }

            let v = self.stream.next()?;
            let span = v.span();

            let Ok(s) = v.coerce_into_string() else {
                return Some(Value::error(
                    ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: self.span,
                        src_span: span,
                    },
                    span,
                ));
            };

            let parsed = stream_helper(
                self.regex.clone(),
                span,
                s,
                self.columns.clone(),
                &mut self.excess,
            );

            if parsed.is_none() {
                continue;
            };

            return parsed;
        }
    }
}

pub struct ParseStreamerExternal {
    span: Span,
    excess: Vec<Value>,
    regex: Regex,
    columns: Vec<String>,
    stream: Box<dyn Iterator<Item = Result<Vec<u8>, ShellError>> + Send + 'static>,
}

impl Iterator for ParseStreamerExternal {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        if !self.excess.is_empty() {
            return Some(self.excess.remove(0));
        }

        let mut chunk = self.stream.next();

        // Collect all `stream` chunks into a single `chunk` to be able to deal with matches that
        // extend across chunk boundaries.
        // This is a stop-gap solution until the `regex` crate supports streaming or an alternative
        // solution is found.
        // See https://github.com/nushell/nushell/issues/9795
        while let Some(Ok(chunks)) = &mut chunk {
            match self.stream.next() {
                Some(Ok(mut next_chunk)) => chunks.append(&mut next_chunk),
                error @ Some(Err(_)) => chunk = error,
                None => break,
            }
        }

        let chunk = match chunk {
            Some(Ok(chunk)) => chunk,
            Some(Err(err)) => return Some(Value::error(err, self.span)),
            _ => return None,
        };

        let Ok(chunk) = String::from_utf8(chunk) else {
            return Some(Value::error(
                ShellError::PipelineMismatch {
                    exp_input_type: "string".into(),
                    dst_span: self.span,
                    src_span: self.span,
                },
                self.span,
            ));
        };

        stream_helper(
            self.regex.clone(),
            self.span,
            chunk,
            self.columns.clone(),
            &mut self.excess,
        )
    }
}

fn stream_helper(
    regex: Regex,
    span: Span,
    s: String,
    columns: Vec<String>,
    excess: &mut Vec<Value>,
) -> Option<Value> {
    let results = regex.captures_iter(&s);

    for c in results {
        let captures = match c {
            Ok(c) => c,
            Err(e) => {
                return Some(Value::error(
                    ShellError::GenericError {
                        error: "Error with regular expression captures".into(),
                        msg: e.to_string(),
                        span: Some(span),
                        help: Some(e.to_string()),
                        inner: vec![],
                    },
                    span,
                ))
            }
        };

        let record = columns
            .iter()
            .zip(captures.iter().skip(1))
            .map(|(column_name, cap)| {
                let cap_string = cap.map(|v| v.as_str()).unwrap_or("");
                (column_name.clone(), Value::string(cap_string, span))
            })
            .collect();

        excess.push(Value::record(record, span));
    }

    if !excess.is_empty() {
        Some(excess.remove(0))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(Parse)
    }
}
