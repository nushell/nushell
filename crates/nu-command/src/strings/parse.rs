use fancy_regex::{Captures, Regex};
use nu_engine::command_prelude::*;
use nu_protocol::{engine::StateWorkingSet, ListStream, Signals};
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
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a string into two named columns",
                example: "\"hi there\" | parse \"{foo} {bar}\"",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "foo" => Value::test_string("hi"),
                        "bar" => Value::test_string("there"),
                    })])),
            },
            Example {
                description: "This is how the first example is interpreted in the source code",
                example: "\"hi there\" | parse --regex '(?s)\\A(?P<foo>.*?) (?P<bar>.*?)\\z'",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "foo" => Value::test_string("hi"),
                        "bar" => Value::test_string("there"),
                    })])),
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
        operate(engine_state, pattern, regex, call, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Spanned<String> = call.req_const(working_set, 0)?;
        let regex: bool = call.has_flag_const(working_set, "regex")?;
        operate(working_set.permanent(), pattern, regex, call, input)
    }
}

fn operate(
    engine_state: &EngineState,
    pattern: Spanned<String>,
    regex: bool,
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

    let regex = Regex::new(&item_to_parse).map_err(|e| ShellError::GenericError {
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
                let captures = regex
                    .captures_iter(&val)
                    .map(|captures| captures_to_value(captures, &columns, head))
                    .collect::<Result<_, _>>()?;

                Ok(Value::list(captures, head).into_pipeline_data())
            }
            Value::List { vals, .. } => {
                let iter = vals.into_iter().map(move |val| {
                    let span = val.span();
                    val.into_string().map_err(|_| ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: head,
                        src_span: span,
                    })
                });

                let iter = ParseIter {
                    captures: VecDeque::new(),
                    regex,
                    columns,
                    iter,
                    span: head,
                    signals: engine_state.signals().clone(),
                };

                Ok(ListStream::new(iter, head, Signals::empty()).into())
            }
            value => Err(ShellError::PipelineMismatch {
                exp_input_type: "string".into(),
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

                ParseIter {
                    captures: VecDeque::new(),
                    regex,
                    columns,
                    iter,
                    span: head,
                    signals: engine_state.signals().clone(),
                }
            })
            .into()),
        PipelineData::ByteStream(stream, ..) => {
            if let Some(lines) = stream.lines() {
                let iter = ParseIter {
                    captures: VecDeque::new(),
                    regex,
                    columns,
                    iter: lines,
                    span: head,
                    signals: engine_state.signals().clone(),
                };

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

struct ParseIter<I: Iterator<Item = Result<String, ShellError>>> {
    captures: VecDeque<Value>,
    regex: Regex,
    columns: Vec<String>,
    iter: I,
    span: Span,
    signals: Signals,
}

impl<I: Iterator<Item = Result<String, ShellError>>> ParseIter<I> {
    fn populate_captures(&mut self, str: &str) -> Result<(), ShellError> {
        for captures in self.regex.captures_iter(str) {
            self.captures
                .push_back(captures_to_value(captures, &self.columns, self.span)?);
        }
        Ok(())
    }
}

impl<I: Iterator<Item = Result<String, ShellError>>> Iterator for ParseIter<I> {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        loop {
            if self.signals.interrupted() {
                return None;
            }

            if let Some(val) = self.captures.pop_front() {
                return Some(val);
            }

            let result = self
                .iter
                .next()?
                .and_then(|str| self.populate_captures(&str));

            if let Err(err) = result {
                return Some(Value::error(err, self.span));
            }
        }
    }
}

fn captures_to_value(
    captures: Result<Captures, fancy_regex::Error>,
    columns: &[String],
    span: Span,
) -> Result<Value, ShellError> {
    let captures = captures.map_err(|err| ShellError::GenericError {
        error: "Error with regular expression captures".into(),
        msg: err.to_string(),
        span: Some(span),
        help: None,
        inner: vec![],
    })?;

    let record = columns
        .iter()
        .zip(captures.iter().skip(1))
        .map(|(column, match_)| {
            let match_str = match_.map(|m| m.as_str()).unwrap_or("");
            (column.clone(), Value::string(match_str, span))
        })
        .collect();

    Ok(Value::record(record, span))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(Parse)
    }
}
