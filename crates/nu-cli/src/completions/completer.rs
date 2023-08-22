use crate::completions::{
    CommandCompletion, Completer, CompletionOptions, CustomCompletion, DirectoryCompletion,
    DotNuCompletion, FileCompletion, FlagCompletion, MatchAlgorithm, VariableCompletion,
};
use nu_engine::eval_block;
use nu_parser::{flatten_expression, parse, FlatShape};
use nu_protocol::{
    ast::PipelineElement,
    engine::{EngineState, Stack, StateWorkingSet},
    BlockId, PipelineData, Span, Value,
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

    fn external_completion(
        &self,
        block_id: BlockId,
        spans: &[String],
        offset: usize,
        span: Span,
    ) -> Option<Vec<Suggestion>> {
        let stack = self.stack.clone();
        let block = self.engine_state.get_block(block_id);
        let mut callee_stack = stack.gather_captures(&block.captures);

        // Line
        if let Some(pos_arg) = block.signature.required_positional.get(0) {
            if let Some(var_id) = pos_arg.var_id {
                callee_stack.add_var(
                    var_id,
                    Value::List {
                        vals: spans
                            .iter()
                            .map(|it| Value::string(it, Span::unknown()))
                            .collect(),
                        span: Span::unknown(),
                    },
                );
            }
        }

        let result = eval_block(
            &self.engine_state,
            &mut callee_stack,
            block,
            PipelineData::empty(),
            true,
            true,
        );

        match result {
            Ok(pd) => {
                let value = pd.into_value(span);
                if let Value::List { vals, span: _ } = value {
                    let result =
                        map_value_completions(vals.iter(), Span::new(span.start, span.end), offset);

                    return Some(result);
                }
            }
            Err(err) => println!("failed to eval completer block: {err}"),
        }

        None
    }

    fn completion_helper(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let offset = working_set.next_span_start();
        let initial_line = line.to_string();
        let mut line = line.to_string();
        line.insert(pos, 'a');
        let pos = offset + pos;
        let config = self.engine_state.get_config();

        let output = parse(&mut working_set, Some("completer"), line.as_bytes(), false);

        for pipeline in output.pipelines.into_iter() {
            for pipeline_element in pipeline.elements {
                match pipeline_element {
                    PipelineElement::Expression(_, expr)
                    | PipelineElement::Redirection(_, _, expr)
                    | PipelineElement::And(_, expr)
                    | PipelineElement::Or(_, expr)
                    | PipelineElement::SameTargetRedirection { cmd: (_, expr), .. }
                    | PipelineElement::SeparateRedirection { out: (_, expr), .. } => {
                        let flattened: Vec<_> = flatten_expression(&working_set, &expr);
                        let mut spans: Vec<String> = vec![];

                        for (flat_idx, flat) in flattened.iter().enumerate() {
                            let is_passthrough_command = spans
                                .first()
                                .filter(|content| *content == &String::from("sudo"))
                                .is_some();
                            // Read the current spam to string
                            let current_span = working_set.get_span_contents(flat.0).to_vec();
                            let current_span_str = String::from_utf8_lossy(&current_span);

                            // Skip the last 'a' as span item
                            if flat_idx == flattened.len() - 1 {
                                let mut chars = current_span_str.chars();
                                chars.next_back();
                                let current_span_str = chars.as_str().to_owned();
                                spans.push(current_span_str.to_string());
                            } else {
                                spans.push(current_span_str.to_string());
                            }

                            // Complete based on the last span
                            if pos >= flat.0.start && pos < flat.0.end {
                                // Context variables
                                let most_left_var =
                                    most_left_variable(flat_idx, &working_set, flattened.clone());

                                // Create a new span
                                let new_span = Span::new(flat.0.start, flat.0.end - 1);

                                // Parses the prefix. Completion should look up to the cursor position, not after.
                                let mut prefix = working_set.get_span_contents(flat.0).to_vec();
                                let index = pos - flat.0.start;
                                prefix.drain(index..);

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
                                        offset,
                                        pos,
                                    );
                                }

                                // Flags completion
                                if prefix.starts_with(b"-") {
                                    // Try to complete flag internally
                                    let mut completer = FlagCompletion::new(expr.clone());
                                    let result = self.process_completion(
                                        &mut completer,
                                        &working_set,
                                        prefix.clone(),
                                        new_span,
                                        offset,
                                        pos,
                                    );

                                    if !result.is_empty() {
                                        return result;
                                    }

                                    // We got no results for internal completion
                                    // now we can check if external completer is set and use it
                                    if let Some(block_id) = config.external_completer {
                                        if let Some(external_result) = self
                                            .external_completion(block_id, &spans, offset, new_span)
                                        {
                                            return external_result;
                                        }
                                    }
                                }

                                // specially check if it is currently empty - always complete commands
                                if (is_passthrough_command && flat_idx == 1)
                                    || (flat_idx == 0
                                        && working_set.get_span_contents(new_span).is_empty())
                                {
                                    let mut completer = CommandCompletion::new(
                                        self.engine_state.clone(),
                                        &working_set,
                                        flattened.clone(),
                                        // flat_idx,
                                        FlatShape::String,
                                        true,
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

                                // Completions that depends on the previous expression (e.g: use, source-env)
                                if (is_passthrough_command && flat_idx > 1) || flat_idx > 0 {
                                    if let Some(previous_expr) = flattened.get(flat_idx - 1) {
                                        // Read the content for the previous expression
                                        let prev_expr_str =
                                            working_set.get_span_contents(previous_expr.0).to_vec();

                                        // Completion for .nu files
                                        if prev_expr_str == b"use" || prev_expr_str == b"source-env"
                                        {
                                            let mut completer =
                                                DotNuCompletion::new(self.engine_state.clone());

                                            return self.process_completion(
                                                &mut completer,
                                                &working_set,
                                                prefix,
                                                new_span,
                                                offset,
                                                pos,
                                            );
                                        } else if prev_expr_str == b"ls" {
                                            let mut completer =
                                                FileCompletion::new(self.engine_state.clone());

                                            return self.process_completion(
                                                &mut completer,
                                                &working_set,
                                                prefix,
                                                new_span,
                                                offset,
                                                pos,
                                            );
                                        }
                                    }
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
                                            offset,
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
                                            offset,
                                            pos,
                                        );
                                    }
                                    FlatShape::Filepath | FlatShape::GlobPattern => {
                                        let mut completer =
                                            FileCompletion::new(self.engine_state.clone());

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
                                            // flat_idx,
                                            flat_shape.clone(),
                                            false,
                                        );

                                        let mut out: Vec<_> = self.process_completion(
                                            &mut completer,
                                            &working_set,
                                            prefix.clone(),
                                            new_span,
                                            offset,
                                            pos,
                                        );

                                        if !out.is_empty() {
                                            return out;
                                        }

                                        // Try to complete using an external completer (if set)
                                        if let Some(block_id) = config.external_completer {
                                            if let Some(external_result) = self.external_completion(
                                                block_id, &spans, offset, new_span,
                                            ) {
                                                return external_result;
                                            }
                                        }

                                        // Check for file completion
                                        let mut completer =
                                            FileCompletion::new(self.engine_state.clone());
                                        out = self.process_completion(
                                            &mut completer,
                                            &working_set,
                                            prefix,
                                            new_span,
                                            offset,
                                            pos,
                                        );

                                        if !out.is_empty() {
                                            return out;
                                        }
                                    }
                                };
                            }
                        }
                    }
                }
            }
        }

        vec![]
    }
}

impl ReedlineCompleter for NuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        self.completion_helper(line, pos)
    }
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
            FlatShape::Variable(_) => {
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

pub fn map_value_completions<'a>(
    list: impl Iterator<Item = &'a Value>,
    span: Span,
    offset: usize,
) -> Vec<Suggestion> {
    list.filter_map(move |x| {
        // Match for string values
        if let Ok(s) = x.as_string() {
            return Some(Suggestion {
                value: s,
                description: None,
                extra: None,
                span: reedline::Span {
                    start: span.start - offset,
                    end: span.end - offset,
                },
                append_whitespace: false,
            });
        }

        // Match for record values
        if let Ok((cols, vals)) = x.as_record() {
            let mut suggestion = Suggestion {
                value: String::from(""), // Initialize with empty string
                description: None,
                extra: None,
                span: reedline::Span {
                    start: span.start - offset,
                    end: span.end - offset,
                },
                append_whitespace: false,
            };

            // Iterate the cols looking for `value` and `description`
            cols.iter().zip(vals).for_each(|it| {
                // Match `value` column
                if it.0 == "value" {
                    // Convert the value to string
                    if let Ok(val_str) = it.1.as_string() {
                        // Update the suggestion value
                        suggestion.value = val_str;
                    }
                }

                // Match `description` column
                if it.0 == "description" {
                    // Convert the value to string
                    if let Ok(desc_str) = it.1.as_string() {
                        // Update the suggestion value
                        suggestion.description = Some(desc_str);
                    }
                }
            });

            return Some(suggestion);
        }

        None
    })
    .collect()
}

#[cfg(test)]
mod completer_tests {
    use super::*;

    #[test]
    fn test_completion_helper() {
        let mut engine_state = nu_command::create_default_context();

        // Custom additions
        let delta = {
            let working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
            working_set.render()
        };

        let result = engine_state.merge_delta(delta);
        assert!(
            result.is_ok(),
            "Error merging delta: {:?}",
            result.err().unwrap()
        );

        let mut completer = NuCompleter::new(engine_state.into(), Stack::new());
        let dataset = vec![
            ("sudo", false, "", Vec::new()),
            ("sudo l", true, "l", vec!["ls", "let", "lines", "loop"]),
            (" sudo", false, "", Vec::new()),
            (" sudo le", true, "le", vec!["let", "length"]),
            (
                "ls | c",
                true,
                "c",
                vec!["cd", "config", "const", "cp", "cal"],
            ),
            ("ls | sudo m", true, "m", vec!["mv", "mut", "move"]),
        ];
        for (line, has_result, begins_with, expected_values) in dataset {
            let result = completer.completion_helper(line, line.len());
            // Test whether the result is empty or not
            assert_eq!(!result.is_empty(), has_result, "line: {}", line);

            // Test whether the result begins with the expected value
            result
                .iter()
                .for_each(|x| assert!(x.value.starts_with(begins_with)));

            // Test whether the result contains all the expected values
            assert_eq!(
                result
                    .iter()
                    .map(|x| expected_values.contains(&x.value.as_str()))
                    .filter(|x| *x)
                    .count(),
                expected_values.len(),
                "line: {}",
                line
            );
        }
    }
}
