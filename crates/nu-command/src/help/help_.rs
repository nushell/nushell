use crate::help::{help_aliases, help_commands, help_modules};
use fancy_regex::Regex;
use nu_ansi_term::Style;
use nu_engine::command_prelude::*;
use nu_protocol::span;
use nu_utils::IgnoreCaseExt;

#[derive(Clone)]
pub struct Help;

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> Signature {
        Signature::build("help")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .rest(
                "rest",
                SyntaxShape::String,
                "The name of command, alias or module to get help on.",
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

            let result = if let Err(ShellError::AliasNotFound { .. }) = result {
                help_commands(engine_state, stack, call)
            } else {
                result
            };

            let result = if let Err(ShellError::CommandNotFound { .. }) = result {
                help_modules(engine_state, stack, call)
            } else {
                result
            };

            if let Err(ShellError::ModuleNotFoundAtRuntime {
                mod_name: _,
                span: _,
            }) = result
            {
                let rest_spans: Vec<Span> = rest.iter().map(|arg| arg.span).collect();
                Err(ShellError::NotFound {
                    span: span(&rest_spans),
                })
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
                example: "help str join",
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
    highlight_style: &Style,
) -> Result<Vec<Value>, ShellError> {
    let orig_search_string = search_string;
    let search_string = search_string.to_folded_case();
    let mut matches = vec![];

    for mut value in table {
        let Value::Record {
            val: ref mut record,
            ..
        } = value
        else {
            return Err(ShellError::NushellFailedSpanned {
                msg: "Expected record".to_string(),
                label: format!("got {}", value.get_type()),
                span: value.span(),
            });
        };

        let has_match = record.to_mut().iter_mut().try_fold(
            false,
            |acc: bool, (col, val)| -> Result<bool, ShellError> {
                if !searched_cols.contains(&col.as_str()) {
                    // don't search this column
                    return Ok(acc);
                }
                let span = val.span();
                if let Value::String { val: s, .. } = val {
                    if s.to_folded_case().contains(&search_string) {
                        *val = Value::string(
                            highlight_search_string(
                                s,
                                orig_search_string,
                                string_style,
                                highlight_style,
                            )?,
                            span,
                        );
                        return Ok(true);
                    }
                }
                // column does not contain the searched string
                // ignore non-string values
                Ok(acc)
            },
        )?;

        if has_match {
            matches.push(value);
        }
    }

    Ok(matches)
}

// Highlight the search string using ANSI escape sequences and regular expressions.
pub fn highlight_search_string(
    haystack: &str,
    needle: &str,
    string_style: &Style,
    highlight_style: &Style,
) -> Result<String, ShellError> {
    let regex_string = format!("(?i){needle}");
    let regex = match Regex::new(&regex_string) {
        Ok(regex) => regex,
        Err(err) => {
            return Err(ShellError::GenericError {
                error: "Could not compile regex".into(),
                msg: err.to_string(),
                span: Some(Span::test_data()),
                help: None,
                inner: vec![],
            });
        }
    };
    // strip haystack to remove existing ansi style
    let stripped_haystack = nu_utils::strip_ansi_likely(haystack);
    let mut last_match_end = 0;
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
                highlighted.push_str(
                    &highlight_style
                        .paint(&stripped_haystack[start..end])
                        .to_string(),
                );
                last_match_end = end;
            }
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "Error with regular expression capture".into(),
                    msg: e.to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                });
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
