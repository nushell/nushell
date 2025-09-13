use crate::completions::{
    Completer, CompletionOptions, MatchAlgorithm, SemanticSuggestion,
    completer::map_value_completions,
};
use nu_engine::eval_call;
use nu_protocol::{
    DeclId, PipelineData, Span, Type, Value,
    ast::{Argument, Call, Expr, Expression},
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use std::collections::HashMap;

use super::completion_options::NuMatcher;

pub struct CustomCompletion<T: Completer> {
    decl_id: DeclId,
    line: String,
    line_pos: usize,
    fallback: T,
}

impl<T: Completer> CustomCompletion<T> {
    pub fn new(decl_id: DeclId, line: String, line_pos: usize, fallback: T) -> Self {
        Self {
            decl_id,
            line,
            line_pos,
            fallback,
        }
    }
}

impl<T: Completer> Completer for CustomCompletion<T> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        orig_options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        // Call custom declaration
        let mut stack_mut = stack.clone();
        let mut eval = |engine_state: &EngineState| {
            eval_call::<WithoutDebug>(
                engine_state,
                &mut stack_mut,
                &Call {
                    decl_id: self.decl_id,
                    head: span,
                    arguments: vec![
                        Argument::Positional(Expression::new_unknown(
                            Expr::String(self.line.clone()),
                            Span::unknown(),
                            Type::String,
                        )),
                        Argument::Positional(Expression::new_unknown(
                            Expr::Int(self.line_pos as i64),
                            Span::unknown(),
                            Type::Int,
                        )),
                    ],
                    parser_info: HashMap::new(),
                },
                PipelineData::empty(),
            )
        };
        let result = if self.decl_id.get() < working_set.permanent_state.num_decls() {
            eval(working_set.permanent_state)
        } else {
            let mut engine_state = working_set.permanent_state.clone();
            let _ = engine_state.merge_delta(working_set.delta.clone());
            eval(&engine_state)
        };

        let mut completion_options = orig_options.clone();
        let mut should_sort = true;

        // Parse result
        let suggestions = match result.and_then(|data| data.into_value(span)) {
            Ok(value) => match &value {
                Value::Record { val, .. } => {
                    let completions = val
                        .get("completions")
                        .and_then(|val| {
                            val.as_list()
                                .ok()
                                .map(|it| map_value_completions(it.iter(), span, offset))
                        })
                        .unwrap_or_default();
                    let options = val.get("options");

                    if let Some(Value::Record { val: options, .. }) = &options {
                        if let Some(sort) = options.get("sort").and_then(|val| val.as_bool().ok()) {
                            should_sort = sort;
                        }

                        if let Some(case_sensitive) = options
                            .get("case_sensitive")
                            .and_then(|val| val.as_bool().ok())
                        {
                            completion_options.case_sensitive = case_sensitive;
                        }
                        let positional =
                            options.get("positional").and_then(|val| val.as_bool().ok());
                        if positional.is_some() {
                            log::warn!(
                                "Use of the positional option is deprecated. Use the substring match algorithm instead."
                            );
                        }
                        if let Some(algorithm) = options
                            .get("completion_algorithm")
                            .and_then(|option| option.coerce_string().ok())
                            .and_then(|option| option.try_into().ok())
                        {
                            completion_options.match_algorithm = algorithm;
                            if let Some(false) = positional
                                && completion_options.match_algorithm == MatchAlgorithm::Prefix
                            {
                                completion_options.match_algorithm = MatchAlgorithm::Substring
                            }
                        }
                    }

                    completions
                }
                Value::List { vals, .. } => map_value_completions(vals.iter(), span, offset),
                Value::Nothing { .. } => {
                    return self.fallback.fetch(
                        working_set,
                        stack,
                        prefix,
                        span,
                        offset,
                        orig_options,
                    );
                }
                _ => {
                    log::error!(
                        "Custom completer returned invalid value of type {}",
                        value.get_type()
                    );
                    return vec![];
                }
            },
            Err(e) => {
                log::error!("Error getting custom completions: {e}");
                return vec![];
            }
        };

        let mut matcher = NuMatcher::new(prefix, &completion_options);

        if should_sort {
            for sugg in suggestions {
                matcher.add_semantic_suggestion(sugg);
            }
            matcher.results()
        } else {
            suggestions
                .into_iter()
                .filter(|sugg| matcher.matches(&sugg.suggestion.value))
                .collect()
        }
    }
}
