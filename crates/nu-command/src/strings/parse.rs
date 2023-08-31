use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use fancy_regex::Regex;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, Record, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
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
        vec!["pattern", "match", "regex"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("parse")
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to match. Eg) \"{foo}: {bar}\"",
            )
            .input_output_types(vec![
                (Type::String, Type::Table(vec![])),
                (Type::List(Box::new(Type::Any)), Type::Table(vec![])),
            ])
            .switch("regex", "use full regex syntax for patterns", Some('r'))
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<Example> {
        let result = Value::list(
            vec![Value::test_record(Record {
                cols: vec!["foo".to_string(), "bar".to_string()],
                vals: vec![Value::test_string("hi"), Value::test_string("there")],
            })],
            Span::test_data(),
        );

        vec![
            Example {
                description: "Parse a string into two named columns",
                example: "\"hi there\" | parse \"{foo} {bar}\"",
                result: Some(result.clone()),
            },
            Example {
                description: "Parse a string using regex pattern",
                example: "\"hi there\" | parse -r '(?P<foo>\\w+) (?P<bar>\\w+)'",
                result: Some(result),
            },
            Example {
                description: "Parse a string using fancy-regex named capture group pattern",
                example: "\"foo bar.\" | parse -r '\\s*(?<name>\\w+)(?=\\.)'",
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["name".to_string()],
                        vals: vec![Value::test_string("bar")],
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Parse a string using fancy-regex capture group pattern",
                example: "\"foo! bar.\" | parse -r '(\\w+)(?=\\.)|(\\w+)(?=!)'",
                result: Some(Value::list(
                    vec![
                        Value::test_record(Record {
                            cols: vec!["capture0".to_string(), "capture1".to_string()],
                            vals: vec![Value::test_string(""), Value::test_string("foo")],
                        }),
                        Value::test_record(Record {
                            cols: vec!["capture0".to_string(), "capture1".to_string()],
                            vals: vec![Value::test_string("bar"), Value::test_string("")],
                        }),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Parse a string using fancy-regex look behind pattern",
                example:
                    "\" @another(foo bar)   \" | parse -r '\\s*(?<=[() ])(@\\w+)(\\([^)]*\\))?\\s*'",
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["capture0".to_string(), "capture1".to_string()],
                        vals: vec![
                            Value::test_string("@another"),
                            Value::test_string("(foo bar)"),
                        ],
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Parse a string using fancy-regex look ahead atomic group pattern",
                example: "\"abcd\" | parse -r '^a(bc(?=d)|b)cd$'",
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["capture0".to_string()],
                        vals: vec![Value::test_string("b")],
                    })],
                    Span::test_data(),
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
    let regex: bool = call.has_flag("regex");
    let ctrlc = engine_state.ctrlc.clone();

    let pattern_item = pattern.item;
    let pattern_span = pattern.span;

    let item_to_parse = if regex {
        pattern_item
    } else {
        build_regex(&pattern_item, pattern_span)?
    };

    let regex_pattern = Regex::new(&item_to_parse).map_err(|err| {
        ShellError::GenericError(
            "Error with regular expression".into(),
            err.to_string(),
            Some(pattern_span),
            None,
            Vec::new(),
        )
    })?;

    let columns = column_names(&regex_pattern);

    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(..) => {
            let mut parsed: Vec<Value> = Vec::new();

            for v in input {
                match v.as_string() {
                    Ok(s) => {
                        let results = regex_pattern.captures_iter(&s);

                        for c in results {
                            let captures = match c {
                                Ok(c) => c,
                                Err(e) => {
                                    return Err(ShellError::GenericError(
                                        "Error with regular expression captures".into(),
                                        e.to_string(),
                                        None,
                                        None,
                                        Vec::new(),
                                    ))
                                }
                            };

                            let v_span = v.span();
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
                            src_span: v.span(),
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

            let Some(v) = self.stream.next() else { return None };

            let Ok(s) = v.as_string() else {
                return Some(Value::error (
                    ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: self.span,
                        src_span: v.span(),
                    },
                     v.span(),
                ))
            };

            let parsed = stream_helper(
                self.regex.clone(),
                v.span(),
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
            ))
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
                    ShellError::GenericError(
                        "Error with regular expression captures".into(),
                        e.to_string(),
                        Some(span),
                        Some(e.to_string()),
                        Vec::new(),
                    ),
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
