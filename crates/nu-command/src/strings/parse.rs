use fancy_regex::{Regex, RegexBuilder};
use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, Signals, engine::StateWorkingSet};
use std::collections::VecDeque;

#[derive(Clone)]
pub struct Parse;

impl Command for Parse {
    fn name(&self) -> &str {
        "parse"
    }

    fn description(&self) -> &str {
        "Parse columns from string data using a simple pattern or a supplied regular expression."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match", "regex", "str extract"]
    }

    fn extra_description(&self) -> &str {
        "The parse command always uses regular expressions even when you use a simple pattern. If a simple pattern is supplied, parse will transform that pattern into a regular expression."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("parse")
            .required("pattern", SyntaxShape::String, "The pattern to match.")
            .input_output_types(vec![
                (Type::String, Type::table()),
                (Type::List(Box::new(Type::Any)), Type::table()),
            ])
            .switch("regex", "use full regex syntax for patterns", Some('r'))
            .named(
                "backtrack",
                SyntaxShape::Int,
                "set the max backtrack limit for regex",
                Some('b'),
            )
            .allow_variants_without_examples(true)
            .named(
                "before",
                SyntaxShape::String,
                "add a column with the given name for the unmatched text before each match",
                None,
            )
            .named(
                "after",
                SyntaxShape::String,
                "add a column with the given name for the unmatched text after each match",
                None,
            )
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a string into two named columns",
                example: "\"hi there\" | parse \"{foo} {bar}\"",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "foo" => Value::test_string("hi"),
                    "bar" => Value::test_string("there"),
                })])),
            },
            Example {
                description: "Parse a string, ignoring a column with _",
                example: "\"hello world\" | parse \"{foo} {_}\"",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "foo" => Value::test_string("hello"),
                })])),
            },
            Example {
                description: "This is how the first example is interpreted in the source code",
                example: "\"hi there\" | parse --regex '(?s)\\A(?P<foo>.*?) (?P<bar>.*?)\\z'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "foo" => Value::test_string("hi"),
                    "bar" => Value::test_string("there"),
                })])),
            },
            Example {
                description: "Parse a string using fancy-regex named capture group pattern",
                example: "\"foo bar.\" | parse --regex '\\s*(?<name>\\w+)(?=\\.)'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::test_string("bar"),
                })])),
            },
            Example {
                description: "Parse a string using fancy-regex capture group pattern",
                example: "\"foo! bar.\" | parse --regex '(\\w+)(?=\\.)|(\\w+)(?=!)'",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "capture0" => Value::test_string(""),
                        "capture1" => Value::test_string("foo"),
                    }),
                    Value::test_record(record! {
                        "capture0" => Value::test_string("bar"),
                        "capture1" => Value::test_string(""),
                    }),
                ])),
            },
            Example {
                description: "Parse a string using fancy-regex look behind pattern",
                example: "\" @another(foo bar)   \" | parse --regex '\\s*(?<=[() ])(@\\w+)(\\([^)]*\\))?\\s*'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "capture0" => Value::test_string("@another"),
                    "capture1" => Value::test_string("(foo bar)"),
                })])),
            },
            Example {
                description: "Parse a string using fancy-regex look ahead atomic group pattern",
                example: "\"abcd\" | parse --regex '^a(bc(?=d)|b)cd$'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "capture0" => Value::test_string("b"),
                })])),
            },
            Example {
                description: "Parse a string with a manually set fancy-regex backtrack limit",
                example: "\"hi there\" | parse --backtrack 1500000 \"{foo} {bar}\"",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "foo" => Value::test_string("hi"),
                    "bar" => Value::test_string("there"),
                })])),
            },
            Example {
                description: "Parse a string and collect the text after each match",
                example: r#""1) first entry 2) second entry \n(multiline) 3) final entry" | parse -r '(?P<number>\d)\) ' --after "content""#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "number" => Value::test_string("1"),
                        "content" => Value::test_string("first entry "),
                    }),
                    Value::test_record(record! {
                        "number" => Value::test_string("2"),
                        "content" => Value::test_string("second entry \n(multiline) "),
                    }),
                    Value::test_record(record! {
                        "number" => Value::test_string("3"),
                        "content" => Value::test_string("final entry"),
                    }),
                ])),
            },
            Example {
                description: "Parse a string and collect the text before each match",
                example: r#"'some text (page 7) some more text (page 12)' | parse -r ' \(page (?P<page>.+?)\);? ?' --before "text""#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "text" => Value::test_string("some text"),
                        "page" => Value::test_string("7"),
                    }),
                    Value::test_record(record! {
                        "text" => Value::test_string("some more text"),
                        "page" => Value::test_string("12"),
                    }),
                ])),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
        let regex: bool = call.has_flag(engine_state, stack, "regex")?;
        let backtrack_limit: usize = call
            .get_flag(engine_state, stack, "backtrack")?
            .unwrap_or(1_000_000); // 1_000_000 is fancy_regex default
        let before = call.get_flag(engine_state, stack, "before")?;
        let after = call.get_flag(engine_state, stack, "after")?;
        operate(
            engine_state,
            pattern,
            regex,
            backtrack_limit,
            before,
            after,
            call,
            input,
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Spanned<String> = call.req_const(working_set, 0)?;
        let regex: bool = call.has_flag_const(working_set, "regex")?;
        let backtrack_limit: usize = call
            .get_flag_const(working_set, "backtrack")?
            .unwrap_or(1_000_000);
        let before = call.get_flag_const(working_set, "before")?;
        let after = call.get_flag_const(working_set, "after")?;
        operate(
            working_set.permanent(),
            pattern,
            regex,
            backtrack_limit,
            before,
            after,
            call,
            input,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn operate(
    engine_state: &EngineState,
    pattern: Spanned<String>,
    regex: bool,
    backtrack_limit: usize,
    before: Option<String>,
    after: Option<String>,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;

    let pattern_item = pattern.item;
    let pattern_span = pattern.span;

    let item_to_parse = if regex {
        pattern_item
    } else {
        build_regex(&pattern_item, pattern_span)?
    };

    let regex = RegexBuilder::new(&item_to_parse)
        .backtrack_limit(backtrack_limit)
        .build()
        .map_err(|e| ShellError::GenericError {
            error: "Error with regular expression".into(),
            msg: e.to_string(),
            span: Some(pattern_span),
            help: None,
            inner: vec![],
        })?;

    let columns = regex
        .capture_names()
        .skip(1)
        .enumerate()
        .map(|(i, name)| {
            name.map(String::from)
                .unwrap_or_else(|| format!("capture{i}"))
        })
        .collect::<Vec<_>>();

    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(value, ..) => match value {
            Value::String { val, .. } => {
                let captures: Vec<Value> = ParseIter::new(
                    regex,
                    [Ok(val)].into_iter(),
                    head,
                    engine_state.signals().clone(),
                    columns,
                    before,
                    after,
                )
                .collect();
                Ok(Value::list(captures, head).into_pipeline_data())
            }
            Value::List { vals, .. } => {
                let iter = vals.into_iter().map(move |val| {
                    let span = val.span();
                    let type_ = val.get_type();
                    val.into_string()
                        .map_err(|_| ShellError::OnlySupportsThisInputType {
                            exp_input_type: "string".into(),
                            wrong_type: type_.to_string(),
                            dst_span: head,
                            src_span: span,
                        })
                });
                let iter = ParseIter::new(
                    regex,
                    iter,
                    head,
                    engine_state.signals().clone(),
                    columns,
                    before,
                    after,
                );

                Ok(ListStream::new(iter, head, Signals::empty()).into())
            }
            value => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: value.get_type().to_string(),
                dst_span: head,
                src_span: value.span(),
            }),
        },
        PipelineData::ListStream(stream, ..) => Ok(stream
            .modify(|stream| {
                let iter = stream.map(move |val| {
                    let span = val.span();
                    val.into_string().map_err(|_| ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: head,
                        src_span: span,
                    })
                });

                ParseIter::new(
                    regex,
                    iter,
                    head,
                    engine_state.signals().clone(),
                    columns,
                    before,
                    after,
                )
            })
            .into()),
        PipelineData::ByteStream(stream, ..) => {
            if let Some(lines) = stream.lines() {
                let iter = ParseIter::new(
                    regex,
                    lines.map(|line_res| {
                        // The lines iterator will remove the newline characters. Put them back in.
                        line_res.map(|mut line| {
                            line.push('\n');
                            line
                        })
                    }),
                    head,
                    engine_state.signals().clone(),
                    columns,
                    before,
                    after,
                );

                Ok(ListStream::new(iter, head, Signals::empty()).into())
            } else {
                Ok(PipelineData::Empty)
            }
        }
    }
}

fn build_regex(input: &str, span: Span) -> Result<String, ShellError> {
    let mut output = "(?s)\\A".to_string();

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
            output.push_str("(?");
            if column == "_" {
                // discard placeholder column(s)
                output.push(':');
            } else {
                // create capture group for column
                output.push_str("P<");
                output.push_str(&column);
                output.push('>');
            }
            output.push_str(".*?)");
        }

        if before.is_empty() && column.is_empty() {
            break;
        }
    }

    output.push_str("\\z");
    Ok(output)
}

struct ParseIter<I: Iterator<Item = Result<String, ShellError>>> {
    regex: Regex,
    iter: I,
    columns: Vec<String>,
    before: Option<String>,
    after: Option<String>,
    span: Span,
    signals: Signals,

    captures: VecDeque<Value>,
    last_capture: Option<Record>,
    buffer: String,
}

impl<I: Iterator<Item = Result<String, ShellError>>> ParseIter<I> {
    fn new(
        regex: Regex,
        iter: I,
        span: Span,
        signals: Signals,
        columns: Vec<String>,
        before: Option<String>,
        after: Option<String>,
    ) -> Self {
        let captures = VecDeque::new();
        let last_capture = None;
        let buffer = String::new();

        Self {
            regex,
            iter,
            columns,
            before,
            after,
            span,
            signals,
            captures,
            last_capture,
            buffer,
        }
    }

    fn try_next(&mut self) -> Result<Option<Value>, ShellError> {
        loop {
            if self.signals.interrupted() {
                return Ok(None);
            }

            if let Some(val) = self.captures.pop_front() {
                return Ok(Some(val));
            }

            let next_s = match self.iter.next() {
                Some(res) => res?,
                None => break,
            };

            let mut idx = 0;
            let prev_len = self.buffer.len();
            self.buffer.push_str(&next_s);

            for captures in self.regex.captures_iter(&next_s) {
                let captures = captures.map_err(|err| self.convert_regex_error(err))?;

                let Some(m) = captures.get(0) else {
                    return Err(ShellError::NushellFailed {
                        msg: "capture.get(0) should always return the full regex match".to_string(),
                    });
                };

                // trim the newlines at the end of the captured text
                let text_before = trim_newline(&self.buffer[idx..(m.start() + prev_len)]);

                let mut record = Record::new();

                if let Some(column_name) = &self.before {
                    record.push(column_name, Value::string(text_before, self.span));
                }

                self.columns
                    .iter()
                    .zip(captures.iter().skip(1))
                    .for_each(|(column, match_)| {
                        let match_str = match_.map(|m| m.as_str()).unwrap_or("");
                        record.push(column.clone(), Value::string(match_str, self.span))
                    });

                if let Some(mut last_record) = self.last_capture.take() {
                    if let Some(column_name) = &self.after {
                        last_record.push(column_name, Value::string(text_before, self.span));
                    }

                    self.captures
                        .push_back(Value::record(last_record, self.span));
                }
                self.last_capture = Some(record);
                idx = m.end() + prev_len;
            }
            self.buffer = self.buffer.split_off(idx);
        }
        Ok(self.last_capture.take().map(|mut last_record| {
            if let Some(column_name) = &self.after {
                last_record.push(
                    column_name,
                    Value::string(trim_newline(&self.buffer), self.span),
                );
            }
            Value::record(last_record, self.span)
        }))
    }

    fn convert_regex_error(&self, err: fancy_regex::Error) -> ShellError {
        ShellError::GenericError {
            error: "Error with regular expression captures".into(),
            msg: err.to_string(),
            span: Some(self.span),
            help: None,
            inner: vec![],
        }
    }
}

impl<I: Iterator<Item = Result<String, ShellError>>> Iterator for ParseIter<I> {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        match self.try_next() {
            Ok(res) => res,
            Err(err) => Some(Value::error(err, self.span)),
        }
    }
}

fn trim_newline(s: &str) -> &str {
    if let Some('\n') = s.chars().last() {
        &s[0..(s.len() - 1)]
    } else {
        s
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
