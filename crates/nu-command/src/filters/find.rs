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
                "Regex to match with.",
                Some('r'),
            )
            .switch(
                "ignore-case",
                "Case-insensitive; when in regex mode, this is equivalent to (?i).",
                Some('i'),
            )
            .switch(
                "multiline",
                "Don't split multi-line strings into lists of lines. you should use this option when using the (?m) or (?s) flags in regex mode.",
                Some('m'),
            )
            .switch(
                "dotall",
                "Dotall regex mode: allow a dot . to match newlines \\n; equivalent to (?s).",
                Some('s'),
            )
            .named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Column names to be searched.",
                Some('c'),
            )
            .switch(
                "no-highlight",
                "No-highlight mode: find without marking with ansi code.",
                Some('n'),
            )
            .switch("invert", "Invert the match.", Some('v'))
            .switch(
                "only-matching",
                "Print only the matched parts, with each match on a separate line.",
                Some('o'),
            )
            .switch(
                "rfind",
                "Search from the end of the string and only return the first match.",
                Some('R'),
            )
            .rest("rest", SyntaxShape::Any, "Terms to search.")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Search for terms in the input data."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Search for multiple terms in a command output.",
                example: "ls | find toml md sh",
                result: None,
            },
            Example {
                description: "Search and highlight text for a term in a string.",
                example: "'Cargo.toml' | find Cargo",
                result: Some(Value::test_string(
                    "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mCargo\u{1b}[0m\u{1b}[39m.toml\u{1b}[0m"
                        .to_owned(),
                )),
            },
            Example {
                description: "Search a number or a file size in a list of numbers.",
                example: "[1 5 3kb 4 35 3Mb] | find 5 3kb",
                result: Some(Value::list(
                    vec![Value::test_int(5), Value::test_filesize(3000)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Search a char in a list of string.",
                example: "[moe larry curly] | find l",
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[39m\u{1b}[0m\u{1b}[41;39ml\u{1b}[0m\u{1b}[39marry\u{1b}[0m",
                        ),
                        Value::test_string(
                            "\u{1b}[39mcur\u{1b}[0m\u{1b}[41;39ml\u{1b}[0m\u{1b}[39my\u{1b}[0m",
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Search using regex.",
                example: r#"[abc odb arc abf] | find --regex "b.""#,
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[39ma\u{1b}[0m\u{1b}[41;39mbc\u{1b}[0m\u{1b}[39m\u{1b}[0m"
                                .to_string(),
                        ),
                        Value::test_string(
                            "\u{1b}[39ma\u{1b}[0m\u{1b}[41;39mbf\u{1b}[0m\u{1b}[39m\u{1b}[0m"
                                .to_string(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Case insensitive search.",
                example: r#"[aBc bde Arc abf] | find "ab" -i"#,
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[39m\u{1b}[0m\u{1b}[41;39maB\u{1b}[0m\u{1b}[39mc\u{1b}[0m"
                                .to_string(),
                        ),
                        Value::test_string(
                            "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mab\u{1b}[0m\u{1b}[39mf\u{1b}[0m"
                                .to_string(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find value in records using regex.",
                example: r#"[[version name]; ['0.1.0' nushell] ['0.1.1' fish] ['0.2.0' zsh]] | find --regex "nu""#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                        "version" => Value::test_string("0.1.0"),
                        "name" => Value::test_string("\u{1b}[39m\u{1b}[0m\u{1b}[41;39mnu\u{1b}[0m\u{1b}[39mshell\u{1b}[0m".to_string()),
                })])),
            },
            Example {
                description: "Find inverted values in records using regex.",
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
                description: "Find value in list using regex.",
                example: r#"[["Larry", "Moe"], ["Victor", "Marina"]] | find --regex "rr""#,
                result: Some(Value::list(
                    vec![Value::list(
                        vec![
                            Value::test_string(
                                "\u{1b}[39mLa\u{1b}[0m\u{1b}[41;39mrr\u{1b}[0m\u{1b}[39my\u{1b}[0m",
                            ),
                            Value::test_string("Moe"),
                        ],
                        Span::test_data(),
                    )],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find inverted values in records using regex.",
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
                description: "Remove ANSI sequences from result.",
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
                description: "Find and highlight text in specific columns.",
                example: "[[col1 col2 col3]; [moe larry curly] [larry curly moe]] | find moe --columns [col1]",
                result: Some(Value::list(
                    vec![Value::test_record(record! {
                            "col1" => Value::test_string(
                                "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mmoe\u{1b}[0m\u{1b}[39m\u{1b}[0m"
                                    .to_string(),
                            ),
                            "col2" => Value::test_string("larry".to_string()),
                            "col3" => Value::test_string("curly".to_string()),
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find in a multi-line string.",
                example: "'Violets are red\nAnd roses are blue\nWhen metamaterials\nAlter their hue' | find ue",
                result: Some(Value::list(
                    vec![
                        Value::test_string(
                            "\u{1b}[39mAnd roses are bl\u{1b}[0m\u{1b}[41;39mue\u{1b}[0m\u{1b}[39m\u{1b}[0m",
                        ),
                        Value::test_string(
                            "\u{1b}[39mAlter their h\u{1b}[0m\u{1b}[41;39mue\u{1b}[0m\u{1b}[39m\u{1b}[0m",
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Find in a multi-line string without splitting the input into a list of lines.",
                example: "'Violets are red\nAnd roses are blue\nWhen metamaterials\nAlter their hue' | find --multiline ue",
                result: Some(Value::test_string(
                    "\u{1b}[39mViolets are red\nAnd roses are bl\u{1b}[0m\u{1b}[41;39mue\u{1b}[0m\u{1b}[39m\nWhen metamaterials\nAlter their h\u{1b}[0m\u{1b}[41;39mue\u{1b}[0m\u{1b}[39m\u{1b}[0m",
                )),
            },
            Example {
                description: "Find and highlight the last occurrence in a string.",
                example: "'hello world hello' | find --rfind hello",
                result: Some(Value::test_string(
                    "\u{1b}[39mhello world \u{1b}[0m\u{1b}[41;39mhello\u{1b}[0m\u{1b}[39m\u{1b}[0m",
                )),
            },
            Example {
                description: "Return only matching parts of a string.",
                example: "'abc abc' | find --only-matching ab",
                result: Some(Value::test_list(vec![
                    Value::test_string("ab"),
                    Value::test_string("ab"),
                ])),
            },
            Example {
                description: "Return only the last matching part of a string.",
                example: "'abc abc' | find --only-matching --rfind ab",
                result: Some(Value::test_list(vec![Value::test_string("ab")])),
            },
            Example {
                description: "Return only matching parts while preserving original case.",
                example: "'ABC' | find --only-matching --ignore-case a",
                result: Some(Value::test_list(vec![Value::test_string("A")])),
            },
            Example {
                description: "Return only exact scalar matches for search terms.",
                example: "[5 35] | find --only-matching 5",
                result: Some(Value::test_list(vec![Value::test_string("5")])),
            },
            Example {
                description: "Return only matches from selected columns of a table.",
                example: "[[name desc]; [foo bar]] | find --only-matching --columns [name] bar",
                result: Some(Value::test_list(vec![])),
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

    /// return only matched substrings
    only_matching: bool,

    /// search from the end (find last occurrence)
    rfind: bool,

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
    let only_matching = call.has_flag(engine_state, stack, "only-matching")?;
    let rfind = call.has_flag(engine_state, stack, "rfind")?;

    let ignore_case = call.has_flag(engine_state, stack, "ignore-case")?;

    let dotall = call.has_flag(engine_state, stack, "dotall")?;

    if invert && only_matching {
        return Err(ShellError::IncompatibleParameters {
            left_message: "inverts matches".into(),
            left_span: call.get_flag_span(stack, "invert").expect("has flag"),
            right_message: "returns matches".into(),
            right_span: call
                .get_flag_span(stack, "only-matching")
                .expect("has flag"),
        });
    }

    let style_computer = StyleComputer::from_config(engine_state, stack);
    // Currently, search results all use the same style.
    // Also note that this sample string is passed into user-written code (the closure that may or may not be
    // defined for "string").
    let string_style = style_computer.compute("string", &Value::string("search result", span));
    let highlight_style =
        style_computer.compute("search_result", &Value::string("search result", span));

    let (regex_str, search_terms) = match (regex, terms.as_slice()) {
        (Some(_), [_, ..]) => {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "Cannot use a `--regex` parameter with additional search terms".into(),
                span: call.get_flag_span(stack, "regex").expect("has flag"),
            });
        }
        (Some(regex), []) => {
            let flags = match (ignore_case, dotall) {
                (false, false) => "",
                (true, false) => "(?i)", // case insensitive
                (false, true) => "(?s)", // allow . to match \n
                (true, true) => "(?is)", // case insensitive and allow . to match \n
            };

            (flags.to_string() + regex.as_str(), Vec::new())
        }
        (None, _) if dotall => {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "Flag --dotall only works for regex search".into(),
                span: call.get_flag_span(stack, "dotall").expect("has flag"),
            });
        }
        // NOTE: Should this be an error? It doesn't make sense to call `find` with no arguments.
        // (None, []) => {}
        (None, terms) => {
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

            if let [first, rest @ ..] = search_terms.as_slice() {
                regex.push_str(escape(first).as_ref());
                for term in rest {
                    regex.push('|');
                    regex.push_str(escape(term).as_ref());
                }
            }

            (regex, search_terms)
        }
    };

    let regex = engine_state.compile_regex(regex_str.as_str(), span)?;

    Ok(MatchPattern {
        regex,
        search_terms,
        ignore_case,
        invert,
        only_matching,
        highlight,
        rfind,
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

    if pattern.rfind {
        highlight_last_match(pattern, &stripped_val)
    } else {
        highlight_all_matches(pattern, &stripped_val)
    }
}

fn highlight_last_match(pattern: &MatchPattern, text: &str) -> String {
    // Find the last match using fold to avoid collecting all matches
    let last_match = pattern.regex.find_iter(text).fold(None, |_, m| m.ok());

    match last_match {
        Some(m) => {
            let start = m.start();
            let end = m.end();
            format!(
                "{}{}{}",
                pattern.string_style.paint(&text[..start]),
                pattern.highlight_style.paint(&text[start..end]),
                pattern.string_style.paint(&text[end..])
            )
        }
        None => pattern.string_style.paint(text).to_string(),
    }
}

fn highlight_all_matches(pattern: &MatchPattern, text: &str) -> String {
    let mut last_match_end = 0;
    let mut highlighted = String::new();

    for cap in pattern.regex.captures_iter(text) {
        let Ok(capture) = cap else {
            return pattern.string_style.paint(text).to_string();
        };

        let Some(m) = capture.get(0) else { continue };

        highlighted.push_str(
            &pattern
                .string_style
                .paint(&text[last_match_end..m.start()])
                .to_string(),
        );
        highlighted.push_str(
            &pattern
                .highlight_style
                .paint(&text[m.start()..m.end()])
                .to_string(),
        );
        last_match_end = m.end();
    }

    highlighted.push_str(
        &pattern
            .string_style
            .paint(&text[last_match_end..])
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

    if pattern.only_matching {
        return find_only_matching_in_pipelinedata(
            pattern,
            columns_to_search,
            engine_state,
            input,
            &config,
        );
    }

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

fn find_only_matching_in_pipelinedata(
    pattern: MatchPattern,
    columns_to_search: Vec<String>,
    engine_state: &EngineState,
    input: PipelineData,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    // Byte streams are split into lines so each line is matched like a string,
    // mirroring the plain `find` byte-stream arm. Every other input shape is
    // delegated to `PipelineData::flat_map`, which already walks lists, ranges,
    // iterable custom values, preserves metadata and honors ctrl-c.
    if let PipelineData::ByteStream(stream, ..) = input {
        let span = stream.span();
        let Some(lines) = stream.lines() else {
            return Ok(PipelineData::empty());
        };

        let mut output = vec![];
        for line in lines {
            let line = line?;
            output.extend(only_matching_matches_in_string(&pattern, &line, span));
        }

        return Ok(Value::list(output, span).into_pipeline_data());
    }

    // A top-level error should flow through unchanged, just like plain `find`,
    // rather than being wrapped in a one-element list.
    if let PipelineData::Value(value, _) = &input
        && value.is_error()
    {
        return Ok(input);
    }

    let config = config.clone();
    input.flat_map(
        move |value| only_matching_matches_in_value(&pattern, value, &columns_to_search, &config),
        engine_state.signals(),
    )
}

fn only_matching_matches_in_value(
    pattern: &MatchPattern,
    value: Value,
    columns_to_search: &[String],
    config: &Config,
) -> Vec<Value> {
    let span = value.span();

    match value {
        Value::String { val, .. } => only_matching_matches_in_string(pattern, &val, span),
        Value::List { vals, .. } => vals
            .into_iter()
            .flat_map(|item| only_matching_matches_in_value(pattern, item, &[], config))
            .collect(),
        Value::Record { val: record, .. } => {
            let col_select = !columns_to_search.is_empty();
            record
                .into_owned()
                .into_iter()
                .filter(|(col, _)| !col_select || columns_to_search.contains(col))
                .flat_map(|(_, val)| only_matching_matches_in_value(pattern, val, &[], config))
                .collect()
        }
        Value::Bool { .. }
        | Value::Int { .. }
        | Value::Filesize { .. }
        | Value::Duration { .. }
        | Value::Date { .. }
        | Value::Range { .. }
        | Value::Float { .. }
        | Value::Closure { .. }
        | Value::Nothing { .. } => only_matching_matches_in_scalar(pattern, value, config),
        Value::Binary { .. } => Vec::new(),
        Value::Error { .. } => vec![value],
        // `to_expanded_string` is only computed here (and in the scalar arm) so
        // string/list/record values don't pay for a rendering they never use.
        _ => only_matching_matches_in_string(pattern, &value.to_expanded_string("", config), span),
    }
}

fn only_matching_matches_in_scalar(
    pattern: &MatchPattern,
    value: Value,
    config: &Config,
) -> Vec<Value> {
    let span = value.span();
    let value_as_string = value.to_expanded_string("", config);

    // Reuse `value_should_be_printed` for the exact-match rule so `find` and
    // `find -o` agree on what counts as a scalar match.
    if !pattern.search_terms.is_empty() {
        return if value_should_be_printed(pattern, &value, &[], config) {
            vec![Value::string(value_as_string, span)]
        } else {
            Vec::new()
        };
    }

    only_matching_matches_in_string(pattern, &value_as_string, span)
}

fn only_matching_matches_in_string(pattern: &MatchPattern, text: &str, span: Span) -> Vec<Value> {
    // Build the match iterator once; `rfind` keeps the last forward match
    // (matching `highlight_last_match`), otherwise all non-empty matches.
    let mut matches = Vec::new();
    for found in pattern.regex.find_iter(text) {
        match found {
            Ok(m) if !m.as_str().is_empty() => matches.push(m.as_str().to_string()),
            Ok(_) => {}
            // Surface regex runtime errors (e.g. backtrack-limit exceeded) as an
            // error value instead of silently returning a short or empty list.
            Err(err) => {
                return vec![Value::error(
                    ShellError::Generic(
                        nu_protocol::shell_error::generic::GenericError::new_internal(
                            "Regex error while matching",
                            err.to_string(),
                        ),
                    ),
                    span,
                )];
            }
        }
    }

    if pattern.rfind {
        matches
            .pop()
            .map(|m| vec![Value::string(m, span)])
            .unwrap_or_default()
    } else {
        matches
            .into_iter()
            .map(|m| Value::string(m, span))
            .collect()
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
        PipelineData::Value(Value::String { ref val, .. }, metadata) if val.contains('\n') => {
            Value::list(
                val.lines()
                    .map(|s| Value::string(s.to_string(), span))
                    .collect(),
                span,
            )
            .into_pipeline_data_with_metadata(metadata)
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
    head: Span,
) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(head);

    let style_computer = StyleComputer::from_config(engine_state, stack);
    let string_style = style_computer.compute("string", &Value::string("search result", span));
    let highlight_style =
        style_computer.compute("search_result", &Value::string("search result", span));

    let regex_str = format!("(?i){}", escape(search_term));

    let regex = engine_state.compile_regex(regex_str.as_str(), head)?;

    let pattern = MatchPattern {
        regex,
        search_terms: vec![search_term.to_lowercase()],
        ignore_case: true,
        highlight,
        invert: false,
        only_matching: false,
        rfind: false,
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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Find)
    }

    #[test]
    fn only_matching_preserves_error_values() {
        let pattern = MatchPattern {
            regex: Regex::new("needle").expect("valid regex"),
            search_terms: vec!["needle".to_string()],
            ignore_case: false,
            highlight: true,
            invert: false,
            only_matching: true,
            rfind: false,
            string_style: Style::new(),
            highlight_style: Style::new(),
        };
        let error = Value::error(
            ShellError::Generic(
                nu_protocol::shell_error::generic::GenericError::new_internal(
                    "test error",
                    "test error",
                ),
            ),
            Span::test_data(),
        );

        let matches = only_matching_matches_in_value(&pattern, error, &[], &Config::default());

        assert!(matches[0].is_error());
    }

    #[test]
    fn only_matching_preserves_original_case_for_globs() {
        let pattern = MatchPattern {
            regex: Regex::new("(?i)a").expect("valid regex"),
            search_terms: vec!["a".to_string()],
            ignore_case: true,
            highlight: true,
            invert: false,
            only_matching: true,
            rfind: false,
            string_style: Style::new(),
            highlight_style: Style::new(),
        };

        let matches = only_matching_matches_in_value(
            &pattern,
            Value::test_glob("ABC"),
            &[],
            &Config::default(),
        );

        assert_eq!(matches, vec![Value::test_string("A")]);
    }

    #[test]
    fn only_matching_scalar_uses_exact_search_term_match() {
        let pattern = MatchPattern {
            regex: Regex::new("5").expect("valid regex"),
            search_terms: vec!["5".to_string()],
            ignore_case: false,
            highlight: true,
            invert: false,
            only_matching: true,
            rfind: false,
            string_style: Style::new(),
            highlight_style: Style::new(),
        };

        // An exact scalar match (e.g. an expanded range element) is returned.
        assert_eq!(
            only_matching_matches_in_value(&pattern, Value::test_int(5), &[], &Config::default()),
            vec![Value::test_string("5")]
        );
        // A substring like 35 is not an exact scalar match, so nothing is returned.
        assert!(
            only_matching_matches_in_value(&pattern, Value::test_int(35), &[], &Config::default())
                .is_empty()
        );
    }

    #[test]
    fn only_matching_respects_selected_columns_on_record() {
        let pattern = MatchPattern {
            regex: Regex::new("bar").expect("valid regex"),
            search_terms: vec!["bar".to_string()],
            ignore_case: false,
            highlight: true,
            invert: false,
            only_matching: true,
            rfind: false,
            string_style: Style::new(),
            highlight_style: Style::new(),
        };

        let record = Value::test_record(nu_protocol::record! {
            "name" => Value::test_string("foo"),
            "desc" => Value::test_string("bar"),
        });

        // Only the `name` column is searched, so the match in `desc` is skipped.
        assert!(
            only_matching_matches_in_value(
                &pattern,
                record,
                &["name".to_string()],
                &Config::default()
            )
            .is_empty()
        );
    }
}
