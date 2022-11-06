use crate::completions::{Completer, CompletionOptions, MatchAlgorithm, SortBy};
use nu_parser::FlatShape;
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span,
};
use reedline::Suggestion;
use std::sync::Arc;

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

        let paths = self.engine_state.get_env_var("PATH");

        if let Some(paths) = paths {
            if let Ok(paths) = paths.as_list() {
                for path in paths {
                    let path = path.as_string().unwrap_or_default();

                    if let Ok(mut contents) = std::fs::read_dir(path) {
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
    ) -> Vec<Suggestion> {
        let partial = working_set.get_span_contents(span);

        let filter_predicate = |command: &[u8]| match_algorithm.matches_u8(command, partial);

        let results = working_set
            .find_commands_by_predicate(filter_predicate)
            .into_iter()
            .map(move |x| Suggestion {
                value: String::from_utf8_lossy(&x.0).to_string(),
                description: x.1,
                extra: None,
                span: reedline::Span {
                    start: span.start - offset,
                    end: span.end - offset,
                },
                append_whitespace: true,
            });

        let results_aliases = working_set
            .find_aliases_by_predicate(filter_predicate)
            .into_iter()
            .map(move |x| Suggestion {
                value: String::from_utf8_lossy(&x).to_string(),
                description: None,
                extra: None,
                span: reedline::Span {
                    start: span.start - offset,
                    end: span.end - offset,
                },
                append_whitespace: true,
            });

        let mut results = results.chain(results_aliases).collect::<Vec<_>>();

        let partial = working_set.get_span_contents(span);
        let partial = String::from_utf8_lossy(partial).to_string();

        if find_externals {
            let results_external = self
                .external_command_completion(&partial, match_algorithm)
                .into_iter()
                .map(move |x| Suggestion {
                    value: x,
                    description: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    append_whitespace: true,
                });

            for external in results_external {
                if results.contains(&external) {
                    results.push(Suggestion {
                        value: format!("^{}", external.value),
                        description: None,
                        extra: None,
                        span: external.span,
                        append_whitespace: true,
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
    ) -> Vec<Suggestion> {
        let last = self
            .flattened
            .iter()
            .rev()
            .skip_while(|x| x.0.end > pos)
            .take_while(|x| {
                matches!(
                    x.1,
                    FlatShape::InternalCall
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
                Span {
                    start: last.0.start,
                    end: pos,
                },
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
            || matches!(self.flat_shape, nu_parser::FlatShape::InternalCall)
            || ((span.end - span.start) == 0)
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

        subcommands
            .into_iter()
            .chain(commands.into_iter())
            .collect::<Vec<_>>()
    }

    fn get_sort_by(&self) -> SortBy {
        SortBy::LevenshteinDistance
    }
}
