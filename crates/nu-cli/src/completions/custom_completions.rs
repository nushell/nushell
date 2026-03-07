use crate::completions::{Completer, CompletionOptions, MatchAlgorithm, SemanticSuggestion};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_engine::{compile, eval_call};
use nu_parser::flatten_expression;
use nu_protocol::{
    BlockId, DeclId, GetSpan, IntoSpanned, PipelineData, Record, ShellError, Span, Spanned,
    SuggestionKind, Type, Value, VarId,
    ast::{Argument, Call, Expr, Expression},
    debugger::WithoutDebug,
    engine::{Closure, EngineState, Stack, StateWorkingSet},
};
use nu_utils::{SharedCow, strip_ansi_string_unlikely};
use reedline::Suggestion;
use std::{collections::HashMap, sync::Arc};

use super::completion_options::NuMatcher;

fn map_value_completions<'a>(
    list: impl Iterator<Item = &'a Value>,
    span: Span,
    input_start: usize,
    offset: usize,
) -> Vec<SemanticSuggestion> {
    list.filter_map(move |x| {
        // Match for string values
        if let Ok(s) = x.coerce_string() {
            return Some(SemanticSuggestion {
                suggestion: Suggestion {
                    value: strip_ansi_string_unlikely(s),
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Value(x.get_type())),
            });
        }

        // Match for record values
        if let Ok(record) = x.as_record() {
            let mut suggestion = Suggestion {
                value: String::from(""),
                span: reedline::Span {
                    start: span.start - offset,
                    end: span.end - offset,
                },
                ..Suggestion::default()
            };
            let mut value_type = Type::String;

            // Iterate the cols looking for `value` and `description`
            record.iter().for_each(|(key, value)| {
                match key.as_str() {
                    "value" => {
                        value_type = value.get_type();
                        if let Ok(val_str) = value.coerce_string() {
                            suggestion.value = strip_ansi_string_unlikely(val_str);
                        }
                    }
                    "display_override" => {
                        if let Ok(display_str) = value.coerce_string() {
                            suggestion.display_override = Some(display_str);
                        }
                    }
                    "description" => {
                        if let Ok(desc_str) = value.coerce_string() {
                            suggestion.description = Some(desc_str);
                        }
                    }
                    "style" => {
                        suggestion.style = match value {
                            Value::String { val, .. } => Some(lookup_ansi_color_style(val)),
                            Value::Record { .. } => Some(color_record_to_nustyle(value)),
                            _ => None,
                        };
                    }
                    "span" => {
                        if let Value::Record { val: span_rec, .. } = value {
                            // TODO: error on invalid spans?
                            if let Some(end) = read_span_field(span_rec, "end") {
                                suggestion.span.end = suggestion.span.end.min(end + input_start);
                            }
                            if let Some(start) = read_span_field(span_rec, "start") {
                                suggestion.span.start = start + input_start;
                            }
                            if suggestion.span.start > suggestion.span.end {
                                suggestion.span.start = suggestion.span.end;
                                log::error!(
                                    "Custom span start ({}) is greater than end ({})",
                                    suggestion.span.start,
                                    suggestion.span.end
                                );
                            }
                        }
                    }
                    _ => (),
                }
            });

            return Some(SemanticSuggestion {
                suggestion,
                kind: Some(SuggestionKind::Value(value_type)),
            });
        }

        None
    })
    .collect()
}

fn read_span_field(span: &SharedCow<Record>, field: &str) -> Option<usize> {
    let Ok(val) = span.get(field)?.as_int() else {
        log::error!("Expected span field {field} to be int");
        return None;
    };
    let Ok(val) = usize::try_from(val) else {
        log::error!("Couldn't convert span {field} to usize");
        return None;
    };

    Some(val)
}

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
        let mut should_filter = true;

        // Parse result
        let suggestions = match result.and_then(|data| data.into_value(span)) {
            Ok(value) => match &value {
                Value::Record { val, .. } => {
                    let completions = val
                        .get("completions")
                        .and_then(|val| {
                            val.as_list().ok().map(|it| {
                                map_value_completions(
                                    it.iter(),
                                    span,
                                    self.line_pos - self.line.len(),
                                    offset,
                                )
                            })
                        })
                        .unwrap_or_default();
                    let options = val.get("options");

                    if let Some(Value::Record { val: options, .. }) = &options {
                        if let Some(filter) =
                            options.get("filter").and_then(|val| val.as_bool().ok())
                        {
                            should_filter = filter;
                        }

                        if let Some(sort) = options.get("sort").and_then(|val| val.as_bool().ok()) {
                            should_sort = sort;

                            if should_sort && !should_filter {
                                log::warn!("Sorting won't happen because filtering is disabled.")
                            };
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
                Value::List { vals, .. } => map_value_completions(
                    vals.iter(),
                    span,
                    self.line_pos - self.line.len(),
                    offset,
                ),
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

        if !should_filter {
            return suggestions;
        }

        let mut matcher = NuMatcher::new(prefix.as_ref(), &completion_options, should_sort);

        for sugg in suggestions {
            matcher.add(
                strip_ansi_string_unlikely(sugg.suggestion.display_value().to_string()),
                sugg,
            );
        }
        matcher.suggestion_results()
    }
}

pub fn get_command_arguments(
    working_set: &StateWorkingSet<'_>,
    element_expression: &Expression,
) -> Spanned<Vec<Spanned<String>>> {
    let span = element_expression.span(&working_set);
    flatten_expression(working_set, element_expression)
        .iter()
        .map(|(span, _)| {
            String::from_utf8_lossy(working_set.get_span_contents(*span))
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

        let mut block = working_set.get_block(self.block_id).clone();

        // LSP completion where custom def is parsed but not compiled
        if block.ir_block.is_none()
            && let Ok(ir_block) = compile(working_set, &block)
        {
            let mut new_block = (*block).clone();
            new_block.ir_block = Some(ir_block);
            block = Arc::new(new_block);
        }

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
            &block,
            PipelineData::empty(),
        )
        .map(|p| p.body);

        let command_span = working_set.get_span(self.expression.span_id);
        if let Some(results) =
            convert_whole_command_completion_results(offset, new_span, result, command_span)
        {
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
    command_span: Span,
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
            span,
            command_span.start - offset,
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
