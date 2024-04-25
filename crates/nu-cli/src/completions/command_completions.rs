use crate::{
    completions::{Completer, CompletionOptions, MatchAlgorithm, SortBy},
    SuggestionKind,
};
use nu_parser::FlatShape;
use nu_protocol::{
    engine::{CachedFile, EngineState, StateWorkingSet},
    Span,
};
use reedline::Suggestion;
use std::sync::Arc;

use super::SemanticSuggestion;

pub struct CommandCompletion {
    engine_state: Arc<EngineState>,
    flattened: Vec<(Span, FlatShape)>,
    flat_shape: FlatShape,
    force_completion_after_space: bool,
}

impl CommandCompletion {
    pub fn new(
        engine_state: Arc<EngineState>,
        _: &StateWorkingSet,
        flattened: Vec<(Span, FlatShape)>,
        flat_shape: FlatShape,
        force_completion_after_space: bool,
    ) -> Self {
        Self {
            engine_state,
            flattened,
            flat_shape,
            force_completion_after_space,
        }
    }

    fn external_command_completion(
        &self,
        prefix: &str,
        match_algorithm: MatchAlgorithm,
    ) -> Vec<String> {
        let mut executables = vec![];

        // os agnostic way to get the PATH env var
        let paths = self.engine_state.get_path_env_var();

        if let Some(paths) = paths {
            if let Ok(paths) = paths.as_list() {
                for path in paths {
                    let path = path.coerce_str().unwrap_or_default();

                    if let Ok(mut contents) = std::fs::read_dir(path.as_ref()) {
                        while let Some(Ok(item)) = contents.next() {
                            if self.engine_state.config.max_external_completion_results
                                > executables.len() as i64
                                && !executables.contains(
                                    &item
                                        .path()
                                        .file_name()
                                        .map(|x| x.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                )
                                && matches!(
                                    item.path().file_name().map(|x| match_algorithm
                                        .matches_str(&x.to_string_lossy(), prefix)),
                                    Some(true)
                                )
                                && is_executable::is_executable(item.path())
                            {
                                if let Ok(name) = item.file_name().into_string() {
                                    executables.push(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        executables
    }

    fn complete_commands(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
        find_externals: bool,
        match_algorithm: MatchAlgorithm,
    ) -> Vec<SemanticSuggestion> {
        let partial = working_set.get_span_contents(span);

        let filter_predicate = |command: &[u8]| match_algorithm.matches_u8(command, partial);

        let mut results = working_set
            .find_commands_by_predicate(filter_predicate, true)
            .into_iter()
            .map(move |x| SemanticSuggestion {
                suggestion: Suggestion {
                    value: String::from_utf8_lossy(&x.0).to_string(),
                    description: x.1,
                    style: None,
                    extra: None,
                    span: reedline::Span::new(span.start - offset, span.end - offset),
                    append_whitespace: true,
                },
                kind: Some(SuggestionKind::Command(x.2)),
            })
            .collect::<Vec<_>>();

        let partial = working_set.get_span_contents(span);
        let partial = String::from_utf8_lossy(partial).to_string();

        if find_externals {
            let results_external = self
                .external_command_completion(&partial, match_algorithm)
                .into_iter()
                .map(move |x| SemanticSuggestion {
                    suggestion: Suggestion {
                        value: x,
                        description: None,
                        style: None,
                        extra: None,
                        span: reedline::Span::new(span.start - offset, span.end - offset),
                        append_whitespace: true,
                    },
                    // TODO: is there a way to create a test?
                    kind: None,
                });

            let results_strings: Vec<String> =
                results.iter().map(|x| x.suggestion.value.clone()).collect();

            for external in results_external {
                if results_strings.contains(&external.suggestion.value) {
                    results.push(SemanticSuggestion {
                        suggestion: Suggestion {
                            value: format!("^{}", external.suggestion.value),
                            description: None,
                            style: None,
                            extra: None,
                            span: external.suggestion.span,
                            append_whitespace: true,
                        },
                        kind: external.kind,
                    })
                } else {
                    results.push(external)
                }
            }

            results
        } else {
            results
        }
    }
}

impl Completer for CommandCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _prefix: Vec<u8>,
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
                options.match_algorithm,
            )
        } else {
            vec![]
        };

        if !subcommands.is_empty() {
            return subcommands;
        }

        let config = working_set.get_config();
        let commands = if matches!(self.flat_shape, nu_parser::FlatShape::External)
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
                config.enable_external_completion,
                options.match_algorithm,
            )
        } else {
            vec![]
        };

        subcommands.into_iter().chain(commands).collect::<Vec<_>>()
    }

    fn get_sort_by(&self) -> SortBy {
        SortBy::LevenshteinDistance
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
