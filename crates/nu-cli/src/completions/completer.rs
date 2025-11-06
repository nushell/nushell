use crate::completions::{
    AttributableCompletion, AttributeCompletion, CellPathCompletion, CommandCompletion, Completer,
    CompletionOptions, CustomCompletion, DirectoryCompletion, DotNuCompletion,
    ExportableCompletion, FileCompletion, FlagCompletion, OperatorCompletion, VariableCompletion,
    base::{SemanticSuggestion, SuggestionKind},
};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_parser::{parse, parse_module_file_or_dir};
use nu_protocol::{
    CommandWideCompleter, Completion, GetSpan, Span, Type, Value,
    ast::{
        Argument, Block, Expr, Expression, FindMapResult, ListItem, PipelineRedirection,
        RedirectionTarget, Traverse,
    },
    engine::{EngineState, Stack, StateWorkingSet},
};
use reedline::{Completer as ReedlineCompleter, Suggestion};
use std::sync::Arc;

use super::{StaticCompletion, custom_completions::CommandWideCompletion};

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
        Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Block(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            // check redirection target for sub blocks before diving recursively into them
            check_redirection_in_block(block.as_ref(), pos)
                .map(FindMapResult::Found)
                .unwrap_or_default()
        }
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
            .or_else(|| {
                // For aliased external_call, the span of original external command head should fail the
                // contains(pos) check, thus avoiding recursion into its head expression.
                // See issue #7648 for details.
                let span = working_set.get_span(head.span_id);
                if span.contains(pos) {
                    // This is for complicated external head expressions, e.g. `^(echo<tab> foo)`
                    head.as_ref().find_map(working_set, &closure)
                } else {
                    None
                }
            })
            .or(Some(expr))
            .map(FindMapResult::Found)
            .unwrap_or_default(),
        // complete the operator
        Expr::BinaryOp(lhs, _, rhs) => lhs
            .find_map(working_set, &closure)
            .or_else(|| rhs.find_map(working_set, &closure))
            .or(Some(expr))
            .map(FindMapResult::Found)
            .unwrap_or_default(),
        Expr::FullCellPath(fcp) => fcp
            .head
            .find_map(working_set, &closure)
            .map(FindMapResult::Found)
            // e.g. use std/util [<tab>
            .or_else(|| {
                (fcp.head.span.contains(pos) && matches!(fcp.head.expr, Expr::List(_)))
                    .then_some(FindMapResult::Continue)
            })
            .or(Some(FindMapResult::Found(expr)))
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

/// Helper function to extract file-path expression from redirection target
fn check_redirection_target(target: &RedirectionTarget, pos: usize) -> Option<&Expression> {
    let expr = target.expr();
    expr.and_then(|expression| {
        if let Expr::String(_) = expression.expr
            && expression.span.contains(pos)
        {
            expr
        } else {
            None
        }
    })
}

/// For redirection target completion
/// https://github.com/nushell/nushell/issues/16827
fn check_redirection_in_block(block: &Block, pos: usize) -> Option<&Expression> {
    block.pipelines.iter().find_map(|pipeline| {
        pipeline.elements.iter().find_map(|element| {
            element.redirection.as_ref().and_then(|redir| match redir {
                PipelineRedirection::Single { target, .. } => check_redirection_target(target, pos),
                PipelineRedirection::Separate { out, err } => check_redirection_target(out, pos)
                    .or_else(|| check_redirection_target(err, pos)),
            })
        })
    })
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

/// For argument completion
struct PositionalArguments<'a> {
    /// command name
    command_head: &'a str,
    /// indices of positional arguments
    positional_arg_indices: Vec<usize>,
    /// argument list
    arguments: &'a [Argument],
    /// expression of current argument
    expr: &'a Expression,
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
            format!("{line}a").as_bytes(),
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
        let Some(element_expression) = block
            .find_map(working_set, &|expr: &Expression| {
                find_pipeline_element_by_position(expr, working_set, pos_to_search)
            })
            .or_else(|| check_redirection_in_block(block.as_ref(), pos_to_search))
        else {
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
                let signature = working_set.get_decl(call.decl_id).signature();
                // NOTE: the argument to complete is not necessarily the last one
                // for lsp completion, we don't trim the text,
                // so that `def`s after pos can be completed
                let mut positional_arg_indices = Vec::new();

                for (arg_idx, arg) in call.arguments.iter().enumerate() {
                    let span = arg.span();

                    if !span.contains(pos) {
                        match arg {
                            Argument::Named(_) => (),
                            _ => positional_arg_indices.push(arg_idx),
                        }
                        continue;
                    }

                    // Get custom completion from PositionalArg or Flag
                    let completion = {
                        // Check PositionalArg or Flag from Signature
                        match arg {
                            // For named arguments, check Flag
                            Argument::Named((name, short, value)) => {
                                if value.as_ref().is_none_or(|e| !e.span.contains(pos)) {
                                    None
                                } else {
                                    // If we're completing the value of the flag,
                                    // search for the matching custom completion decl_id (long or short)
                                    let flag = signature.get_long_flag(&name.item).or_else(|| {
                                        short.as_ref().and_then(|s| {
                                            signature.get_short_flag(
                                                s.item.chars().next().unwrap_or('_'),
                                            )
                                        })
                                    });
                                    flag.and_then(|f| f.completion)
                                }
                            }
                            // For positional arguments, check PositionalArg
                            Argument::Positional(_) => {
                                // Find the right positional argument by index
                                let arg_pos = positional_arg_indices.len();
                                signature
                                    .get_positional(arg_pos)
                                    .and_then(|pos_arg| pos_arg.completion.clone())
                            }
                            _ => None,
                        }
                    };

                    if let Some(completion) = completion {
                        // for `--foo ..a|` and `--foo=..a|` (`|` represents the cursor), the
                        // arg span should be trimmed:
                        // - split the given span with `predicate` (b == '=' || b == ' '), and
                        //   take the rightmost part:
                        //   - "--foo ..a" => ["--foo", "..a"] => "..a"
                        //   - "--foo=..a" => ["--foo", "..a"] => "..a"
                        // - strip placeholder (`a`) if present
                        let (new_span, prefix) = match arg {
                            Argument::Named(_) => strip_placeholder_with_rsplit(
                                working_set,
                                &span,
                                |b| *b == b'=' || *b == b' ',
                                strip,
                            ),
                            _ => strip_placeholder_if_any(working_set, &span, strip),
                        };

                        let ctx = Context::new(working_set, new_span, prefix, offset);

                        match completion {
                            Completion::Command(decl_id) => {
                                let mut completer = CustomCompletion::new(
                                    decl_id,
                                    prefix_str.into(),
                                    pos - offset,
                                    FileCompletion,
                                );
                                // Prioritize argument completions over (sub)commands
                                suggestions
                                    .splice(0..0, self.process_completion(&mut completer, &ctx));
                                break;
                            }
                            Completion::List(list) => {
                                let mut completer = StaticCompletion::new(list);
                                // Prioritize argument completions over (sub)commands
                                suggestions
                                    .splice(0..0, self.process_completion(&mut completer, &ctx));
                                // We don't want to fallback to file completion here
                                return suggestions;
                            }
                        }
                    } else if let Some(command_wide_completer) = signature.complete {
                        let flag_completions = {
                            let (new_span, prefix) =
                                strip_placeholder_if_any(working_set, &span, strip);
                            let ctx = Context::new(working_set, new_span, prefix, offset);
                            let flag_completion_helper = || {
                                let mut flag_completions = FlagCompletion {
                                    decl_id: call.decl_id,
                                };
                                self.process_completion(&mut flag_completions, &ctx)
                            };

                            match arg {
                                // flags
                                Argument::Named(_) | Argument::Unknown(_)
                                    if prefix.starts_with(b"-") =>
                                {
                                    flag_completion_helper()
                                }
                                // only when `strip` == false
                                Argument::Positional(_) if prefix == b"-" => {
                                    flag_completion_helper()
                                }
                                _ => vec![],
                            }
                        };

                        let completion = match command_wide_completer {
                            CommandWideCompleter::Command(decl_id) => {
                                CommandWideCompletion::command(
                                    working_set,
                                    decl_id,
                                    element_expression,
                                    strip,
                                )
                            }
                            CommandWideCompleter::External => self
                                .engine_state
                                .get_config()
                                .completions
                                .external
                                .completer
                                .as_ref()
                                .map(|closure| {
                                    CommandWideCompletion::closure(
                                        closure,
                                        element_expression,
                                        strip,
                                    )
                                }),
                        };

                        if let Some(mut completion) = completion {
                            let ctx = Context::new(working_set, span, b"", offset);
                            let results = self.process_completion(&mut completion, &ctx);

                            // Prioritize flag completions above everything else
                            let flags_length = flag_completions.len();
                            suggestions.splice(0..0, flag_completions);

                            // Prioritize external results over (sub)commands
                            suggestions.splice(flags_length..flags_length, results);

                            if !completion.need_fallback {
                                return suggestions;
                            }
                        }
                    }

                    // normal arguments completion
                    let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
                    let ctx = Context::new(working_set, new_span, prefix, offset);
                    let flag_completion_helper = || {
                        let mut flag_completions = FlagCompletion {
                            decl_id: call.decl_id,
                        };
                        self.process_completion(&mut flag_completions, &ctx)
                    };
                    // Prioritize argument completions over (sub)commands
                    suggestions.splice(
                        0..0,
                        match arg {
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
                                let command_head = working_set.get_decl(call.decl_id).name();
                                positional_arg_indices.push(arg_idx);
                                let mut need_fallback = suggestions.is_empty();
                                let results = self.argument_completion_helper(
                                    PositionalArguments {
                                        command_head,
                                        positional_arg_indices,
                                        arguments: &call.arguments,
                                        expr,
                                    },
                                    pos,
                                    &ctx,
                                    &mut need_fallback,
                                );
                                // for those arguments that don't need any fallback, return early
                                if !need_fallback && suggestions.is_empty() {
                                    return results;
                                }
                                results
                            }
                            _ => vec![],
                        },
                    );
                    break;
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
                        let completion = self
                            .engine_state
                            .get_config()
                            .completions
                            .external
                            .completer
                            .as_ref()
                            .map(|closure| {
                                CommandWideCompletion::closure(closure, element_expression, strip)
                            });

                        if let Some(mut completion) = completion {
                            let ctx = Context::new(working_set, span, b"", offset);
                            let results = self.process_completion(&mut completion, &ctx);

                            // Prioritize external results over (sub)commands
                            suggestions.splice(0..0, results);

                            if !completion.need_fallback {
                                return suggestions;
                            }
                        }

                        // for external path arguments with spaces, please check issue #15790
                        if suggestions.is_empty() {
                            let (new_span, prefix) =
                                strip_placeholder_if_any(working_set, &span, strip);
                            let ctx = Context::new(working_set, new_span, prefix, offset);
                            return self.process_completion(&mut FileCompletion, &ctx);
                        }
                        break;
                    }
                }
            }
            _ => (),
        }

        // if no suggestions yet, fallback to file completion
        if suggestions.is_empty() {
            let (new_span, prefix) =
                strip_placeholder_if_any(working_set, &element_expression.span, strip);
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
        let config = self.engine_state.get_config();
        let mut command_completions = CommandCompletion {
            internals,
            externals: !internals || (externals && config.completions.external.enable),
        };
        let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
        let ctx = Context::new(working_set, new_span, prefix, offset);
        self.process_completion(&mut command_completions, &ctx)
    }

    fn argument_completion_helper(
        &self,
        argument_info: PositionalArguments,
        pos: usize,
        ctx: &Context,
        need_fallback: &mut bool,
    ) -> Vec<SemanticSuggestion> {
        let PositionalArguments {
            command_head,
            positional_arg_indices,
            arguments,
            expr,
        } = argument_info;
        // special commands
        match command_head {
            // complete module file/directory
            "use" | "export use" | "overlay use" | "source-env"
                if positional_arg_indices.len() <= 1 =>
            {
                *need_fallback = false;

                return self.process_completion(
                    &mut DotNuCompletion {
                        std_virtual_path: command_head != "source-env",
                    },
                    ctx,
                );
            }
            // NOTE: if module file already specified,
            // should parse it to get modules/commands/consts to complete
            "use" | "export use" => {
                *need_fallback = false;

                let Some(Argument::Positional(Expression {
                    expr: Expr::String(module_name),
                    span,
                    ..
                })) = positional_arg_indices
                    .first()
                    .and_then(|i| arguments.get(*i))
                else {
                    return vec![];
                };
                let module_name = module_name.as_bytes();
                let (module_id, temp_working_set) = match ctx.working_set.find_module(module_name) {
                    Some(module_id) => (module_id, None),
                    None => {
                        let mut temp_working_set =
                            StateWorkingSet::new(ctx.working_set.permanent_state);
                        let Some(module_id) = parse_module_file_or_dir(
                            &mut temp_working_set,
                            module_name,
                            *span,
                            None,
                        ) else {
                            return vec![];
                        };
                        (module_id, Some(temp_working_set))
                    }
                };
                let mut exportable_completion = ExportableCompletion {
                    module_id,
                    temp_working_set,
                };
                let mut complete_on_list_items = |items: &[ListItem]| -> Vec<SemanticSuggestion> {
                    for item in items {
                        let span = item.expr().span;
                        if span.contains(pos) {
                            let offset = span.start.saturating_sub(ctx.span.start);
                            let end_offset =
                                ctx.prefix.len().min(pos.min(span.end) - ctx.span.start + 1);
                            let new_ctx = Context::new(
                                ctx.working_set,
                                Span::new(span.start, ctx.span.end.min(span.end)),
                                ctx.prefix.get(offset..end_offset).unwrap_or_default(),
                                ctx.offset,
                            );
                            return self.process_completion(&mut exportable_completion, &new_ctx);
                        }
                    }
                    vec![]
                };

                match &expr.expr {
                    Expr::String(_) => {
                        return self.process_completion(&mut exportable_completion, ctx);
                    }
                    Expr::FullCellPath(fcp) => match &fcp.head.expr {
                        Expr::List(items) => {
                            return complete_on_list_items(items);
                        }
                        _ => return vec![],
                    },
                    _ => return vec![],
                }
            }
            "which" => {
                *need_fallback = false;

                let mut completer = CommandCompletion {
                    internals: true,
                    externals: true,
                };
                return self.process_completion(&mut completer, ctx);
            }
            "attr complete" => {
                *need_fallback = false;

                let mut completer = CommandCompletion {
                    internals: true,
                    externals: false,
                };
                return self.process_completion(&mut completer, ctx);
            }
            _ => (),
        }

        // general positional arguments
        let file_completion_helper = || self.process_completion(&mut FileCompletion, ctx);
        match &expr.expr {
            Expr::Directory(_, _) => {
                *need_fallback = false;
                self.process_completion(&mut DirectoryCompletion, ctx)
            }
            Expr::Filepath(_, _) | Expr::GlobPattern(_, _) => file_completion_helper(),
            // fallback to file completion if necessary
            _ if *need_fallback => file_completion_helper(),
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
                    "span" => {
                        if let Value::Record { val: span, .. } = value {
                            let start = span
                                .get("start")
                                .and_then(|val| val.as_int().ok())
                                .and_then(|x| usize::try_from(x).ok());
                            let end = span
                                .get("end")
                                .and_then(|val| val.as_int().ok())
                                .and_then(|x| usize::try_from(x).ok());
                            if let (Some(start), Some(end)) = (start, end) {
                                suggestion.span = reedline::Span {
                                    start: start.min(end),
                                    end,
                                };
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
            assert_eq!(!result.is_empty(), has_result, "line: {line}");

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
                "line: {line}"
            );
        }
    }
}
