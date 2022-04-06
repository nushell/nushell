use crate::completions::{
    CommandCompletion, Completer, CompletionOptions, CustomCompletion, FileCompletion,
    FlagCompletion, VariableCompletion,
};
use nu_parser::{flatten_expression, parse, FlatShape};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span, Value,
};
use reedline::{Completer as ReedlineCompleter, Span as ReedlineSpan, Suggestion};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct NuCompleter {
    engine_state: Arc<EngineState>,
    stack: Stack,
    config: Option<Value>,
    cached_results: Option<(Vec<Suggestion>, CompletionOptions)>,
    last_fetch: Option<Instant>,
}

impl NuCompleter {
    pub fn new(engine_state: Arc<EngineState>, stack: Stack, config: Option<Value>) -> Self {
        Self {
            engine_state,
            stack,
            config,
            cached_results: None,
            last_fetch: None,
        }
    }

    // Process the completion for a given completer
    fn process_completion<T: Completer>(
        &mut self,
        completer: &mut T,
        working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        new_span: Span,
        offset: usize,
        pos: usize,
    ) -> Vec<Suggestion> {
        // Cleanup the result cache if it's old
        if let Some(instant) = self.last_fetch {
            if instant.elapsed() > Duration::from_millis(1000) {
                self.cached_results = None;
            }
        }

        // Fetch
        let (mut suggestions, options) = match self.cached_results.clone() {
            Some((suggestions, options)) => {
                // Update cached spans
                let suggestions = suggestions
                    .into_iter()
                    .map(|suggestion| Suggestion {
                        value: suggestion.value,
                        description: suggestion.description,
                        extra: suggestion.extra,
                        span: ReedlineSpan {
                            start: new_span.start - offset,
                            end: new_span.end - offset,
                        },
                    })
                    .collect();

                (suggestions, options)
            }
            None => {
                let result = completer.fetch(working_set, prefix.clone(), new_span, offset, pos);

                // Update cache results
                self.cached_results = Some(result.clone());

                result
            }
        };

        // Filter
        suggestions = completer.filter(prefix.clone(), suggestions, options.clone());

        // Sort
        suggestions = completer.sort(suggestions, prefix, options);

        // Update last fetch
        self.last_fetch = Some(Instant::now());

        suggestions
    }

    fn completion_helper(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let engine_state = self.engine_state.clone();
        let mut working_set = StateWorkingSet::new(&engine_state);
        let offset = working_set.next_span_start();
        let mut line = line.to_string();
        line.insert(pos, 'a');
        let pos = offset + pos;
        let (output, _err) = parse(
            &mut working_set,
            Some("completer"),
            line.as_bytes(),
            false,
            &[],
        );

        for pipeline in output.pipelines.into_iter() {
            for expr in pipeline.expressions {
                let flattened: Vec<_> = flatten_expression(&working_set, &expr);

                for (flat_idx, flat) in flattened.iter().enumerate() {
                    if pos >= flat.0.start && pos < flat.0.end {
                        // Create a new span
                        let new_span = Span {
                            start: flat.0.start,
                            end: flat.0.end - 1,
                        };

                        // Parses the prefix
                        let mut prefix = working_set.get_span_contents(flat.0).to_vec();
                        prefix.remove(pos - flat.0.start);

                        // Variables completion
                        if prefix.starts_with(b"$") {
                            let mut completer = VariableCompletion::new(self.engine_state.clone());

                            return self.process_completion(
                                &mut completer,
                                &working_set,
                                prefix,
                                new_span,
                                offset,
                                pos,
                            );
                        }

                        // Flags completion
                        if prefix.starts_with(b"-") {
                            let mut completer = FlagCompletion::new(expr);

                            return self.process_completion(
                                &mut completer,
                                &working_set,
                                prefix,
                                new_span,
                                offset,
                                pos,
                            );
                        }

                        // Match other types
                        match &flat.1 {
                            FlatShape::Custom(decl_id) => {
                                let mut completer = CustomCompletion::new(
                                    self.engine_state.clone(),
                                    self.stack.clone(),
                                    self.config.clone(),
                                    *decl_id,
                                    line,
                                );

                                return self.process_completion(
                                    &mut completer,
                                    &working_set,
                                    prefix,
                                    new_span,
                                    offset,
                                    pos,
                                );
                            }
                            FlatShape::Filepath | FlatShape::GlobPattern => {
                                let mut completer = FileCompletion::new(self.engine_state.clone());

                                return self.process_completion(
                                    &mut completer,
                                    &working_set,
                                    prefix,
                                    new_span,
                                    offset,
                                    pos,
                                );
                            }
                            flat_shape => {
                                let mut completer = CommandCompletion::new(
                                    self.engine_state.clone(),
                                    &working_set,
                                    &flattened,
                                    flat_idx,
                                    flat_shape,
                                );

                                return self.process_completion(
                                    &mut completer,
                                    &working_set,
                                    prefix,
                                    new_span,
                                    offset,
                                    pos,
                                );
                            }
                        };
                    }
                }
            }
        }

        return vec![];
    }
}

impl ReedlineCompleter for NuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        self.completion_helper(line, pos)
    }
}
