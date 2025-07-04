use std::collections::HashMap;

use crate::{
    SuggestionKind,
    completions::{Completer, CompletionOptions},
};
use nu_protocol::{
    Span,
    engine::{CommandType, Stack, StateWorkingSet},
};
use reedline::Suggestion;

use super::{SemanticSuggestion, completion_options::NuMatcher};

pub struct CommandCompletion {
    /// Whether to include internal commands
    pub internals: bool,
    /// Whether to include external commands
    pub externals: bool,
}

impl CommandCompletion {
    fn external_command_completion(
        &self,
        working_set: &StateWorkingSet,
        sugg_span: reedline::Span,
        matched_internal: impl Fn(&str) -> bool,
        matcher: &mut NuMatcher<String>,
    ) -> HashMap<String, SemanticSuggestion> {
        let mut suggs = HashMap::new();

        let paths = working_set.permanent_state.get_env_var_insensitive("path");

        if let Some((_, paths)) = paths {
            if let Ok(paths) = paths.as_list() {
                for path in paths {
                    let path = path.coerce_str().unwrap_or_default();

                    if let Ok(mut contents) = std::fs::read_dir(path.as_ref()) {
                        while let Some(Ok(item)) = contents.next() {
                            if working_set
                                .permanent_state
                                .config
                                .completions
                                .external
                                .max_results
                                <= suggs.len() as i64
                            {
                                break;
                            }
                            let Ok(name) = item.file_name().into_string() else {
                                continue;
                            };
                            let value = if matched_internal(&name) {
                                format!("^{name}")
                            } else {
                                name.clone()
                            };
                            if suggs.contains_key(&value) {
                                continue;
                            }
                            // TODO: check name matching before a relative heavy IO involved
                            // `is_executable` for performance consideration, should avoid
                            // duplicated `match_aux` call for matched items in the future
                            if matcher.matches(&name) && is_executable::is_executable(item.path()) {
                                // If there's an internal command with the same name, adds ^cmd to the
                                // matcher so that both the internal and external command are included
                                matcher.add(&name, value.clone());
                                suggs.insert(
                                    value.clone(),
                                    SemanticSuggestion {
                                        suggestion: Suggestion {
                                            value,
                                            span: sugg_span,
                                            append_whitespace: true,
                                            ..Default::default()
                                        },
                                        kind: Some(SuggestionKind::Command(
                                            CommandType::External,
                                            None,
                                        )),
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        suggs
    }
}

impl Completer for CommandCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut matcher = NuMatcher::new(prefix, options);

        let sugg_span = reedline::Span::new(span.start - offset, span.end - offset);

        let mut internal_suggs = HashMap::new();
        if self.internals {
            let filtered_commands = working_set.find_commands_by_predicate(
                |name| {
                    let name = String::from_utf8_lossy(name);
                    matcher.add(&name, name.to_string())
                },
                true,
            );
            for (decl_id, name, description, typ) in filtered_commands {
                let name = String::from_utf8_lossy(&name);
                internal_suggs.insert(
                    name.to_string(),
                    SemanticSuggestion {
                        suggestion: Suggestion {
                            value: name.to_string(),
                            description,
                            span: sugg_span,
                            append_whitespace: true,
                            ..Suggestion::default()
                        },
                        kind: Some(SuggestionKind::Command(typ, Some(decl_id))),
                    },
                );
            }
        }

        let mut external_suggs = if self.externals {
            self.external_command_completion(
                working_set,
                sugg_span,
                |name| internal_suggs.contains_key(name),
                &mut matcher,
            )
        } else {
            HashMap::new()
        };

        let mut res = Vec::new();
        for cmd_name in matcher.results() {
            if let Some(sugg) = internal_suggs
                .remove(&cmd_name)
                .or_else(|| external_suggs.remove(&cmd_name))
            {
                res.push(sugg);
            }
        }
        res
    }
}
