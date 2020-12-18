use crate::completion::command::CommandCompleter;
use crate::completion::flag::FlagCompleter;
use crate::completion::matchers;
use crate::completion::matchers::Matcher;
use crate::completion::path::{PathCompleter, PathSuggestion};
use crate::completion::{self, Completer, Suggestion};
use crate::evaluation_context::EvaluationContext;
use nu_parser::ParserScope;
use nu_source::Tag;

use std::borrow::Cow;

pub(crate) struct NuCompleter {}

impl NuCompleter {}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &completion::CompletionContext,
    ) -> (usize, Vec<Suggestion>) {
        use completion::engine::LocationType;

        let nu_context: &EvaluationContext = context.as_ref();

        nu_context.scope.enter_scope();
        let (block, _) = nu_parser::parse(line, 0, &nu_context.scope);
        nu_context.scope.exit_scope();

        let locations = completion::engine::completion_location(line, &block, pos);

        let matcher = nu_data::config::config(Tag::unknown())
            .ok()
            .and_then(|cfg| cfg.get("line_editor").cloned())
            .and_then(|le| {
                le.row_entries()
                    .find(|(idx, _value)| idx.as_str() == "completion_match_method")
                    .and_then(|(_idx, value)| value.as_string().ok())
            })
            .unwrap_or_else(String::new);

        let matcher = matcher.as_str();
        let matcher: &dyn Matcher = match matcher {
            "case-insensitive" => &matchers::case_insensitive::Matcher,
            _ => &matchers::case_sensitive::Matcher,
        };

        if locations.is_empty() {
            (pos, Vec::new())
        } else {
            let pos = locations[0].span.start();
            let suggestions = locations
                .into_iter()
                .flat_map(|location| {
                    let partial = location.span.slice(line);
                    match location.item {
                        LocationType::Command => {
                            let command_completer = CommandCompleter;
                            command_completer.complete(context, partial, matcher.to_owned())
                        }

                        LocationType::Flag(cmd) => {
                            let flag_completer = FlagCompleter { cmd };
                            flag_completer.complete(context, partial, matcher.to_owned())
                        }

                        LocationType::Argument(cmd, _arg_name) => {
                            let path_completer = PathCompleter;

                            const QUOTE_CHARS: &[char] = &['\'', '"', '`'];

                            // TODO Find a better way to deal with quote chars. Can the completion
                            //      engine relay this back to us? Maybe have two spans: inner and
                            //      outer. The former is what we want to complete, the latter what
                            //      we'd need to replace.
                            let (quote_char, partial) = if partial.starts_with(QUOTE_CHARS) {
                                let (head, tail) = partial.split_at(1);
                                (Some(head), tail)
                            } else {
                                (None, partial)
                            };

                            let partial = if let Some(quote_char) = quote_char {
                                if partial.ends_with(quote_char) {
                                    &partial[..partial.len() - 1]
                                } else {
                                    partial
                                }
                            } else {
                                partial
                            };

                            let completed_paths = path_completer.path_suggestions(partial, matcher);
                            match cmd.as_deref().unwrap_or("") {
                                "cd" => select_directory_suggestions(completed_paths),
                                _ => completed_paths,
                            }
                            .into_iter()
                            .map(|s| Suggestion {
                                replacement: requote(s.suggestion.replacement),
                                display: s.suggestion.display,
                            })
                            .collect()
                        }

                        LocationType::Variable => Vec::new(),
                    }
                })
                .collect();

            (pos, suggestions)
        }
    }
}

fn select_directory_suggestions(completed_paths: Vec<PathSuggestion>) -> Vec<PathSuggestion> {
    completed_paths
        .into_iter()
        .filter(|suggestion| {
            suggestion
                .path
                .metadata()
                .map(|md| md.is_dir())
                .unwrap_or(false)
        })
        .collect()
}

fn requote(orig_value: String) -> String {
    let value: Cow<str> = rustyline::completion::unescape(&orig_value, Some('\\'));

    let mut quotes = vec!['"', '\'', '`'];
    let mut should_quote = false;
    for c in value.chars() {
        if c.is_whitespace() {
            should_quote = true;
        } else if let Some(index) = quotes.iter().position(|q| *q == c) {
            should_quote = true;
            quotes.swap_remove(index);
        }
    }

    if should_quote {
        if quotes.is_empty() {
            // TODO we don't really have an escape character, so there isn't a great option right
            //      now. One possibility is `{{$(char backtick)}}`
            value.to_string()
        } else {
            let quote = quotes[0];
            format!("{}{}{}", quote, value, quote)
        }
    } else {
        value.to_string()
    }
}
