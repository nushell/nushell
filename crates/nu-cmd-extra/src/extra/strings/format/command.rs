use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::{Config, ListStream, ast::PathMember, casing::Casing};

#[derive(Clone)]
pub struct FormatPattern;

impl Command for FormatPattern {
    fn name(&self) -> &str {
        "format pattern"
    }

    fn signature(&self) -> Signature {
        Signature::build("format pattern")
            .input_output_types(vec![
                (Type::table(), Type::List(Box::new(Type::String))),
                (Type::record(), Type::Any),
            ])
            .required(
                "pattern",
                SyntaxShape::String,
                "The pattern to output. e.g.) \"{foo}: {bar}\".",
            )
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Format columns into a string using a simple pattern."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
        let input_val = input.into_value(call.head)?;

        let ops = extract_formatting_operations(pattern, call.head)?;
        let config = stack.get_config(engine_state);

        format(input_val, &ops, engine_state, &config, call.head)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Print filenames with their sizes",
                example: "ls | format pattern '{name}: {size}'",
                result: None,
            },
            Example {
                description: "Print elements from some columns of a table",
                example: "[[col1, col2]; [v1, v2] [v3, v4]] | format pattern '{col2}'",
                result: Some(Value::test_list(vec![
                    Value::test_string("v2"),
                    Value::test_string("v4"),
                ])),
            },
            Example {
                description: "Escape braces by repeating them",
                example: r#"{start: 3, end: 5} | format pattern 'if {start} < {end} {{ "correct" }} else {{ "incorrect" }}'"#,
                result: Some(Value::test_string(
                    r#"if 3 < 5 { "correct" } else { "incorrect" }"#,
                )),
            },
        ]
    }
}

// NOTE: The reason to split {column1.column2} and {$it.column1.column2}:
// for {column1.column2}, we just need to follow given record or list.
// for {$it.column1.column2} or {$variable}, we need to manually evaluate the expression.
//
// Have thought about converting from {column1.column2} to {$it.column1.column2}, but that
// will extend input relative span, finally make `nu` panic out with message: span missing in file
// contents cache.
#[derive(Debug)]
enum FormatOperation {
    FixedText(String),
    // raw input is something like {column1.column2}
    ValueFromColumn { content: String, span: Option<Span> },
}

impl FormatOperation {
    fn update_span(mut self, f: impl FnOnce(Option<Span>) -> Option<Span>) -> Self {
        if let FormatOperation::ValueFromColumn { span, .. } = &mut self {
            *span = f(*span);
        }
        self
    }
}

/// Given a pattern that is fed into the Format command, we can process it and subdivide it
/// in two kind of operations.
/// FormatOperation::FixedText contains a portion of the pattern that has to be placed
/// there without any further processing.
/// FormatOperation::ValueFromColumn contains the name of a column whose values will be
/// formatted according to the input pattern.
/// "$it.column1.column2" or "$variable"
fn extract_formatting_operations(
    input: Spanned<String>,
    call_head: Span,
) -> Result<Vec<FormatOperation>, ShellError> {
    let Spanned {
        item: pattern,
        span: pattern_span,
    } = input;

    // To have proper spans for the extracted operations, we need the span of the pattern string.
    // Specifically we need the *string content*, without any surrounding quotes.
    //
    // NOTE: This implementation can't accurately derive spans for strings containing escape
    // sequences ("\n", "\t", "\u{3bd}", ...). I don't think we can without parser support.
    // NOTE: Pattern strings provided with variables are also problematic. The spans we get for
    // arguments are from the call site, we can't get the original span of a value passed as a
    // variable.
    let pattern_span = {
        //
        //    .----------span len: 21
        //    |     .--string len: 12
        //    |     |       delta:  9
        //  .-+-----|-----------.
        //  |    .--+-------.   |
        //  r###'hello {user}'###
        //
        let delta = pattern_span.len() - pattern.len();
        // might be `r'foo'` or `$'foo'`
        // either 1 or 0
        let str_prefix_len = delta % 2;
        //
        //    r###'hello {user}'###
        //    ^^^^
        let span_str_start_delta = delta / 2 + str_prefix_len;
        pattern_span.subspan(span_str_start_delta, span_str_start_delta + pattern.len())
    };

    let mut is_fixed = true;
    let ops = pattern.char_indices().peekable().batching(move |it| {
        let start_index = it.peek()?.0;
        let mut buf = String::new();
        while let Some((index, ch)) = it.next() {
            match ch {
                '{' if is_fixed => {
                    if it.next_if(|(_, next_ch)| *next_ch == '{').is_some() {
                        buf.push(ch);
                    } else {
                        is_fixed = false;
                        return Some(Ok(FormatOperation::FixedText(buf)));
                    };
                }
                '}' => {
                    if is_fixed {
                        if it.next_if(|(_, next_ch)| *next_ch == '}').is_some() {
                            buf.push(ch);
                        } else {
                            return Some(Err(()));
                        }
                    } else {
                        is_fixed = true;
                        return Some(Ok(FormatOperation::ValueFromColumn {
                            content: buf,
                            // span is relative to `pattern`
                            span: Some(Span::new(start_index, index)),
                        }));
                    }
                }
                _ => {
                    buf.push(ch);
                }
            }
        }
        if is_fixed {
            Some(std::mem::take(&mut buf))
                .filter(|buf| !buf.is_empty())
                .map(FormatOperation::FixedText)
                .map(Ok)
        } else {
            Some(Err(()))
        }
    });

    let adjust_span = move |col_span: Span| -> Option<Span> {
        pattern_span?.subspan(col_span.start, col_span.end)
    };

    let make_delimiter_error = move |_| ShellError::DelimiterError {
        msg: "there are unmatched curly braces".to_string(),
        span: call_head,
    };

    let make_removed_functionality_error = |span: Span| ShellError::GenericError {
        error: "Removed functionality".into(),
        msg: "The ability to use variables ($it) in `format pattern` has been removed.".into(),
        span: Some(span),
        help: Some("You can use other formatting options, such as string interpolation.".into()),
        inner: vec![],
    };

    ops.map(|res_op| {
        res_op
            .map(|op| op.update_span(|col_span| col_span.and_then(adjust_span)))
            .map_err(make_delimiter_error)
            .and_then(|op| match op {
                FormatOperation::ValueFromColumn { content, span } if content.starts_with('$') => {
                    Err(make_removed_functionality_error(span.unwrap_or(call_head)))
                }
                op => Ok(op),
            })
    })
    .collect()
}

/// Format the incoming PipelineData according to the pattern
fn format(
    input_data: Value,
    format_operations: &[FormatOperation],
    engine_state: &EngineState,
    config: &Config,
    head_span: Span,
) -> Result<PipelineData, ShellError> {
    let data_as_value = input_data;

    //  We can only handle a Record or a List of Records
    match data_as_value {
        Value::Record { .. } => {
            match format_record(format_operations, &data_as_value, config, head_span) {
                Ok(value) => Ok(PipelineData::value(Value::string(value, head_span), None)),
                Err(value) => Err(value),
            }
        }

        Value::List { vals, .. } => {
            let mut list = vec![];
            for val in vals.iter() {
                match val {
                    Value::Record { .. } => {
                        match format_record(format_operations, val, config, head_span) {
                            Ok(value) => {
                                list.push(Value::string(value, head_span));
                            }
                            Err(value) => {
                                return Err(value);
                            }
                        }
                    }
                    Value::Error { error, .. } => return Err(*error.clone()),
                    _ => {
                        return Err(ShellError::OnlySupportsThisInputType {
                            exp_input_type: "record".to_string(),
                            wrong_type: val.get_type().to_string(),
                            dst_span: head_span,
                            src_span: val.span(),
                        });
                    }
                }
            }

            Ok(ListStream::new(list.into_iter(), head_span, engine_state.signals().clone()).into())
        }
        // Unwrapping this ShellError is a bit unfortunate.
        // Ideally, its Span would be preserved.
        Value::Error { error, .. } => Err(*error),
        _ => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "record".to_string(),
            wrong_type: data_as_value.get_type().to_string(),
            dst_span: head_span,
            src_span: data_as_value.span(),
        }),
    }
}

fn format_record(
    format_operations: &[FormatOperation],
    data_as_value: &Value,
    config: &Config,
    head_span: Span,
) -> Result<String, ShellError> {
    let mut output = String::new();

    for op in format_operations {
        match op {
            FormatOperation::FixedText(s) => output.push_str(s.as_str()),
            FormatOperation::ValueFromColumn {
                content: col_name,
                span,
            } => {
                // path member should split by '.' to handle for nested structure.
                let path_members: Vec<PathMember> = col_name
                    .split('.')
                    .map(|path| PathMember::String {
                        val: path.to_string(),
                        span: span.unwrap_or(head_span),
                        optional: false,
                        casing: Casing::Sensitive,
                    })
                    .collect();

                let expanded_string = data_as_value
                    .follow_cell_path(&path_members)?
                    .to_expanded_string(", ", config);
                output.push_str(expanded_string.as_str())
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::FormatPattern;
        use crate::test_examples;
        test_examples(FormatPattern {})
    }
}
