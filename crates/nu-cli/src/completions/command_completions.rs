use crate::completions::{
    file_completions::file_path_completion, Completer, CompletionOptions, SortBy,
};
use nu_parser::{trim_quotes, FlatShape};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span,
};
use reedline::Suggestion;
use std::sync::Arc;

pub struct CommandCompletion {
    engine_state: Arc<EngineState>,
    flattened: Vec<(Span, FlatShape)>,
    flat_idx: usize,
    flat_shape: FlatShape,
}

impl CommandCompletion {
    pub fn new(
        engine_state: Arc<EngineState>,
        _: &StateWorkingSet,
        flattened: Vec<(Span, FlatShape)>,
        flat_idx: usize,
        flat_shape: FlatShape,
    ) -> Self {
        Self {
            engine_state,
            flattened,
            flat_idx,
            flat_shape,
        }
    }

    fn external_command_completion(&self, prefix: &str) -> Vec<String> {
        let mut executables = vec![];

        let paths = self.engine_state.env_vars.get("PATH");

        if let Some(paths) = paths {
            if let Ok(paths) = paths.as_list() {
                for path in paths {
                    let path = path.as_string().unwrap_or_default();

                    if let Ok(mut contents) = std::fs::read_dir(path) {
                        while let Some(Ok(item)) = contents.next() {
                            if !executables.contains(
                                &item
                                    .path()
                                    .file_name()
                                    .map(|x| x.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                            ) && matches!(
                                item.path()
                                    .file_name()
                                    .map(|x| x.to_string_lossy().starts_with(prefix)),
                                Some(true)
                            ) && is_executable::is_executable(&item.path())
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
    ) -> Vec<Suggestion> {
        let prefix = working_set.get_span_contents(span);

        let results = working_set
            .find_commands_by_prefix(prefix)
            .into_iter()
            .map(move |x| Suggestion {
                value: String::from_utf8_lossy(&x.0).to_string(),
                description: x.1,
                extra: None,
                span: reedline::Span {
                    start: span.start - offset,
                    end: span.end - offset,
                },
            });

        let results_aliases =
            working_set
                .find_aliases_by_prefix(prefix)
                .into_iter()
                .map(move |x| Suggestion {
                    value: String::from_utf8_lossy(&x).to_string(),
                    description: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                });

        let mut results = results.chain(results_aliases).collect::<Vec<_>>();

        let prefix = working_set.get_span_contents(span);
        let prefix = String::from_utf8_lossy(prefix).to_string();
        let results = if find_externals {
            let results_external =
                self.external_command_completion(&prefix)
                    .into_iter()
                    .map(move |x| Suggestion {
                        value: x,
                        description: None,
                        extra: None,
                        span: reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                    });

            for external in results_external {
                if results.contains(&external) {
                    results.push(Suggestion {
                        value: format!("^{}", external.value),
                        description: None,
                        extra: None,
                        span: external.span,
                    })
                } else {
                    results.push(external)
                }
            }

            results
        } else {
            results
        };

        results
    }
}

impl Completer for CommandCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        pos: usize,
    ) -> (Vec<Suggestion>, CompletionOptions) {
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

        // Options
        let options = CompletionOptions::new(true, true, SortBy::LevenshteinDistance);

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
            )
        } else {
            vec![]
        };

        if !subcommands.is_empty() {
            return (subcommands, options);
        }

        let commands = if matches!(self.flat_shape, nu_parser::FlatShape::External)
            || matches!(self.flat_shape, nu_parser::FlatShape::InternalCall)
            || ((span.end - span.start) == 0)
        {
            // we're in a gap or at a command
            self.complete_commands(working_set, span, offset, true)
        } else {
            vec![]
        };

        let cwd = if let Some(d) = self.engine_state.env_vars.get("PWD") {
            match d.as_string() {
                Ok(s) => s,
                Err(_) => "".to_string(),
            }
        } else {
            "".to_string()
        };

        let preceding_byte = if span.start > offset {
            working_set
                .get_span_contents(Span {
                    start: span.start - 1,
                    end: span.start,
                })
                .to_vec()
        } else {
            vec![]
        };
        // let prefix = working_set.get_span_contents(flat.0);
        let prefix = String::from_utf8_lossy(&prefix).to_string();
        let output = file_path_completion(span, &prefix, &cwd)
            .into_iter()
            .map(move |x| {
                if self.flat_idx == 0 {
                    // We're in the command position
                    if x.1.starts_with('"') && !matches!(preceding_byte.get(0), Some(b'^')) {
                        let trimmed = trim_quotes(x.1.as_bytes());
                        let trimmed = String::from_utf8_lossy(trimmed).to_string();
                        let expanded = nu_path::canonicalize_with(trimmed, &cwd);

                        if let Ok(expanded) = expanded {
                            if is_executable::is_executable(expanded) {
                                (x.0, format!("^{}", x.1))
                            } else {
                                (x.0, x.1)
                            }
                        } else {
                            (x.0, x.1)
                        }
                    } else {
                        (x.0, x.1)
                    }
                } else {
                    (x.0, x.1)
                }
            })
            .map(move |x| Suggestion {
                value: x.1,
                description: None,
                extra: None,
                span: reedline::Span {
                    start: x.0.start - offset,
                    end: x.0.end - offset,
                },
            })
            .chain(subcommands.into_iter())
            .chain(commands.into_iter())
            .collect::<Vec<_>>();

        (output, options)
    }

    fn get_sort_by(&self) -> SortBy {
        SortBy::LevenshteinDistance
    }
}
