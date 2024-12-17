use std::collections::HashMap;

use crate::{
    completions::{Completer, CompletionOptions},
    SuggestionKind,
};
use nu_parser::FlatShape;
use nu_protocol::{
    engine::{CachedFile, Stack, StateWorkingSet},
    Span,
};
use reedline::Suggestion;

use super::{completion_options::NuMatcher, SemanticSuggestion};

pub struct CommandCompletion {
    flattened: Vec<(Span, FlatShape)>,
    flat_shape: FlatShape,
    force_completion_after_space: bool,
}

impl CommandCompletion {
    pub fn new(
        flattened: Vec<(Span, FlatShape)>,
        flat_shape: FlatShape,
        force_completion_after_space: bool,
    ) -> Self {
        Self {
            flattened,
            flat_shape,
            force_completion_after_space,
        }
    }

    fn external_command_completion(
        &self,
        working_set: &StateWorkingSet,
        sugg_span: reedline::Span,
        matched_internal: impl Fn(&str) -> bool,
        matcher: &mut NuMatcher<String>,
    ) -> HashMap<String, SemanticSuggestion> {
        let mut suggs = HashMap::new();

        let paths = working_set.permanent_state.get_env_var_insensitive("path");

        if let Some(paths) = paths {
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
                                format!("^{}", name)
                            } else {
                                name.clone()
                            };
                            if suggs.contains_key(&value) {
                                continue;
                            }
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
                                        // TODO: is there a way to create a test?
                                        kind: None,
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

    fn complete_commands(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
        find_externals: bool,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let partial = working_set.get_span_contents(span);
        let mut matcher = NuMatcher::new(String::from_utf8_lossy(partial), options.clone());

        let sugg_span = reedline::Span::new(span.start - offset, span.end - offset);

        let mut internal_suggs = HashMap::new();
        let filtered_commands = working_set.find_commands_by_predicate(
            |name| {
                let name = String::from_utf8_lossy(name);
                matcher.add(&name, name.to_string())
            },
            true,
        );
        for (name, description, typ) in filtered_commands {
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
                    kind: Some(SuggestionKind::Command(typ)),
                },
            );
        }

        let mut external_suggs = if find_externals {
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

impl Completer for CommandCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        _prefix: &[u8],
        span: Span,
        offset: usize,
        pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let last = self
            .flattened
            .iter()
            .rev()
            .skip_while(|x| x.0.end > pos)
            .take_while(|x| {
                matches!(
                    x.1,
                    FlatShape::InternalCall(_)
                        | FlatShape::External
                        | FlatShape::ExternalArg
                        | FlatShape::Literal
                        | FlatShape::String
                )
            })
            .last();

        // The last item here would be the earliest shape that could possible by part of this subcommand
        let subcommands = if let Some(last) = last {
            self.complete_commands(
                working_set,
                Span::new(last.0.start, pos),
                offset,
                false,
                options,
            )
        } else {
            vec![]
        };

        if !subcommands.is_empty() {
            return subcommands;
        }

        let config = working_set.get_config();
        if matches!(self.flat_shape, nu_parser::FlatShape::External)
            || matches!(self.flat_shape, nu_parser::FlatShape::InternalCall(_))
            || ((span.end - span.start) == 0)
            || is_passthrough_command(working_set.delta.get_file_contents())
        {
            // we're in a gap or at a command
            if working_set.get_span_contents(span).is_empty() && !self.force_completion_after_space
            {
                return vec![];
            }
            self.complete_commands(
                working_set,
                span,
                offset,
                config.completions.external.enable,
                options,
            )
        } else {
            vec![]
        }
    }
}

pub fn find_non_whitespace_index(contents: &[u8], start: usize) -> usize {
    match contents.get(start..) {
        Some(contents) => {
            contents
                .iter()
                .take_while(|x| x.is_ascii_whitespace())
                .count()
                + start
        }
        None => start,
    }
}

pub fn is_passthrough_command(working_set_file_contents: &[CachedFile]) -> bool {
    for cached_file in working_set_file_contents {
        let contents = &cached_file.content;
        let last_pipe_pos_rev = contents.iter().rev().position(|x| x == &b'|');
        let last_pipe_pos = last_pipe_pos_rev.map(|x| contents.len() - x).unwrap_or(0);

        let cur_pos = find_non_whitespace_index(contents, last_pipe_pos);

        let result = match contents.get(cur_pos..) {
            Some(contents) => contents.starts_with(b"sudo ") || contents.starts_with(b"doas "),
            None => false,
        };
        if result {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod command_completions_tests {
    use super::*;
    use nu_protocol::engine::EngineState;
    use std::sync::Arc;

    #[test]
    fn test_find_non_whitespace_index() {
        let commands = [
            ("    hello", 4),
            ("sudo ", 0),
            (" 	sudo ", 2),
            ("	 sudo ", 2),
            ("	hello ", 1),
            ("	  hello ", 3),
            ("    hello | sudo ", 4),
            ("     sudo|sudo", 5),
            ("sudo | sudo ", 0),
            ("	hello sud", 1),
        ];
        for (idx, ele) in commands.iter().enumerate() {
            let index = find_non_whitespace_index(ele.0.as_bytes(), 0);
            assert_eq!(index, ele.1, "Failed on index {}", idx);
        }
    }

    #[test]
    fn test_is_last_command_passthrough() {
        let commands = [
            ("    hello", false),
            ("    sudo ", true),
            ("sudo ", true),
            ("	hello", false),
            ("	sudo", false),
            ("	sudo ", true),
            (" 	sudo ", true),
            ("	 sudo ", true),
            ("	hello ", false),
            ("    hello | sudo ", true),
            ("    sudo|sudo", false),
            ("sudo | sudo ", true),
            ("	hello sud", false),
            ("	sudo | sud ", false),
            ("	sudo|sudo ", true),
            (" 	sudo | sudo ls | sudo ", true),
        ];
        for (idx, ele) in commands.iter().enumerate() {
            let input = ele.0.as_bytes();

            let mut engine_state = EngineState::new();
            engine_state.add_file("test.nu".into(), Arc::new([]));

            let delta = {
                let mut working_set = StateWorkingSet::new(&engine_state);
                let _ = working_set.add_file("child.nu".into(), input);
                working_set.render()
            };

            let result = engine_state.merge_delta(delta);
            assert!(
                result.is_ok(),
                "Merge delta has failed: {}",
                result.err().unwrap()
            );

            let is_passthrough_command = is_passthrough_command(engine_state.get_file_contents());
            assert_eq!(
                is_passthrough_command, ele.1,
                "index for '{}': {}",
                ele.0, idx
            );
        }
    }
}
