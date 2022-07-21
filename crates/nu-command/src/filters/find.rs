use crate::help::highlight_search_string;
use lscolors::Style as LsColors_Style;
use nu_ansi_term::{Color::Default, Style};
use nu_color_config::get_color_config;
use nu_engine::{env_to_string, eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{CaptureBlock, Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, ListStream, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Value,
};
use nu_utils::get_ls_colors;
use regex::Regex;

#[derive(Clone)]
pub struct Find;

impl Command for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "predicate",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the predicate to satisfy",
                Some('p'),
            )
            .named(
                "regex",
                SyntaxShape::String,
                "regex to match with",
                Some('r'),
            )
            .switch(
                "insensitive",
                "case-insensitive search for regex (?i)",
                Some('i'),
            )
            .switch(
                "multiline",
                "multi-line mode: ^ and $ match begin/end of line for regex (?m)",
                Some('m'),
            )
            .switch(
                "dotall",
                "dotall mode: allow a dot . to match newline character \\n for regex (?s)",
                Some('s'),
            )
            .switch("invert", "invert the match", Some('v'))
            .rest("rest", SyntaxShape::Any, "terms to search")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Searches terms in the input or for elements of the input that satisfies the predicate."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Search for multiple terms in a command output",
                example: r#"ls | find toml md sh"#,
                result: None,
            },
            Example {
                description: "Search for a term in a string",
                example: r#"echo Cargo.toml | find toml"#,
                result: Some(Value::test_string("Cargo.toml".to_owned()))
            },
            Example {
                description: "Search a number or a file size in a list of numbers",
                example: r#"[1 5 3kb 4 3Mb] | find 5 3kb"#,
                result: Some(Value::List {
                    vals: vec![Value::test_int(5), Value::test_filesize(3000)],
                    span: Span::test_data()
                }),
            },
            Example {
                description: "Search a char in a list of string",
                example: r#"[moe larry curly] | find l"#,
                result: Some(Value::List {
                    vals: vec![Value::test_string("larry"), Value::test_string("curly")],
                    span: Span::test_data()
                })
            },
            Example {
                description: "Find odd values",
                example: "[2 4 3 6 5 8] | find --predicate { |it| ($it mod 2) == 1 }",
                result: Some(Value::List {
                    vals: vec![Value::test_int(3), Value::test_int(5)],
                    span: Span::test_data()
                })
            },
            Example {
                description: "Find if a service is not running",
                example: "[[version patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | find -p { |it| $it.patch }",
                result: Some(Value::List {
                    vals: vec![Value::test_record(
                            vec!["version", "patch"],
                            vec![Value::test_string("0.1.1"), Value::test_bool(true)]
                        )],
                    span: Span::test_data()
                }),
            },
            Example {
                description: "Find using regex",
                example: r#"[abc bde arc abf] | find --regex "ab""#,
                result: Some(Value::List {
                    vals: vec![Value::test_string("abc".to_string()), Value::test_string("abf".to_string())],
                    span: Span::test_data()
                })
            },
            Example {
                description: "Find using regex case insensitive",
                example: r#"[aBc bde Arc abf] | find --regex "ab" -i"#,
                result: Some(Value::List {
                    vals: vec![Value::test_string("aBc".to_string()), Value::test_string("abf".to_string())],
                    span: Span::test_data()
                })
            },
            Example {
                description: "Find value in records",
                example: r#"[[version name]; [0.1.0 nushell] [0.1.1 fish] [0.2.0 zsh]] | find -r "nu""#,
                result: Some(Value::List {
                    vals: vec![Value::test_record(
                            vec!["version", "name"],
                            vec![Value::test_string("0.1.0"), Value::test_string("nushell".to_string())]
                        )],
                    span: Span::test_data()
                }),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "regex", "search", "condition"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let predicate = call.get_flag::<CaptureBlock>(engine_state, stack, "predicate")?;
        let regex = call.get_flag::<String>(engine_state, stack, "regex")?;

        match (regex, predicate) {
            (None, Some(predicate)) => {
                find_with_predicate(predicate, engine_state, stack, call, input)
            }
            (Some(regex), None) => find_with_regex(regex, engine_state, stack, call, input),
            (None, None) => find_with_rest_and_highlight(engine_state, stack, call, input),
            (Some(_), Some(_)) => Err(ShellError::IncompatibleParametersSingle(
                "expected either predicate or regex flag, not both".to_owned(),
                call.head,
            )),
        }
    }
}

fn find_with_regex(
    regex: String,
    engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrlc = engine_state.ctrlc.clone();
    let config = engine_state.get_config().clone();

    let insensitive = call.has_flag("insensitive");
    let multiline = call.has_flag("multiline");
    let dotall = call.has_flag("dotall");
    let invert = call.has_flag("invert");

    let flags = match (insensitive, multiline, dotall) {
        (false, false, false) => "",
        (true, false, false) => "(?i)",
        (false, true, false) => "(?m)",
        (false, false, true) => "(?s)",
        (true, true, false) => "(?im)",
        (true, false, true) => "(?is)",
        (false, true, true) => "(?ms)",
        (true, true, true) => "(?ims)",
    };

    let regex = flags.to_string() + &regex;

    let re = Regex::new(regex.as_str())
        .map_err(|e| ShellError::UnsupportedInput(format!("incorrect regex: {}", e), span))?;

    input.filter(
        move |value| match value {
            Value::String { val, .. } => re.is_match(val.as_str()) != invert,
            Value::Record { cols: _, vals, .. } => {
                let matches: Vec<bool> = vals
                    .iter()
                    .map(|v| re.is_match(v.into_string(" ", &config).as_str()) != invert)
                    .collect();
                matches.iter().any(|b| *b)
            }
            Value::List { vals, .. } => {
                let matches: Vec<bool> = vals
                    .iter()
                    .map(|v| re.is_match(v.into_string(" ", &config).as_str()) != invert)
                    .collect();
                matches.iter().any(|b| *b)
            }
            _ => false,
        },
        ctrlc,
    )
}

fn find_with_predicate(
    predicate: CaptureBlock,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrlc = engine_state.ctrlc.clone();
    let metadata = input.metadata();
    let redirect_stdout = call.redirect_stdout;
    let redirect_stderr = call.redirect_stderr;
    let engine_state = engine_state.clone();
    let invert = call.has_flag("invert");

    let capture_block = predicate;
    let block_id = capture_block.block_id;

    if !call.rest::<Value>(&engine_state, stack, 0)?.is_empty() {
        return Err(ShellError::IncompatibleParametersSingle(
            "expected either a predicate or terms, not both".to_owned(),
            span,
        ));
    }

    let block = engine_state.get_block(block_id).clone();
    let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);

    let mut stack = stack.captures_to_stack(&capture_block.captures);

    input.filter(
        move |value| {
            if let Some(var_id) = var_id {
                stack.add_var(var_id, value.clone());
            }

            eval_block(
                &engine_state,
                &mut stack,
                &block,
                PipelineData::new_with_metadata(metadata.clone(), span),
                redirect_stdout,
                redirect_stderr,
            )
            .map_or(false, |pipeline_data| {
                pipeline_data.into_value(span).is_true() != invert
            })
        },
        ctrlc,
    )
}

fn find_with_rest_and_highlight(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrlc = engine_state.ctrlc.clone();
    let engine_state = engine_state.clone();
    let config = engine_state.get_config().clone();
    let filter_config = engine_state.get_config().clone();
    let invert = call.has_flag("invert");
    let terms = call.rest::<Value>(&engine_state, stack, 0)?;
    let lower_terms = terms
        .iter()
        .map(|v| {
            if let Ok(span) = v.span() {
                Value::string(v.into_string("", &config).to_lowercase(), span)
            } else {
                v.clone()
            }
        })
        .collect::<Vec<Value>>();

    let color_hm = get_color_config(&config);
    let default_style = Style::new().fg(Default).on(Default);
    let string_style = match color_hm.get("string") {
        Some(style) => *style,
        None => default_style,
    };
    let ls_colors_env_str = match stack.get_env_var(&engine_state, "LS_COLORS") {
        Some(v) => Some(env_to_string("LS_COLORS", &v, &engine_state, stack)?),
        None => None,
    };
    let ls_colors = get_ls_colors(ls_colors_env_str);

    match input {
        PipelineData::Value(_, _) => input.filter(
            move |value| {
                let lower_value = if let Ok(span) = value.span() {
                    Value::string(value.into_string("", &config).to_lowercase(), span)
                } else {
                    value.clone()
                };

                lower_terms.iter().any(|term| match value {
                    Value::Bool { .. }
                    | Value::Int { .. }
                    | Value::Filesize { .. }
                    | Value::Duration { .. }
                    | Value::Date { .. }
                    | Value::Range { .. }
                    | Value::Float { .. }
                    | Value::Block { .. }
                    | Value::Nothing { .. }
                    | Value::Error { .. } => lower_value
                        .eq(span, term, span)
                        .map_or(false, |val| val.is_true()),
                    Value::String { .. }
                    | Value::List { .. }
                    | Value::CellPath { .. }
                    | Value::CustomValue { .. } => term
                        .r#in(span, &lower_value, span)
                        .map_or(false, |val| val.is_true()),
                    Value::Record { vals, .. } => vals.iter().any(|val| {
                        if let Ok(span) = val.span() {
                            let lower_val = Value::string(
                                val.into_string("", &config).to_lowercase(),
                                Span::test_data(),
                            );

                            term.r#in(span, &lower_val, span)
                                .map_or(false, |aval| aval.is_true())
                        } else {
                            term.r#in(span, val, span)
                                .map_or(false, |aval| aval.is_true())
                        }
                    }),
                    Value::Binary { .. } => false,
                }) != invert
            },
            ctrlc,
        ),
        PipelineData::ListStream(stream, meta) => {
            Ok(ListStream::from_stream(
                stream
                    .map(move |mut x| match &mut x {
                        Value::Record { cols, vals, span } => {
                            let mut output = vec![];
                            for val in vals {
                                let val_str = val.into_string("", &config);
                                let lower_val = val.into_string("", &config).to_lowercase();
                                let mut term_added_to_output = false;
                                for term in terms.clone() {
                                    let term_str = term.into_string("", &config);
                                    let lower_term = term.into_string("", &config).to_lowercase();
                                    if lower_val.contains(&lower_term) {
                                        if config.use_ls_colors {
                                            // Get the original LS_COLORS color
                                            let style = ls_colors.style_for_path(val_str.clone());
                                            let ansi_style = style
                                                .map(LsColors_Style::to_crossterm_style)
                                                .unwrap_or_default();

                                            let ls_colored_val =
                                                ansi_style.apply(&val_str).to_string();
                                            let hi = match highlight_search_string(
                                                &ls_colored_val,
                                                &term_str,
                                                &string_style,
                                            ) {
                                                Ok(hi) => hi,
                                                Err(_) => string_style
                                                    .paint(term_str.to_string())
                                                    .to_string(),
                                            };
                                            output.push(Value::String {
                                                val: hi,
                                                span: *span,
                                            });
                                            term_added_to_output = true;
                                        } else {
                                            // No LS_COLORS support, so just use the original value
                                            let hi = match highlight_search_string(
                                                &val_str,
                                                &term_str,
                                                &string_style,
                                            ) {
                                                Ok(hi) => hi,
                                                Err(_) => string_style
                                                    .paint(term_str.to_string())
                                                    .to_string(),
                                            };
                                            output.push(Value::String {
                                                val: hi,
                                                span: *span,
                                            });
                                        }
                                    }
                                }
                                if !term_added_to_output {
                                    output.push(val.clone());
                                }
                            }
                            Value::Record {
                                cols: cols.to_vec(),
                                vals: output,
                                span: *span,
                            }
                        }
                        _ => x,
                    })
                    .filter(move |value| {
                        let lower_value = if let Ok(span) = value.span() {
                            Value::string(
                                value.into_string("", &filter_config).to_lowercase(),
                                span,
                            )
                        } else {
                            value.clone()
                        };

                        lower_terms.iter().any(|term| match value {
                            Value::Bool { .. }
                            | Value::Int { .. }
                            | Value::Filesize { .. }
                            | Value::Duration { .. }
                            | Value::Date { .. }
                            | Value::Range { .. }
                            | Value::Float { .. }
                            | Value::Block { .. }
                            | Value::Nothing { .. }
                            | Value::Error { .. } => lower_value
                                .eq(span, term, span)
                                .map_or(false, |value| value.is_true()),
                            Value::String { .. }
                            | Value::List { .. }
                            | Value::CellPath { .. }
                            | Value::CustomValue { .. } => term
                                .r#in(span, &lower_value, span)
                                .map_or(false, |value| value.is_true()),
                            Value::Record { vals, .. } => vals.iter().any(|val| {
                                if let Ok(span) = val.span() {
                                    let lower_val = Value::string(
                                        val.into_string("", &filter_config).to_lowercase(),
                                        Span::test_data(),
                                    );

                                    term.r#in(span, &lower_val, span)
                                        .map_or(false, |value| value.is_true())
                                } else {
                                    term.r#in(span, val, span)
                                        .map_or(false, |value| value.is_true())
                                }
                            }),
                            Value::Binary { .. } => false,
                        }) != invert
                    }),
                ctrlc.clone(),
            )
            .into_pipeline_data(ctrlc)
            .set_metadata(meta))
        }
        PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::new(span)),
        PipelineData::ExternalStream {
            stdout: Some(stream),
            ..
        } => {
            let mut output: Vec<Value> = vec![];
            for filter_val in stream {
                match filter_val {
                    Ok(value) => match value {
                        Value::String { val, span } => {
                            let split_char = if val.contains("\r\n") { "\r\n" } else { "\n" };

                            for line in val.split(split_char) {
                                for term in lower_terms.iter() {
                                    let term_str = term.into_string("", &filter_config);
                                    let lower_val = line.to_lowercase();
                                    if lower_val
                                        .contains(&term.into_string("", &config).to_lowercase())
                                    {
                                        output.push(Value::String {
                                            val: highlight_search_string(
                                                line,
                                                &term_str,
                                                &string_style,
                                            )?,
                                            span,
                                        })
                                    }
                                }
                            }
                        }
                        _ => {
                            return Err(ShellError::UnsupportedInput(
                                format!(
                                    "Unsupport value type '{}' from raw stream",
                                    value.get_type()
                                ),
                                span,
                            ))
                        }
                    },
                    _ => {
                        return Err(ShellError::UnsupportedInput(
                            "Unsupport type from raw stream".to_string(),
                            span,
                        ))
                    }
                };
            }
            Ok(output.into_pipeline_data(ctrlc))
        }
    }
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
