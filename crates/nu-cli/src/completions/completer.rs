use crate::completions::{
    ArgValueCompletion, AttributableCompletion, AttributeCompletion, CellPathCompletion,
    CommandCompletion, CommandScope, Completer, CompletionOptions, DotNuCompletion,
    EnvVarCompletion, FileCompletion, FlagCompletion, NuMatcher, OperatorCompletion,
    VariableCompletion,
    base::{Fetched, SemanticSuggestion},
};
use lru::LruCache;
use nu_parser::{parse, parse_shorter_head_reading};
use nu_protocol::{
    BlockId, BuiltinCompletion, CommandWideCompleter, Completion, Config, DeclId, Flag, Record,
    Signature, Span, SuggestionKind, SyntaxShape, Value,
    ast::{
        Argument, AttributeBlock, Block, Call, Expr, Expression, ExternalArgument, FlagRef,
        FullCellPath, PipelineRedirection, RedirectionTarget, Traverse,
    },
    engine::{ArgType, Closure, Command, EngineState, Stack, StateWorkingSet},
};
use nu_utils::time::Instant;
use reedline::{
    Completer as ReedlineCompleter, CompletionOrigin, CompletionResult, CompletionStatus, Partial,
    Suggestion, Suggestions,
};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use std::{borrow::Cow, ops::ControlFlow, path::is_separator};

const DEFAULT_CACHE_SIZE: usize = 100;

use super::{
    StaticCompletion,
    custom_completions::{DeclaredInputs, UserCompletion, completer_input},
};

fn find_pipeline_element_by_position<'a>(
    expr: &'a Expression,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> ControlFlow<Option<&'a Expression>> {
    if !expr.span.contains(pos) && expr.span.end != pos {
        return ControlFlow::Break(None);
    }

    let recurse = |e: &'a Expression| find_pipeline_element_by_position(e, working_set, pos);
    let found = |x| ControlFlow::Break(Some(x));
    let or_self = |opt: Option<&'a Expression>| opt.map_or(found(expr), found);

    match &expr.expr {
        Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Block(block_id)
        | Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            check_redirection_in_block(block, pos).map_or(ControlFlow::Continue(()), found)
        }
        Expr::Call(call) => or_self(
            call.arguments
                .iter()
                .find_map(|arg| arg.expr().and_then(|e| e.find_map(working_set, &recurse))),
        ),
        Expr::ExternalCall(head, arguments) => or_self(
            arguments
                .iter()
                .find_map(|arg| arg.expr().find_map(working_set, &recurse))
                .or_else(|| {
                    // `touches` includes the head's trailing edge (#7648).
                    touches(head.span, pos)
                        .then(|| head.as_ref().find_map(working_set, &recurse))
                        .flatten()
                }),
        ),
        Expr::BinaryOp(lhs, _, rhs) => or_self(
            lhs.find_map(working_set, &recurse)
                .or_else(|| rhs.find_map(working_set, &recurse)),
        ),
        Expr::FullCellPath(fcp) => {
            // `use std/util [E, T⌶`: the import list is the enclosing call's slot.
            if touches(fcp.head.span, pos) && matches!(fcp.head.expr, Expr::List(_)) {
                return ControlFlow::Continue(());
            }
            or_self(fcp.head.find_map(working_set, &recurse))
        }
        Expr::Var(_) => found(expr),
        Expr::AttributeBlock(ab) => or_self(
            ab.attributes
                .iter()
                .map(|attr| &attr.expr)
                .chain(std::iter::once(ab.item.as_ref()))
                .find_map(|e| e.find_map(working_set, &recurse)),
        ),
        _ => ControlFlow::Continue(()),
    }
}

/// True if expr owns trailing gap after its span.
fn owns_trailing_gap(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Call(_) | Expr::ExternalCall(_, _) | Expr::AttributeBlock(_)
    )
}

/// Whether `pos` is inside `span` or at its trailing edge.
pub(crate) fn touches(span: Span, position: usize) -> bool {
    span.contains(position) || span.end == position
}

/// The block a nesting expression runs.
fn nested_block(expr: &Expression) -> Option<BlockId> {
    match expr.expr {
        Expr::RowCondition(block_id)
        | Expr::Subexpression(block_id)
        | Expr::Block(block_id)
        | Expr::Closure(block_id) => Some(block_id),
        _ => None,
    }
}

/// The block the cursor descends into from `element`, and its span.
fn descent_from<'a>(
    element: &'a Expression,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> Option<(BlockId, Span)> {
    element.find_map(working_set, &|expr: &'a Expression| {
        if !touches(expr.span, pos) {
            // Skip this subtree; the caller keeps searching its siblings.
            return ControlFlow::Break(None);
        }
        match nested_block(expr) {
            Some(block_id) => ControlFlow::Break(Some((block_id, expr.span))),
            None => ControlFlow::Continue(()),
        }
    })
}

/// The element the cursor is on; a trailing gap belongs to the last.
fn element_at<'a>(
    block: &'a Block,
    working_set: &StateWorkingSet,
    pos: usize,
) -> Option<&'a Expression> {
    block
        .pipelines
        .iter()
        .flat_map(|pipeline| &pipeline.elements)
        .map(|element| &element.expr)
        .find(|expr| touches(expr.span, pos))
        .or_else(|| trailing_gap_element(block, working_set, pos))
}

/// The elements enclosing the cursor, outermost first.
fn enclosing_elements<'a>(
    block: &'a Block,
    working_set: &'a StateWorkingSet,
    pos: usize,
) -> Vec<(&'a Expression, Option<Span>)> {
    let mut chain = Vec::new();
    let mut block = block;

    while let Some(element) = element_at(block, working_set, pos) {
        let descent = descent_from(element, working_set, pos);
        chain.push((element, descent.map(|(_, span)| span)));

        let Some((block_id, _)) = descent else { break };
        block = working_set.get_block(block_id);
    }

    chain
}

/// The expression the cursor completes; a strictly inner chain element beats the search.
fn innermost_expression<'a>(
    found: Option<&'a Expression>,
    chain: &[(&'a Expression, Option<Span>)],
) -> Option<&'a Expression> {
    let innermost = chain.last().map(|&(element, _)| element);
    let (Some(outer), Some(inner)) = (found, innermost) else {
        return found.or(innermost);
    };

    let strictly_inside = outer.span.start <= inner.span.start
        && inner.span.end <= outer.span.end
        && inner.span != outer.span;

    Some(if strictly_inside { inner } else { outer })
}

/// The last element when the cursor trails it over whitespace.
fn trailing_gap_element<'a>(
    block: &'a Block,
    working_set: &StateWorkingSet,
    absolute_position: usize,
) -> Option<&'a Expression> {
    let expression = &block.pipelines.last()?.elements.last()?.expr;

    // The element can end past the cursor (whole files, later statements); a backwards
    // span would panic.
    if expression.span.end > absolute_position {
        return None;
    }

    let gap = working_set.get_span_contents(Span::new(expression.span.end, absolute_position));
    gap.iter()
        .all(u8::is_ascii_whitespace)
        .then_some(expression)
}

/// The span a command-name completion replaces.
fn command_name_span(head: Span, element: Span) -> Span {
    Span::new(head.start, head.end.max(element.end))
}

/// Whether the token starts a flag; in-progress flags parse as positionals.
pub(crate) fn is_flag_text(token: impl AsRef<[u8]>) -> bool {
    token.as_ref().starts_with(b"-")
}

/// [`is_flag_text`] for the token occupying `span`.
fn is_flag_token(working_set: &StateWorkingSet, span: Span) -> bool {
    is_flag_text(working_set.get_span_contents(span))
}

/// Whether `expr` can be an operator's left-hand side.
fn is_operator_lhs(expr: &Expr) -> bool {
    match expr {
        Expr::Int(_)
        | Expr::Float(_)
        | Expr::Binary(_)
        | Expr::Bool(_)
        | Expr::String(_)
        | Expr::RawString(_)
        | Expr::StringInterpolation(_)
        | Expr::GlobInterpolation(_, _)
        | Expr::DateTime(_)
        | Expr::ValueWithUnit(_)
        | Expr::Range(_)
        | Expr::FullCellPath(_)
        | Expr::CellPath(_)
        | Expr::Var(_)
        | Expr::List(_)
        | Expr::Record(_)
        | Expr::Table(_)
        | Expr::Nothing
        | Expr::Subexpression(_)
        | Expr::Block(_)
        | Expr::Closure(_) => true,
        Expr::AttributeBlock(_)
        | Expr::VarDecl(_)
        | Expr::Call(_)
        | Expr::ExternalCall(_, _)
        | Expr::Operator(_)
        | Expr::RowCondition(_)
        | Expr::UnaryNot(_)
        | Expr::BinaryOp(_, _, _)
        | Expr::Collect(_, _)
        | Expr::MatchBlock(_)
        | Expr::Keyword(_)
        | Expr::Filepath(_, _)
        | Expr::Directory(_, _)
        | Expr::GlobPattern(_, _)
        | Expr::ImportPattern(_)
        | Expr::Overlay(_)
        | Expr::Signature(_)
        | Expr::Garbage => false,
    }
}

/// The flag a [`FlagRef`] refers to, preserving the long/short distinction.
fn find_flag(signature: &Signature, flag: FlagRef<'_>) -> Option<Flag> {
    match flag {
        FlagRef::Long(n) => signature.get_long_flag(n),
        FlagRef::Short(s) => s.chars().next().and_then(|c| signature.get_short_flag(c)),
    }
}

/// Positional arguments before `before_index`.
fn count_positionals(call: &Call, before_index: usize) -> usize {
    call.arguments
        .iter()
        .take(before_index)
        .filter(|argument| !matches!(argument, Argument::Named(_)))
        .count()
}

/// A call and the pipeline element it lives in: the shared payload of every
/// call-bound [`SiteKind`]. Carrying them as one value keeps an element from
/// being paired with another call's arguments.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CallSite<'a> {
    pub call: &'a Call,
    pub element: &'a Expression,
}

/// A call's `Expr::RowCondition` argument. Values of this type prove the
/// condition exists, so operator/value-gap reads share one lookup and one
/// signature-slot rule instead of re-searching the arguments each time.
#[derive(Debug, Clone, Copy)]
struct RowCondition<'a> {
    site: CallSite<'a>,
    arg_slot: usize,
    block_id: BlockId,
    arg_span: Span,
}

impl<'a> RowCondition<'a> {
    fn find(site: CallSite<'a>, want: Option<Span>) -> Option<Self> {
        site.call
            .arguments
            .iter()
            .enumerate()
            .rev()
            .find_map(|(arg_slot, arg)| match arg {
                Argument::Positional(Expression {
                    expr: Expr::RowCondition(block_id),
                    span,
                    ..
                }) if want.is_none_or(|wanted| wanted == *span) => Some(Self {
                    site,
                    arg_slot,
                    block_id: *block_id,
                    arg_span: *span,
                }),
                _ => None,
            })
    }

    /// The trailing condition of `site`, if any.
    fn trailing(site: CallSite<'a>) -> Option<Self> {
        Self::find(site, None)
    }

    /// The condition whose span is `descent_span` (a File leaf promoting outward), if any.
    fn matching(site: CallSite<'a>, descent_span: Span) -> Option<Self> {
        Self::find(site, Some(descent_span))
    }

    /// Declared slot of the condition (e.g. 0 for `where`), not raw arg position, so a
    /// leading path arg cannot steal the condition's index.
    fn index(&self, working_set: &StateWorkingSet) -> usize {
        fn is_cond(shape: &SyntaxShape) -> bool {
            matches!(shape, SyntaxShape::RowCondition)
                || matches!(shape, SyntaxShape::OneOf(shapes) if shapes.iter().any(is_cond))
        }
        let signature = working_set.get_decl(self.site.call.decl_id).signature();
        signature
            .required_positional
            .iter()
            .chain(signature.optional_positional.iter())
            .enumerate()
            .find(|(_, p)| is_cond(&p.shape))
            .map(|(i, _)| i)
            .unwrap_or_else(|| count_positionals(self.site.call, self.arg_slot))
    }

    /// Positional site for this condition.
    fn site_at(
        &self,
        working_set: &StateWorkingSet,
        cursor: usize,
        span: Span,
    ) -> CompletionSite<'a> {
        CompletionSite::at(
            cursor,
            SiteKind::Positional {
                site: self.site,
                sig_positional: self.index(working_set),
                arg_slot: self.arg_slot,
            },
            span,
        )
    }

    /// Last term of the condition body.
    fn last_term(&self, working_set: &'a StateWorkingSet) -> Option<&'a Expression> {
        Some(
            &working_set
                .get_block(self.block_id)
                .pipelines
                .last()?
                .elements
                .last()?
                .expr,
        )
    }
}

/// A File leaf inside a row condition is really the enclosing condition positional.
fn promote_row_condition_file<'a>(
    site: CompletionSite<'a>,
    chain: &[(&'a Expression, Option<Span>)],
    working_set: &StateWorkingSet,
    cursor: usize,
) -> CompletionSite<'a> {
    if !matches!(site.kind, SiteKind::File) {
        return site;
    }
    for &(element, descent) in chain {
        let Some(descent_span) = descent else {
            continue;
        };
        let Expr::Call(call) = &element.expr else {
            continue;
        };
        if let Some(cond) = RowCondition::matching(CallSite { call, element }, descent_span) {
            return cond.site_at(working_set, cursor, site.span);
        }
    }
    site
}

/// A redirection target the cursor touches, as a file path.
fn check_redirection_target(target: &RedirectionTarget, pos: usize) -> Option<&Expression> {
    let expr = target.expr();
    expr.and_then(|expression| {
        if let Expr::String(_) = expression.expr
            && touches(expression.span, pos)
        {
            expr
        } else {
            None
        }
    })
}

/// A redirection target the cursor touches, in any block.
fn check_redirection_in_block(block: &Block, pos: usize) -> Option<&Expression> {
    block
        .pipelines
        .iter()
        .flat_map(|p| &p.elements)
        .filter_map(|e| e.redirection.as_ref())
        .find_map(|redir| match redir {
            PipelineRedirection::Single { target, .. } => check_redirection_target(target, pos),
            PipelineRedirection::Separate { out, err } => {
                check_redirection_target(out, pos).or_else(|| check_redirection_target(err, pos))
            }
        })
}

/// The text typed up to the cursor; also the worker message identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CompletionQuery {
    typed: Arc<str>,
}

impl CompletionQuery {
    fn new(line: &str, cursor: usize) -> Self {
        let floored = line.floor_char_boundary(cursor.min(line.len()));
        Self {
            typed: Arc::from(&line[..floored]),
        }
    }

    fn typed(&self) -> &str {
        &self.typed
    }

    fn cursor(&self) -> usize {
        self.typed.len()
    }

    /// Whether `self` extends `base` within one token.
    fn narrows(&self, base: &CompletionQuery, token: reedline::Span) -> bool {
        let Some(appended) = self.typed().strip_prefix(base.typed()) else {
            return false;
        };

        if appended.is_empty() || appended.contains(is_completion_boundary) {
            return false;
        }

        let (Some(base_token), Some(narrowed_token)) = (
            base.typed().get(token.start..),
            self.typed().get(token.start..),
        ) else {
            return false;
        };

        is_flag_text(base_token) == is_flag_text(narrowed_token)
    }
}

fn is_completion_boundary(c: char) -> bool {
    c.is_whitespace()
        || is_separator(c)
        || matches!(
            c,
            '|' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '=' | ','
        )
}

/// The environment a cached completion was computed against.
///
/// Results depend on cwd, `PATH`, known declarations, and `$env.config`, which
/// change between prompts while the query text does not — so the query alone is
/// not a sound cache key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CacheEnv(u64);

impl CacheEnv {
    /// Fingerprint the completion-relevant parts of `engine_state`/`stack`, once per
    /// completer.
    fn of(engine_state: &EngineState, stack: &Stack) -> Self {
        let mut hasher = DefaultHasher::new();

        engine_state.num_decls().hash(&mut hasher);
        stack
            .get_env_var(engine_state, "PATH")
            .map(|path| path.to_expanded_string(":", &stack.get_config(engine_state)))
            .hash(&mut hasher);

        let cwd = engine_state.cwd(Some(stack)).ok();
        // The cwd mtime, so adding/removing files invalidates stale file completions.
        cwd.as_ref()
            .and_then(|cwd| std::fs::metadata(cwd).ok()?.modified().ok())
            .hash(&mut hasher);
        cwd.hash(&mut hasher);

        engine_state.config_epoch().hash(&mut hasher);
        // Stack-local config changes do not bump the engine epoch.
        stack.config.is_some().hash(&mut hasher);
        if let Some(config) = &stack.config {
            format!("{config:?}").hash(&mut hasher);
        }

        Self(hasher.finish())
    }
}

/// One generation of cached completions. Entries are only valid for the
/// environment they were computed in, so the whole map carries a single env:
/// a mismatch clears it in O(1) instead of purging entry by entry.
struct CacheState {
    /// `None` disables the cache; the map is then never consulted.
    capacity: Option<NonZeroUsize>,
    /// The generation `entries` were computed in.
    env: Option<CacheEnv>,
    entries: LruCache<CompletionQuery, Suggestions>,
}

impl CacheState {
    /// The live map for `env`, clearing a stale generation; `None` when disabled.
    fn live(
        state: &mut Self,
        env: CacheEnv,
    ) -> Option<&mut LruCache<CompletionQuery, Suggestions>> {
        state.capacity?;
        if state.env != Some(env) {
            state.env = Some(env);
            state.entries.clear();
        }
        Some(&mut state.entries)
    }
}

/// Cross-prompt completion cache, LRU by entry count; capacity `0` disables it.
#[derive(Clone)]
pub(crate) struct NarrowingCache {
    state: Arc<Mutex<CacheState>>,
}

impl Default for NarrowingCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_SIZE)
    }
}

impl NarrowingCache {
    /// `0` isn't a valid `LruCache` capacity; it means the cache is disabled.
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(CacheState {
                capacity: NonZeroUsize::new(capacity),
                env: None,
                // Never consulted while disabled; resized on enable.
                entries: LruCache::new(NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN)),
            })),
        }
    }

    /// Resize in place; `0` disables the cache. Reapplied each prompt.
    pub(crate) fn set_capacity(&self, capacity: usize) {
        if let Ok(mut state) = self.state.lock() {
            state.capacity = NonZeroUsize::new(capacity);
            match state.capacity {
                Some(capacity) => state.entries.resize(capacity),
                None => {
                    state.entries.clear();
                    state.env = None;
                }
            }
        }
    }

    pub(crate) fn fresh(
        &self,
        query: &CompletionQuery,
        environment: CacheEnv,
    ) -> Option<Suggestions> {
        let mut state = self.state.lock().ok()?;
        CacheState::live(&mut state, environment)?
            .get(query)
            .cloned()
    }

    pub(crate) fn store(
        &self,
        query: CompletionQuery,
        environment: CacheEnv,
        suggestions: Suggestions,
    ) {
        if let Ok(mut state) = self.state.lock()
            && let Some(cache) = CacheState::live(&mut state, environment)
        {
            cache.put(query, suggestions);
        }
    }

    pub(crate) fn narrowed_fallback(
        &self,
        query: &CompletionQuery,
        environment: CacheEnv,
        options: &CompletionOptions,
    ) -> Suggestions {
        let Some((base_suggestions, ref_span, search_token)) =
            self.state.lock().ok().and_then(|mut state| {
                let cache = CacheState::live(&mut state, environment)?;
                let (_, suggestions, span) = cache
                    .iter()
                    .filter_map(|(base, suggestions)| {
                        // The span the last suggestion replaces; the one the cursor extends.
                        let span = suggestions.last().map(|suggestion| suggestion.span)?;
                        query
                            .narrows(base, span)
                            .then_some((base.cursor(), suggestions, span))
                    })
                    .max_by_key(|&(cursor, ..)| cursor)?;

                let token = query.typed().get(span.start..)?;
                Some((Arc::clone(suggestions), span, token))
            })
        else {
            return Suggestions::default();
        };

        // Keep the producer's ranking: re-sorting a stale list makes the menu flip between keystrokes.
        let mut matcher = NuMatcher::new(search_token, options, false);

        base_suggestions
            .iter()
            .enumerate()
            .filter(|(_, s)| s.span == ref_span)
            .for_each(|(i, s)| {
                matcher.add(s.display_value(), i);
            });

        let updated_span = reedline::Span::new(ref_span.start, query.cursor());

        matcher
            .results()
            .into_iter()
            .map(|(index, match_indices)| {
                let mut suggestion = base_suggestions[index].clone();
                suggestion.span = updated_span;
                suggestion.match_indices = Some(match_indices);
                suggestion
            })
            .collect()
    }
}

struct Completed {
    query: CompletionQuery,
    suggestions: Suggestions,
    cacheable: bool,
}

struct CompletionWorker {
    request_tx: mpsc::Sender<CompletionQuery>,
    result_rx: mpsc::Receiver<Completed>,
    pending: Option<CompletionQuery>,
}

/// The completion behaviour configured in `$env.config.completions`.
fn configured_options(config: &Config) -> CompletionOptions {
    CompletionOptions {
        case_sensitive: config.completions.case_sensitive,
        match_algorithm: config.completions.algorithm.into(),
        sort: config.completions.sort,
        match_description: false,
    }
}

fn isolated_stack(parent: Arc<Stack>, suppress_stdin: bool) -> Arc<Stack> {
    let stack = Stack::with_parent(parent)
        .reset_out_dest()
        .suppress_output()
        .collect_value();
    Arc::new(if suppress_stdin {
        stack.suppress_stdin()
    } else {
        stack
    })
}

/// The user completer that would run first at `site`, mirroring dispatch order. `None` when
/// no user completer runs there.
fn site_completer(site: &CompletionSite, working_set: &StateWorkingSet) -> Option<SiteCompleter> {
    let call = match &site.kind {
        SiteKind::ExternalArg { .. } => return Some(SiteCompleter::External),
        kind => kind.call_site()?.call,
    };
    let signature = working_set.get_decl(call.decl_id).signature();

    // An argument's own completer runs before the command-wide one.
    let argument_completer = match &site.kind {
        SiteKind::FlagValue { flag, .. } => {
            find_flag(&signature, *flag).and_then(|flag| flag.completion)
        }
        SiteKind::Positional { sig_positional, .. } => signature
            .get_positional(*sig_positional)
            .and_then(|positional| positional.completion.clone()),
        _ => None,
    };
    if let Some(Completion::Command(decl_id)) = argument_completer {
        return Some(SiteCompleter::Decl(decl_id));
    }

    match signature.complete {
        Some(CommandWideCompleter::Command(decl_id)) => Some(SiteCompleter::Decl(decl_id)),
        Some(CommandWideCompleter::External) => Some(SiteCompleter::External),
        None => None,
    }
}

/// The user completer a site would run.
enum SiteCompleter {
    /// A named declaration carrying `@interactive` directly.
    Decl(DeclId),
    /// The configured external closure; interactivity is read from the command it dispatches to.
    External,
}

/// Whether `decl` names an `@interactive` command, seeing through aliases as execution does.
pub(crate) fn decl_is_interactive(working_set: &StateWorkingSet, decl_id: DeclId) -> bool {
    fn command_is_interactive(command: &dyn Command) -> bool {
        command
            .attributes()
            .iter()
            .any(|(name, value)| name == "interactive" && value.as_bool().unwrap_or(false))
            || command
                .as_alias()
                .and_then(|alias| alias.command.as_deref())
                .is_some_and(command_is_interactive)
    }
    command_is_interactive(working_set.get_decl(decl_id))
}

/// Whether the configured external completer closure runs an `@interactive` command. A closure
/// cannot carry the attribute itself, so we see through it to the command producing its result
/// — the terminal call of its block — exactly as a named completer carries the attribute.
pub(crate) fn closure_is_interactive(working_set: &StateWorkingSet, closure: &Closure) -> bool {
    let block = working_set.get_block(closure.block_id);
    matches!(
        block
            .pipelines
            .last()
            .and_then(|pipeline| pipeline.elements.last())
            .map(|element| &element.expr.expr),
        Some(Expr::Call(call)) if decl_is_interactive(working_set, call.decl_id)
    )
}

/// What the cursor is completing; each variant carries exactly the AST it needs.
#[derive(Debug, Clone)]
pub(crate) enum SiteKind<'a> {
    /// A command head; `node` is the whole call (for `^`/`%` sigils).
    Command { node: Option<&'a Expression> },
    /// A flag name being typed (`--`, `-x`).
    FlagName(CallSite<'a>),
    /// The value of a flag; `flag` keeps long/short identity.
    FlagValue {
        site: CallSite<'a>,
        flag: FlagRef<'a>,
        arg_slot: usize,
    },
    /// A positional argument; `sig_positional` indexes the signature.
    Positional {
        site: CallSite<'a>,
        sig_positional: usize,
        arg_slot: usize,
    },
    /// A binary-operator position trailing `lhs`.
    Operator { lhs: &'a Expression },
    /// A cell path into `path`.
    CellPath { path: &'a FullCellPath },
    /// A `$var` name.
    Variable,
    /// An attribute name.
    AttributeName,
    /// The item an attribute block decorates.
    AttributableItem,
    /// An argument of a bare external call.
    ExternalArg { call: &'a Expression, index: usize },
    /// A file path (the fallback).
    File,
}

impl<'a> SiteKind<'a> {
    /// A command head backed by its whole call (for sigil detection).
    fn command(node: &'a Expression) -> Self {
        Self::Command { node: Some(node) }
    }

    /// The [`ResolvedCursor`] a user completer sees.
    fn resolved(&self) -> ResolvedCursor<'a> {
        match *self {
            Self::Command { .. } => ResolvedCursor::Command,
            Self::FlagName { .. } => ResolvedCursor::FlagName,
            Self::FlagValue { flag, .. } => ResolvedCursor::FlagValue { flag: flag.name() },
            Self::Positional { sig_positional, .. } => ResolvedCursor::Positional {
                index: sig_positional,
            },
            Self::Operator { .. } => ResolvedCursor::Operator,
            Self::CellPath { .. } => ResolvedCursor::CellPath,
            Self::Variable => ResolvedCursor::Variable,
            Self::AttributeName => ResolvedCursor::AttributeName,
            Self::AttributableItem => ResolvedCursor::AttributableItem,
            Self::ExternalArg { index, .. } => ResolvedCursor::ExternalArg { index },
            Self::File => ResolvedCursor::File,
        }
    }

    /// The element this kind is backed by, if any.
    fn element(&self) -> Option<&'a Expression> {
        match *self {
            Self::Command { node } => node,
            Self::FlagName(site) | Self::FlagValue { site, .. } | Self::Positional { site, .. } => {
                Some(site.element)
            }
            Self::ExternalArg { call, .. } => Some(call),
            _ => None,
        }
    }

    /// The call this kind completes arguments for, if any.
    fn call_site(&self) -> Option<CallSite<'a>> {
        match *self {
            Self::FlagName(site) | Self::FlagValue { site, .. } | Self::Positional { site, .. } => {
                Some(site)
            }
            _ => None,
        }
    }
}

/// What the cursor completes, as a user completer sees it in `cursor.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedCursor<'a> {
    /// A command head.
    Command,
    /// A flag name.
    FlagName,
    /// The value of a flag; `flag` is the long name.
    FlagValue { flag: &'a str },
    /// A positional argument.
    Positional { index: usize },
    /// A binary-operator position.
    Operator,
    /// A cell path.
    CellPath,
    /// A `$var` name.
    Variable,
    /// An attribute name.
    AttributeName,
    /// The item an attribute block decorates.
    AttributableItem,
    /// An argument of a bare external call; externals have no signature, so flags count.
    ExternalArg { index: usize },
    /// A file path.
    File,
}

impl ResolvedCursor<'_> {
    /// The `cursor.kind` discriminant.
    fn kind(&self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::FlagName => "flag-name",
            Self::FlagValue { .. } => "flag-value",
            Self::Positional { .. } => "positional",
            Self::Operator => "operator",
            Self::CellPath => "cell-path",
            Self::Variable => "variable",
            Self::AttributeName => "attribute-name",
            Self::AttributableItem => "attributable-item",
            Self::ExternalArg { .. } => "external-arg",
            Self::File => "file",
        }
    }

    /// The record a completer's `cursor` gets; the caller adds `token`/`byte`.
    pub(crate) fn into_record(self, span: Span) -> Record {
        let mut record = Record::new();
        record.insert("kind", Value::string(self.kind(), span));

        match self {
            Self::FlagValue { flag } => record.insert("flag", Value::string(flag, span)),
            Self::Positional { index } | Self::ExternalArg { index } => {
                record.insert("index", Value::int(index as i64, span))
            }
            _ => None,
        };

        record
    }
}

/// One nesting level the cursor lives in, as a completer sees it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CompletionContext<'a> {
    /// What the cursor completes at this level.
    pub cursor: ResolvedCursor<'a>,
    /// The element this level is; the source of its tokens.
    pub element: Option<&'a Expression>,
    /// The nesting expression the cursor descends into (all but the last level).
    pub descent: Option<Span>,
}

/// A resolved completion site: kind, span, typed prefix, cursor, and enclosing contexts.
#[derive(Debug, Clone)]
pub(crate) struct CompletionSite<'a> {
    pub kind: SiteKind<'a>,
    pub span: Span,
    pub typed_prefix: Cow<'a, str>,
    /// The cursor, in absolute working-set (span) coordinates.
    pub cursor: usize,
    /// Enclosing contexts, outermost first; empty until resolved.
    pub contexts: Vec<CompletionContext<'a>>,
}

impl<'a> CompletionSite<'a> {
    /// Site at cursor; asserts span contains cursor.
    fn at(cursor: usize, kind: SiteKind<'a>, span: Span) -> Self {
        debug_assert!(
            touches(span, cursor),
            "a {} site spans {}..{}, which the cursor at {cursor} is not on",
            kind.resolved().kind(),
            span.start,
            span.end,
        );
        Self {
            kind,
            span,
            typed_prefix: Cow::Borrowed(""),
            cursor: 0,
            contexts: Vec::new(),
        }
    }

    /// This site as its own single context.
    fn own_context(&self) -> CompletionContext<'a> {
        CompletionContext {
            cursor: self.kind.resolved(),
            element: self.kind.element(),
            descent: None,
        }
    }
}

/// Completions for one commandline against borrowed state.
pub struct CompletionEngine<'a> {
    engine_state: &'a EngineState,
    /// An isolated child of the caller's stack.
    stack: Arc<Stack>,
    options: CompletionOptions,
}

/// The commandline facts constant for one dispatch.
#[derive(Clone, Copy)]
pub(crate) struct Buffer<'a> {
    /// The commandline (or file) up to the cursor.
    pub text: &'a str,
    /// Start of `text` in working-set coordinates; `span - offset` indexes `text`.
    pub offset: usize,
}

impl Buffer<'_> {
    /// The cursor as a byte offset into [`Self::text`].
    fn cursor(&self) -> usize {
        self.text.len()
    }

    /// The cursor in working-set (span) coordinates.
    fn absolute_cursor(&self) -> usize {
        self.offset + self.text.len()
    }

    /// A working-set offset as a byte index into [`Self::text`], saturating so
    /// spans before the buffer can't underflow.
    fn relative(&self, absolute: usize) -> usize {
        absolute.saturating_sub(self.offset)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Context<'a> {
    pub working_set: &'a StateWorkingSet<'a>,
    pub stack: &'a Stack,
    pub options: &'a CompletionOptions,
    pub span: Span,
    pub prefix: &'a [u8],
    pub offset: usize,
    /// The commandline up to the cursor. See [`Buffer`].
    pub buffer: &'a str,
    /// The nesting levels the cursor lives in, read by a completer declaring `contexts`.
    pub contexts: &'a [CompletionContext<'a>],
}

impl<'a> Context<'a> {
    pub(crate) fn prefix_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.prefix)
    }

    /// Attach a resolved site, read by a completer declaring a `contexts` parameter.
    fn at_site(mut self, site: &'a CompletionSite<'a>) -> Self {
        self.contexts = &site.contexts;
        self
    }
}

impl<'engine> CompletionEngine<'engine> {
    /// A completer for one synchronous query; only the stack is copied.
    pub fn new(engine_state: &'engine EngineState, stack: &Stack) -> Self {
        Self::isolated(engine_state, Arc::new(stack.clone()), false)
    }

    /// Like [`Self::new`], on a shared, stdin-suppressed stack.
    fn isolated(
        engine_state: &'engine EngineState,
        stack: Arc<Stack>,
        suppress_stdin: bool,
    ) -> Self {
        let stack = isolated_stack(stack, suppress_stdin);
        let options = configured_options(&stack.get_config(engine_state));
        Self {
            engine_state,
            stack,
            options,
        }
    }

    /// Answer one request, returning suggestions and whether the result is cacheable.
    fn suggestions_for(&self, query: &CompletionQuery) -> (Suggestions, bool) {
        let dispatched = self.dispatch_completions_at(query.typed(), query.cursor());
        let reusable = dispatched.is_reusable();
        let suggestions = dispatched
            .into_suggestions()
            .into_iter()
            .map(|semantic_suggestion| semantic_suggestion.suggestion)
            .collect();

        (suggestions, reusable)
    }

    pub fn fetch_completions_at(&self, line: &str, position: usize) -> Vec<SemanticSuggestion> {
        self.dispatch_completions_at(line, position)
            .into_suggestions()
    }

    /// Input record for a completer, menu source, or `commandline complete --input`.
    /// Site identity (`$token`/`$place`) is always the parsed site; reedline's
    /// replacement range never enters here.
    pub fn completer_input_at(&self, line: &str, position: usize, wanted: DeclaredInputs) -> Value {
        self.input_at(line, position, wanted)
    }

    fn input_at(&self, line: &str, position: usize, wanted: DeclaredInputs) -> Value {
        let cursor = line.floor_char_boundary(position.min(line.len()));
        let sliced_line = &line[..cursor];

        let mut working_set = StateWorkingSet::new(self.engine_state);
        let offset = working_set.next_span_start();
        let block = parse(
            &mut working_set,
            Some("completer"),
            sliced_line.as_bytes(),
            false,
        );

        let buffer = Buffer {
            text: sliced_line,
            offset,
        };
        let site = Self::resolve_completion_site(&block, &working_set, buffer, sliced_line);

        completer_input(
            &self
                .context(
                    &working_set,
                    buffer,
                    site.span,
                    site.typed_prefix.as_bytes(),
                )
                .at_site(&site),
            wanted,
        )
    }

    /// Whether the completer that would run at `position` must run inline (`@interactive`)
    /// rather than on the background worker. Parses and resolves the site; runs no completer.
    pub(crate) fn interactive_completer_at(&self, line: &str, position: usize) -> bool {
        let safe_position = line.floor_char_boundary(position);
        let sliced_line = &line[..safe_position];

        let mut working_set = StateWorkingSet::new(self.engine_state);
        let offset = working_set.next_span_start();
        let block = parse(
            &mut working_set,
            Some("completer"),
            sliced_line.as_bytes(),
            false,
        );

        let buffer = Buffer {
            text: sliced_line,
            offset,
        };
        let site = Self::resolve_completion_site(&block, &working_set, buffer, sliced_line);

        self.site_is_interactive(&site, &working_set)
            || self
                .with_shorter_head_reading(
                    &site,
                    &working_set,
                    buffer,
                    sliced_line,
                    |engine, shorter, shorter_ws| engine.site_is_interactive(shorter, shorter_ws),
                )
                .unwrap_or(false)
    }

    /// Whether the completer that would run first at `site` is interactive.
    fn site_is_interactive(&self, site: &CompletionSite, working_set: &StateWorkingSet) -> bool {
        match site_completer(site, working_set) {
            Some(SiteCompleter::Decl(decl_id)) => decl_is_interactive(working_set, decl_id),
            Some(SiteCompleter::External) => self
                .stack
                .get_config(self.engine_state)
                .completions
                .external
                .completer
                .as_ref()
                .is_some_and(|closure| closure_is_interactive(working_set, closure)),
            None => false,
        }
    }

    fn dispatch_completions_at(&self, line: &str, position: usize) -> Fetched {
        let safe_position = line.floor_char_boundary(position);
        // Parse only up to the cursor.
        let sliced_line = &line[..safe_position];

        let mut working_set = StateWorkingSet::new(self.engine_state);
        let span_offset = working_set.next_span_start();

        let block = parse(
            &mut working_set,
            Some("completer"),
            sliced_line.as_bytes(),
            false,
        );

        self.fetch_completions_by_block(
            block,
            &working_set,
            Buffer {
                text: sliced_line,
                offset: span_offset,
            },
            sliced_line,
        )
    }

    pub fn fetch_completions_within_file(
        &self,
        filename: &str,
        position: usize,
        contents: &str,
    ) -> Vec<SemanticSuggestion> {
        let mut working_set = StateWorkingSet::new(self.engine_state);

        // `parse` must run first: it registers the file and its spans in `working_set`.
        let block = parse(&mut working_set, Some(filename), contents.as_bytes(), false);

        let Some(file_span) = working_set.get_span_for_filename(filename) else {
            return Vec::new();
        };

        self.fetch_completions_by_block(
            block,
            &working_set,
            Buffer {
                text: contents.get(..position).unwrap_or(contents),
                offset: file_span.start,
            },
            contents,
        )
        .into_suggestions()
    }

    /// `buffer` is the commandline; `contents` is the text the block was parsed from.
    fn fetch_completions_by_block(
        &self,
        block: Arc<Block>,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        contents: &str,
    ) -> Fetched {
        let site = Self::resolve_completion_site(&block, working_set, buffer, contents);
        let mut dispatched = self.dispatch_completion_site(&site, working_set, buffer);

        // A multi-word head is ambiguous: offer the shorter command's argument reading too.
        dispatched.prepend_from(self.complete_multiword_head_as_argument(
            &site,
            working_set,
            buffer,
            contents,
        ));
        dispatched
    }

    /// `baz --test bar` can also read as the value of `baz --test`'s flag.
    fn complete_multiword_head_as_argument(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        contents: &str,
    ) -> Fetched {
        self.with_shorter_head_reading(
            site,
            working_set,
            buffer,
            contents,
            |engine, shorter, shorter_ws| {
                let mut dispatched = engine.dispatch_completion_site(shorter, shorter_ws, buffer);
                // Keep only the argument value; drop command-kind results.
                dispatched.retain(|candidate| {
                    !matches!(candidate.kind, Some(SuggestionKind::Command(..)))
                });
                dispatched
            },
        )
        .unwrap_or_default()
    }

    /// Run `f` on the shorter command's reading of an ambiguous multi-word head (`foo bar`
    /// also reading as `foo`'s argument). One re-parse shared by dispatch and interactive
    /// routing so they cannot drift. `None` when there is no distinct argument site.
    fn with_shorter_head_reading<T>(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        contents: &str,
        f: impl FnOnce(&Self, &CompletionSite, &StateWorkingSet) -> T,
    ) -> Option<T> {
        if !matches!(site.kind, SiteKind::Command { .. })
            || !working_set
                .get_span_contents(site.span)
                .iter()
                .any(u8::is_ascii_whitespace)
        {
            return None;
        }

        // Owns the spans the resolved site borrows.
        let mut shorter_ws = StateWorkingSet::new(self.engine_state);
        let _ = shorter_ws.add_file("completer", contents.as_bytes());
        let shorter = parse_shorter_head_reading(&mut shorter_ws, site.span, None)?;

        let mut shorter_site = Self::finalize_site(
            Self::resolve_expression_site(&shorter, site.cursor, &shorter_ws),
            buffer,
            contents,
        );
        // A command-head result means no distinct argument; leave it to the primary dispatch.
        if matches!(shorter_site.kind, SiteKind::Command { .. }) {
            return None;
        }
        // A single-expression re-parse has no enclosing chain.
        shorter_site.contexts = vec![shorter_site.own_context()];

        Some(f(self, &shorter_site, &shorter_ws))
    }

    /// Dispatch the site to the appropriate specialized completer.
    fn dispatch_completion_site<'a>(
        &'a self,
        site: &'a CompletionSite<'a>,
        working_set: &'a StateWorkingSet,
        buffer: Buffer<'a>,
    ) -> Fetched {
        let completion_context = self
            .context(working_set, buffer, site.span, site.typed_prefix.as_bytes())
            .at_site(site);

        match &site.kind {
            SiteKind::Command { node } => {
                let completions = self.command_completion_helper(
                    working_set,
                    buffer,
                    site.span,
                    self.command_completion_for_head(*node, site.span, working_set),
                );

                if completions.is_empty() {
                    self.suggestions_at(&mut FileCompletion, &completion_context)
                } else {
                    completions
                }
            }

            SiteKind::FlagName(_) | SiteKind::FlagValue { .. } | SiteKind::Positional { .. } => {
                self.dispatch_call_completion_site(site, working_set, buffer, &completion_context)
            }

            SiteKind::Operator { lhs } => OperatorCompletion {
                left_hand_side: lhs,
            }
            .fetch(&completion_context),

            SiteKind::CellPath { path } => CellPathCompletion {
                full_cell_path: path,
                cursor: site.cursor,
            }
            .fetch(&completion_context),

            SiteKind::Variable => self.variable_names_completion_helper(&completion_context),

            SiteKind::AttributeName => AttributeCompletion.fetch(&completion_context),

            SiteKind::AttributableItem => AttributableCompletion.fetch(&completion_context),

            SiteKind::ExternalArg { .. } => {
                self.dispatch_external_arg(site, working_set, buffer, &completion_context)
            }

            SiteKind::File => self.suggestions_at(&mut FileCompletion, &completion_context),
        }
    }

    /// Complete an external call argument.
    fn dispatch_external_arg(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        completion_context: &Context,
    ) -> Fetched {
        let SiteKind::ExternalArg {
            call: external_call,
            index,
        } = &site.kind
        else {
            return Fetched::default();
        };
        let external_call = *external_call;
        let Expr::ExternalCall(head, _) = &external_call.expr else {
            return Fetched::default();
        };

        // The first argument of `sudo`/`doas` is a command run under the wrapper.
        if *index == 0 {
            let head_command = working_set.get_span_contents(head.span);
            if head_command == b"sudo" || head_command == b"doas" {
                let commands = self.command_completion_helper(
                    working_set,
                    buffer,
                    site.span,
                    CommandCompletion::new(CommandScope::All),
                );
                if !commands.is_empty() {
                    return commands;
                }
            }
        }

        // Neutral seed: answered until a source declines, unreusable until one keeps.
        let mut dispatched = Fetched::default();

        // The user's configured external completer.
        let external_answered = self
            .stack
            .get_config(self.engine_state)
            .completions
            .external
            .completer
            .as_ref()
            .is_some_and(|completer| {
                dispatched.absorb(
                    UserCompletion::closure(working_set, completer).fetch(completion_context),
                )
            });

        // Subcommands extending this call suppress the file fallback.
        let subcommands =
            self.subcommand_suggestions(working_set, buffer, external_call.span.start, site.cursor);

        // Add file completion when the source leaves the slot open.
        let wants_file = subcommands.is_empty()
            && (!dispatched.answered() || (!external_answered && dispatched.is_empty()));
        if wants_file {
            dispatched.merge(self.suggestions_at(&mut FileCompletion, completion_context));
        }

        dispatched.merge(subcommands);
        dispatched
    }

    /// Dispatch completions for call-bound sites.
    fn dispatch_call_completion_site(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        completion_context: &Context,
    ) -> Fetched {
        // Only call-bound kinds carry a call; anything else is an error here.
        let call = match &site.kind {
            SiteKind::FlagName(site)
            | SiteKind::FlagValue { site, .. }
            | SiteKind::Positional { site, .. } => site.call,
            _ => return Fetched::default(),
        };

        let signature = working_set.get_decl(call.decl_id).signature();

        // Subcommands are always offered and suppress the file-path fallback.
        let subcommands =
            self.subcommand_suggestions(working_set, buffer, call.head.start, site.cursor);

        // Value kinds share one shape; only the `ArgType`, completer lookup, and the
        // `declared_shape` used for the empty-slot fallback differ.
        let argument_value = |engine: &Self, arg_type, custom, arg_slot, declared_shape| {
            engine.complete_argument_value(
                custom,
                ArgValueCompletion {
                    call,
                    arg_type,
                    need_fallback: subcommands.is_empty(),
                    arg_idx: arg_slot,
                    declared_shape,
                    cursor: site.cursor,
                },
                completion_context,
                &signature,
            )
        };

        let mut results = match &site.kind {
            SiteKind::FlagName(_) => {
                self.complete_flag_names(call.decl_id, completion_context, &signature)
            }
            SiteKind::FlagValue { flag, arg_slot, .. } => {
                let resolved = find_flag(&signature, *flag);
                argument_value(
                    self,
                    ArgType::Flag(Cow::Borrowed(flag.name())),
                    resolved.as_ref().and_then(|flag| flag.completion.clone()),
                    *arg_slot,
                    resolved.as_ref().and_then(|flag| flag.arg.clone()),
                )
            }
            SiteKind::Positional {
                sig_positional,
                arg_slot,
                ..
            } => {
                let positional = signature.get_positional(*sig_positional);
                argument_value(
                    self,
                    ArgType::Positional(*sig_positional),
                    positional.and_then(|positional| positional.completion.clone()),
                    *arg_slot,
                    positional.map(|positional| positional.shape.clone()),
                )
            }
            _ => Fetched::default(),
        };

        results.merge(subcommands);
        results
    }

    /// Resolve the contextual state and constraints at the cursor's location.
    pub(crate) fn resolve_completion_site<'a>(
        block: &'a Block,
        working_set: &'a StateWorkingSet,
        buffer: Buffer,
        contents: &'a str,
    ) -> CompletionSite<'a> {
        let absolute_position = buffer.absolute_cursor();

        // The closures and subexpressions the cursor is nested in, outermost first.
        let chain = enclosing_elements(block, working_set, absolute_position);

        // The token whose span the cursor is inside of, or at the trailing edge of.
        let touched_expression = block
            .find_map(working_set, &|expression: &Expression| {
                find_pipeline_element_by_position(expression, working_set, absolute_position)
            })
            .or_else(|| check_redirection_in_block(block, absolute_position));

        let mut site = match innermost_expression(touched_expression, &chain) {
            Some(expression) => {
                Self::resolve_expression_site(expression, absolute_position, working_set)
            }
            None => Self::resolve_fallback_site(block, working_set, absolute_position),
        };

        site = promote_row_condition_file(site, &chain, working_set, absolute_position);

        site.contexts = Self::contexts_of(&chain, working_set, absolute_position, &site);
        Self::finalize_site(site, buffer, contents)
    }

    /// The chain of contexts the cursor lives in, outermost first. `site` supplies the
    /// innermost one, so it stays the resolution the dispatcher itself acts on rather than
    /// a second, possibly disagreeing, reading of the same position.
    fn contexts_of<'a>(
        chain: &[(&'a Expression, Option<Span>)],
        working_set: &'a StateWorkingSet,
        absolute_position: usize,
        site: &CompletionSite<'a>,
    ) -> Vec<CompletionContext<'a>> {
        let target_element = site.kind.element();

        // Locate the target element's index or default to the final index safely.
        let innermost_index = target_element
            .and_then(|target| {
                chain
                    .iter()
                    .position(|(candidate, _)| std::ptr::eq(*candidate, target))
            })
            .unwrap_or_else(|| chain.len().saturating_sub(1));

        chain[..innermost_index]
            .iter()
            .map(|&(expression, descent)| CompletionContext {
                cursor: Self::resolve_expression_site(expression, absolute_position, working_set)
                    .kind
                    .resolved(),
                element: Some(expression),
                descent,
            })
            .chain(std::iter::once(CompletionContext {
                element: target_element.or_else(|| {
                    chain
                        .get(innermost_index)
                        .map(|&(expression, _)| expression)
                }),
                ..site.own_context()
            }))
            .collect()
    }

    /// Fill `typed_prefix`/`cursor` from the final span so they never disagree.
    fn finalize_site<'a>(
        mut site: CompletionSite<'a>,
        buffer: Buffer,
        contents: &'a str,
    ) -> CompletionSite<'a> {
        let token_start = buffer.relative(site.span.start);
        site.typed_prefix = contents
            .get(token_start..buffer.cursor())
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed(""));
        site.cursor = buffer.absolute_cursor();
        site
    }

    fn resolve_expression_site<'a>(
        expression: &'a Expression,
        absolute_position: usize,
        working_set: &'a StateWorkingSet,
    ) -> CompletionSite<'a> {
        // Cursor past expression: only calls/attribute blocks own trailing gap.
        if absolute_position > expression.span.end && !owns_trailing_gap(&expression.expr) {
            let kind = if is_operator_lhs(&expression.expr) {
                SiteKind::Operator { lhs: expression }
            } else {
                SiteKind::File
            };
            return CompletionSite::at(absolute_position, kind, Span::point(absolute_position));
        }

        // Default to file completion; overridden below where the expression warrants it.
        match &expression.expr {
            Expr::Call(call) => Self::resolve_call_site(
                CallSite {
                    call,
                    element: expression,
                },
                absolute_position,
                working_set,
            ),
            Expr::ExternalCall(head, arguments) => {
                Self::resolve_external_call_site(expression, head, arguments, absolute_position)
            }
            Expr::AttributeBlock(attribute_block) => {
                Self::resolve_attribute_site(attribute_block, absolute_position)
            }
            Expr::Var(_) => {
                CompletionSite::at(absolute_position, SiteKind::Variable, expression.span)
            }
            // `$foo` alone is the variable; `$foo.bar` or `$foo.` is a cell path.
            Expr::FullCellPath(full_cell_path) => {
                let has_dot = working_set
                    .get_span_contents(expression.span)
                    .ends_with(b".");

                let kind = if full_cell_path.tail.is_empty() && !has_dot {
                    SiteKind::Variable
                } else {
                    SiteKind::CellPath {
                        path: full_cell_path,
                    }
                };

                CompletionSite::at(absolute_position, kind, expression.span)
            }
            // Operator site only on its token; otherwise resolve value side.
            Expr::BinaryOp(left_hand_side, operator, right_hand_side) => {
                let value_side = match absolute_position {
                    before if before < operator.span.start => Some(left_hand_side),
                    after if after > operator.span.end => Some(right_hand_side),
                    _ => None,
                };

                match value_side {
                    Some(side) => {
                        Self::resolve_expression_site(side, absolute_position, working_set)
                    }
                    None => CompletionSite::at(
                        absolute_position,
                        SiteKind::Operator {
                            lhs: left_hand_side.as_ref(),
                        },
                        operator.span,
                    ),
                }
            }
            _ => CompletionSite::at(absolute_position, SiteKind::File, expression.span),
        }
    }

    /// Resolve a bare external call; the head completes as a command, else
    /// [`SiteKind::ExternalArg`].
    fn resolve_external_call_site<'a>(
        expression: &'a Expression,
        head: &'a Expression,
        arguments: &'a [ExternalArgument],
        absolute_position: usize,
    ) -> CompletionSite<'a> {
        if absolute_position <= head.span.end {
            return CompletionSite::at(
                absolute_position,
                SiteKind::command(expression),
                command_name_span(head.span, expression.span),
            );
        }

        // An existing argument the cursor touches, or else the trailing empty slot.
        let (index, span) = arguments
            .iter()
            .enumerate()
            .find_map(|(index, argument)| {
                touches(argument.expr().span, absolute_position)
                    .then_some((index, argument.expr().span))
            })
            .unwrap_or((arguments.len(), Span::point(absolute_position)));

        CompletionSite::at(
            absolute_position,
            SiteKind::ExternalArg {
                call: expression,
                index,
            },
            span,
        )
    }

    /// Classify the cursor in a call, most specific first: head, existing argument,
    /// row-condition operator/value gaps, then the trailing slot.
    fn resolve_call_site<'a>(
        site: CallSite<'a>,
        absolute_position: usize,
        working_set: &'a StateWorkingSet,
    ) -> CompletionSite<'a> {
        if absolute_position <= site.call.head.end {
            return CompletionSite::at(
                absolute_position,
                SiteKind::command(site.element),
                command_name_span(site.call.head, site.element.span),
            );
        }
        Self::argument_touching(site, absolute_position, working_set)
            .or_else(|| Self::row_condition_operator(site, working_set, absolute_position))
            .or_else(|| Self::row_condition_value_gap(site, working_set, absolute_position))
            .unwrap_or_else(|| Self::trailing_slot(site, working_set, absolute_position))
    }

    /// The existing argument the cursor touches, if any.
    fn argument_touching<'a>(
        site: CallSite<'a>,
        absolute_position: usize,
        working_set: &StateWorkingSet,
    ) -> Option<CompletionSite<'a>> {
        let (argument_index, argument) = site
            .call
            .arguments
            .iter()
            .enumerate()
            .find(|(_, argument)| touches(argument.span(), absolute_position))?;
        Some(Self::resolve_argument_site(
            site,
            argument,
            argument_index,
            absolute_position,
            working_set,
        ))
    }

    /// Classify the trailing slot (flag value, flag name, or positional) by the
    /// trailing non-whitespace token.
    fn trailing_slot<'a>(
        site: CallSite<'a>,
        working_set: &StateWorkingSet,
        absolute_position: usize,
    ) -> CompletionSite<'a> {
        let CallSite { call, .. } = site;
        let gap_start = call
            .arguments
            .last()
            .map_or(call.head.end, |argument| argument.span().end);

        let gap = working_set.get_span_contents(Span::new(gap_start, absolute_position));

        // Start just past the last whitespace in the gap.
        let token_start = gap
            .iter()
            .rposition(u8::is_ascii_whitespace)
            .map_or(gap_start, |index| gap_start + index + 1);

        let trailing_token = Span::new(token_start, absolute_position);
        let token_is_flag = is_flag_token(working_set, trailing_token);

        let point = Span::point(absolute_position);

        if let Some(flag_ref) = Self::pending_flag_value(call, working_set) {
            // `arg_slot` past the last argument: no node exists yet; `ArgValueCompletion` reads `None`.
            CompletionSite::at(
                absolute_position,
                SiteKind::FlagValue {
                    site,
                    flag: flag_ref,
                    arg_slot: call.arguments.len(),
                },
                point,
            )
        } else if token_is_flag {
            CompletionSite::at(absolute_position, SiteKind::FlagName(site), trailing_token)
        } else {
            CompletionSite::at(
                absolute_position,
                SiteKind::Positional {
                    site,
                    sig_positional: count_positionals(call, call.arguments.len()),
                    arg_slot: call.arguments.len(),
                },
                point,
            )
        }
    }

    /// Trailing `where name ⌶` is an operator position.
    fn row_condition_operator<'a>(
        site: CallSite<'a>,
        working_set: &'a StateWorkingSet,
        absolute_position: usize,
    ) -> Option<CompletionSite<'a>> {
        let last_term = RowCondition::trailing(site)?.last_term(working_set)?;
        (absolute_position > last_term.span.end
            && is_operator_lhs(&last_term.expr)
            && working_set
                .get_span_contents(Span::new(last_term.span.end, absolute_position))
                .iter()
                .all(u8::is_ascii_whitespace))
        .then(|| {
            CompletionSite::at(
                absolute_position,
                SiteKind::Operator { lhs: last_term },
                Span::point(absolute_position),
            )
        })
    }

    /// `where size > ⌶` / `where size > 1 ⌶` is still the condition, not a new positional.
    fn row_condition_value_gap<'a>(
        site: CallSite<'a>,
        working_set: &'a StateWorkingSet,
        absolute_position: usize,
    ) -> Option<CompletionSite<'a>> {
        let cond = RowCondition::trailing(site)?;
        let last_term = cond.last_term(working_set)?;
        let Expr::BinaryOp(_, operator, _) = &last_term.expr else {
            return None;
        };
        (absolute_position >= cond.arg_span.end
            && absolute_position > operator.span.end
            && working_set
                .get_span_contents(Span::new(cond.arg_span.end, absolute_position))
                .iter()
                .all(u8::is_ascii_whitespace))
        .then(|| {
            cond.site_at(
                working_set,
                absolute_position,
                Span::point(absolute_position),
            )
        })
    }

    /// The [`FlagRef`] of a last-argument flag still awaiting its value.
    fn pending_flag_value<'a>(
        call: &'a Call,
        working_set: &StateWorkingSet,
    ) -> Option<FlagRef<'a>> {
        let Argument::Named((name, short, None)) = call.arguments.last()? else {
            return None;
        };

        let flag_ref = FlagRef::from_named(name, short.as_ref());
        let signature = working_set.get_decl(call.decl_id).signature();

        find_flag(&signature, flag_ref)?
            .arg
            .is_some()
            .then_some(flag_ref)
    }

    fn resolve_argument_site<'a>(
        site: CallSite<'a>,
        argument: &'a Argument,
        argument_index: usize,
        absolute_position: usize,
        working_set: &StateWorkingSet,
    ) -> CompletionSite<'a> {
        let flag_name = SiteKind::FlagName(site);

        let (kind, span) = match argument {
            Argument::Named((name, short, optional_value)) => {
                if let Some(value_expression) = optional_value
                    .as_ref()
                    .filter(|value| touches(value.span, absolute_position))
                {
                    (
                        SiteKind::FlagValue {
                            site,
                            flag: FlagRef::from_named(name, short.as_ref()),
                            arg_slot: argument_index,
                        },
                        value_expression.span,
                    )
                } else {
                    // Only the name is completed; `Argument::span` would cover the value after it.
                    (flag_name, name.span)
                }
            }
            // A positional/unknown token starting with `-` is a flag name being typed.
            Argument::Positional(_) | Argument::Unknown(_) => {
                let kind = if is_flag_token(working_set, argument.span()) {
                    flag_name
                } else {
                    SiteKind::Positional {
                        site,
                        sig_positional: count_positionals(site.call, argument_index),
                        arg_slot: argument_index,
                    }
                };
                (kind, argument.span())
            }
            Argument::Spread(_) => (SiteKind::File, argument.span()),
        };

        CompletionSite::at(absolute_position, kind, span)
    }

    fn resolve_attribute_site<'a>(
        attribute_block: &'a AttributeBlock,
        absolute_position: usize,
    ) -> CompletionSite<'a> {
        if let Some(attribute) = attribute_block
            .attributes
            .iter()
            .find(|attribute| touches(attribute.expr.span, absolute_position))
        {
            return CompletionSite::at(
                absolute_position,
                SiteKind::AttributeName,
                attribute.expr.span,
            );
        }

        if touches(attribute_block.item.span, absolute_position) {
            return CompletionSite::at(
                absolute_position,
                SiteKind::AttributableItem,
                attribute_block.item.span,
            );
        }

        // Past the last attribute is the item's slot; earlier gaps type another attribute.
        let kind = match attribute_block.attributes.last() {
            Some(last) if absolute_position >= last.expr.span.end => SiteKind::AttributableItem,
            _ => SiteKind::AttributeName,
        };

        CompletionSite::at(absolute_position, kind, Span::point(absolute_position))
    }

    fn resolve_fallback_site<'a>(
        block: &'a Block,
        working_set: &'a StateWorkingSet,
        absolute_position: usize,
    ) -> CompletionSite<'a> {
        let last_element = block
            .pipelines
            .last()
            .and_then(|pipeline| pipeline.elements.last())
            .map(|element| &element.expr);

        // A bare `@` opens an attribute name; trailing an attribute block completes its item.
        let kind = if last_element
            .map(|element| working_set.get_span_contents(element.span))
            .is_some_and(|bytes| bytes.ends_with(b"@"))
        {
            SiteKind::AttributeName
        } else if matches!(last_element.map(|e| &e.expr), Some(Expr::AttributeBlock(_))) {
            SiteKind::AttributableItem
        } else {
            SiteKind::Command { node: None }
        };

        CompletionSite::at(absolute_position, kind, Span::point(absolute_position))
    }
    fn complete_argument_value(
        &self,
        custom: Option<Completion>,
        mut arg_value: ArgValueCompletion,
        context: &Context,
        signature: &Signature,
    ) -> Fetched {
        // Neutral seed: answered until a source declines, unreusable until one keeps.
        let mut results = Fetched::default();

        if let Some(custom) = custom {
            let attempt = match custom {
                // A command declared an engine-provided completion for this argument.
                Completion::Builtin(kind) => self.complete_builtin(kind, &arg_value, context),
                // A user-defined completer, called with the unified input record.
                Completion::Command(decl_id) => {
                    match UserCompletion::parameter(context.working_set, decl_id) {
                        Some(mut completer) => completer.fetch(context),
                        None => {
                            // A builtin/plugin runs no block; empty beats dumping the directory.
                            log::error!(
                                "`{}` cannot be used as a completer: it runs no block",
                                context.working_set.get_decl(decl_id).name()
                            );
                            Fetched::answering(vec![]).worth_keeping()
                        }
                    }
                }
                Completion::List(list) => StaticCompletion::new(list).fetch(context),
            };
            if results.absorb(attempt) {
                return results;
            }
        }

        if results.absorb(self.command_wide_completion_helper(signature, context)) {
            return results;
        }

        // A fallthrough result keeps type-based completion enabled.
        arg_value.need_fallback &= results.is_empty() || !results.answered();
        results.merge(arg_value.fetch(context));
        results
    }

    /// Dispatch a [`BuiltinCompletion`] a command declared for its argument.
    fn complete_builtin(
        &self,
        kind: BuiltinCompletion,
        arg_value: &ArgValueCompletion,
        context: &Context,
    ) -> Fetched {
        match kind {
            BuiltinCompletion::NuFile { std_virtual_path } => {
                DotNuCompletion { std_virtual_path }.fetch(context)
            }
            BuiltinCompletion::ModuleExports => {
                arg_value.complete_module_exports(context, context.working_set)
            }
            BuiltinCompletion::EnvVar => EnvVarCompletion.fetch(context),
            BuiltinCompletion::Command { internal_only } => {
                let scope = if internal_only {
                    CommandScope::InternalsOnly
                } else {
                    CommandScope::All
                };
                CommandCompletion::quoted(scope).fetch(context)
            }
        }
    }

    fn complete_flag_names(
        &self,
        decl_id: DeclId,
        context: &Context,
        signature: &Signature,
    ) -> Fetched {
        let mut results = FlagCompletion { decl_id }.fetch(context);
        results.merge(self.command_wide_completion_helper(signature, context));
        results
    }

    fn suggestions_at<C: Completer>(&self, completer: &mut C, context: &Context) -> Fetched {
        completer.fetch(context)
    }

    fn variable_names_completion_helper(&self, context: &Context) -> Fetched {
        if !context.prefix.starts_with(b"$") {
            return Fetched::default();
        }
        VariableCompletion.fetch(context)
    }

    fn command_completion_helper(
        &self,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        span: Span,
        mut command_completion: CommandCompletion,
    ) -> Fetched {
        let prefix = working_set.get_span_contents(span);
        let ctx = self.context(working_set, buffer, span, prefix);
        command_completion.fetch(&ctx)
    }

    /// Command scope for a head, honouring a leading sigil: `^` → externals only, `%` →
    /// builtins only, otherwise everything. The sigil is the byte between the call's own
    /// span and its head span.
    fn command_completion_for_head(
        &self,
        node: Option<&Expression>,
        span: Span,
        working_set: &StateWorkingSet,
    ) -> CommandCompletion {
        let sigil = node
            .filter(|node| node.span.start < span.start)
            .and_then(|node| working_set.get_span_contents(node.span).first().copied());

        CommandCompletion::new(match sigil {
            Some(b'^') => CommandScope::ExternalsOnly,
            Some(b'%') => CommandScope::BuiltinsOnly,
            _ => CommandScope::All,
        })
    }

    /// Internal commands whose name extends the line so far (`foo test⌶` also offers
    /// `foo test bar`); externals are excluded, since a multi-word line names only
    /// internal subcommands.
    fn subcommand_suggestions(
        &self,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        command_start: usize,
        cursor: usize,
    ) -> Fetched {
        if cursor <= command_start {
            return Fetched::default();
        }
        self.command_completion_helper(
            working_set,
            buffer,
            Span::new(command_start, cursor),
            CommandCompletion::new(CommandScope::InternalsOnly),
        )
    }

    fn command_wide_completion_helper(&self, signature: &Signature, context: &Context) -> Fetched {
        let completion = match signature.complete {
            Some(CommandWideCompleter::Command(decl_id)) => {
                UserCompletion::command(context.working_set, decl_id)
            }
            Some(CommandWideCompleter::External) => self
                .stack
                .get_config(self.engine_state)
                .completions
                .external
                .completer
                .as_ref()
                .map(|closure| UserCompletion::closure(context.working_set, closure)),
            None => None,
        };

        match completion {
            // Answers for the whole call, never narrowed against the token prefix.
            Some(mut completion) => completion.fetch(&Context {
                prefix: b"",
                ..*context
            }),
            None => Fetched::absent(),
        }
    }

    pub(crate) fn context<'a>(
        &'a self,
        working_set: &'a StateWorkingSet,
        buffer: Buffer<'a>,
        span: Span,
        prefix: &'a [u8],
    ) -> Context<'a> {
        Context {
            working_set,
            stack: self.stack.as_ref(),
            options: &self.options,
            span,
            prefix,
            offset: buffer.offset,
            buffer: buffer.text,
            // No site resolved yet; `Context::at_site` fills the chain in.
            contexts: &[],
        }
    }
}

pub struct NuCompleter {
    engine_state: Arc<EngineState>,
    stack: Arc<Stack>,
    options: CompletionOptions,
    cache: NarrowingCache,
    cache_env: CacheEnv,
    worker: Option<CompletionWorker>,
    /// Latest answer and its query.
    latest: Option<Completed>,
}

impl NuCompleter {
    pub fn new(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self::with_cache(engine_state, stack, NarrowingCache::default())
    }

    pub(crate) fn with_cache(
        engine_state: Arc<EngineState>,
        stack: Arc<Stack>,
        cache: NarrowingCache,
    ) -> Self {
        let cache_env = CacheEnv::of(&engine_state, &stack);
        // Read fresh each prompt so `cache_size` config changes take effect.
        let config = stack.get_config(&engine_state);
        let cache_size = config.completions.cache_size;
        cache.set_capacity(cache_size.try_into().unwrap_or(0));
        Self {
            options: configured_options(&config),
            engine_state,
            stack,
            cache,
            cache_env,
            worker: None,
            latest: None,
        }
    }

    fn fresh_for(&self, query: &CompletionQuery) -> Option<Suggestions> {
        if let Some(latest) = &self.latest
            && &latest.query == query
        {
            return Some(latest.suggestions.clone());
        }
        self.cache.fresh(query, self.cache_env)
    }

    fn settle_pending(&mut self, query: &CompletionQuery) {
        if let Some(worker) = self.worker.as_mut()
            && worker.pending.as_ref() == Some(query)
        {
            worker.pending = None;
        }
    }

    fn stale_fallback(&self, query: &CompletionQuery) -> Suggestions {
        self.cache
            .narrowed_fallback(query, self.cache_env, &self.options)
    }

    fn spawn_worker(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> CompletionWorker {
        let (request_tx, request_rx) = mpsc::channel::<CompletionQuery>();
        let (result_tx, result_rx) = mpsc::channel::<Completed>();

        thread::spawn(move || {
            // The thread owns the state; nothing is cloned per query.
            let engine = CompletionEngine::isolated(&engine_state, stack, true);

            while let Ok(mut query) = request_rx.recv() {
                while let Ok(newer) = request_rx.try_recv() {
                    query = newer;
                }

                let (suggestions, cacheable) = engine.suggestions_for(&query);
                let done = Completed {
                    query,
                    suggestions,
                    cacheable,
                };
                if result_tx.send(done).is_err() {
                    return;
                }
            }
        });

        CompletionWorker {
            request_tx,
            result_rx,
            pending: None,
        }
    }

    fn fold_completed(&mut self, done: Completed) -> bool {
        let settled = self
            .worker
            .as_mut()
            .is_some_and(|worker| worker.pending.as_ref() == Some(&done.query));

        if done.cacheable {
            self.cache
                .store(done.query.clone(), self.cache_env, done.suggestions.clone());
        }
        self.latest = Some(done);
        settled
    }

    fn try_recv_completed(&self) -> Option<Completed> {
        self.worker.as_ref()?.result_rx.try_recv().ok()
    }

    fn recv_completed(&self, timeout: Duration) -> Option<Completed> {
        self.worker.as_ref()?.result_rx.recv_timeout(timeout).ok()
    }

    fn drain_completed(&mut self) -> bool {
        let mut settled = false;
        while let Some(done) = self.try_recv_completed() {
            settled |= self.fold_completed(done);
        }
        settled
    }

    pub fn complete_blocking(&mut self, line: &str, pos: usize) -> Suggestions {
        const BLOCKING_TIMEOUT: Duration = Duration::from_secs(30);

        let fallback = match self.complete(line, pos) {
            CompletionResult::Fresh { suggestions, .. } => return suggestions,
            in_flight => in_flight.into_shared().unwrap_or_default(),
        };

        let deadline = Instant::now() + BLOCKING_TIMEOUT;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            let Some(done) = self.recv_completed(remaining) else {
                break;
            };
            if self.fold_completed(done) {
                return self.complete(line, pos).into_shared().unwrap_or_default();
            }
        }

        fallback
    }
}

/// Byte length of the longest prefix `a` and `b` share. Always a char boundary in both.
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .find_map(|((index, x), y)| (x != y).then_some(index))
        .unwrap_or_else(|| a.len().min(b.len()))
}

fn partial_of(line: &str, suggestions: &[Suggestion]) -> Option<Partial> {
    let span = suggestions.first()?.span;

    let mut matching_values = suggestions
        .iter()
        .filter(|suggestion| suggestion.span == span)
        .map(|suggestion| suggestion.value.as_str());

    // Slice a window into the first value rather than allocating a `String`; runs every
    // keystroke.
    let first = matching_values.next()?;
    let shared_len = matching_values.try_fold(first.len(), |shared, value| {
        let common = common_prefix_len(first.get(..shared)?, value);
        (common > 0).then_some(common)
    })?;
    let shared_prefix = first.get(..shared_len)?;

    let entered = line.get(span.start..span.end)?;
    let extends = shared_prefix != entered
        && shared_prefix
            .to_lowercase()
            .starts_with(&entered.to_lowercase());

    extends.then_some(Partial {
        span,
        insert: shared_prefix.to_string(),
    })
}

impl ReedlineCompleter for NuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let query = CompletionQuery::new(line, pos);
        self.drain_completed();

        if let Some(suggestions) = self.fresh_for(&query) {
            self.settle_pending(&query);
            let partial = partial_of(line, &suggestions);
            return CompletionResult::fresh(suggestions).with_partial(partial);
        }

        // Interactive completers run on UI thread to own terminal.
        let engine = CompletionEngine::isolated(&self.engine_state, Arc::clone(&self.stack), false);
        if engine.interactive_completer_at(line, pos) {
            let (suggestions, _cacheable) = engine.suggestions_for(&query);
            self.latest = Some(Completed {
                query,
                suggestions: suggestions.clone(),
                cacheable: false,
            });
            let partial = partial_of(line, &suggestions);
            return CompletionResult::fresh(suggestions).with_partial(partial);
        }

        let fallback = self.stale_fallback(&query);
        let partial = partial_of(line, &fallback);

        let worker = self.worker.get_or_insert_with(|| {
            Self::spawn_worker(Arc::clone(&self.engine_state), Arc::clone(&self.stack))
        });

        if worker.pending.as_ref() != Some(&query) {
            if worker.request_tx.send(query.clone()).is_ok() {
                worker.pending = Some(query);
            } else {
                // Worker died (a user completer closure panicked); drop it so the next
                // request spawns a replacement.
                self.worker = None;
            }
        }

        CompletionResult::stale_or_pending(fallback, CompletionOrigin::new(line, pos))
            .with_partial(partial)
    }

    fn poll_completion(&mut self) -> CompletionStatus {
        let settled = self.drain_completed();

        match self.worker.as_mut() {
            Some(worker) if worker.pending.is_some() => {
                if settled {
                    worker.pending = None;
                    CompletionStatus::Ready
                } else {
                    CompletionStatus::Pending
                }
            }
            _ => CompletionStatus::Idle,
        }
    }
}

#[cfg(test)]
mod completer_tests {
    use super::*;

    fn test_engine() -> Arc<EngineState> {
        let mut engine =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let delta = StateWorkingSet::new(&engine).render();
        engine.merge_delta(delta).expect("merge_delta");
        Arc::new(engine)
    }

    /// Corpus covering resolver branches.
    const CORPUS: &[&str] = &[
        "ls",
        "ls foo",
        "ls foo bar",
        "ls --long",
        "ls --long foo",
        "ls -a foo",
        "^ls foo",
        "sudo ls foo",
        "ls | where size > 1",
        "ls | where size > 1 ",
        "ls foo ",
        "1 + 2 ",
        "$env.PATH ",
        "ls | where size > ",
        "ls | where size >",
        "ls | where name == foo and size > 2",
        "1 + 2",
        "1 + ",
        "1 ",
        "$env.PATH",
        "$env.",
        "$nu",
        "each { |x| $x.name }",
        "let x = (ls | where name == foo)",
        "def foo [] { ls }",
        "@example\ndef foo [] {}",
        "use std/util [",
        "ls é foo",
        "",
        " ",
    ];

    /// All sites must contain cursor.
    #[test]
    fn every_site_holds_the_cursor() {
        let engine_state = test_engine();

        for line in CORPUS {
            for cursor in (0..=line.len()).filter(|at| line.is_char_boundary(*at)) {
                let contents = &line[..cursor];
                let mut working_set = StateWorkingSet::new(&engine_state);
                let offset = working_set.next_span_start();
                let block = parse(&mut working_set, Some("sweep"), contents.as_bytes(), false);
                let buffer = Buffer {
                    text: contents,
                    offset,
                };

                let site = CompletionEngine::resolve_completion_site(
                    &block,
                    &working_set,
                    buffer,
                    contents,
                );
                assert!(
                    touches(site.span, cursor + offset),
                    "{line:?} at {cursor}: a {} site spans {}..{}, off the cursor at {}",
                    site.kind.resolved().kind(),
                    site.span.start,
                    site.span.end,
                    cursor + offset,
                );
            }
        }
    }

    /// `commandline complete --input` place.kind/index for the HackMD where-size cases.
    fn place_of(engine: &CompletionEngine, line: &str) -> (String, Option<i64>) {
        let input = engine.completer_input_at(line, line.len(), DeclaredInputs::all());
        let place = input.get_data_by_key("place").expect("place field");
        let kind = place
            .get_data_by_key("kind")
            .expect("kind")
            .as_str()
            .expect("kind string")
            .to_string();
        let index = place.get_data_by_key("index").and_then(|v| v.as_int().ok());
        (kind, index)
    }

    /// HackMD issue 6: operator vs value vs trailing positional index of `where`.
    #[test]
    fn where_size_place_kind_matches_hackmd() {
        let engine_state = test_engine();
        let engine = CompletionEngine::new(&engine_state, &Stack::new());

        assert_eq!(
            place_of(&engine, "ls | where size >"),
            ("operator".into(), None),
            "cursor on `>` is an operator site"
        );
        assert_eq!(
            place_of(&engine, "ls | where size > 1"),
            ("positional".into(), Some(0)),
            "cursor on the comparison value is where's condition, not a file"
        );
        assert_eq!(
            place_of(&engine, "ls | where size > "),
            ("positional".into(), Some(0)),
            "trailing space after `>` is still where's only positional (index 0)"
        );
    }

    /// Mid-token and mid-flag adversarial cases for place/token agreement.
    #[test]
    fn token_span_matches_place_target_for_flags_and_values() {
        let engine_state = test_engine();
        let engine = CompletionEngine::new(&engine_state, &Stack::new());

        let check = |line: &str, cursor: usize, text: &str, start: usize| {
            let input = engine.completer_input_at(line, cursor, DeclaredInputs::all());
            let token = input.get_data_by_key("token").expect("token");
            assert_eq!(
                token.get_data_by_key("text").unwrap().as_str().unwrap(),
                text,
                "token.text for {line:?}@{cursor}"
            );
            assert_eq!(
                token
                    .get_data_by_key("span")
                    .unwrap()
                    .get_data_by_key("start")
                    .unwrap()
                    .as_int()
                    .unwrap(),
                start as i64,
                "token.span.start for {line:?}@{cursor}"
            );
            let place = input.get_data_by_key("place").expect("place");
            assert_eq!(
                place
                    .get_data_by_key("target")
                    .unwrap()
                    .get_data_by_key("start")
                    .unwrap()
                    .as_int()
                    .unwrap(),
                start as i64,
                "place.target.start for {line:?}@{cursor}"
            );
        };

        check("ls -al", 6, "-al", 3);
        // Cursor mid-flag: only text up to the cursor is parsed, so the token is the prefix.
        check("ls -al", 4, "-", 3);
        check("ls -al", 5, "-a", 3);
        check("str tri", 7, "tri", 4);
        check("str tri", 5, "t", 4); // mid-token prefix
        check("str tri", 6, "tr", 4);

        // Mid-cursor where-size: prefix-parsed site matches place.target.
        check("ls | where size > 1", 13, "si", 11); // mid-`size`
        check("ls | where size > 1", 15, "size", 11);
        check("ls | where size > 1", 17, ">", 16); // on operator
        check("ls | where size > 1", 19, "1", 18); // on value
        check("ls | where size > ", 18, "", 18); // trailing space after `>`
    }

    /// Token text/span equals target shown to completer.
    #[test]
    fn the_token_is_what_the_answer_replaces() {
        let engine_state = test_engine();
        let engine = CompletionEngine::new(&engine_state, &Stack::new());

        let range = |record: &Value, field: &str| {
            let span = record.get_data_by_key(field).expect("a span field");
            let at = |end| {
                span.get_data_by_key(end)
                    .and_then(|value| value.as_int().ok())
                    .expect("an offset") as usize
            };
            (at("start"), at("end"))
        };

        for line in CORPUS {
            for cursor in (0..=line.len()).filter(|at| line.is_char_boundary(*at)) {
                let contents = &line[..cursor];
                let input = engine.completer_input_at(contents, cursor, DeclaredInputs::all());

                let token = input.get_data_by_key("token").expect("a token");
                let place = input.get_data_by_key("place").expect("a place");
                let text = token
                    .get_data_by_key("text")
                    .and_then(|value| value.coerce_into_string().ok())
                    .expect("the token's text");

                let at = range(&token, "span");
                let target = range(&place, "target");
                assert_eq!(
                    at,
                    (target.0, target.1.min(contents.len())),
                    "{line:?} at {cursor}: the token is not the target the completer saw"
                );
                assert_eq!(
                    Some(text.as_str()),
                    contents.get(at.0..at.1),
                    "{line:?} at {cursor}: the token is not the text it spans"
                );
            }
        }
    }

    fn q(s: &str) -> CompletionQuery {
        CompletionQuery::new(s, s.len())
    }

    /// The token being extended starts at `start`; suggestions replace from there.
    fn token(start: usize) -> reedline::Span {
        reedline::Span::new(start, start)
    }

    /// Adversarial: bare `where` must stay a Call (not Garbage→File). `ls | where` is
    /// command; `ls | where ` is positional 0 — same shape as `get`.
    #[test]
    fn pipeline_head_where_is_command_not_file() {
        let engine_state = test_engine();
        let engine = CompletionEngine::new(&engine_state, &Stack::new());
        for line in ["ls | where", "ls | get", "ls | each", "ls | filter"] {
            let (kind, _) = place_of(&engine, line);
            assert_eq!(
                kind, "command",
                "{line:?} at EOL should be command head, got {kind}"
            );
        }
        // Trailing space after where: first positional (condition), not bare File.
        let (kind, index) = place_of(&engine, "ls | where ");
        assert_eq!(kind, "positional", "ls | where  trailing: got {kind}");
        assert_eq!(index, Some(0), "where condition is positional 0");
    }

    /// RowConditionSlot prefers the signature's condition index, not raw arg counting —
    /// a leading path arg must not steal index 0 from the condition.
    #[test]
    fn row_condition_index_follows_signature_slot_not_arg_order() {
        use nu_protocol::engine::{Call as EngineCall, Command, CommandType};
        use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

        #[derive(Clone)]
        struct MyWhere;
        impl Command for MyWhere {
            fn name(&self) -> &str {
                "mywhere"
            }
            fn description(&self) -> &str {
                "test: path then row condition"
            }
            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .required("path", SyntaxShape::Filepath, "path")
                    .required("cond", SyntaxShape::RowCondition, "condition")
                    .category(Category::Custom("test".into()))
            }
            fn run(
                &self,
                _engine_state: &EngineState,
                _stack: &mut Stack,
                _call: &EngineCall,
                _input: PipelineData,
            ) -> Result<PipelineData, ShellError> {
                Ok(PipelineData::empty())
            }
            fn command_type(&self) -> CommandType {
                CommandType::Builtin
            }
        }

        let mut engine_state =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        {
            let mut working_set = StateWorkingSet::new(&engine_state);
            working_set.add_decl(Box::new(MyWhere));
            engine_state
                .merge_delta(working_set.render())
                .expect("merge mywhere");
        }
        let engine_state = Arc::new(engine_state);
        let engine = CompletionEngine::new(&engine_state, &Stack::new());

        assert_eq!(
            place_of(&engine, "mywhere ./data size > 1"),
            ("positional".into(), Some(1)),
            "condition after a path arg is signature index 1"
        );
        assert_eq!(
            place_of(&engine, "mywhere ./data size > "),
            ("positional".into(), Some(1)),
            "trailing gap after operator stays condition index 1"
        );
        assert_eq!(
            place_of(&engine, "mywhere ./da"),
            ("positional".into(), Some(0)),
            "path arg stays index 0"
        );
    }

    /// Adversarial operators/units/nested pipeline places.
    #[test]
    fn where_operator_variants_and_downstream_get() {
        let engine_state = test_engine();
        let engine = CompletionEngine::new(&engine_state, &Stack::new());
        assert_eq!(
            place_of(&engine, "ls | where size >= 1"),
            ("positional".into(), Some(0))
        );
        assert_eq!(
            place_of(&engine, "ls | where size =="),
            ("operator".into(), None)
        );
        assert_eq!(
            place_of(&engine, "ls | where size == "),
            ("positional".into(), Some(0))
        );
        assert_eq!(
            place_of(&engine, "ls | where size > 1kb"),
            ("positional".into(), Some(0))
        );
        assert_eq!(
            place_of(&engine, "ls | where size > 1 | get name"),
            ("positional".into(), Some(0))
        );
        assert_eq!(
            place_of(&engine, "ls | where size > 1 | get "),
            ("positional".into(), Some(0))
        );
        // Incomplete pipe is a command head.
        assert_eq!(place_of(&engine, "ls |"), ("command".into(), None));
        assert_eq!(place_of(&engine, "ls | "), ("command".into(), None));
    }

    #[test]
    fn narrows_stays_within_one_token() {
        assert!(q("ls foobar").narrows(&q("ls foo"), token(3)));

        // Not narrowing: no new text, or text removed.
        assert!(!q("ls foo").narrows(&q("ls foo"), token(3)));
        assert!(!q("ls fo").narrows(&q("ls foo"), token(3)));

        // Each boundary character starts a new token, which a cached entry cannot answer.
        for narrowed in [
            "ls foo|from",
            "ls foo;ls",
            "ls foo/bar",
            "ls foo=1",
            "ls foo,2",
        ] {
            assert!(
                !q(narrowed).narrows(&q("ls foo"), token(3)),
                "narrowed across a boundary: {narrowed:?}"
            );
        }
    }

    #[test]
    fn narrows_rejects_a_token_that_becomes_a_flag() {
        // The empty slot after `from csv ` is answered at a point span; `--sep` adds no
        // boundary, so only the flag check keeps it from following.
        let base = q("from csv ");
        assert!(!q("from csv --sep").narrows(&q("from csv "), token(base.cursor())));

        // Extending a flag the user was already typing stays sound.
        assert!(q("from csv --sep").narrows(&q("from csv --s"), token(9)));
    }

    /// A cursor mid-char or past the end must not panic or split a char; `typed()` slices
    /// the buffer directly.
    #[test]
    fn the_cursor_is_clamped_to_a_char_boundary() {
        // "é" occupies bytes 3..5, so a cursor at 4 is mid-character.
        assert_eq!(CompletionQuery::new("ls é", 4).typed(), "ls ");

        let past_end = CompletionQuery::new("ls", 99);
        assert_eq!(past_end.typed(), "ls");
        assert_eq!(past_end.cursor(), 2);

        let empty = CompletionQuery::new("", 7);
        assert_eq!(empty.typed(), "");
        assert_eq!(empty.cursor(), 0);
    }

    fn apply_source(engine: &mut EngineState, stack: &mut Stack, source: &[u8]) {
        use nu_engine::eval_block;
        use nu_protocol::{PipelineData, debugger::WithoutDebug};

        let mut working_set = StateWorkingSet::new(engine);
        let block = parse(&mut working_set, None, source, false);
        assert!(
            working_set.parse_errors.is_empty(),
            "{:?}",
            working_set.parse_errors
        );
        engine
            .merge_delta(working_set.render())
            .expect("merge_delta");
        eval_block::<WithoutDebug>(engine, stack, &block, PipelineData::empty())
            .expect("eval source");
        engine.merge_env(stack).expect("merge_env");
    }

    fn apply_stack_local_source(engine: &mut EngineState, stack: &mut Stack, source: &[u8]) {
        use nu_engine::eval_block;
        use nu_protocol::{PipelineData, debugger::WithoutDebug};

        let mut working_set = StateWorkingSet::new(engine);
        let block = parse(&mut working_set, None, source, false);
        assert!(
            working_set.parse_errors.is_empty(),
            "{:?}",
            working_set.parse_errors
        );
        engine
            .merge_delta(working_set.render())
            .expect("merge_delta");
        eval_block::<WithoutDebug>(engine, stack, &block, PipelineData::empty())
            .expect("eval source");
    }

    /// The worker runs on an isolated stack and must still produce identical results.
    #[test]
    fn background_result_matches_the_synchronous_engine() {
        let engine = test_engine();
        let mut completer = NuCompleter::new(engine.clone(), Arc::new(Stack::new()));

        let sorted = |mut values: Vec<String>| {
            values.sort();
            values
        };
        let expected = sorted(
            CompletionEngine::new(&engine, &Stack::new())
                .fetch_completions_at("ls | c", 6)
                .into_iter()
                .map(|s| s.suggestion.value)
                .collect(),
        );
        assert!(expected.iter().any(|value| value == "cd"));

        // Nothing is cached yet, so the first non-blocking call can only be pending.
        assert!(completer.complete("ls | c", 6).is_pending());

        let settled = sorted(
            completer
                .complete_blocking("ls | c", 6)
                .iter()
                .map(|s| s.value.clone())
                .collect(),
        );
        assert_eq!(expected, settled);
    }

    /// A cache handed to a new per-prompt completer must still answer the previous prompt's
    /// queries.
    #[test]
    fn cache_outlives_the_completer_that_filled_it() {
        let engine = test_engine();
        let cache = NarrowingCache::default();

        let mut filling_prompt =
            NuCompleter::with_cache(engine.clone(), Arc::new(Stack::new()), cache.clone());
        let warmed = filling_prompt.complete_blocking("ls | c", 6);
        assert!(warmed.iter().any(|s| s.value == "cd"));
        drop(filling_prompt);

        let mut next_prompt = NuCompleter::with_cache(engine, Arc::new(Stack::new()), cache);
        let answer = next_prompt.complete("ls | c", 6);
        assert!(
            matches!(answer, CompletionResult::Fresh { .. }),
            "a carried-over cache entry should answer outright, got {answer:?}"
        );
        assert!(answer.suggestions().iter().any(|s| s.value == "cd"));
    }

    /// ...but not across a `cd`/`$env.PATH` change, the reason [`CacheEnv`] exists.
    #[test]
    fn cache_is_not_reused_in_a_different_environment() {
        use nu_protocol::Value;

        let engine = test_engine();
        let cache = NarrowingCache::default();

        let mut filling_prompt =
            NuCompleter::with_cache(engine.clone(), Arc::new(Stack::new()), cache.clone());
        assert!(!filling_prompt.complete_blocking("ls | c", 6).is_empty());

        let mut moved = Stack::new();
        moved.add_env_var(
            "PATH".into(),
            Value::string("/somewhere/else", Span::unknown()),
        );
        let mut next_prompt = NuCompleter::with_cache(engine, Arc::new(moved), cache);
        assert!(
            next_prompt.complete("ls | c", 6).is_pending(),
            "entries from another environment must not answer"
        );
    }

    /// Any `$env.config` assignment, including ones that are not completion
    /// settings, must not keep serving the previous prompt's suggestions.
    #[rstest::rstest]
    #[case::external_completer(
        b"$env.config.completions.external.completer = {|spans| [from-second]}",
        Some("from-second")
    )]
    #[case::unrelated_config(b"$env.config.float_precision = 5", None)]
    fn cache_is_not_reused_after_config_changes(
        #[case] later: &[u8],
        #[case] expected_external: Option<&str>,
    ) {
        let mut engine = (*test_engine()).clone();
        let mut stack = Stack::new();
        apply_source(
            &mut engine,
            &mut stack,
            b"$env.config.completions.external.completer = {|spans| [from-first]}",
        );
        let first_env = CacheEnv::of(&engine, &stack);
        let engine = Arc::new(engine);
        let stack = Arc::new(stack);
        let cache = NarrowingCache::default();

        let mut filling_prompt =
            NuCompleter::with_cache(engine.clone(), stack.clone(), cache.clone());
        assert!(!filling_prompt.complete_blocking("ls | c", 6).is_empty());
        drop(filling_prompt);

        let mut engine = (*engine).clone();
        let mut stack = (*stack).clone();
        apply_source(&mut engine, &mut stack, later);
        let second_env = CacheEnv::of(&engine, &stack);
        assert_ne!(
            first_env, second_env,
            "a config assignment must change the cache fingerprint"
        );

        let mut next_prompt = NuCompleter::with_cache(Arc::new(engine), Arc::new(stack), cache);
        assert!(
            next_prompt.complete("ls | c", 6).is_pending(),
            "entries computed under a previous config must not answer"
        );

        if let Some(expected) = expected_external {
            let values: Vec<_> = next_prompt
                .complete_blocking("extcommand x", 12)
                .iter()
                .map(|s| s.value.clone())
                .collect();
            assert_eq!(values, [expected], "the newly assigned completer must run");
        }
    }

    /// Stack-local config updates have not reached `engine_state.config_epoch()` yet, but
    /// they still shape completion behavior and must invalidate shared cache entries.
    #[test]
    fn cache_is_not_reused_after_stack_local_config_changes() {
        let engine = test_engine();
        let cache = NarrowingCache::default();

        let mut first_stack = Stack::new();
        first_stack.config = Some({
            let mut config = engine.get_config().as_ref().clone();
            config.float_precision = 5;
            Arc::new(config)
        });

        let mut filling_prompt =
            NuCompleter::with_cache(engine.clone(), Arc::new(first_stack), cache.clone());
        assert!(!filling_prompt.complete_blocking("ls | c", 6).is_empty());
        drop(filling_prompt);

        let mut second_stack = Stack::new();
        second_stack.config = Some({
            let mut config = engine.get_config().as_ref().clone();
            config.float_precision = 6;
            Arc::new(config)
        });

        let mut next_prompt = NuCompleter::with_cache(engine, Arc::new(second_stack), cache);
        assert!(
            next_prompt.complete("ls | c", 6).is_pending(),
            "entries from another stack-local config must not answer"
        );
    }

    /// A stack-local external completer change must not keep serving the previous
    /// completer's cached suggestions.
    #[test]
    fn cache_is_not_reused_after_stack_local_external_completer_changes() {
        let mut engine = (*test_engine()).clone();
        let mut first_stack = Stack::new();
        apply_stack_local_source(
            &mut engine,
            &mut first_stack,
            b"$env.config.completions.external.completer = {|buffer| [from-first]}",
        );
        let mut second_stack = Stack::new();
        apply_stack_local_source(
            &mut engine,
            &mut second_stack,
            b"$env.config.completions.external.completer = {|buffer| [from-second]}",
        );
        let engine = Arc::new(engine);
        let cache = NarrowingCache::default();

        let mut filling_prompt =
            NuCompleter::with_cache(engine.clone(), Arc::new(first_stack), cache.clone());
        let first_values: Vec<_> = filling_prompt
            .complete_blocking("extcommand x", 12)
            .iter()
            .map(|s| s.value.clone())
            .collect();
        assert_eq!(first_values, ["from-first"]);
        drop(filling_prompt);

        let mut next_prompt = NuCompleter::with_cache(engine, Arc::new(second_stack), cache);
        assert!(
            next_prompt.complete("extcommand x", 12).is_pending(),
            "the cached result from the first stack-local completer must not answer"
        );

        let second_values: Vec<_> = next_prompt
            .complete_blocking("extcommand x", 12)
            .iter()
            .map(|s| s.value.clone())
            .collect();
        assert_eq!(second_values, ["from-second"]);
    }

    /// `cache_size = 0` must disable the cache entirely, even a carried-over one.
    #[test]
    fn cache_size_zero_disables_the_cache() {
        let mut engine = (*test_engine()).clone();
        let mut config = engine.get_config().as_ref().clone();
        config.completions.cache_size = 0;
        engine.set_config(config);
        let engine = Arc::new(engine);
        let cache = NarrowingCache::default();

        let mut filling_prompt =
            NuCompleter::with_cache(engine.clone(), Arc::new(Stack::new()), cache.clone());
        assert!(!filling_prompt.complete_blocking("ls | c", 6).is_empty());
        drop(filling_prompt);

        let mut next_prompt = NuCompleter::with_cache(engine, Arc::new(Stack::new()), cache);
        assert!(
            next_prompt.complete("ls | c", 6).is_pending(),
            "a disabled cache must not answer a query it could have answered"
        );
    }

    /// Stack-local `cache_size` changes should take effect before the config is merged
    /// into `EngineState`.
    #[test]
    fn stack_local_cache_size_zero_disables_the_cache() {
        let engine = test_engine();
        let mut stack = Stack::new();
        stack.config = Some({
            let mut config = engine.get_config().as_ref().clone();
            config.completions.cache_size = 0;
            Arc::new(config)
        });
        let cache = NarrowingCache::default();

        let mut disabled_prompt =
            NuCompleter::with_cache(engine.clone(), Arc::new(stack), cache.clone());
        assert!(!disabled_prompt.complete_blocking("ls | c", 6).is_empty());
        drop(disabled_prompt);

        let mut next_prompt = NuCompleter::with_cache(engine, Arc::new(Stack::new()), cache);
        assert!(
            next_prompt.complete("ls | c", 6).is_pending(),
            "a stack-local cache_size of zero must prevent storing carried-over entries"
        );
    }

    /// A narrowed fallback keeps the producer's order rather than re-sorting.
    #[test]
    fn a_narrowed_fallback_keeps_the_producers_order() {
        let mut engine =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        // An unsorted (`sort: false`) command-wide completer seeds the narrowing cache.
        let src = "
            def comp [token] { {completions: [zzz-two, zzz-one], options: {sort: false, filter: false}} }
            @complete comp
            def --wrapped my-cmd [...rest] {}";
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine);
            parse(&mut working_set, None, src.as_bytes(), false);
            assert!(
                working_set.parse_errors.is_empty(),
                "{:?}",
                working_set.parse_errors
            );
            working_set.render()
        };
        engine.merge_delta(delta).expect("merge_delta");
        let engine = Arc::new(engine);

        let cache = NarrowingCache::default();
        let mut warming =
            NuCompleter::with_cache(engine.clone(), Arc::new(Stack::new()), cache.clone());
        let warmed: Vec<_> = warming
            .complete_blocking("my-cmd zzz", 10)
            .iter()
            .map(|s| s.value.clone())
            .collect();
        assert_eq!(warmed, ["zzz-two", "zzz-one"], "producer order, unsorted");
        drop(warming);

        // Narrowing one more character keeps that order instead of sorting.
        let mut narrowing = NuCompleter::with_cache(engine, Arc::new(Stack::new()), cache);
        let values: Vec<_> = narrowing
            .complete("my-cmd zzz-", 11)
            .suggestions()
            .iter()
            .map(|s| s.value.clone())
            .collect();
        assert_eq!(
            values,
            ["zzz-two", "zzz-one"],
            "narrowed fallback must not re-sort"
        );
    }
}
