use fancy_regex::Regex;
use nu_ansi_term::{
    Color::{Red, White},
    Style,
};
use nu_color_config::StyleComputer;
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

    // 🚩The following two-lines are copied from filters/find.rs:
    let style_computer = StyleComputer::from_config(engine_state, stack);
    // Currently, search results all use the same style.
    // Also note that this sample string is passed into user-written code (the closure that may or may not be
    // defined for "string").
    let string_style = style_computer.compute("string", &Value::string("search result", head));

    if let Some(f) = find {
        let all_cmds_vec = build_help_commands(engine_state, head);
        let found_cmds_vec = highlight_search_in_table(
            all_cmds_vec,
            &f.item,
            &["name", "usage", "search_terms"],
            &string_style,
        )?;

        return Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()));
    }

    if !rest.is_empty() {
        if rest[0].item == "commands" {
            let found_cmds_vec = build_help_commands(engine_state, head);

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
                .filter(|(signature, _, _, _, _)| signature.name == name)
                .map(|(signature, examples, _, _, is_parser_keyword)| {
                    get_full_help(signature, examples, engine_state, stack, *is_parser_keyword)
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

        Ok(Value::string(msg, head).into_pipeline_data())
    }
}

fn build_help_commands(engine_state: &EngineState, span: Span) -> Vec<Value> {
    let command_ids = engine_state.get_decl_ids_sorted(false);
    let mut found_cmds_vec = Vec::new();

    for decl_id in command_ids {
        let mut cols = vec![];
        let mut vals = vec![];

        let decl = engine_state.get_decl(decl_id);
        let sig = decl.signature().update_from_command(decl.borrow());

        let signatures = sig.to_string();
        let key = sig.name;
        let usage = sig.usage;
        let search_terms = sig.search_terms;

        cols.push("name".into());
        vals.push(Value::String { val: key, span });

        cols.push("category".into());
        vals.push(Value::string(sig.category.to_string(), span));

        cols.push("command_type".into());
        vals.push(Value::String {
            val: format!("{:?}", decl.command_type()).to_lowercase(),
            span,
        });

        cols.push("usage".into());
        vals.push(Value::String { val: usage, span });

        cols.push("signatures".into());
        vals.push(Value::String {
            val: if decl.is_parser_keyword() {
                "".to_string()
            } else {
                signatures
            },
            span,
        });

        cols.push("search_terms".into());
        vals.push(if search_terms.is_empty() {
            Value::nothing(span)
        } else {
            Value::String {
                val: search_terms.join(", "),
                span,
            }
        });

        found_cmds_vec.push(Value::Record { cols, vals, span });
    }

    found_cmds_vec
}

fn highlight_search_in_table(
    table: Vec<Value>, // list of records
    search_string: &str,
    searched_cols: &[&str],
    string_style: &Style,
) -> Result<Vec<Value>, ShellError> {
    let orig_search_string = search_string;
    let search_string = search_string.to_lowercase();
    let mut matches = vec![];

    for record in table {
        let (cols, mut vals, record_span) = if let Value::Record { cols, vals, span } = record {
            (cols, vals, span)
        } else {
            return Err(ShellError::NushellFailedSpanned(
                "Expected record".to_string(),
                format!("got {}", record.get_type()),
                record.span()?,
            ));
        };

        let has_match = cols.iter().zip(vals.iter_mut()).fold(
            Ok(false),
            |acc: Result<bool, ShellError>, (col, val)| {
                if searched_cols.contains(&col.as_str()) {
                    if let Value::String { val: s, span } = val {
                        if s.to_lowercase().contains(&search_string) {
                            *val = Value::String {
                                val: highlight_search_string(s, orig_search_string, string_style)?,
                                span: *span,
                            };
                            Ok(true)
                        } else {
                            Err(ShellError::TypeMismatchHelp(
                                format!("expected string, got {}", val.get_type()),
                                val.span()?,
                                "Only columns containing strings can be searched with highlighting."
                                    .to_string(),
                            ))
                        }
                    } else {
                        // ignore non-string values
                        acc
                    }
                } else {
                    // don't search this column
                    acc
                }
            },
        )?;

        if has_match {
            matches.push(Value::Record {
                cols,
                vals,
                span: record_span,
            });
        }
    }

    Ok(matches)
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
