use crate::completions::{
    completer::map_value_completions, Completer, CompletionOptions, MatchAlgorithm,
    SemanticSuggestion,
};
use nu_engine::eval_call;
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression},
    debugger::WithoutDebug,
    engine::{Stack, StateWorkingSet},
    CompletionSort, DeclId, PipelineData, Span, Type, Value,
};
use std::collections::HashMap;

use super::completion_options::NuMatcher;

pub struct CustomCompletion {
    stack: Stack,
    decl_id: DeclId,
    line: String,
}

impl CustomCompletion {
    pub fn new(stack: Stack, decl_id: DeclId, line: String) -> Self {
        Self {
            stack,
            decl_id,
            line,
        }
    }
}

impl Completer for CustomCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: &[u8],
        span: Span,
        offset: usize,
        pos: usize,
        completion_options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        // Line position
        let line_pos = pos - offset;

        // Call custom declaration
        let result = eval_call::<WithoutDebug>(
            working_set.permanent_state,
            &mut self.stack,
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
                        Expr::Int(line_pos as i64),
                        Span::unknown(),
                        Type::Int,
                    )),
                ],
                parser_info: HashMap::new(),
            },
            PipelineData::empty(),
        );

        let mut custom_completion_options = None;

        // Parse result
        let suggestions = result
            .and_then(|data| data.into_value(span))
            .map(|value| match &value {
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
                        let should_sort = options
                            .get("sort")
                            .and_then(|val| val.as_bool().ok())
                            .unwrap_or(false);

                        custom_completion_options = Some(CompletionOptions {
                            case_sensitive: options
                                .get("case_sensitive")
                                .and_then(|val| val.as_bool().ok())
                                .unwrap_or(true),
                            positional: options
                                .get("positional")
                                .and_then(|val| val.as_bool().ok())
                                .unwrap_or(true),
                            match_algorithm: match options.get("completion_algorithm") {
                                Some(option) => option
                                    .coerce_string()
                                    .ok()
                                    .and_then(|option| option.try_into().ok())
                                    .unwrap_or(MatchAlgorithm::Prefix),
                                None => completion_options.match_algorithm,
                            },
                            sort: if should_sort {
                                CompletionSort::Alphabetical
                            } else {
                                CompletionSort::Smart
                            },
                        });
                    }

                    completions
                }
                Value::List { vals, .. } => map_value_completions(vals.iter(), span, offset),
                _ => vec![],
            })
            .unwrap_or_default();

        let options = custom_completion_options.unwrap_or(completion_options.clone());
        let mut matcher = NuMatcher::new(String::from_utf8_lossy(prefix), options);
        for sugg in suggestions {
            matcher.add_semantic_suggestion(sugg);
        }
        matcher.results()
    }
}
