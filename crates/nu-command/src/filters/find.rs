use fancy_regex::{Regex, escape};
use nu_ansi_term::Style;
use nu_color_config::StyleComputer;
use nu_engine::command_prelude::*;
use nu_protocol::Config;

#[derive(Clone)]
pub struct Find;

impl Command for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter it would be List<T> -> List<T>.
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::String, Type::Any),
            ])
            .named(
                "regex",
                SyntaxShape::String,
                "regex to match with",
                Some('r'),
            )
            .switch(
                "ignore-case",
                "case-insensitive; when in regex mode, this is equivalent to (?i)",
                Some('i'),
            )
            .switch(
                "multiline",
                "don't split multi-line strings into lists of lines. you should use this option when using the (?m) or (?s) flags in regex mode",
                Some('m'),
            )
            .switch(
                "dotall",
                "dotall regex mode: allow a dot . to match newlines \\n; equivalent to (?s)",
                Some('s'),
            )
            .named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names to be searched",
                Some('c'),
            )
            .switch(
                "no-highlight",
                "no-highlight mode: find without marking with ansi code",
                Some('n'),
            )
            .switch("invert", "invert the match", Some('v'))
            .rest("rest", SyntaxShape::Any, "Terms to search.")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Searches terms in the input."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Search for multiple terms in a command output",
                example: r#"ls | find toml md sh"#,
                result: None,
            },
            Example {
                description: "Search and highlight text for a term in a string.",
                example: r#"'Cargo.toml' | find Cargo"#,
                result: Some(Value::test_string(
                    "\u{1b}[37m\u{1b}[0m\u{1b}[41;37mCargo\u{1b}[0m\u{1b}[37m.toml\u{1b}[0m"
                        .to_owned(),
                )),
            },
            Example {
                description: "Search a number or a file size in a list of numbers",
                example: r#"[1 5 3kb 4 35 3Mb] | find 5 3kb"#,
                result: Some(Value::list(
                    vec![Value::test_int(5), Value::test_filesize(3000)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Search a char in a list of string",
                example: r#"[moe larry curly] | find l"#,
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[37m\u{1b}[0m\u{1b}[41;37ml\u{1b}[0m\u{1b}[37marry\u{1b}[0m",
                        ),
                        Value::test_string(
                            "\u{1b}[37mcur\u{1b}[0m\u{1b}[41;37ml\u{1b}[0m\u{1b}[37my\u{1b}[0m",
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Search using regex",
                example: r#"[abc odb arc abf] | find --regex "b.""#,
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[37ma\u{1b}[0m\u{1b}[41;37mbc\u{1b}[0m\u{1b}[37m\u{1b}[0m"
                                .to_string(),
                        ),
                        Value::test_string(
                            "\u{1b}[37ma\u{1b}[0m\u{1b}[41;37mbf\u{1b}[0m\u{1b}[37m\u{1b}[0m"
                                .to_string(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Case insensitive search",
                example: r#"[aBc bde Arc abf] | find "ab" -i"#,
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[37m\u{1b}[0m\u{1b}[41;37maB\u{1b}[0m\u{1b}[37mc\u{1b}[0m"
                                .to_string(),
                        ),
                        Value::test_string(
                            "\u{1b}[37m\u{1b}[0m\u{1b}[41;37mab\u{1b}[0m\u{1b}[37mf\u{1b}[0m"
                                .to_string(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find value in records using regex",
                example: r#"[[version name]; ['0.1.0' nushell] ['0.1.1' fish] ['0.2.0' zsh]] | find --regex "nu""#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                        "version" => Value::test_string("0.1.0"),
                        "name" => Value::test_string("\u{1b}[37m\u{1b}[0m\u{1b}[41;37mnu\u{1b}[0m\u{1b}[37mshell\u{1b}[0m".to_string()),
                })])),
            },
            Example {
                description: "Find inverted values in records using regex",
                example: r#"[[version name]; ['0.1.0' nushell] ['0.1.1' fish] ['0.2.0' zsh]] | find --regex "nu" --invert"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                            "version" => Value::test_string("0.1.1"),
                            "name" => Value::test_string("fish".to_string()),
                    }),
                    Value::test_record(record! {
                            "version" => Value::test_string("0.2.0"),
                            "name" =>Value::test_string("zsh".to_string()),
                    }),
                ])),
            },
            Example {
                description: "Find value in list using regex",
                example: r#"[["Larry", "Moe"], ["Victor", "Marina"]] | find --regex "rr""#,
                result: Some(Value::list(
                    vec![Value::list(
                        vec![
                            Value::test_string(
                                "\u{1b}[37mLa\u{1b}[0m\u{1b}[41;37mrr\u{1b}[0m\u{1b}[37my\u{1b}[0m",
                            ),
                            Value::test_string("Moe"),
                        ],
                        Span::test_data(),
                    )],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find inverted values in records using regex",
                example: r#"[["Larry", "Moe"], ["Victor", "Marina"]] | find --regex "rr" --invert"#,
                result: Some(Value::list(
                    vec![Value::list(
                        vec![Value::test_string("Victor"), Value::test_string("Marina")],
                        Span::test_data(),
                    )],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Remove ANSI sequences from result",
                example: "[[foo bar]; [abc 123] [def 456]] | find --no-highlight 123",
                result: Some(Value::list(
                    vec![Value::test_record(record! {
                        "foo" => Value::test_string("abc"),
                        "bar" => Value::test_int(123)
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find and highlight text in specific columns",
                example: "[[col1 col2 col3]; [moe larry curly] [larry curly moe]] | find moe --columns [col1]",
                result: Some(Value::list(
                    vec![Value::test_record(record! {
                            "col1" => Value::test_string(
                                "\u{1b}[37m\u{1b}[0m\u{1b}[41;37mmoe\u{1b}[0m\u{1b}[37m\u{1b}[0m"
                                    .to_string(),
                            ),
                            "col2" => Value::test_string("larry".to_string()),
                            "col3" => Value::test_string("curly".to_string()),
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find in a multi-line string",
                example: r#""Violets are red\nAnd roses are blue\nWhen metamaterials\nAlter their hue" | find "ue""#,
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[37mAnd roses are bl\u{1b}[0m\u{1b}[41;37mue\u{1b}[0m\u{1b}[37m\u{1b}[0m",
                        ),
                        Value::test_string(
                            "\u{1b}[37mAlter their h\u{1b}[0m\u{1b}[41;37mue\u{1b}[0m\u{1b}[37m\u{1b}[0m",
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find in a multi-line string without splitting the input into a list of lines",
                example: r#""Violets are red\nAnd roses are blue\nWhen metamaterials\nAlter their hue" | find --multiline "ue""#,
                result: Some(Value::test_string(
                    "\u{1b}[37mViolets are red\nAnd roses are bl\u{1b}[0m\u{1b}[41;37mue\u{1b}[0m\u{1b}[37m\nWhen metamaterials\nAlter their h\u{1b}[0m\u{1b}[41;37mue\u{1b}[0m\u{1b}[37m\u{1b}[0m",
                )),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "regex", "search", "condition", "grep"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern = get_match_pattern_from_arguments(engine_state, stack, call)?;

        let multiline = call.has_flag(engine_state, stack, "multiline")?;

        let columns_to_search: Vec<_> = call
            .get_flag(engine_state, stack, "columns")?
            .unwrap_or_default();

        let input = if multiline {
            if let PipelineData::ByteStream(..) = input {
                // ByteStream inputs are processed by iterating over the lines, which necessarily
                // breaks the multi-line text being streamed into a list of lines.
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "Flag `--multiline` currently doesn't work for byte stream inputs. Consider using `collect`".into(),
                    span: call.get_flag_span(stack, "multiline").expect("has flag"),
                });
            };
            input
        } else {
            split_string_if_multiline(input, call.head)
        };

        find_in_pipelinedata(pattern, columns_to_search, engine_state, stack, input)
    }
}

#[derive(Clone)]
struct MatchPattern {
    /// the regex to be used for matching in text
    regex: Regex,

    /// the list of match terms (converted to lowercase if needed), or empty if a regex was provided
    search_terms: Vec<String>,

    /// case-insensitive match
    ignore_case: bool,

    /// return a modified version of the value where matching parts are highlighted
    highlight: bool,

    /// return the values that aren't a match instead
    invert: bool,

    /// style of the non-highlighted string sections
    string_style: Style,

    /// style of the highlighted string sections
    highlight_style: Style,
}

fn get_match_pattern_from_arguments(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<MatchPattern, ShellError> {
    let config = stack.get_config(engine_state);

    let span = call.head;
    let regex = call.get_flag::<String>(engine_state, stack, "regex")?;
    let terms = call.rest::<Value>(engine_state, stack, 0)?;

    let invert = call.has_flag(engine_state, stack, "invert")?;
    let highlight = !call.has_flag(engine_state, stack, "no-highlight")?;

    let ignore_case = call.has_flag(engine_state, stack, "ignore-case")?;

    let dotall = call.has_flag(engine_state, stack, "dotall")?;

    let style_computer = StyleComputer::from_config(engine_state, stack);
    // Currently, search results all use the same style.
    // Also note that this sample string is passed into user-written code (the closure that may or may not be
    // defined for "string").
    let string_style = style_computer.compute("string", &Value::string("search result", span));
    let highlight_style =
        style_computer.compute("search_result", &Value::string("search result", span));

    let (regex_str, search_terms) = if let Some(regex) = regex {
        if !terms.is_empty() {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "Cannot use a `--regex` parameter with additional search terms".into(),
                span: call.get_flag_span(stack, "regex").expect("has flag"),
            });
        }

        let flags = match (ignore_case, dotall) {
            (false, false) => "",
            (true, false) => "(?i)", // case insensitive
            (false, true) => "(?s)", // allow . to match \n
            (true, true) => "(?is)", // case insensitive and allow . to match \n
        };

        (flags.to_string() + regex.as_str(), Vec::new())
    } else {
        if dotall {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "Flag --dotall only works for regex search".into(),
                span: call.get_flag_span(stack, "dotall").expect("has flag"),
            });
        }

        let mut regex = String::new();

        if ignore_case {
            regex += "(?i)";
        }

        let search_terms = terms
            .iter()
            .map(|v| {
                if ignore_case {
                    v.to_expanded_string("", &config).to_lowercase()
                } else {
                    v.to_expanded_string("", &config)
                }
            })
            .collect::<Vec<String>>();

        let escaped_terms = search_terms
            .iter()
            .map(|v| escape(v).into())
            .collect::<Vec<String>>();

        if let Some(term) = escaped_terms.first() {
            regex += term;
        }

        for term in escaped_terms.iter().skip(1) {
            regex += "|";
            regex += term;
        }

        (regex, search_terms)
    };

    let regex = Regex::new(regex_str.as_str()).map_err(|e| ShellError::TypeMismatch {
        err_message: format!("invalid regex: {e}"),
        span,
    })?;

    Ok(MatchPattern {
        regex,
        search_terms,
        ignore_case,
        invert,
        highlight,
        string_style,
        highlight_style,
    })
}

// map functions

fn highlight_matches_in_string(pattern: &MatchPattern, val: String) -> String {
    if !pattern.regex.is_match(&val).unwrap_or(false) {
        return val;
    }

    let stripped_val = nu_utils::strip_ansi_string_unlikely(val);
    let mut last_match_end = 0;
    let mut highlighted = String::new();

    for cap in pattern.regex.captures_iter(stripped_val.as_ref()) {
        match cap {
            Ok(capture) => {
                let start = match capture.get(0) {
                    Some(acap) => acap.start(),
                    None => 0,
                };
                let end = match capture.get(0) {
                    Some(acap) => acap.end(),
                    None => 0,
                };
                highlighted.push_str(
                    &pattern
                        .string_style
                        .paint(&stripped_val[last_match_end..start])
                        .to_string(),
                );
                highlighted.push_str(
                    &pattern
                        .highlight_style
                        .paint(&stripped_val[start..end])
                        .to_string(),
                );
                last_match_end = end;
            }
            Err(_e) => {
                // in case of error, return the string with no highlight
                return pattern.string_style.paint(&stripped_val).to_string();
            }
        }
    }

    highlighted.push_str(
        &pattern
            .string_style
            .paint(&stripped_val[last_match_end..])
            .to_string(),
    );
    highlighted
}

fn highlight_matches_in_value(
    pattern: &MatchPattern,
    value: Value,
    columns_to_search: &[String],
) -> Value {
    if !pattern.highlight || pattern.invert {
        return value;
    }
    let span = value.span();

    match value {
        Value::Record { val: record, .. } => {
            let col_select = !columns_to_search.is_empty();

            // TODO: change API to mutate in place
            let mut record = record.into_owned();

            for (col, val) in record.iter_mut() {
                if col_select && !columns_to_search.contains(col) {
                    continue;
                }

                *val = highlight_matches_in_value(pattern, std::mem::take(val), &[]);
            }

            Value::record(record, span)
        }
        Value::List { vals, .. } => vals
            .into_iter()
            .map(|item| highlight_matches_in_value(pattern, item, &[]))
            .collect::<Vec<Value>>()
            .into_value(span),
        Value::String { val, .. } => highlight_matches_in_string(pattern, val).into_value(span),
        _ => value,
    }
}

fn find_in_pipelinedata(
    pattern: MatchPattern,
    columns_to_search: Vec<String>,
    engine_state: &EngineState,
    stack: &mut Stack,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let config = stack.get_config(engine_state);

    let map_pattern = pattern.clone();
    let map_columns_to_search = columns_to_search.clone();

    match input {
        PipelineData::Empty => Ok(PipelineData::empty()),
        PipelineData::Value(_, _) => input
            .filter(
                move |value| {
                    value_should_be_printed(&pattern, value, &columns_to_search, &config)
                        != pattern.invert
                },
                engine_state.signals(),
            )?
            .map(
                move |x| highlight_matches_in_value(&map_pattern, x, &map_columns_to_search),
                engine_state.signals(),
            ),
        PipelineData::ListStream(stream, metadata) => {
            let stream = stream.modify(|iter| {
                iter.filter(move |value| {
                    value_should_be_printed(&pattern, value, &columns_to_search, &config)
                        != pattern.invert
                })
                .map(move |x| highlight_matches_in_value(&map_pattern, x, &map_columns_to_search))
            });

            Ok(PipelineData::list_stream(stream, metadata))
        }
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            if let Some(lines) = stream.lines() {
                let mut output: Vec<Value> = vec![];
                for line in lines {
                    let line = line?;
                    if string_should_be_printed(&pattern, &line) != pattern.invert {
                        if pattern.highlight && !pattern.invert {
                            output
                                .push(highlight_matches_in_string(&pattern, line).into_value(span))
                        } else {
                            output.push(line.into_value(span))
                        }
                    }
                }
                Ok(Value::list(output, span).into_pipeline_data())
            } else {
                Ok(PipelineData::empty())
            }
        }
    }
}

// filter functions

fn string_should_be_printed(pattern: &MatchPattern, value: &str) -> bool {
    pattern.regex.is_match(value).unwrap_or(false)
}

fn value_should_be_printed(
    pattern: &MatchPattern,
    value: &Value,
    columns_to_search: &[String],
    config: &Config,
) -> bool {
    let value_as_string = if pattern.ignore_case {
        value.to_expanded_string("", config).to_lowercase()
    } else {
        value.to_expanded_string("", config)
    };

    match value {
        Value::Bool { .. }
        | Value::Int { .. }
        | Value::Filesize { .. }
        | Value::Duration { .. }
        | Value::Date { .. }
        | Value::Range { .. }
        | Value::Float { .. }
        | Value::Closure { .. }
        | Value::Nothing { .. } => {
            if !pattern.search_terms.is_empty() {
                // look for exact match when searching with terms
                pattern
                    .search_terms
                    .iter()
                    .any(|term: &String| term == &value_as_string)
            } else {
                string_should_be_printed(pattern, &value_as_string)
            }
        }
        Value::Glob { .. } | Value::CellPath { .. } | Value::Custom { .. } => {
            string_should_be_printed(pattern, &value_as_string)
        }
        Value::String { val, .. } => string_should_be_printed(pattern, val),
        Value::List { vals, .. } => vals
            .iter()
            .any(|item| value_should_be_printed(pattern, item, &[], config)),
        Value::Record { val: record, .. } => {
            let col_select = !columns_to_search.is_empty();
            record.iter().any(|(col, val)| {
                if col_select && !columns_to_search.contains(col) {
                    return false;
                }
                value_should_be_printed(pattern, val, &[], config)
            })
        }
        Value::Binary { .. } => false,
        Value::Error { .. } => true,
    }
}

// utility

fn split_string_if_multiline(input: PipelineData, head_span: Span) -> PipelineData {
    let span = input.span().unwrap_or(head_span);
    match input {
        PipelineData::Value(Value::String { ref val, .. }, _) => {
            if val.contains('\n') {
                Value::list(
                    val.lines()
                        .map(|s| Value::string(s.to_string(), span))
                        .collect(),
                    span,
                )
                .into_pipeline_data_with_metadata(input.metadata())
            } else {
                input
            }
        }
        _ => input,
    }
}

/// function for using find from other commands
pub fn find_internal(
    input: PipelineData,
    engine_state: &EngineState,
    stack: &mut Stack,
    search_term: &str,
    columns_to_search: &[&str],
    highlight: bool,
) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(Span::unknown());

    let style_computer = StyleComputer::from_config(engine_state, stack);
    let string_style = style_computer.compute("string", &Value::string("search result", span));
    let highlight_style =
        style_computer.compute("search_result", &Value::string("search result", span));

    let regex_str = format!("(?i){}", escape(search_term));

    let regex = Regex::new(regex_str.as_str()).map_err(|e| ShellError::TypeMismatch {
        err_message: format!("invalid regex: {e}"),
        span: Span::unknown(),
    })?;

    let pattern = MatchPattern {
        regex,
        search_terms: vec![search_term.to_lowercase()],
        ignore_case: true,
        highlight,
        invert: false,
        string_style,
        highlight_style,
    };

    let columns_to_search = columns_to_search
        .iter()
        .map(|str| String::from(*str))
        .collect();

    find_in_pipelinedata(pattern, columns_to_search, engine_state, stack, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Find)
    }
}
