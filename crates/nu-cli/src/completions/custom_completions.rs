use crate::completions::{
    completer::map_value_completions, Completer, CompletionOptions, MatchAlgorithm,
    SemanticSuggestion, SortBy,
};
use nu_engine::eval_call;
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression},
    debugger::WithoutDebug,
    engine::{Stack, StateWorkingSet},
    PipelineData, Span, Type, Value,
};
use std::collections::HashMap;

use super::completion_options::{MatcherOptions, NuMatcher};

pub struct CustomCompletion {
    stack: Stack,
    decl_id: usize,
    line: String,
    sort_by: SortBy,
}

impl CustomCompletion {
    pub fn new(stack: Stack, decl_id: usize, line: String) -> Self {
        Self {
            stack,
            decl_id,
            line,
            sort_by: SortBy::None,
        }
    }
}

impl Completer for CustomCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: Vec<u8>,
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
                    Argument::Positional(Expression {
                        span: Span::unknown(),
                        ty: Type::String,
                        expr: Expr::String(self.line.clone()),
                        custom_completion: None,
                    }),
                    Argument::Positional(Expression {
                        span: Span::unknown(),
                        ty: Type::Int,
                        expr: Expr::Int(line_pos as i64),
                        custom_completion: None,
                    }),
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

                        if should_sort {
                            self.sort_by = SortBy::Ascending;
                        }

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
                        });
                    }

                    completions
                }
                Value::List { vals, .. } => map_value_completions(vals.iter(), span, offset),
                _ => vec![],
            })
            .unwrap_or_default();

        filter(
            &prefix,
            suggestions,
            MatcherOptions {
                completion_options: custom_completion_options.unwrap_or(completion_options.clone()),
                sort_by: self.get_sort_by(),
                match_paths: false,
            },
        )
    }

    fn get_sort_by(&self) -> SortBy {
        self.sort_by
    }
}

fn filter(
    prefix: &[u8],
    items: Vec<SemanticSuggestion>,
    options: MatcherOptions,
) -> Vec<SemanticSuggestion> {
    let mut matcher = NuMatcher::from_u8(prefix, options);

    for it in items {
        matcher.add_str(it.suggestion.value.clone(), it);
    }

    matcher.get_results()
}
