use fancy_regex::Regex;
use itertools::Itertools;
use nu_ansi_term::{
    Color::{Default, Red, White},
    Style,
};
use nu_color_config::get_color_config;
use nu_engine::{get_full_help, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    span, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};
use std::borrow::Borrow;
#[derive(Clone)]
pub struct Help;

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> Signature {
        Signature::build("help")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .rest(
                "rest",
                SyntaxShape::String,
                "the name of command to get help on",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in command names, usage, and search terms",
                Some('f'),
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Display help information about commands."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        help(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "show all commands and sub-commands",
                example: "help commands",
                result: None,
            },
            Example {
                description: "show help for single command",
                example: "help match",
                result: None,
            },
            Example {
                description: "show help for single sub-command",
                example: "help str lpad",
                result: None,
            },
            Example {
                description: "search for string in command names, usage and search terms",
                example: "help --find char",
                result: None,
            },
        ]
    }
}

fn help(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;
    let commands = engine_state.get_decl_ids_sorted(false);
    let config = engine_state.get_config();
    let color_hm = get_color_config(config);
    let default_style = Style::new().fg(Default).on(Default);
    let string_style = match color_hm.get("string") {
        Some(style) => style,
        None => &default_style,
    };

    if let Some(f) = find {
        let org_search_string = f.item.clone();
        let search_string = f.item.to_lowercase();
        let mut found_cmds_vec = Vec::new();

        for decl_id in commands {
            let mut cols = vec![];
            let mut vals = vec![];
            let decl = engine_state.get_decl(decl_id);
            let sig = decl.signature().update_from_command(decl.borrow());
            let key = sig.name;
            let usage = sig.usage;
            let search_terms = sig.search_terms;

            let matches_term = if !search_terms.is_empty() {
                search_terms
                    .iter()
                    .any(|term| term.to_lowercase().contains(&search_string))
            } else {
                false
            };

            let key_match = key.to_lowercase().contains(&search_string);
            let usage_match = usage.to_lowercase().contains(&search_string);
            if key_match || usage_match || matches_term {
                cols.push("name".into());
                vals.push(Value::String {
                    val: if key_match {
                        highlight_search_string(&key, &org_search_string, string_style)?
                    } else {
                        key
                    },
                    span: head,
                });

                cols.push("category".into());
                vals.push(Value::String {
                    val: sig.category.to_string(),
                    span: head,
                });

                cols.push("command_type".into());
                vals.push(Value::String {
                    val: format!("{:?}", decl.command_type()).to_lowercase(),
                    span: head,
                });

                cols.push("usage".into());
                vals.push(Value::String {
                    val: if usage_match {
                        highlight_search_string(&usage, &org_search_string, string_style)?
                    } else {
                        usage
                    },
                    span: head,
                });

                cols.push("signatures".into());
                vals.push(Value::String {
                    val: sig
                        .input_output_types
                        .iter()
                        .map(|(i, o)| format!("{:?} => {:?}", i.to_shape(), o.to_shape()))
                        .join("\n"),
                    span: head,
                });

                cols.push("search_terms".into());
                vals.push(if search_terms.is_empty() {
                    Value::nothing(head)
                } else {
                    Value::String {
                        val: if matches_term {
                            search_terms
                                .iter()
                                .map(|term| {
                                    if term.to_lowercase().contains(&search_string) {
                                        match highlight_search_string(
                                            term,
                                            &org_search_string,
                                            string_style,
                                        ) {
                                            Ok(s) => s,
                                            Err(_) => {
                                                string_style.paint(term.to_string()).to_string()
                                            }
                                        }
                                    } else {
                                        string_style.paint(term.to_string()).to_string()
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        } else {
                            search_terms.join(", ")
                        },
                        span: head,
                    }
                });

                found_cmds_vec.push(Value::Record {
                    cols,
                    vals,
                    span: head,
                });
            }
        }

        return Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()));
    }

    if !rest.is_empty() {
        let mut found_cmds_vec = Vec::new();

        if rest[0].item == "commands" {
            for decl_id in commands {
                let mut cols = vec![];
                let mut vals = vec![];

                let decl = engine_state.get_decl(decl_id);
                let sig = decl.signature().update_from_command(decl.borrow());

                let key = sig.name;
                let usage = sig.usage;
                let search_terms = sig.search_terms;

                cols.push("name".into());
                vals.push(Value::String {
                    val: key,
                    span: head,
                });

                cols.push("category".into());
                vals.push(Value::String {
                    val: sig.category.to_string(),
                    span: head,
                });

                cols.push("command_type".into());
                vals.push(Value::String {
                    val: format!("{:?}", decl.command_type()).to_lowercase(),
                    span: head,
                });

                cols.push("usage".into());
                vals.push(Value::String {
                    val: usage,
                    span: head,
                });

                cols.push("signatures".into());
                vals.push(Value::String {
                    val: sig
                        .input_output_types
                        .iter()
                        .map(|(i, o)| format!("{:?} => {:?}", i.to_shape(), o.to_shape()))
                        .join("\n"),
                    span: head,
                });

                cols.push("search_terms".into());
                vals.push(if search_terms.is_empty() {
                    Value::nothing(head)
                } else {
                    Value::String {
                        val: search_terms.join(", "),
                        span: head,
                    }
                });

                found_cmds_vec.push(Value::Record {
                    cols,
                    vals,
                    span: head,
                });
            }

            Ok(found_cmds_vec
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone()))
        } else {
            let mut name = String::new();

            for r in &rest {
                if !name.is_empty() {
                    name.push(' ');
                }
                name.push_str(&r.item);
            }

            let output = engine_state
                .get_signatures_with_examples(false)
                .iter()
                .filter(|(signature, _, _, _)| signature.name == name)
                .map(|(signature, examples, _, _)| {
                    get_full_help(signature, examples, engine_state, stack)
                })
                .collect::<Vec<String>>();

            if !output.is_empty() {
                Ok(Value::String {
                    val: output.join("======================\n\n"),
                    span: call.head,
                }
                .into_pipeline_data())
            } else {
                Err(ShellError::CommandNotFound(span(&[
                    rest[0].span,
                    rest[rest.len() - 1].span,
                ])))
            }
        }
    } else {
        let msg = r#"Welcome to Nushell.

Here are some tips to help you get started.
  * help commands - list all available commands
  * help <command name> - display help about a particular command
  * help --find <text to search> - search through all of help

Nushell works on the idea of a "pipeline". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

[Examples]

List the files in the current directory, sorted by size:
    ls | sort-by size

Get information about the current system:
    sys | get host

Get the processes on your system actively using CPU:
    ps | where cpu > 0

You can also learn more at https://www.nushell.sh/book/"#;

        Ok(Value::String {
            val: msg.into(),
            span: head,
        }
        .into_pipeline_data())
    }
}

// Highlight the search string using ANSI escape sequences and regular expressions.
pub fn highlight_search_string(
    haystack: &str,
    needle: &str,
    string_style: &Style,
) -> Result<String, ShellError> {
    let regex_string = format!("(?i){}", needle);
    let regex = match Regex::new(&regex_string) {
        Ok(regex) => regex,
        Err(err) => {
            return Err(ShellError::GenericError(
                "Could not compile regex".into(),
                err.to_string(),
                Some(Span::test_data()),
                None,
                Vec::new(),
            ));
        }
    };
    // strip haystack to remove existing ansi style
    let stripped_haystack = nu_utils::strip_ansi_likely(haystack);
    let mut last_match_end = 0;
    let style = Style::new().fg(White).on(Red);
    let mut highlighted = String::new();

    for cap in regex.captures_iter(stripped_haystack.as_ref()) {
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
                    &string_style
                        .paint(&stripped_haystack[last_match_end..start])
                        .to_string(),
                );
                highlighted.push_str(&style.paint(&stripped_haystack[start..end]).to_string());
                last_match_end = end;
            }
            Err(e) => {
                return Err(ShellError::GenericError(
                    "Error with regular expression capture".into(),
                    e.to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        }
    }

    highlighted.push_str(
        &string_style
            .paint(&stripped_haystack[last_match_end..])
            .to_string(),
    );
    Ok(highlighted)
}
