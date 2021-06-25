use std::borrow::Cow;

use nu_parser::NewlineMode;
use nu_source::{Span, Tag};

use crate::command::CommandCompleter;
use crate::engine;
use crate::flag::FlagCompleter;
use crate::matchers;
use crate::matchers::Matcher;
use crate::path::{PathCompleter, PathSuggestion};
use crate::variable::VariableCompleter;
use crate::{Completer, CompletionContext, Suggestion};

pub struct NuCompleter {}

impl NuCompleter {}

impl NuCompleter {
    pub fn complete<Context: CompletionContext>(
        &self,
        line: &str,
        pos: usize,
        context: &Context,
    ) -> (usize, Vec<Suggestion>) {
        use engine::LocationType;

        let tokens = nu_parser::lex(line, 0, NewlineMode::Normal).0;

        let locations = Some(nu_parser::parse_block(tokens).0)
            .map(|block| nu_parser::classify_block(&block, context.scope()))
            .map(|(block, _)| engine::completion_location(line, &block, pos))
            .unwrap_or_default();

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
            "case-sensitive" => &matchers::case_sensitive::Matcher,
            #[cfg(target_os = "windows")]
            _ => &matchers::case_insensitive::Matcher,
            #[cfg(not(target_os = "windows"))]
            _ => &matchers::case_sensitive::Matcher,
        };

        if locations.is_empty() {
            (pos, Vec::new())
        } else {
            let mut pos = locations[0].span.start();

            for location in &locations {
                if location.span.start() < pos {
                    pos = location.span.start();
                }
            }
            let suggestions = locations
                .into_iter()
                .flat_map(|location| {
                    let partial = location.span.slice(line).to_string();
                    match location.item {
                        LocationType::Command => {
                            let command_completer = CommandCompleter;
                            command_completer.complete(context, &partial, matcher.to_owned())
                        }

                        LocationType::Flag(cmd) => {
                            let flag_completer = FlagCompleter { cmd };
                            flag_completer.complete(context, &partial, matcher.to_owned())
                        }

                        LocationType::Argument(cmd, _arg_name) => {
                            let path_completer = PathCompleter;
                            let prepend = Span::new(pos, location.span.start()).slice(line);

                            const QUOTE_CHARS: &[char] = &['\'', '"'];

                            // TODO Find a better way to deal with quote chars. Can the completion
                            //      engine relay this back to us? Maybe have two spans: inner and
                            //      outer. The former is what we want to complete, the latter what
                            //      we'd need to replace.
                            let (quote_char, partial) = if partial.starts_with(QUOTE_CHARS) {
                                let (head, tail) = partial.split_at(1);
                                (Some(head), tail.to_string())
                            } else {
                                (None, partial)
                            };

                            let (mut partial, quoted) = if let Some(quote_char) = quote_char {
                                if partial.ends_with(quote_char) {
                                    (partial[..partial.len() - 1].to_string(), true)
                                } else {
                                    (partial, false)
                                }
                            } else {
                                (partial, false)
                            };

                            partial = partial.split('"').collect::<Vec<_>>().join("");
                            let completed_paths =
                                path_completer.path_suggestions(&partial, matcher);
                            match cmd.as_deref().unwrap_or("") {
                                "cd" => select_directory_suggestions(completed_paths),
                                _ => completed_paths,
                            }
                            .into_iter()
                            .map(|s| Suggestion {
                                replacement: format!(
                                    "{}{}",
                                    prepend,
                                    requote(s.suggestion.replacement, quoted)
                                ),
                                display: s.suggestion.display,
                            })
                            .collect()
                        }

                        LocationType::Variable => {
                            let variable_completer = VariableCompleter;
                            variable_completer.complete(context, &partial, matcher.to_owned())
                        }
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

fn requote(orig_value: String, previously_quoted: bool) -> String {
    let value: Cow<str> = {
        #[cfg(feature = "rustyline-support")]
        {
            rustyline::completion::unescape(&orig_value, Some('\\'))
        }
        #[cfg(not(feature = "rustyline-support"))]
        {
            orig_value.into()
        }
    };

    let mut quotes = vec!['"', '\''];
    let mut should_quote = false;
    for c in value.chars() {
        if c.is_whitespace() || c == '#' {
            should_quote = true;
        } else if let Some(index) = quotes.iter().position(|q| *q == c) {
            should_quote = true;
            quotes.swap_remove(index);
        }
    }

    if should_quote {
        if quotes.is_empty() {
            // TODO we don't really have an escape character, so there isn't a great option right
            //      now. One possibility is `{{(char backtick)}}`
            value.to_string()
        } else {
            let quote = quotes[0];
            if previously_quoted {
                format!("{}{}", quote, value)
            } else {
                format!("{}{}{}", quote, value, quote)
            }
        }
    } else {
        value.to_string()
    }
}
