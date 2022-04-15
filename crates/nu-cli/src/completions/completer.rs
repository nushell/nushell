use crate::completions::{
    CommandCompletion, Completer, CustomCompletion, FileCompletion, FlagCompletion,
    VariableCompletion,
};
use nu_parser::{flatten_expression, parse, FlatShape};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span, Value,
};
use reedline::{Completer as ReedlineCompleter, Suggestion};
use std::str;
use std::sync::Arc;

#[derive(Clone)]
pub struct NuCompleter {
    engine_state: Arc<EngineState>,
    stack: Stack,
    config: Option<Value>,
}

impl NuCompleter {
    pub fn new(engine_state: Arc<EngineState>, stack: Stack, config: Option<Value>) -> Self {
        Self {
            engine_state,
            stack,
            config,
        }
    }

    // Process the completion for a given completer
    fn process_completion<T: Completer>(
        &self,
        completer: &mut T,
        working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        new_span: Span,
        offset: usize,
        pos: usize,
    ) -> Vec<Suggestion> {
        // Fetch
        let (mut suggestions, options) =
            completer.fetch(working_set, prefix.clone(), new_span, offset, pos);

        // Filter
        suggestions = completer.filter(prefix.clone(), suggestions, options.clone());

        // Sort
        suggestions = completer.sort(suggestions, prefix, options);

        suggestions
    }

    fn completion_helper(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
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
                        // Context variables
                        let mut is_variable_completion = false;
                        let mut previous_expr: Vec<u8> = vec![];

                        // Create a new span
                        let new_span = Span {
                            start: flat.0.start,
                            end: flat.0.end - 1,
                        };

                        // Parses the prefix
                        let mut prefix = working_set.get_span_contents(flat.0).to_vec();
                        prefix.remove(pos - flat.0.start);

                        // Try to get the previous expression
                        if flat_idx > 0 {
                            match flattened.get(flat_idx - 1) {
                                Some(value) => {
                                    let previous_prefix =
                                        working_set.get_span_contents(value.0).to_vec();

                                    // Update the previous expression
                                    previous_expr = previous_prefix;

                                    // Check if should match variable completion
                                    if matches!(value.1, FlatShape::Variable) {
                                        is_variable_completion = true;
                                    }
                                }
                                None => {}
                            }
                        }

                        // Variables completion
                        if prefix.starts_with(b"$") || is_variable_completion {
                            let mut completer = VariableCompletion::new(
                                self.engine_state.clone(),
                                self.stack.clone(),
                                previous_expr,
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
                                    flattened.clone(),
                                    flat_idx,
                                    flat_shape.clone(),
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
