use crate::completions::{
    CommandCompletion, Completer, CompletionOptions, CustomCompletion, DirectoryCompletion,
    DotNuCompletion, FileCompletion, FlagCompletion, MatchAlgorithm, VariableCompletion,
};
use nu_parser::{flatten_expression, parse, FlatShape};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span,
};
use reedline::{Completer as ReedlineCompleter, Suggestion};
use std::str;
use std::sync::Arc;

#[derive(Clone)]
pub struct NuCompleter {
    engine_state: Arc<EngineState>,
    stack: Stack,
}

impl NuCompleter {
    pub fn new(engine_state: Arc<EngineState>, stack: Stack) -> Self {
        Self {
            engine_state,
            stack,
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
        let config = self.engine_state.get_config();

        let mut options = CompletionOptions {
            case_sensitive: config.case_sensitive_completions,
            ..Default::default()
        };

        if config.completion_algorithm == "fuzzy" {
            options.match_algorithm = MatchAlgorithm::Fuzzy;
        }

        // Fetch
        let mut suggestions =
            completer.fetch(working_set, prefix.clone(), new_span, offset, pos, &options);

        // Sort
        suggestions = completer.sort(suggestions, prefix);

        suggestions
    }

    fn completion_helper(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let (is_alias, new_line) = find_alias(line.as_bytes(), &working_set);
        let span_offset = working_set.next_span_start();
        let initial_line = line.to_string();
        let mut line = line.to_string();
        let mut alias_offset = 0;
        if is_alias {
            alias_offset = new_line.len() - line.len();
            line = new_line;
        }
        line.insert(pos+alias_offset, 'a');
        let pos = span_offset + pos;
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
                    if pos >= flat.0.start - alias_offset && pos < flat.0.end - alias_offset {
                        // Context variables
                        let most_left_var =
                            most_left_variable(flat_idx, &working_set, flattened.clone());

                        // Create a new span
                        let new_span = Span {
                            start: flat.0.start - alias_offset,
                            end: flat.0.end - 1 - alias_offset,
                        };

                        // Parses the prefix
                        let mut prefix = working_set.get_span_contents(flat.0).to_vec();
                        prefix.remove(pos - (flat.0.start - alias_offset));

                        // Completions that depends on the previous expression (e.g: use, source)
                        if flat_idx > 0 {
                            if let Some(previous_expr) = flattened.get(flat_idx - 1) {
                                // Read the content for the previous expression
                                let prev_expr_str =
                                    working_set.get_span_contents(previous_expr.0).to_vec();

                                // Completion for .nu files
                                if prev_expr_str == b"use" || prev_expr_str == b"source" {
                                    let mut completer =
                                        DotNuCompletion::new(self.engine_state.clone());

                                    return self.process_completion(
                                        &mut completer,
                                        &working_set,
                                        prefix,
                                        new_span,
                                        span_offset,
                                        pos,
                                    );
                                }
                            }
                        }

                        // Variables completion
                        if prefix.starts_with(b"$") || most_left_var.is_some() {
                            let mut completer = VariableCompletion::new(
                                self.engine_state.clone(),
                                self.stack.clone(),
                                most_left_var.unwrap_or((vec![], vec![])),
                            );

                            return self.process_completion(
                                &mut completer,
                                &working_set,
                                prefix,
                                new_span,
                                span_offset,
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
                                span_offset,
                                pos,
                            );
                        }

                        // Match other types
                        match &flat.1 {
                            FlatShape::Custom(decl_id) => {
                                let mut completer = CustomCompletion::new(
                                    self.engine_state.clone(),
                                    self.stack.clone(),
                                    *decl_id,
                                    initial_line,
                                );

                                return self.process_completion(
                                    &mut completer,
                                    &working_set,
                                    prefix,
                                    new_span,
                                    span_offset,
                                    pos,
                                );
                            }
                            FlatShape::Directory => {
                                let mut completer =
                                    DirectoryCompletion::new(self.engine_state.clone());

                                return self.process_completion(
                                    &mut completer,
                                    &working_set,
                                    prefix,
                                    new_span,
                                    span_offset,
                                    pos,
                                );
                            }
                            flat_shape => {
                                let mut completer = CommandCompletion::new(
                                    self.engine_state.clone(),
                                    &working_set,
                                    flattened.clone(),
                                    // flat_idx,
                                    flat_shape.clone(),
                                );

                                let out: Vec<_> = self.process_completion(
                                    &mut completer,
                                    &working_set,
                                    prefix.clone(),
                                    new_span,
                                    span_offset,
                                    pos,
                                );

                                if out.is_empty() {
                                    let mut completer =
                                        FileCompletion::new(self.engine_state.clone());

                                    return self.process_completion(
                                        &mut completer,
                                        &working_set,
                                        prefix,
                                        new_span,
                                        span_offset,
                                        pos,
                                    );
                                }

                                return out;
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

// fn parse_input (){
//             line.insert(pos, 'a');
//             let pos = offset + pos;
//             let (output, _err) = parse(
//                 &mut working_set,
//                 Some("completer"),
//                 line.as_bytes(),
//                 false,
//                 &[],
//             );
// }

fn find_alias(input: &[u8], working_set: &StateWorkingSet) -> (bool, String) {
    let mut names: Vec<_> = vec![];
    let mut vec_alias: Vec<_> = vec![];
    let mut pos = 0;
    let mut count_of_whitespace = 0;
    let mut is_alias = false;
    for (index, character) in input.iter().enumerate() {
        if *character == b' ' {
            let range = &input[pos..index];
            names.push(range);
            count_of_whitespace += 1;
            pos = index + 1;
        }
    }
    for name in names {
        if let Some(alias_id) = working_set.find_alias(name) {
            let alias_span = working_set.get_alias(alias_id);
            is_alias = true;
            for alias in alias_span {
                let name = working_set.get_span_contents(*alias);
                if !name.is_empty() {
                    vec_alias.push(name.to_vec());
                }
            }
            if count_of_whitespace > 0 {
                vec_alias.push(vec![b' ']);
            }
        } else {
            vec_alias.push(name.to_vec());
            if count_of_whitespace > 0 {
                vec_alias.push(vec![b' ']);
            }
        }
    }

    let out: Vec<_> = vec_alias.into_iter().flatten().collect();
    let line = String::from_utf8_lossy(&out).to_string();

    (is_alias, line)
}

// reads the most left variable returning it's name (e.g: $myvar)
// and the depth (a.b.c)
fn most_left_variable(
    idx: usize,
    working_set: &StateWorkingSet<'_>,
    flattened: Vec<(Span, FlatShape)>,
) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
    // Reverse items to read the list backwards and truncate
    // because the only items that matters are the ones before the current index
    let mut rev = flattened;
    rev.truncate(idx);
    rev = rev.into_iter().rev().collect();

    // Store the variables and sub levels found and reverse to correct order
    let mut variables_found: Vec<Vec<u8>> = vec![];
    let mut found_var = false;
    for item in rev.clone() {
        let result = working_set.get_span_contents(item.0).to_vec();

        match item.1 {
            FlatShape::Variable => {
                variables_found.push(result);
                found_var = true;

                break;
            }
            FlatShape::String => {
                variables_found.push(result);
            }
            _ => {
                break;
            }
        }
    }

    // If most left var was not found
    if !found_var {
        return None;
    }

    // Reverse the order back
    variables_found = variables_found.into_iter().rev().collect();

    // Extract the variable and the sublevels
    let var = variables_found.first().unwrap_or(&vec![]).to_vec();
    let sublevels: Vec<Vec<u8>> = variables_found.into_iter().skip(1).collect();

    Some((var, sublevels))
}
