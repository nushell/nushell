use crate::completions::{
    completer::map_value_completions, Completer, CompletionOptions, MatchAlgorithm,
    SemanticSuggestion, SortBy,
};
use nu_engine::eval_call;
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression},
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, Span, Type, Value,
};
use nu_utils::IgnoreCaseExt;
use std::{collections::HashMap, sync::Arc};

pub struct CustomCompletion {
    engine_state: Arc<EngineState>,
    stack: Stack,
    decl_id: usize,
    line: String,
    sort_by: SortBy,
}

impl CustomCompletion {
    pub fn new(engine_state: Arc<EngineState>, stack: Stack, decl_id: usize, line: String) -> Self {
        Self {
            engine_state,
            stack: stack.reset_out_dest().capture(),
            decl_id,
            line,
            sort_by: SortBy::None,
        }
    }
}

impl Completer for CustomCompletion {
    fn fetch(
        &mut self,
        _: &StateWorkingSet,
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
            &self.engine_state,
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
            .map(|pd| {
                let value = pd.into_value(span);
                match &value {
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
                }
            })
            .unwrap_or_default();

        if let Some(custom_completion_options) = custom_completion_options {
            filter(&prefix, suggestions, &custom_completion_options)
        } else {
            filter(&prefix, suggestions, completion_options)
        }
    }

    fn get_sort_by(&self) -> SortBy {
        self.sort_by
    }
}

fn filter(
    prefix: &[u8],
    items: Vec<SemanticSuggestion>,
    options: &CompletionOptions,
) -> Vec<SemanticSuggestion> {
    items
        .into_iter()
        .filter(|it| match options.match_algorithm {
            MatchAlgorithm::Prefix => match (options.case_sensitive, options.positional) {
                (true, true) => it.suggestion.value.as_bytes().starts_with(prefix),
                (true, false) => it
                    .suggestion
                    .value
                    .contains(std::str::from_utf8(prefix).unwrap_or("")),
                (false, positional) => {
                    let value = it.suggestion.value.to_folded_case();
                    let prefix = std::str::from_utf8(prefix).unwrap_or("").to_folded_case();
                    if positional {
                        value.starts_with(&prefix)
                    } else {
                        value.contains(&prefix)
                    }
                }
            },
            MatchAlgorithm::Fuzzy => options
                .match_algorithm
                .matches_u8(it.suggestion.value.as_bytes(), prefix),
        })
        .collect()
}
