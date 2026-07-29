use crate::completions::{
    ArgValueCompletion, AttributableCompletion, AttributeCompletion, CellPathCompletion,
    CommandCompletion, Completer, CompletionOptions, CustomCompletion, FileCompletion,
    FlagCompletion, NuMatcher, OperatorCompletion, VariableCompletion, base::SemanticSuggestion,
};
use nu_parser::parse;
use nu_protocol::{
    CommandWideCompleter, Completion, GetSpan, Signature, Span,
    ast::{Argument, Block, Expr, Expression, PipelineRedirection, RedirectionTarget, Traverse},
    engine::{ArgType, EngineState, Stack, StateWorkingSet},
};
use nu_utils::time::Instant;
use reedline::{
    Completer as ReedlineCompleter, CompletionResult, CompletionStatus, Suggestion, Suggestions,
};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use std::{borrow::Cow, ops::ControlFlow};
use std::{collections::HashMap, path::is_separator};

const CACHE_TTL: Duration = Duration::from_secs(5);

use super::{StaticCompletion, custom_completions::CommandWideCompletion};

/// Used as the function `f` in find_map Traverse
///
/// returns the inner-most pipeline_element of interest
/// i.e. the one that contains given position and needs completion
fn find_pipeline_element_by_position<'a>(
    expr: &'a Expression,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> ControlFlow<Option<&'a Expression>> {
    // skip the entire expression if the position is not in it
    if !expr.span.contains(pos) {
        return ControlFlow::Break(None);
    }
    let closure = |expr: &'a Expression| find_pipeline_element_by_position(expr, working_set, pos);
    let found = |x| ControlFlow::Break(Some(x));
    match &expr.expr {
        Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Block(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            // check redirection target for sub blocks before diving recursively into them
            check_redirection_in_block(block.as_ref(), pos)
                .map(found)
                .unwrap_or(ControlFlow::Continue(()))
        }
        Expr::Call(call) => call
            .arguments
            .iter()
            .find_map(|arg| arg.expr().and_then(|e| e.find_map(working_set, &closure)))
            .map(found)
            .unwrap_or(found(expr)),
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
            .map(found)
            .unwrap_or(found(expr)),
        // complete the operator
        Expr::BinaryOp(lhs, _, rhs) => lhs
            .find_map(working_set, &closure)
            .or_else(|| rhs.find_map(working_set, &closure))
            .map(found)
            .unwrap_or(found(expr)),
        Expr::FullCellPath(fcp) => fcp
            .head
            .find_map(working_set, &closure)
            .map(found)
            // e.g. use std/util [<tab>
            .or_else(|| {
                (fcp.head.span.contains(pos) && matches!(fcp.head.expr, Expr::List(_)))
                    .then_some(ControlFlow::Continue(()))
            })
            .unwrap_or(found(expr)),
        Expr::Var(_) => found(expr),
        Expr::AttributeBlock(ab) => ab
            .attributes
            .iter()
            .map(|attr| &attr.expr)
            .chain(Some(ab.item.as_ref()))
            .find_map(|expr| expr.find_map(working_set, &closure))
            .map(found)
            .unwrap_or(found(expr)),
        _ => ControlFlow::Continue(()),
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

/// Cache key and worker message identity: (line buffer, cursor byte offset).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CompletionQuery {
    line: String,
    current_position: usize,
}

impl CompletionQuery {
    /// Returns the buffer text up to the cursor.
    /// This represents the only portion of the text that completion depends on.
    fn input_text(&self) -> &str {
        let fallback = &self.line; // full buffer is where cursor is

        self.line.get(..self.current_position).unwrap_or(fallback)
    }

    /// Determines if this query is a strict narrowing of a previous query,
    /// meaning the base's results are a superset that can be filtered in place.
    fn strictly_narrows(&self, base_query: &CompletionQuery) -> bool {
        let Some(appended) = self.input_text().strip_prefix(base_query.input_text()) else {
            return false;
        };

        !appended.is_empty() && !appended.contains(|c: char| c.is_whitespace() || is_separator(c))
    }
}

struct CacheEntry {
    /// Held as the shared `Arc` so a cache hit hands the list to reedline (and on
    /// to the menu) with a refcount bump instead of copying it into a `Vec`.
    suggestions: Suggestions,
    at: Instant,
}

/// Reedline-side handle to the single background worker for this completer.
///
/// `pending` is the latest enqueued request; the worker may only wake reedline
/// when it finishes that generation (stale ready signals are drained/ignored).
struct CompletionWorker {
    request_tx: mpsc::Sender<(CompletionQuery, u64)>,
    ready_rx: mpsc::Receiver<u64>,
    next_generation: u64,
    /// `(query, generation)` still awaited from the worker, if any.
    pending: Option<(CompletionQuery, u64)>,
}

/// Isolate a stack for completion evaluation.
///
/// Completions may run arbitrary code; output is always captured so it cannot
/// corrupt reedline / LSP. Background workers also null child stdin and detach
/// from the terminal (via `Stack::suppress_stdin`) so subprocesses cannot race
/// the line editor.
fn isolated_stack(parent: Arc<Stack>, background: bool) -> Arc<Stack> {
    let stack = Stack::with_parent(parent)
        .reset_out_dest()
        .suppress_output()
        .collect_value();
    Arc::new(if background {
        stack.suppress_stdin()
    } else {
        stack
    })
}

pub struct NuCompleter {
    engine_state: Arc<EngineState>,
    stack: Arc<Stack>,
    /// Shared with the background worker; reedline thread only.
    cache: Arc<Mutex<HashMap<CompletionQuery, CacheEntry>>>,
    /// Lazily spawned on the first cache miss; reedline thread only (no lock).
    worker: Option<CompletionWorker>,
}

/// Common arguments required for Completer
pub(crate) struct Context<'a> {
    pub working_set: &'a StateWorkingSet<'a>,
    pub span: Span,
    pub prefix: &'a [u8],
    pub offset: usize,
}

impl Context<'_> {
    pub(crate) fn new<'a>(
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
        Self::with_stack(engine_state, isolated_stack(stack, false))
    }

    /// Background worker: same isolation as [`Self::new`], plus suppressed stdin.
    fn for_background(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self::with_stack(engine_state, isolated_stack(stack, true))
    }

    fn with_stack(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self {
            engine_state,
            stack,
            cache: Arc::new(Mutex::new(HashMap::new())),
            worker: None,
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
                    let ctx = Context::new(working_set, element_expression.span, &[], offset);
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
                let force_external = prefix_str.starts_with('^');
                let force_internal = prefix_str.starts_with('%');
                let force_builtins_only = force_internal;

                let need_externals = !prefix_str.contains(' ') && !force_internal;
                let need_internals = !force_external;
                let mut span = element_expression.span;
                if force_external || force_internal {
                    span.start += 1;
                };
                suggestions.extend(self.command_completion_helper(
                    working_set,
                    span,
                    offset,
                    CommandCompletionOptions {
                        internals: need_internals,
                        externals: need_externals,
                        builtins_only: force_builtins_only,
                        quote_internals: false,
                    },
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
                let mut positional_arg_index = 0;

                for (arg_idx, arg) in call.arguments.iter().enumerate() {
                    let span = arg.span();

                    // Skip arguments before the cursor
                    if !span.contains(pos) {
                        match arg {
                            Argument::Named(_) => (),
                            _ => positional_arg_index += 1,
                        }
                        continue;
                    }

                    // Context defaults to the whole argument, needs adjustments for specific situations
                    let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
                    let ctx = Context::new(working_set, new_span, prefix, offset);
                    let flag_completion_helper = |ctx: Context| {
                        let mut flag_completions = FlagCompletion {
                            decl_id: call.decl_id,
                        };
                        let mut res = self.process_completion(&mut flag_completions, &ctx);
                        // For external command wrappers, which are parsed as internal calls,
                        // also try command-wide completion for flag names
                        // TODO: duplication?
                        let command_wide_ctx = Context::new(working_set, span, b"", offset);
                        res.extend(
                            self.command_wide_completion_helper(
                                &signature,
                                element_expression,
                                &command_wide_ctx,
                                strip,
                            )
                            .1,
                        );
                        res
                    };

                    // Basically 2 kinds of argument completions for now:
                    // 1. Flag name: 2 sources combined:
                    //    * Signature based internal flags
                    //    * Command-wide external flags
                    // 2. Flag value/positional: try the following in order:
                    //    1. Custom completion
                    //    2. Command-wide completion
                    //    3. Dynamic completion defined in trait `Command`
                    //    4. Type-based default completion
                    //    5. Fallback(file) completion
                    match arg {
                        // Flag value completion
                        Argument::Named((name, short, Some(val))) if val.span.contains(pos) => {
                            // for `--foo ..a|` and `--foo=..a|` (`|` represents the cursor), the
                            // arg span should be trimmed:
                            // - split the given span with `predicate` (b == '=' || b == ' '), and
                            //   take the rightmost part:
                            //   - "--foo ..a" => ["--foo", "..a"] => "..a"
                            //   - "--foo=..a" => ["--foo", "..a"] => "..a"
                            // - strip placeholder (`a`) if present
                            let mut new_span = val.span;
                            if strip {
                                new_span.end = new_span.end.saturating_sub(1);
                            }
                            let prefix = working_set.get_span_contents(new_span);
                            let ctx = Context::new(working_set, new_span, prefix, offset);

                            // If we're completing the value of the flag,
                            // search for the matching custom completion decl_id (long or short)
                            let flag = signature.get_long_flag(&name.item).or_else(|| {
                                short.as_ref().and_then(|s| {
                                    signature.get_short_flag(s.item.chars().next().unwrap_or('_'))
                                })
                            });
                            // Prioritize custom completion results over everything else
                            if let Some(custom_completer) = flag.and_then(|f| f.completion) {
                                let (need_fallback, new_suggestions) = self
                                    .custom_completion_helper(
                                        custom_completer,
                                        prefix_str,
                                        &ctx,
                                        if strip { pos } else { pos + 1 },
                                    );
                                suggestions.splice(0..0, new_suggestions);
                                if !need_fallback {
                                    return suggestions;
                                }
                            }

                            // Try command-wide completion if specified by attributes
                            // NOTE: `CommandWideCompletion` handles placeholder stripping internally
                            let command_wide_ctx = Context::new(working_set, val.span, b"", offset);
                            let (need_fallback, command_wide_res) = self
                                .command_wide_completion_helper(
                                    &signature,
                                    element_expression,
                                    &command_wide_ctx,
                                    strip,
                                );
                            suggestions.splice(0..0, command_wide_res);
                            if !need_fallback {
                                return suggestions;
                            }

                            let mut flag_value_completion = ArgValueCompletion {
                                arg_type: ArgType::Flag(Cow::from(name.as_ref().item.as_str())),
                                // flag value doesn't need to fallback, just fill a
                                // temp value false.
                                need_fallback: false,
                                completer: self,
                                call,
                                arg_idx,
                                pos,
                                strip,
                            };
                            suggestions.splice(
                                0..0,
                                self.process_completion(&mut flag_value_completion, &ctx),
                            );
                            return suggestions;
                        }
                        // Flag name completion
                        Argument::Named((_, _, None)) => {
                            suggestions.splice(0..0, flag_completion_helper(ctx));
                        }
                        // Edge case of lsp completion where the cursor is at the flag name,
                        // with a flag value next to it.
                        Argument::Named((_, _, Some(val))) => {
                            // Span/prefix calibration
                            let mut new_span = Span::new(span.start, val.span.start);
                            let raw_prefix = working_set.get_span_contents(new_span);
                            let prefix = raw_prefix.trim_ascii_end();
                            let mut prefix = prefix.strip_suffix(b"=").unwrap_or(prefix);
                            new_span.end = new_span
                                .end
                                .saturating_sub(raw_prefix.len() - prefix.len())
                                .max(span.start);

                            // Currently never reachable
                            if strip {
                                new_span.end = new_span.end.saturating_sub(1).max(span.start);
                                prefix = prefix[..prefix.len() - 1].as_ref();
                            }

                            let ctx = Context::new(working_set, new_span, prefix, offset);
                            suggestions.splice(0..0, flag_completion_helper(ctx));
                        }
                        Argument::Unknown(_) if prefix.starts_with(b"-") => {
                            suggestions.splice(0..0, flag_completion_helper(ctx));
                        }
                        // only when `strip` == false
                        Argument::Positional(_) if prefix == b"-" => {
                            suggestions.splice(0..0, flag_completion_helper(ctx));
                        }
                        Argument::Positional(_) => {
                            // Prioritize custom completion results over everything else
                            if let Some(custom_completer) = signature
                                // For positional arguments, check PositionalArg
                                // Find the right positional argument by index
                                .get_positional(positional_arg_index)
                                .and_then(|pos_arg| pos_arg.completion.clone())
                            {
                                let (need_fallback, new_suggestions) = self
                                    .custom_completion_helper(
                                        custom_completer,
                                        prefix_str,
                                        &ctx,
                                        if strip { pos } else { pos + 1 },
                                    );
                                suggestions.splice(0..0, new_suggestions);
                                if !need_fallback {
                                    return suggestions;
                                }
                            }

                            // Try command-wide completion if specified by attributes
                            let command_wide_ctx = Context::new(working_set, span, b"", offset);
                            let (need_fallback, command_wide_res) = self
                                .command_wide_completion_helper(
                                    &signature,
                                    element_expression,
                                    &command_wide_ctx,
                                    strip,
                                );
                            suggestions.splice(0..0, command_wide_res);
                            if !need_fallback {
                                return suggestions;
                            }

                            // Default argument value completion
                            let mut positional_value_completion = ArgValueCompletion {
                                // arg_type: ArgType::Positional(positional_arg_index - 1),
                                arg_type: ArgType::Positional(positional_arg_index),
                                need_fallback: suggestions.is_empty(),
                                completer: self,
                                call,
                                arg_idx,
                                pos,
                                strip,
                            };

                            suggestions.splice(
                                0..0,
                                self.process_completion(&mut positional_value_completion, &ctx),
                            );
                            return suggestions;
                        }
                        _ => (),
                    }
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
                                    CommandCompletionOptions {
                                        internals: true,
                                        externals: true,
                                        builtins_only: false,
                                        quote_internals: false,
                                    },
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
        options: CommandCompletionOptions,
        strip: bool,
    ) -> Vec<SemanticSuggestion> {
        let config = self.engine_state.get_config();
        let mut command_completions = CommandCompletion {
            internals: options.internals,
            externals: !options.internals
                || (options.externals && config.completions.external.enable),
            builtins_only: options.builtins_only,
            quote_internals: options.quote_internals,
        };
        let (new_span, prefix) = strip_placeholder_if_any(working_set, &span, strip);
        let ctx = Context::new(working_set, new_span, prefix, offset);
        self.process_completion(&mut command_completions, &ctx)
    }

    fn custom_completion_helper(
        &self,
        custom_completion: Completion,
        input: &str,
        ctx: &Context,
        pos: usize,
    ) -> (bool, Vec<SemanticSuggestion>) {
        match custom_completion {
            Completion::Command(decl_id) => {
                let mut completer =
                    CustomCompletion::new(decl_id, input.into(), pos - ctx.offset, false);
                let suggestions = self.process_completion(&mut completer, ctx);
                (completer.need_fallback, suggestions)
            }
            Completion::List(list) => {
                let mut completer = StaticCompletion::new(list);
                (false, self.process_completion(&mut completer, ctx))
            }
        }
    }

    fn command_wide_completion_helper(
        &self,
        signature: &Signature,
        element_expression: &Expression,
        ctx: &Context,
        strip: bool,
    ) -> (bool, Vec<SemanticSuggestion>) {
        let completion = match signature.complete {
            Some(CommandWideCompleter::Command(decl_id)) => {
                CommandWideCompletion::command(ctx.working_set, decl_id, element_expression, strip)
            }
            Some(CommandWideCompleter::External) => self
                .engine_state
                .get_config()
                .completions
                .external
                .completer
                .as_ref()
                .map(|closure| CommandWideCompletion::closure(closure, element_expression, strip)),
            None => None,
        };

        if let Some(mut completion) = completion {
            let res = self.process_completion(&mut completion, ctx);
            (completion.need_fallback, res)
        } else {
            (true, vec![])
        }
    }

    // Process the completion for a given completer
    pub(crate) fn process_completion<T: Completer>(
        &self,
        completer: &mut T,
        ctx: &Context,
    ) -> Vec<SemanticSuggestion> {
        let config = self.engine_state.get_config();

        let options = CompletionOptions {
            case_sensitive: config.completions.case_sensitive,
            match_algorithm: config.completions.algorithm.into(),
            sort: config.completions.sort,
            match_description: false,
        };

        completer.fetch(
            ctx.working_set,
            self.stack.as_ref(),
            String::from_utf8_lossy(ctx.prefix),
            ctx.span,
            ctx.offset,
            &options,
        )
    }
}

struct CommandCompletionOptions {
    internals: bool,
    externals: bool,
    builtins_only: bool,
    quote_internals: bool,
}

impl NuCompleter {
    fn cached(&self, query: &CompletionQuery) -> Option<Suggestions> {
        let cache = self.cache.lock().ok()?;
        let entry = cache.get(query)?;
        // `Arc` clone: a refcount bump, not a copy of the list.
        (entry.at.elapsed() < CACHE_TTL).then(|| entry.suggestions.clone())
    }

    /// Best-effort suggestions for a cache miss: reuse the freshest still-valid
    /// cache entry that `query` narrows,
    /// filtered down to the longer prefix the user has since typed.
    fn stale_fallback(&self, query: &CompletionQuery) -> Suggestions {
        let closest = self.fetch_closest_cached_suggestions(query);
        self.narrow(&closest, query)
    }

    /// Safely locks the cache and extracts the suggestions from the tightest valid superset.
    fn fetch_closest_cached_suggestions(&self, query: &CompletionQuery) -> Suggestions {
        let Ok(cache) = self.cache.lock() else {
            return Suggestions::default();
        };

        cache
            .iter()
            .filter(|(base_query, cache_entry)| {
                cache_entry.at.elapsed() < CACHE_TTL && query.strictly_narrows(base_query)
            })
            // Prefer the closest superset: most already typed = tightest existing filter
            .max_by_key(|(base_query, _)| base_query.current_position)
            // `Arc` clone: a refcount bump; `narrow` re-filters without mutating it.
            .map(|(_, cache_entry)| cache_entry.suggestions.clone())
            .unwrap_or_default()
    }

    /// Re-filters a superset of suggestions to match the current query.
    fn narrow(&self, suggestions: &[Suggestion], query: &CompletionQuery) -> Suggestions {
        let Some((reference_span, search_token)) = suggestions.first().and_then(|suggestion| {
            let span = suggestion.span;
            let token = query.input_text().get(span.start..)?;
            Some((span, token))
        }) else {
            // Bail if there are no suggestions, or the span exceeds the input length.
            return Suggestions::default();
        };

        // Stretch the span from the original start point to the user's current cursor position.
        let updated_span = reedline::Span::new(reference_span.start, query.current_position);
        let options = self.build_completion_options();
        let mut matcher = NuMatcher::new(search_token, &options, true);

        // If the spans that produced past suggestions mirror the ones that produced this one, they're fit as matches.
        for mut suggestion in suggestions
            .iter()
            .filter(|suggestion| suggestion.span == reference_span)
            .cloned()
        {
            suggestion.span = updated_span;
            // Owned because `suggestion` is moved into the matcher on the same line.
            let haystack = suggestion.display_value().to_string();
            matcher.add(haystack, suggestion);
        }

        Self::extract_matcher_results(matcher)
    }

    /// Constructs completion options from the engine state configuration.
    fn build_completion_options(&self) -> CompletionOptions {
        let configuration = self.engine_state.get_config();

        CompletionOptions {
            case_sensitive: configuration.completions.case_sensitive,
            match_algorithm: configuration.completions.algorithm.into(),
            sort: configuration.completions.sort,
            match_description: false,
        }
    }

    /// Maps the raw matcher output back into finalized `Suggestion` objects.
    fn extract_matcher_results(matcher: NuMatcher<Suggestion>) -> Suggestions {
        matcher
            .results()
            .into_iter()
            .map(|(mut suggestion, match_indices)| {
                suggestion.match_indices = Some(match_indices);
                suggestion
            })
            .collect()
    }

    /// Spawn the long-lived worker. Takes field refs so callers can hold
    /// `&mut self.worker` while building it.
    fn spawn_worker(
        engine_state: &Arc<EngineState>,
        stack: &Arc<Stack>,
        cache: &Arc<Mutex<HashMap<CompletionQuery, CacheEntry>>>,
    ) -> CompletionWorker {
        let (request_tx, request_rx) = mpsc::channel::<(CompletionQuery, u64)>();
        let (ready_tx, ready_rx) = mpsc::channel::<u64>();

        let completer = NuCompleter::for_background(engine_state.clone(), Arc::clone(stack));
        let cache = Arc::clone(cache);

        thread::spawn(move || {
            let mut queued = None;
            loop {
                // Prefer a request that arrived during the previous compute;
                // otherwise block for the next one.
                let (query, generation) = match queued.take().or_else(|| request_rx.recv().ok()) {
                    Some(req) => req,
                    None => return,
                };

                // Coalesce: only the latest query is worth computing.
                let mut latest = (query, generation);
                while let Ok(newer) = request_rx.try_recv() {
                    latest = newer;
                }
                let (query, generation) = latest;

                // Build the shared `Arc` once, here, so every later cache hit and
                // menu refresh is a refcount bump rather than a copy.
                let suggestions: Suggestions = completer
                    .fetch_completions_at(&query.line, query.current_position)
                    .into_iter()
                    .map(|s| s.suggestion)
                    .collect();

                if let Ok(mut guard) = cache.lock() {
                    guard.retain(|_, e| e.at.elapsed() < CACHE_TTL);
                    guard.insert(
                        query,
                        CacheEntry {
                            suggestions,
                            at: Instant::now(),
                        },
                    );
                }

                // If more work arrived while we computed, process it next and
                // do NOT signal ready!
                match request_rx.try_recv() {
                    Ok(newer) => queued = Some(newer),
                    Err(mpsc::TryRecvError::Empty) => {
                        if ready_tx.send(generation).is_err() {
                            return;
                        }
                    }
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }
        });

        CompletionWorker {
            request_tx,
            ready_rx,
            next_generation: 0,
            pending: None,
        }
    }

    /// Complete synchronously, driving any background work to completion.
    ///
    /// The interactive [`complete`](ReedlineCompleter::complete) path returns
    /// immediately with a `Stale`/`Pending` placeholder and relies on the reedline
    /// event loop to poll [`poll_completion`](ReedlineCompleter::poll_completion)
    /// Others don't have this mechanism, and this helper is for them.
    pub fn complete_blocking(&mut self, line: &str, pos: usize) -> Suggestions {
        // Upper bound on how long a single completion may take before we give up
        const BLOCKING_TIMEOUT: Duration = Duration::from_secs(30);

        // `Fresh` is settled and hands over its `Arc` directly. `Stale`/`Pending`
        // mean a compute is still in flight; keep the best-effort `Arc` (empty for
        // `Pending`) as a fallback while we wait for the settled result.
        let fallback = match self.complete(line, pos) {
            CompletionResult::Fresh(values) => return values,
            in_flight => in_flight.into_shared().unwrap_or_default(),
        };

        let deadline = Instant::now() + BLOCKING_TIMEOUT;
        while Instant::now() < deadline {
            if self.poll_completion() == CompletionStatus::Ready {
                return self.complete(line, pos).into_shared().unwrap_or_default();
            }
            thread::sleep(Duration::from_millis(10));
        }

        // Timed out: return the best-effort fallback rather than blocking for like ever.
        fallback
    }
}

impl ReedlineCompleter for NuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let query = CompletionQuery {
            line: line.to_string(),
            current_position: pos,
        };

        if let Some(suggestions) = self.cached(&query) {
            if let Some(worker) = self.worker.as_mut()
                && worker.pending.as_ref().is_some_and(|(q, _)| q == &query)
            {
                worker.pending = None;
            }
            return CompletionResult::fresh(suggestions);
        }

        // Cache miss: keep the menu populated with the last shown results
        let fallback = self.stale_fallback(&query);

        let worker = self.worker.get_or_insert_with(|| {
            Self::spawn_worker(&self.engine_state, &self.stack, &self.cache)
        });

        // Already waiting on this exact query; keep showing the fallback.
        if worker.pending.as_ref().is_some_and(|(q, _)| q == &query) {
            return CompletionResult::stale_or_pending(fallback);
        }

        worker.next_generation = worker.next_generation.wrapping_add(1);
        let generation = worker.next_generation;

        if worker.request_tx.send((query.clone(), generation)).is_ok() {
            worker.pending = Some((query, generation));
        } else {
            worker.pending = None;
        }

        CompletionResult::stale_or_pending(fallback)
    }

    /// Poll the background worker, collapsing the old `has_pending` +
    /// `check_pending` pair into reedline's tri-state
    fn poll_completion(&mut self) -> CompletionStatus {
        let Some(worker) = self.worker.as_mut() else {
            return CompletionStatus::Idle;
        };

        let expected = worker.pending.as_ref().map(|(_, generation)| *generation);

        // Drain so the channel never fills up.
        let mut matched = false;
        while let Ok(generation) = worker.ready_rx.try_recv() {
            matched |= expected == Some(generation);
        }

        match expected {
            Some(_) if matched => {
                worker.pending = None;
                CompletionStatus::Ready
            }
            Some(_) => CompletionStatus::Pending,
            None => CompletionStatus::Idle,
        }
    }
}

#[cfg(test)]
mod completer_tests {
    use super::*;
    use nu_protocol::OutDest;

    fn test_engine() -> Arc<EngineState> {
        let mut engine =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let delta = StateWorkingSet::new(&engine).render();
        engine.merge_delta(delta).expect("merge_delta");
        Arc::new(engine)
    }

    #[test]
    fn non_blocking_complete_fills_cache() {
        let mut completer = NuCompleter::new(test_engine(), Arc::new(Stack::new()));

        let expected: Vec<_> = completer
            .fetch_completions_at("ls | c", 6)
            .into_iter()
            .map(|s| s.suggestion)
            .collect();
        assert!(expected.iter().any(|s| s.value == "cd"));

        let first = completer.complete("ls | c", 6);
        assert!(first.suggestions().is_empty());
        assert!(first.is_pending());

        let cached = completer.complete_blocking("ls | c", 6);
        assert_eq!(expected.len(), cached.len());
        for s in &expected {
            assert!(
                cached.iter().any(|x| x.value == s.value),
                "missing {}",
                s.value
            );
        }
    }

    /// A cache miss while typing more of the same token narrows the previously
    /// shown results in place instead of returning an empty ("NO RECORDS FOUND")
    /// menu while the background worker recomputes.
    #[test]
    fn cache_miss_narrows_previous_results() {
        let mut completer = NuCompleter::new(test_engine(), Arc::new(Stack::new()));

        // Prime the cache with the results for `ls | c`.
        assert!(completer.complete("ls | c", 6).suggestions().is_empty());
        let primed = completer.complete_blocking("ls | c", 6);
        assert!(primed.iter().any(|s| s.value == "config"));
        assert!(primed.iter().any(|s| s.value == "cd"));

        // Typing another char is a cache miss, but the menu should immediately
        // narrow the cached results rather than flashing empty.
        let narrowed_result = completer.complete("ls | co", 7);
        let narrowed = narrowed_result.suggestions();
        assert!(
            !narrowed.is_empty(),
            "expected stale fallback, got empty menu"
        );
        assert!(narrowed.iter().any(|s| s.value == "config"));
        assert!(narrowed.iter().any(|s| s.value == "const"));
        assert!(!narrowed.iter().any(|s| s.value == "cd"));
        // Spans are re-anchored to cover the freshly typed token.
        for s in narrowed {
            assert_eq!(s.span.end, 7, "span not re-anchored for {}", s.value);
        }

        // The accurate async result still lands afterwards.
        let fresh = completer.complete_blocking("ls | co", 7);
        assert!(fresh.iter().any(|s| s.value == "config"));
    }

    /// A superseded in-flight generation must never wake `check_pending`; only
    /// the latest enqueued generation may.
    #[test]
    fn only_latest_generation_wakes_pending() {
        // The external completer runs on the worker thread. For each token it
        // touches `started-<token>` and then blocks until `release-<token>`
        // appears, letting the test drive exactly when each generation finishes
        // instead of racing on sleeps.
        let gate = std::env::temp_dir().join(format!("nu-gen-test-{}", std::process::id()));
        std::fs::create_dir_all(&gate).expect("create gate dir");
        let gate_str = gate.to_string_lossy().replace('\\', "/");

        let mut engine =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let mut stack = Stack::new();
        // `merge_env` needs a valid `$env.PWD`, which can't be set from script.
        stack.add_env_var(
            "PWD".to_string(),
            nu_protocol::Value::string(&gate_str, nu_protocol::Span::unknown()),
        );
        let setup = format!(
            r#"$env.config.completions.external = {{
                enable: true
                completer: {{|spans|
                    let t = $spans | last
                    touch ('{gate_str}' | path join $"started-($t)")
                    while not ('{gate_str}' | path join $"release-($t)" | path exists) {{ sleep 10ms }}
                    [{{value: $"got-($t)"}}]
                }}
            }}"#
        );
        let mut working_set = StateWorkingSet::new(&engine);
        let block = parse(&mut working_set, None, setup.as_bytes(), false);
        assert!(working_set.parse_errors.is_empty());
        engine.merge_delta(working_set.render()).expect("merge");
        nu_engine::eval_block::<nu_protocol::debugger::WithoutDebug>(
            &engine,
            &mut stack,
            &block,
            nu_protocol::PipelineData::empty(),
        )
        .expect("eval setup");
        engine.merge_env(&mut stack).expect("merge env");

        let started = |t: &str| gate.join(format!("started-{t}"));
        let release =
            |t: &str| std::fs::write(gate.join(format!("release-{t}")), b"").expect("release");
        let await_file = |p: std::path::PathBuf| {
            let deadline = Instant::now() + Duration::from_secs(30);
            while !p.exists() {
                assert!(Instant::now() < deadline, "timed out waiting for {p:?}");
                thread::sleep(Duration::from_millis(5));
            }
        };

        let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

        // Enqueue gen1 and wait until the worker is actually computing it, so
        // gen2 can't be coalesced in before gen1 starts.
        assert!(completer.complete("somecmd a", 9).suggestions().is_empty());
        await_file(started("a"));

        // Supersede with gen2 while gen1 is still blocked.
        assert!(
            completer
                .complete("somecmd ab", 10)
                .suggestions()
                .is_empty()
        );

        // Let gen1 finish. Because gen2 is already queued, the worker moves
        // straight to it without signaling gen1's stale generation.
        release("a");
        await_file(started("ab"));
        // A superseded generation must not report Ready; gen2 is still Pending.
        assert_ne!(
            completer.poll_completion(),
            CompletionStatus::Ready,
            "a superseded generation woke the latest generation"
        );
        assert_eq!(completer.poll_completion(), CompletionStatus::Pending);

        // gen2 finishing wakes the latest generation with the latest result.
        release("ab");
        let result = completer.complete_blocking("somecmd ab", 10);
        assert!(result.iter().any(|s| s.value == "got-ab"), "{result:?}");

        let _ = std::fs::remove_dir_all(&gate);
    }

    #[test]
    fn isolation_contracts() {
        let engine = test_engine();
        let bg = NuCompleter::for_background(engine.clone(), Arc::new(Stack::new()));
        assert!(matches!(bg.stack.stdout(), OutDest::Value));
        assert!(matches!(bg.stack.stderr(), OutDest::Null));
        assert!(bg.stack.suppress_stdin);

        let fg = NuCompleter::new(engine, Arc::new(Stack::new()));
        assert!(matches!(fg.stack.stdout(), OutDest::Value));
        assert!(matches!(fg.stack.stderr(), OutDest::Null));
        assert!(!fg.stack.suppress_stdin);
    }

    #[test]
    fn test_completion_helper() {
        let completer = NuCompleter::new(test_engine(), Arc::new(Stack::new()));
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
