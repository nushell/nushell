use crate::completions::{
    Completer, Context, Fetched, MatchAlgorithm, SemanticSuggestion, to_reedline_span,
};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_engine::{compile, eval_call};
use nu_parser::flatten_expression;
use nu_protocol::{
    BlockId, DeclId, GetSpan, IntoSpanned, PipelineData, Record, ShellError, Span, Spanned,
    SuggestionKind, Type, Value, VarId,
    ast::{Argument, Call, Expr, Expression},
    debugger::WithoutDebug,
    engine::{Closure, EngineState, StateWorkingSet},
    shell_error::generic::GenericError,
};
use nu_utils::{SharedCow, strip_ansi_string_unlikely};
use reedline::Suggestion;
use std::{borrow::Cow, collections::HashMap, sync::Arc};

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
                    span: to_reedline_span(span, offset),
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Value(x.get_type())),
            });
        }

        // Match for record values
        if let Ok(record) = x.as_record() {
            let mut suggestion = Suggestion {
                value: String::from(""),
                span: to_reedline_span(span, offset),
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

/// Borrows the permanent engine state when the completer lives in it, avoiding a
/// per-keystroke clone; otherwise clones it and merges the working set delta.
fn engine_state_for_completion<'a>(
    working_set: &'a StateWorkingSet<'_>,
    is_permanent: bool,
) -> Cow<'a, EngineState> {
    if is_permanent {
        Cow::Borrowed(working_set.permanent_state)
    } else {
        let mut engine_state = working_set.permanent_state.clone();
        let _ = engine_state.merge_delta(working_set.delta.clone());
        Cow::Owned(engine_state)
    }
}

pub struct CustomCompletion {
    decl_id: DeclId,
    line: String,
    line_pos: usize,
}

impl CustomCompletion {
    pub fn new(decl_id: DeclId, line: String, line_pos: usize) -> Self {
        Self {
            decl_id,
            line,
            line_pos,
        }
    }
}

impl Completer for CustomCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let span = ctx.span;
        let offset = ctx.offset;
        let orig_options = ctx.options;
        let prefix = ctx.prefix_str();
        // Call custom declaration
        let mut stack_mut = ctx.stack.clone();
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
                            span,
                            Type::String,
                        )),
                        Argument::Positional(Expression::new_unknown(
                            Expr::Int(self.line_pos as i64),
                            span,
                            Type::Int,
                        )),
                    ],
                    parser_info: HashMap::new(),
                },
                PipelineData::empty(),
            )
        };
        let engine_state = engine_state_for_completion(
            working_set,
            self.decl_id.get() < working_set.permanent_state.num_decls(),
        );
        let result = eval(engine_state.as_ref());

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

                        if let Some(md) = options
                            .get("match_description")
                            .and_then(|val| val.as_bool().ok())
                        {
                            completion_options.match_description = md;
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
                Value::Nothing { .. } => return Fetched::Declined,
                _ => {
                    log::error!(
                        "Custom completer returned invalid value of type {}",
                        value.get_type()
                    );
                    return Fetched::Cacheable(vec![]);
                }
            },
            Err(e) => {
                log::error!("Error getting custom completions: {e}");
                return Fetched::Cacheable(vec![]);
            }
        };

        if !should_filter {
            return Fetched::Cacheable(suggestions);
        }

        let mut matcher = NuMatcher::new(prefix.as_ref(), &completion_options, should_sort);

        for sugg in suggestions {
            let value = strip_ansi_string_unlikely(sugg.suggestion.display_value().to_string());
            if matcher.check_match(&value).is_some() {
                matcher.add(value, sugg);
            } else if completion_options.match_description
                && let Some(description) = sugg.suggestion.description.as_deref()
                && matcher.check_match(description).is_some()
            {
                let description = description.to_string();
                matcher.add(description, sugg);
            }
        }
        Fetched::Cacheable(matcher.suggestion_results())
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
}

impl<'a> CommandWideCompletion<'a> {
    pub fn command(
        working_set: &StateWorkingSet<'_>,
        decl_id: DeclId,
        expression: &'a Expression,
    ) -> Option<Self> {
        let block_id = (decl_id.get() < working_set.num_decls())
            .then(|| working_set.get_decl(decl_id))
            .and_then(|command| command.block_id())?;

        Some(Self {
            block_id,
            captures: vec![],
            expression,
        })
    }

    pub fn closure(closure: &'a Closure, expression: &'a Expression) -> Self {
        Self {
            block_id: closure.block_id,
            captures: closure.captures.clone(),
            expression,
        }
    }
}

impl<'a> Completer for CommandWideCompletion<'a> {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let stack = ctx.stack;
        let span = ctx.span;
        let offset = ctx.offset;
        let Spanned {
            mut item,
            span: args_span,
        } = get_command_arguments(working_set, self.expression);
        // An empty span is a trailing argument slot the parser produced no argument for;
        // append an empty entry so the completer sees one `$spans` entry per argument.
        if ctx.span.is_empty() {
            item.push(String::new().into_spanned(ctx.span));
        }
        let args = item;

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
        let engine_state = engine_state_for_completion(
            working_set,
            self.block_id.get() < working_set.permanent_state.num_blocks(),
        );
        let result = nu_engine::eval_block_with_early_return::<WithoutDebug>(
            engine_state.as_ref(),
            &mut callee_stack,
            &block,
            PipelineData::empty(),
        )
        .map(|p| p.body);

        let command_span = working_set.get_span(self.expression.span_id);
        if let Some(results) =
            convert_whole_command_completion_results(offset, span, result, command_span)
        {
            Fetched::Cacheable(results)
        } else {
            Fetched::Declined
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
                ShellError::Generic(
                    GenericError::new_internal(
                        "nu::shell::completion",
                        "failed to eval completer block",
                    )
                    .with_inner([err]),
                )
            );
            // Error does not equal empty success: fall back so file completion
            // still runs when an external completer fails.
            return None;
        }
    };

    match value {
        Value::List { vals, .. } => Some(map_value_completions(
            vals.iter(),
            span,
            command_span.start.saturating_sub(offset),
            offset,
        )),
        Value::Nothing { .. } => None,
        _ => {
            log::error!(
                "{}",
                ShellError::Generic(GenericError::new_internal(
                    "nu::shell::completion",
                    "completer returned invalid value of type",
                )),
            );
            Some(vec![])
        }
    }
}
