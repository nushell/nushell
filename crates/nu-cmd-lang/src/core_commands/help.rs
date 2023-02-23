use crate::help_aliases::help_aliases;
use crate::help_commands::help_commands;
use crate::help_modules::help_modules;
use fancy_regex::Regex;
use nu_ansi_term::{
    Color::{Red, White},
    Style,
};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    span, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
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
                "the name of command, alias or module to get help on",
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
        "Display help information about different parts of Nushell."
    }

    fn extra_usage(&self) -> &str {
        r#"`help word` searches for "word" in commands, aliases and modules, in that order."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
        let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

        if rest.is_empty() && find.is_none() {
            let msg = r#"Welcome to Nushell.

Here are some tips to help you get started.
  * help -h or help help - show available `help` subcommands and examples
  * help commands - list all available commands
  * help <name> - display help about a particular command, alias, or module
  * help --find <text to search> - search through all help commands table

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
        } else if find.is_some() {
            help_commands(engine_state, stack, call)
        } else {
            let result = help_aliases(engine_state, stack, call);

            let result = if let Err(ShellError::AliasNotFound(_)) = result {
                help_commands(engine_state, stack, call)
            } else {
                result
            };

            let result = if let Err(ShellError::CommandNotFound(_)) = result {
                help_modules(engine_state, stack, call)
            } else {
                result
            };

            if let Err(ShellError::ModuleNotFoundAtRuntime(_, _)) = result {
                let rest_spans: Vec<Span> = rest.iter().map(|arg| arg.span).collect();
                Err(ShellError::NotFound(span(&rest_spans)))
            } else {
                result
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "show help for single command, alias, or module",
                example: "help match",
                result: None,
            },
            Example {
                description: "show help for single sub-command, alias, or module",
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

pub fn highlight_search_in_table(
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
                            // column does not contain the searched string
                            acc
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
    let regex_string = format!("(?i){needle}");
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
