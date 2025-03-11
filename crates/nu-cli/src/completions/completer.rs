use crate::completions::{
    AttributableCompletion, AttributeCompletion, CellPathCompletion, CommandCompletion, Completer,
    CompletionOptions, CustomCompletion, DirectoryCompletion, DotNuCompletion, FileCompletion,
    FlagCompletion, OperatorCompletion, VariableCompletion,
};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_engine::eval_block;
use nu_parser::{flatten_expression, parse};
use nu_protocol::{
    ast::{Argument, Block, Expr, Expression, FindMapResult, Traverse},
    debugger::WithoutDebug,
    engine::{Closure, EngineState, Stack, StateWorkingSet},
    PipelineData, Span, Type, Value,
};
use reedline::{Completer as ReedlineCompleter, Suggestion};
use std::{str, sync::Arc};

use super::base::{SemanticSuggestion, SuggestionKind};

/// Used as the function `f` in find_map Traverse
///
/// returns the inner-most pipeline_element of interest
/// i.e. the one that contains given position and needs completion
fn find_pipeline_element_by_position<'a>(
    expr: &'a Expression,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> FindMapResult<&'a Expression> {
    // skip the entire expression if the position is not in it
    if !expr.span.contains(pos) {
        return FindMapResult::Stop;
    }
    let closure = |expr: &'a Expression| find_pipeline_element_by_position(expr, working_set, pos);
    match &expr.expr {
        Expr::Call(call) => call
            .arguments
            .iter()
            .find_map(|arg| arg.expr().and_then(|e| e.find_map(working_set, &closure)))
            // if no inner call/external_call found, then this is the inner-most one
            .or(Some(expr))
            .map(FindMapResult::Found)
            .unwrap_or_default(),
        Expr::ExternalCall(head, arguments) => arguments
            .iter()
            .find_map(|arg| arg.expr().find_map(working_set, &closure))
            .or(head.as_ref().find_map(working_set, &closure))
            .or(Some(expr))
            .map(FindMapResult::Found)
            .unwrap_or_default(),
        // complete the operator
        Expr::BinaryOp(lhs, _, rhs) => lhs
            .find_map(working_set, &closure)
            .or(rhs.find_map(working_set, &closure))
            .or(Some(expr))
            .map(FindMapResult::Found)
            .unwrap_or_default(),
        Expr::FullCellPath(fcp) => fcp
            .head
            .find_map(working_set, &closure)
            .or(Some(expr))
            .map(FindMapResult::Found)
            .unwrap_or_default(),
        Expr::Var(_) => FindMapResult::Found(expr),
        Expr::AttributeBlock(ab) => ab
            .attributes
            .iter()
            .map(|attr| &attr.expr)
            .chain(Some(ab.item.as_ref()))
            .find_map(|expr| expr.find_map(working_set, &closure))
            .or(Some(expr))
            .map(FindMapResult::Found)
            .unwrap_or_default(),
        _ => FindMapResult::Continue,
    }
}

/// Before completion, an additional character `a` is added to the source as a placeholder for correct parsing results.
/// This function helps to strip it
fn strip_placeholder_if_any<'a>(
    working_set: &'a StateWorkingSet,
    span: &Span,
    strip: bool,
) -> (Span, &'a [u8]) {
    let new_span = if strip {
        let new_end = std::cmp::max(span.end - 1, span.start);
        Span::new(span.start, new_end)
    } else {
        span.to_owned()
    };
    let prefix = working_set.get_span_contents(new_span);
    (new_span, prefix)
}

/// Given a span with noise,
/// 1. Call `rsplit` to get the last token
/// 2. Strip the last placeholder from the token
fn strip_placeholder_with_rsplit<'a>(
    working_set: &'a StateWorkingSet,
    span: &Span,
    predicate: impl FnMut(&u8) -> bool,
    strip: bool,
) -> (Span, &'a [u8]) {
    let span_content = working_set.get_span_contents(*span);
    let mut prefix = span_content
        .rsplit(predicate)
        .next()
        .unwrap_or(span_content);
    let start = span.end.saturating_sub(prefix.len());
    if strip && !prefix.is_empty() {
        prefix = &prefix[..prefix.len() - 1];
    }
    let end = start + prefix.len();
    (Span::new(start, end), prefix)
}

#[derive(Clone)]
pub struct NuCompleter {
    engine_state: Arc<EngineState>,
    stack: Stack,
}

/// Common arguments required for Completer
struct Context<'a> {
    working_set: &'a StateWorkingSet<'a>,
    span: Span,
    prefix: &'a [u8],
    offset: usize,
}

impl Context<'_> {
    fn new<'a>(
        working_set: &'a StateWorkingSet,
        span: Span,
        prefix: &'a [u8],
        offset: usize,
    ) -> Context<'a> {
        Context {
            working_set,
            span,
            prefix,
            offset,
        }
    }
}

impl NuCompleter {
    pub fn new(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self {
            engine_state,
            stack: Stack::with_parent(stack).reset_out_dest().collect_value(),
        }
    }

    pub fn fetch_completions_at(&self, line: &str, pos: usize) -> Vec<SemanticSuggestion> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let offset = working_set.next_span_start();
        // TODO: Callers should be trimming the line themselves
        let line = if line.len() > pos { &line[..pos] } else { line };
        let block = parse(
            &mut working_set,
            Some("completer"),
            // Add a placeholder `a` to the end
            format!("{}a", line).as_bytes(),
            false,
        );
        self.fetch_completions_by_block(block, &working_set, pos, offset, line, true)
    }

    /// For completion in LSP server.
    /// We don't truncate the contents in order
    /// to complete the definitions after the cursor.
    ///
    /// And we avoid the placeholder to reuse the parsed blocks
    /// cached while handling other LSP requests, e.g. diagnostics
    pub fn fetch_completions_within_file(
        &self,
        filename: &str,
        pos: usize,
        contents: &str,
    ) -> Vec<SemanticSuggestion> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let block = parse(&mut working_set, Some(filename), contents.as_bytes(), false);
        let Some(file_span) = working_set.get_span_for_filename(filename) else {
            return vec![];
        };
        let offset = file_span.start;
        self.fetch_completions_by_block(block.clone(), &working_set, pos, offset, contents, false)
    }

    fn fetch_completions_by_block(
        &self,
        block: Arc<Block>,
        working_set: &StateWorkingSet,
        pos: usize,
        offset: usize,
        contents: &str,
        extra_placeholder: bool,
    ) -> Vec<SemanticSuggestion> {
        // Adjust offset so that the spans of the suggestions will start at the right
        // place even with `only_buffer_difference: true`
        let mut pos_to_search = pos + offset;
        if !extra_placeholder {
            pos_to_search = pos_to_search.saturating_sub(1);
        }
        let Some(element_expression) = block.find_map(working_set, &|expr: &Expression| {
            find_pipeline_element_by_position(expr, working_set, pos_to_search)
        }) else {
            return vec![];
        };

        // text of element_expression
        let start_offset = element_expression.span.start - offset;
        let Some(text) = contents.get(start_offset..pos) else {
            return vec![];
        };
        self.complete_by_expression(
            working_set,
            element_expression,
            offset,
            pos_to_search,
            text,
            extra_placeholder,
        )
    }

    /// Complete given the expression of interest
    /// Usually, the expression is get from `find_pipeline_element_by_position`
    ///
    /// # Arguments
    /// * `offset` - start offset of current working_set span
    /// * `pos` - cursor position, should be > offset
    /// * `prefix_str` - all the text before the cursor, within the `element_expression`
    /// * `strip` - whether to strip the extra placeholder from a span
    fn complete_by_expression(
        &self,
        working_set: &StateWorkingSet,
        element_expression: &Expression,
        offset: usize,
        pos: usize,
        prefix_str: &str,
        strip: bool,
    ) -> Vec<SemanticSuggestion> {
        let mut suggestions: Vec<SemanticSuggestion> = vec![];

        match &element_expression.expr {
            Expr::Var(_) => {
                return self.variable_names_completion_helper(
                    working_set,
                    element_expression.span,
                    offset,
                    strip,
                );
            }
            Expr::FullCellPath(full_cell_path) => {
                // e.g. `$e<tab>` parsed as FullCellPath
                // but `$e.<tab>` without placeholder should be taken as cell_path
                if full_cell_path.tail.is_empty() && !prefix_str.ends_with('.') {
                    return self.variable_names_completion_helper(
                        working_set,
                        element_expression.span,
                        offset,
                        strip,
                    );
                } else {
                    let mut cell_path_completer = CellPathCompletion {
                        full_cell_path,
                        position: if strip { pos - 1 } else { pos },
                    };
                    let ctx = Context::new(working_set, Span::unknown(), &[], offset);
                    return self.process_completion(&mut cell_path_completer, &ctx);
                }
            }
            Expr::BinaryOp(lhs, op, _) => {
                if op.span.contains(pos) {
                    let mut operator_completions = OperatorCompletion {
                        left_hand_side: lhs.as_ref(),
                    };
                    let (new_span, prefix) = strip_placeholder_if_any(working_set, &op.span, strip);
                    let ctx = Context::new(working_set, new_span, prefix, offset);
                    let results = self.process_completion(&mut operator_completions, &ctx);
                    if !results.is_empty() {
                        return results;
                    }
                }
            }
            Expr::AttributeBlock(ab) => {
                if let Some(span) = ab.attributes.iter().find_map(|attr| {
                    let span = attr.expr.span;
                    span.contains(pos).then_some(span)
                }) {
                    let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
                    let ctx = Context::new(working_set, new_span, prefix, offset);
                    return self.process_completion(&mut AttributeCompletion, &ctx);
                };
                let span = ab.item.span;
                if span.contains(pos) {
                    let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
                    let ctx = Context::new(working_set, new_span, prefix, offset);
                    return self.process_completion(&mut AttributableCompletion, &ctx);
                }
            }

            // NOTE: user defined internal commands can have any length
            // e.g. `def "foo -f --ff bar"`, complete by line text
            // instead of relying on the parsing result in that case
            Expr::Call(_) | Expr::ExternalCall(_, _) => {
                let need_externals = !prefix_str.contains(' ');
                let need_internals = !prefix_str.starts_with('^');
                let mut span = element_expression.span;
                if !need_internals {
                    span.start += 1;
                };
                suggestions.extend(self.command_completion_helper(
                    working_set,
                    span,
                    offset,
                    need_internals,
                    need_externals,
                    strip,
                ))
            }
            _ => (),
        }

        // unfinished argument completion for commands
        match &element_expression.expr {
            Expr::Call(call) => {
                // NOTE: the argument to complete is not necessarily the last one
                // for lsp completion, we don't trim the text,
                // so that `def`s after pos can be completed
                for arg in call.arguments.iter() {
                    let span = arg.span();
                    if span.contains(pos) {
                        // if customized completion specified, it has highest priority
                        if let Some(decl_id) = arg.expr().and_then(|e| e.custom_completion) {
                            // for `--foo <tab>` and `--foo=<tab>`, the arg span should be trimmed
                            let (new_span, prefix) = if matches!(arg, Argument::Named(_)) {
                                strip_placeholder_with_rsplit(
                                    working_set,
                                    &span,
                                    |b| *b == b'=' || *b == b' ',
                                    strip,
                                )
                            } else {
                                strip_placeholder_if_any(working_set, &span, strip)
                            };
                            let ctx = Context::new(working_set, new_span, prefix, offset);

                            let mut completer = CustomCompletion::new(
                                decl_id,
                                prefix_str.into(),
                                pos - offset,
                                FileCompletion,
                            );

                            suggestions.extend(self.process_completion(&mut completer, &ctx));
                            break;
                        }

                        // normal arguments completion
                        let (new_span, prefix) =
                            strip_placeholder_if_any(working_set, &span, strip);
                        let ctx = Context::new(working_set, new_span, prefix, offset);
                        let flag_completion_helper = || {
                            let mut flag_completions = FlagCompletion {
                                decl_id: call.decl_id,
                            };
                            self.process_completion(&mut flag_completions, &ctx)
                        };
                        suggestions.extend(match arg {
                            // flags
                            Argument::Named(_) | Argument::Unknown(_)
                                if prefix.starts_with(b"-") =>
                            {
                                flag_completion_helper()
                            }
                            // only when `strip` == false
                            Argument::Positional(_) if prefix == b"-" => flag_completion_helper(),
                            // complete according to expression type and command head
                            Argument::Positional(expr) => {
                                let command_head = working_set.get_span_contents(call.head);
                                self.argument_completion_helper(
                                    command_head,
                                    expr,
                                    &ctx,
                                    suggestions.is_empty(),
                                )
                            }
                            _ => vec![],
                        });
                        break;
                    }
                }
            }
            Expr::ExternalCall(head, arguments) => {
                for (i, arg) in arguments.iter().enumerate() {
                    let span = arg.expr().span;
                    if span.contains(pos) {
                        // e.g. `sudo l<tab>`
                        // HACK: judge by index 0 is not accurate
                        if i == 0 {
                            let external_cmd = working_set.get_span_contents(head.span);
                            if external_cmd == b"sudo" || external_cmd == b"doas" {
                                let commands = self.command_completion_helper(
                                    working_set,
                                    span,
                                    offset,
                                    true,
                                    true,
                                    strip,
                                );
                                // flags of sudo/doas can still be completed by external completer
                                if !commands.is_empty() {
                                    return commands;
                                }
                            }
                        }
                        // resort to external completer set in config
                        let config = self.engine_state.get_config();
                        if let Some(closure) = config.completions.external.completer.as_ref() {
                            let mut text_spans: Vec<String> =
                                flatten_expression(working_set, element_expression)
                                    .iter()
                                    .map(|(span, _)| {
                                        let bytes = working_set.get_span_contents(*span);
                                        String::from_utf8_lossy(bytes).to_string()
                                    })
                                    .collect();
                            let mut new_span = span;
                            // strip the placeholder
                            if strip {
                                if let Some(last) = text_spans.last_mut() {
                                    last.pop();
                                    new_span = Span::new(span.start, span.end.saturating_sub(1));
                                }
                            }
                            if let Some(external_result) =
                                self.external_completion(closure, &text_spans, offset, new_span)
                            {
                                suggestions.extend(external_result);
                                return suggestions;
                            }
                        }
                        break;
                    }
                }
            }
            _ => (),
        }

        // if no suggestions yet, fallback to file completion
        if suggestions.is_empty() {
            let (new_span, prefix) = strip_placeholder_with_rsplit(
                working_set,
                &element_expression.span,
                |c| *c == b' ',
                strip,
            );
            let ctx = Context::new(working_set, new_span, prefix, offset);
            suggestions.extend(self.process_completion(&mut FileCompletion, &ctx));
        }
        suggestions
    }

    fn variable_names_completion_helper(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
        strip: bool,
    ) -> Vec<SemanticSuggestion> {
        let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
        if !prefix.starts_with(b"$") {
            return vec![];
        }
        let ctx = Context::new(working_set, new_span, prefix, offset);
        self.process_completion(&mut VariableCompletion, &ctx)
    }

    fn command_completion_helper(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
        internals: bool,
        externals: bool,
        strip: bool,
    ) -> Vec<SemanticSuggestion> {
        let mut command_completions = CommandCompletion {
            internals,
            externals,
        };
        let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
        let ctx = Context::new(working_set, new_span, prefix, offset);
        self.process_completion(&mut command_completions, &ctx)
    }

    fn argument_completion_helper(
        &self,
        command_head: &[u8],
        expr: &Expression,
        ctx: &Context,
        need_fallback: bool,
    ) -> Vec<SemanticSuggestion> {
        // special commands
        match command_head {
            // complete module file/directory
            // TODO: if module file already specified,
            // should parse it to get modules/commands/consts to complete
            b"use" | b"export use" | b"overlay use" | b"source-env" => {
                return self.process_completion(&mut DotNuCompletion, ctx);
            }
            b"which" => {
                let mut completer = CommandCompletion {
                    internals: true,
                    externals: true,
                };
                return self.process_completion(&mut completer, ctx);
            }
            _ => (),
        }

        // general positional arguments
        let file_completion_helper = || self.process_completion(&mut FileCompletion, ctx);
        match &expr.expr {
            Expr::Directory(_, _) => self.process_completion(&mut DirectoryCompletion, ctx),
            Expr::Filepath(_, _) | Expr::GlobPattern(_, _) => file_completion_helper(),
            // fallback to file completion if necessary
            _ if need_fallback => file_completion_helper(),
            _ => vec![],
        }
    }

    // Process the completion for a given completer
    fn process_completion<T: Completer>(
        &self,
        completer: &mut T,
        ctx: &Context,
    ) -> Vec<SemanticSuggestion> {
        let config = self.engine_state.get_config();

        let options = CompletionOptions {
            case_sensitive: config.completions.case_sensitive,
            match_algorithm: config.completions.algorithm.into(),
            sort: config.completions.sort,
            ..Default::default()
        };

        completer.fetch(
            ctx.working_set,
            &self.stack,
            String::from_utf8_lossy(ctx.prefix),
            ctx.span,
            ctx.offset,
            &options,
        )
    }

    fn external_completion(
        &self,
        closure: &Closure,
        spans: &[String],
        offset: usize,
        span: Span,
    ) -> Option<Vec<SemanticSuggestion>> {
        let block = self.engine_state.get_block(closure.block_id);
        let mut callee_stack = self
            .stack
            .captures_to_stack_preserve_out_dest(closure.captures.clone());

        // Line
        if let Some(pos_arg) = block.signature.required_positional.first() {
            if let Some(var_id) = pos_arg.var_id {
                callee_stack.add_var(
                    var_id,
                    Value::list(
                        spans
                            .iter()
                            .map(|it| Value::string(it, Span::unknown()))
                            .collect(),
                        Span::unknown(),
                    ),
                );
            }
        }

        let result = eval_block::<WithoutDebug>(
            &self.engine_state,
            &mut callee_stack,
            block,
            PipelineData::empty(),
        );

        match result.and_then(|data| data.into_value(span)) {
            Ok(Value::List { vals, .. }) => {
                let result =
                    map_value_completions(vals.iter(), Span::new(span.start, span.end), offset);
                Some(result)
            }
            Ok(Value::Nothing { .. }) => None,
            Ok(value) => {
                log::error!(
                    "External completer returned invalid value of type {}",
                    value.get_type().to_string()
                );
                Some(vec![])
            }
            Err(err) => {
                log::error!("failed to eval completer block: {err}");
                Some(vec![])
            }
        }
    }
}

impl ReedlineCompleter for NuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        self.fetch_completions_at(line, pos)
            .into_iter()
            .map(|s| s.suggestion)
            .collect()
    }
}

pub fn map_value_completions<'a>(
    list: impl Iterator<Item = &'a Value>,
    span: Span,
    offset: usize,
) -> Vec<SemanticSuggestion> {
    list.filter_map(move |x| {
        // Match for string values
        if let Ok(s) = x.coerce_string() {
            return Some(SemanticSuggestion {
                suggestion: Suggestion {
                    value: s,
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
                value: String::from(""), // Initialize with empty string
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
                        // Convert the value to string
                        if let Ok(val_str) = value.coerce_string() {
                            // Update the suggestion value
                            suggestion.value = val_str;
                        }
                    }
                    "description" => {
                        // Convert the value to string
                        if let Ok(desc_str) = value.coerce_string() {
                            // Update the suggestion value
                            suggestion.description = Some(desc_str);
                        }
                    }
                    "style" => {
                        // Convert the value to string
                        suggestion.style = match value {
                            Value::String { val, .. } => Some(lookup_ansi_color_style(val)),
                            Value::Record { .. } => Some(color_record_to_nustyle(value)),
                            _ => None,
                        };
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

#[cfg(test)]
mod completer_tests {
    use super::*;

    #[test]
    fn test_completion_helper() {
        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());

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

        let completer = NuCompleter::new(engine_state.into(), Arc::new(Stack::new()));
        let dataset = [
            ("1 bit-sh", true, "b", vec!["bit-shl", "bit-shr"]),
            ("1.0 bit-sh", false, "b", vec![]),
            ("1 m", true, "m", vec!["mod"]),
            ("1.0 m", true, "m", vec!["mod"]),
            ("\"a\" s", true, "s", vec!["starts-with"]),
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
            let result = completer.fetch_completions_at(line, line.len());
            // Test whether the result is empty or not
            assert_eq!(!result.is_empty(), has_result, "line: {}", line);

            // Test whether the result begins with the expected value
            result
                .iter()
                .for_each(|x| assert!(x.suggestion.value.starts_with(begins_with)));

            // Test whether the result contains all the expected values
            assert_eq!(
                result
                    .iter()
                    .map(|x| expected_values.contains(&x.suggestion.value.as_str()))
                    .filter(|x| *x)
                    .count(),
                expected_values.len(),
                "line: {}",
                line
            );
        }
    }
}
