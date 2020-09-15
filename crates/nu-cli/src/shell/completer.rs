use crate::completion::{self, Suggestion};
use crate::context;
use std::fs::metadata;

use crate::completion::matchers;
use crate::completion::matchers::Matcher;

use nu_source::Tag;

pub(crate) struct NuCompleter {}

impl NuCompleter {}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &completion::Context,
    ) -> (usize, Vec<Suggestion>) {
        use crate::completion::engine::LocationType;

        let nu_context: &context::Context = context.as_ref();
        let lite_block = match nu_parser::lite_parse(line, 0) {
            Ok(block) => Some(block),
            Err(result) => result.partial,
        };

        let locations = lite_block
            .map(|block| nu_parser::classify_block(&block, &nu_context.registry))
            .map(|block| crate::completion::engine::completion_location(line, &block.block, pos))
            .unwrap_or_default();

        let matcher_config = &nu_data::config::config(Tag::unknown())
            .ok()
            .and_then(|cfg| cfg.get("completion.matcher").cloned())
            .and_then(|v| v.as_string().ok())
            .unwrap_or_else(|| "".to_string());

        let matcher_config = matcher_config.as_str();

        let completion_matcher: Box<dyn Matcher> = match matcher_config {
            "case-insensitive" => Box::new(matchers::case_insensitive::Matcher),
            _ => Box::new(matchers::case_sensitive::Matcher),
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
                            let command_completer = crate::completion::command::Completer;
                            command_completer.complete(context, partial)
                        }

                        LocationType::Flag(cmd) => {
                            let flag_completer = crate::completion::flag::Completer;
                            flag_completer.complete(context, cmd, partial)
                        }

                        LocationType::Argument(cmd, _arg_name) => {
                            let path_completer = crate::completion::path::Completer;

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

                            let completed_paths =
                                path_completer.complete(context, partial, &completion_matcher);
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

fn requote(value: String) -> String {
    let value = rustyline::completion::unescape(&value, Some('\\'));

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
