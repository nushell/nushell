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
                            let command_completer = crate::completion::command::Completer {};
                            command_completer.complete(context, partial, &completion_matcher)
                        }

                        LocationType::Flag(cmd) => {
                            let flag_completer = crate::completion::flag::Completer {};
                            flag_completer.complete(context, cmd, partial, &completion_matcher)
                        }

                        LocationType::Argument(cmd, _arg_name) => {
                            let path_completer = crate::completion::path::Completer::new();
                            let completed_paths = path_completer.complete(context, partial, &completion_matcher);
                            match cmd.as_deref().unwrap_or("") {
                                "cd" => select_directory_suggestions(completed_paths),
                                _ => completed_paths,
                            }
                        }

                        LocationType::Variable => Vec::new(),
                    }
                    .into_iter()
                    .map(requote)
                })
                .collect();

            (pos, suggestions)
        }
    }
}

fn select_directory_suggestions(completed_paths: Vec<Suggestion>) -> Vec<Suggestion> {
    completed_paths
        .into_iter()
        .filter(|suggestion| {
            metadata(&suggestion.replacement)
                .map(|md| md.is_dir())
                .unwrap_or(false)
        })
        .collect()
}

fn requote(item: Suggestion) -> Suggestion {
    let unescaped = rustyline::completion::unescape(&item.replacement, Some('\\'));
    if unescaped != item.replacement {
        Suggestion {
            display: item.display,
            replacement: format!("\"{}\"", unescaped),
        }
    } else {
        item
    }
}
