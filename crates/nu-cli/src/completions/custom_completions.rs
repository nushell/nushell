use crate::completions::{
    Completer, CompletionOptions, MatchAlgorithm, SemanticSuggestion,
    completer::map_value_completions,
};
use nu_engine::eval_call;
use nu_parser::{FlatShape, flatten_expression};
use nu_protocol::{
    BlockId, DeclId, IntoSpanned, PipelineData, ShellError, Span, Spanned, Type, Value, VarId,
    ast::{Argument, Call, Expr, Expression},
    debugger::WithoutDebug,
    engine::{Closure, EngineState, Stack, StateWorkingSet},
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

pub fn get_command_arguments(
    working_set: &StateWorkingSet<'_>,
    element_expression: &Expression,
) -> Spanned<Vec<Spanned<String>>> {
    let span = element_expression.span(&working_set);
    flatten_expression(working_set, element_expression)
        .iter()
        .map(|(span, shape)| {
            let bytes = working_set.get_span_contents(match shape {
                // Use expanded alias span
                FlatShape::External(span) => **span,
                _ => *span,
            });
            String::from_utf8_lossy(bytes)
                .into_owned()
                .into_spanned(*span)
        })
        .collect::<Vec<_>>()
        .into_spanned(span)
}

pub struct CommandWideCompletion<'e> {
    block_id: BlockId,
    captures: Vec<(VarId, Value)>,
    expression: &'e Expression,
    strip: bool,
    pub need_fallback: bool,
}

impl<'a> CommandWideCompletion<'a> {
    pub fn command(
        working_set: &StateWorkingSet<'_>,
        decl_id: DeclId,
        expression: &'a Expression,
        strip: bool,
    ) -> Option<Self> {
        let block_id = (decl_id.get() < working_set.num_decls())
            .then(|| working_set.get_decl(decl_id))
            .and_then(|command| command.block_id())?;

        Some(Self {
            block_id,
            captures: vec![],
            expression,
            strip,
            need_fallback: false,
        })
    }

    pub fn closure(closure: &'a Closure, expression: &'a Expression, strip: bool) -> Self {
        Self {
            block_id: closure.block_id,
            captures: closure.captures.clone(),
            expression,
            strip,
            need_fallback: false,
        }
    }
}

impl<'a> Completer for CommandWideCompletion<'a> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        _prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        _options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let Spanned {
            item: mut args,
            span: args_span,
        } = get_command_arguments(working_set, self.expression);

        let mut new_span = span;
        // strip the placeholder
        if self.strip
            && let Some(last) = args.last_mut()
        {
            last.item.pop();
            new_span = Span::new(span.start, span.end.saturating_sub(1));
        }

        let block = working_set.get_block(self.block_id);
        let mut callee_stack = stack.captures_to_stack_preserve_out_dest(self.captures.clone());

        if let Some(pos_arg) = block.signature.required_positional.first()
            && let Some(var_id) = pos_arg.var_id
        {
            callee_stack.add_var(
                var_id,
                Value::list(
                    args.into_iter()
                        .map(|Spanned { item, span }| Value::string(item, span))
                        .collect(),
                    args_span,
                ),
            );
        }
        let mut engine_state = working_set.permanent_state.clone();
        let _ = engine_state.merge_delta(working_set.delta.clone());

        let result = nu_engine::eval_block::<WithoutDebug>(
            &engine_state,
            &mut callee_stack,
            block,
            PipelineData::empty(),
        )
        .map(|p| p.body);

        if let Some(results) = convert_whole_command_completion_results(offset, new_span, result) {
            results
        } else {
            self.need_fallback = true;
            vec![]
        }
    }
}

/// Converts the output of the external completion closure and whole command custom completion
/// commands'
fn convert_whole_command_completion_results(
    offset: usize,
    span: Span,
    result: Result<PipelineData, nu_protocol::ShellError>,
) -> Option<Vec<SemanticSuggestion>> {
    let value = match result.and_then(|pipeline_data| pipeline_data.into_value(span)) {
        Ok(value) => value,
        Err(err) => {
            log::error!(
                "{}",
                ShellError::GenericError {
                    error: "nu::shell::completion".into(),
                    msg: "failed to eval completer block".into(),
                    span: None,
                    help: None,
                    inner: vec![err],
                }
            );
            return Some(vec![]);
        }
    };

    match value {
        Value::List { vals, .. } => Some(map_value_completions(
            vals.iter(),
            Span::new(span.start, span.end),
            offset,
        )),
        Value::Nothing { .. } => None,
        _ => {
            log::error!(
                "{}",
                ShellError::GenericError {
                    error: "nu::shell::completion".into(),
                    msg: "completer returned invalid value of type".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                },
            );
            Some(vec![])
        }
    }
}
