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
    BlockId, BuiltinCompletion, CommandWideCompleter, Completion, DeclId, Flag, Record, Signature,
    Span, SuggestionKind, Value,
    ast::{
        Argument, AttributeBlock, Block, Call, Expr, Expression, ExternalArgument, FlagRef,
        FullCellPath, PipelineRedirection, RedirectionTarget, Traverse,
    },
    engine::{ArgType, EngineState, Stack, StateWorkingSet},
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
    custom_completions::{InputShape, UserCompletion, completer_input},
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

/// What cached completions were computed against.
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
            .map(|path| path.to_expanded_string(":", engine_state.get_config()))
            .hash(&mut hasher);

        let cwd = engine_state.cwd(Some(stack)).ok();
        // The cwd mtime, so adding/removing files invalidates stale file completions.
        cwd.as_ref()
            .and_then(|cwd| std::fs::metadata(cwd).ok()?.modified().ok())
            .hash(&mut hasher);
        cwd.hash(&mut hasher);

        Self(hasher.finish())
    }
}

struct CacheEntry {
    suggestions: Suggestions,
    env: CacheEnv,
}

impl CacheEntry {
    /// Whether this entry is usable in `env`.
    fn is_usable(&self, env: CacheEnv) -> bool {
        self.env == env
    }

    /// The span the last suggestion replaces; the one the cursor extends.
    fn reference_span(&self) -> Option<reedline::Span> {
        self.suggestions.last().map(|suggestion| suggestion.span)
    }
}

/// Cross-prompt completion cache, LRU by entry count; capacity `0` disables it.
#[derive(Clone)]
pub(crate) struct NarrowingCache {
    entries: Arc<Mutex<Option<LruCache<CompletionQuery, CacheEntry>>>>,
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
            entries: Arc::new(Mutex::new(NonZeroUsize::new(capacity).map(LruCache::new))),
        }
    }

    /// Resize in place; `0` disables the cache. Reapplied each prompt.
    pub(crate) fn set_capacity(&self, capacity: usize) {
        if let Ok(mut cache_guard) = self.entries.lock() {
            *cache_guard = NonZeroUsize::new(capacity).map(|new_capacity| {
                let mut cache = cache_guard
                    .take()
                    .unwrap_or_else(|| LruCache::new(new_capacity));
                cache.resize(new_capacity);
                cache
            });
        }
    }

    pub(crate) fn fresh(
        &self,
        query: &CompletionQuery,
        environment: CacheEnv,
    ) -> Option<Suggestions> {
        let mut cache_guard = self.entries.lock().ok()?;
        let entry = cache_guard.as_mut()?.get(query)?;

        entry
            .is_usable(environment)
            .then(|| entry.suggestions.clone())
    }

    pub(crate) fn store(
        &self,
        query: CompletionQuery,
        environment: CacheEnv,
        suggestions: Suggestions,
    ) {
        if let Ok(mut cache_guard) = self.entries.lock()
            && let Some(cache) = cache_guard.as_mut()
        {
            let stale_keys: Vec<_> = cache
                .iter()
                .filter(|(_, entry)| !entry.is_usable(environment))
                .map(|(key, _)| key.clone())
                .collect();

            for key in stale_keys {
                cache.pop(&key);
            }

            cache.put(
                query,
                CacheEntry {
                    suggestions,
                    env: environment,
                },
            );
        }
    }

    pub(crate) fn narrowed_fallback(
        &self,
        query: &CompletionQuery,
        environment: CacheEnv,
        options: &CompletionOptions,
    ) -> Suggestions {
        let Some((base_suggestions, ref_span, search_token)) =
            self.entries.lock().ok().and_then(|guard| {
                let (_, entry, span) = guard
                    .as_ref()?
                    .iter()
                    .filter_map(|(bq, e)| {
                        let s = e.reference_span()?;
                        (e.is_usable(environment) && query.narrows(bq, s)).then_some((
                            bq.cursor(),
                            e,
                            s,
                        ))
                    })
                    .max_by_key(|&(c, ..)| c)?;

                let token = query.typed().get(span.start..)?;
                Some((Arc::clone(&entry.suggestions), span, token))
            })
        else {
            return Suggestions::default();
        };

        let mut matcher = NuMatcher::new(search_token, options, true);

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
    latest: Option<Completed>,
}

/// The completion behaviour configured in `$env.config.completions`.
fn configured_options(engine_state: &EngineState) -> CompletionOptions {
    let config = engine_state.get_config();
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

/// What the cursor is completing; each variant carries exactly the AST it needs.
#[derive(Debug, Clone)]
pub(crate) enum SiteKind<'a> {
    /// A command head; `node` is the whole call (for `^`/`%` sigils).
    Command { node: Option<&'a Expression> },
    /// A flag name being typed (`--`, `-x`).
    FlagName {
        call: &'a Call,
        element: &'a Expression,
    },
    /// The value of a flag; `flag` keeps long/short identity.
    FlagValue {
        call: &'a Call,
        element: &'a Expression,
        flag: FlagRef<'a>,
        arg_slot: usize,
    },
    /// A positional argument; `sig_positional` indexes the signature.
    Positional {
        call: &'a Call,
        element: &'a Expression,
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
            Self::FlagName { element, .. }
            | Self::FlagValue { element, .. }
            | Self::Positional { element, .. } => Some(element),
            Self::ExternalArg { call, .. } => Some(call),
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
    /// A site with just its kind and span; the rest is filled in later.
    fn new(kind: SiteKind<'a>, span: Span) -> Self {
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

/// Dispatch output, plus whether the result is cacheable.
#[derive(Default)]
struct Dispatched {
    suggestions: Vec<SemanticSuggestion>,
    cacheable: bool,
}

impl Dispatched {
    /// Append another dispatch's suggestions, propagating its cacheability.
    fn merge(&mut self, other: Dispatched) {
        self.cacheable |= other.cacheable;
        self.suggestions.extend(other.suggestions);
    }

    /// Merge one source's outcome; report whether it answered.
    fn absorb(&mut self, attempt: Fetched) -> bool {
        let answered = !attempt.needs_fallback();
        self.merge(attempt.into());
        answered
    }
}

impl From<Fetched> for Dispatched {
    fn from(fetched: Fetched) -> Self {
        Self {
            cacheable: fetched.is_cacheable(),
            suggestions: fetched.into_suggestions(),
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
    /// The nesting levels the cursor lives in, as `$input.contexts`.
    pub contexts: &'a [CompletionContext<'a>],
}

impl<'a> Context<'a> {
    pub(crate) fn prefix_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.prefix)
    }

    /// Attach a resolved site, read by user completers as `$input.contexts`.
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
        Self {
            engine_state,
            stack: isolated_stack(stack, suppress_stdin),
            options: configured_options(engine_state),
        }
    }

    /// Answer one request, returning suggestions and whether the result is cacheable.
    fn suggestions_for(&self, query: &CompletionQuery) -> (Suggestions, bool) {
        let dispatched = self.dispatch_completions_at(query.typed(), query.cursor());
        let suggestions = dispatched
            .suggestions
            .into_iter()
            .map(|semantic_suggestion| semantic_suggestion.suggestion)
            .collect();

        (suggestions, dispatched.cacheable)
    }

    pub fn fetch_completions_at(&self, line: &str, position: usize) -> Vec<SemanticSuggestion> {
        self.dispatch_completions_at(line, position).suggestions
    }

    /// The record a user completer would receive, without running one.
    pub fn completer_input_at(&self, line: &str, position: usize, shape: InputShape) -> Value {
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
        let site = self.resolve_completion_site(&block, &working_set, buffer, sliced_line);

        completer_input(
            &self
                .context(
                    &working_set,
                    buffer,
                    site.span,
                    site.typed_prefix.as_bytes(),
                )
                .at_site(&site),
            shape,
        )
    }

    fn dispatch_completions_at(&self, line: &str, position: usize) -> Dispatched {
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
        .suggestions
    }

    /// `buffer` is the commandline; `contents` is the text the block was parsed from.
    fn fetch_completions_by_block(        &self,
        block: Arc<Block>,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        contents: &str,
    ) -> Dispatched {
        let site = self.resolve_completion_site(&block, working_set, buffer, contents);
        let mut dispatched = self.dispatch_completion_site(&site, working_set, buffer);

        // A multi-word head is ambiguous: offer the shorter command's argument reading too.
        let argument_reading =
            self.complete_multiword_head_as_argument(&site, working_set, buffer, contents);
        dispatched.cacheable |= argument_reading.cacheable;
        dispatched
            .suggestions
            .splice(..0, argument_reading.suggestions);
        dispatched
    }

    /// `baz --test bar` can also read as the value of `baz --test`'s flag.
    fn complete_multiword_head_as_argument(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        contents: &str,
    ) -> Dispatched {
        if !matches!(site.kind, SiteKind::Command { .. })
            || !working_set
                .get_span_contents(site.span)
                .iter()
                .any(u8::is_ascii_whitespace)
        {
            return Dispatched::default();
        }

        let mut parse_ws = StateWorkingSet::new(self.engine_state);
        let _ = parse_ws.add_file("completer", contents.as_bytes());
        let Some(shorter) = parse_shorter_head_reading(&mut parse_ws, site.span, None) else {
            return Dispatched::default();
        };

        let mut shorter_site = self.finalize_site(
            self.resolve_expression_site(&shorter, site.cursor, &parse_ws),
            buffer,
            contents,
        );
        // A command-head result means no distinct argument; leave it to the primary dispatch.
        if matches!(shorter_site.kind, SiteKind::Command { .. }) {
            return Dispatched::default();
        }

        // A single-expression re-parse has no enclosing chain.
        shorter_site.contexts = vec![shorter_site.own_context()];

        let mut dispatched = self.dispatch_completion_site(&shorter_site, &parse_ws, buffer);
        // Keep only the argument value; drop command-kind results.
        dispatched
            .suggestions
            .retain(|candidate| !matches!(candidate.kind, Some(SuggestionKind::Command(..))));
        dispatched
    }

    /// Dispatch the site to the appropriate specialized completer.
    fn dispatch_completion_site<'a>(
        &'a self,
        site: &'a CompletionSite<'a>,
        working_set: &'a StateWorkingSet,
        buffer: Buffer<'a>,
    ) -> Dispatched {
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

                if completions.suggestions.is_empty() {
                    self.suggestions_at(&mut FileCompletion, &completion_context)
                } else {
                    completions
                }
            }

            SiteKind::FlagName { .. }
            | SiteKind::FlagValue { .. }
            | SiteKind::Positional { .. } => {
                self.dispatch_call_completion_site(site, working_set, buffer, &completion_context)
            }

            SiteKind::Operator { lhs } => OperatorCompletion {
                left_hand_side: lhs,
            }
            .fetch(&completion_context)
            .into(),

            SiteKind::CellPath { path } => CellPathCompletion {
                full_cell_path: path,
                cursor: site.cursor,
            }
            .fetch(&completion_context)
            .into(),

            SiteKind::Variable => self.variable_names_completion_helper(&completion_context),

            SiteKind::AttributeName => AttributeCompletion.fetch(&completion_context).into(),

            SiteKind::AttributableItem => AttributableCompletion.fetch(&completion_context).into(),

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
    ) -> Dispatched {
        let SiteKind::ExternalArg {
            call: external_call,
            index,
        } = &site.kind
        else {
            return Dispatched::default();
        };
        let external_call = *external_call;
        let Expr::ExternalCall(head, _) = &external_call.expr else {
            return Dispatched::default();
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
                if !commands.suggestions.is_empty() {
                    return commands;
                }
            }
        }

        let mut dispatched = Dispatched::default();

        // The user's configured external completer.
        let external_answered = self
            .engine_state
            .get_config()
            .completions
            .external
            .completer
            .as_ref()
            .is_some_and(|closure| {
                dispatched.absorb(UserCompletion::closure(closure).fetch(completion_context))
            });

        // Subcommands extending this call suppress the file fallback.
        let subcommands =
            self.subcommand_suggestions(working_set, buffer, external_call.span.start, site.cursor);

        // File completion for paths, only when nothing more specific answered.
        if !external_answered
            && dispatched.suggestions.is_empty()
            && subcommands.suggestions.is_empty()
        {
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
    ) -> Dispatched {
        // Only call-bound kinds carry a call; anything else is an error here.
        let call = match &site.kind {
            SiteKind::FlagName { call, .. }
            | SiteKind::FlagValue { call, .. }
            | SiteKind::Positional { call, .. } => *call,
            _ => return Dispatched::default(),
        };

        let signature = working_set.get_decl(call.decl_id).signature();

        // Subcommands are always offered and suppress the file-path fallback.
        let subcommands =
            self.subcommand_suggestions(working_set, buffer, call.head.start, site.cursor);

        // Value kinds share one shape; only the `ArgType` and completer lookup differ.
        let argument_value = |engine: &Self, arg_type, custom, arg_slot| {
            engine.complete_argument_value(
                custom,
                ArgValueCompletion {
                    call,
                    arg_type,
                    need_fallback: subcommands.suggestions.is_empty(),
                    arg_idx: arg_slot,
                    cursor: site.cursor,
                },
                completion_context,
                &signature,
            )
        };

        let mut results = match &site.kind {
            SiteKind::FlagName { .. } => {
                self.complete_flag_names(call.decl_id, completion_context, &signature)
            }
            SiteKind::FlagValue { flag, arg_slot, .. } => argument_value(
                self,
                ArgType::Flag(Cow::Borrowed(flag.name())),
                find_flag(&signature, *flag).and_then(|flag| flag.completion),
                *arg_slot,
            ),
            SiteKind::Positional {
                sig_positional,
                arg_slot,
                ..
            } => argument_value(
                self,
                ArgType::Positional(*sig_positional),
                signature
                    .get_positional(*sig_positional)
                    .and_then(|positional| positional.completion.clone()),
                *arg_slot,
            ),
            _ => Dispatched::default(),
        };

        results.merge(subcommands);
        results
    }

    /// Resolve the contextual state and constraints at the cursor's location.
    pub(crate) fn resolve_completion_site<'a>(
        &self,
        block: &'a Block,
        working_set: &'a StateWorkingSet,
        buffer: Buffer,
        contents: &'a str,
    ) -> CompletionSite<'a> {
        let absolute_position = buffer.cursor() + buffer.offset;

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
                self.resolve_expression_site(expression, absolute_position, working_set)
            }
            None => self.resolve_fallback_site(block, working_set, absolute_position),
        };

        site.contexts = self.contexts_of(&chain, working_set, absolute_position, &site);
        self.finalize_site(site, buffer, contents)
    }

    /// The chain of contexts the cursor lives in, outermost first. `site` supplies the
    /// innermost one, so it stays the resolution the dispatcher itself acts on rather than
    /// a second, possibly disagreeing, reading of the same position.
    fn contexts_of<'a>(
        &self,
        chain: &[(&'a Expression, Option<Span>)],
        working_set: &'a StateWorkingSet,
        absolute_position: usize,
        site: &CompletionSite<'a>,
    ) -> Vec<CompletionContext<'a>> {
        // Where the site's own element sits in the chain; everything before it encloses it.
        // A kind carrying no element (a bare `$var`, a cell path) belongs to the element the
        // chain walked to, which is its last.
        let innermost = site
            .kind
            .element()
            .and_then(|element| {
                chain
                    .iter()
                    .position(|(candidate, _)| std::ptr::eq(*candidate, element))
            })
            .unwrap_or_else(|| chain.len().saturating_sub(1));

        let mut contexts: Vec<_> = chain
            .iter()
            .take(innermost)
            .map(|&(element, descent)| CompletionContext {
                cursor: self
                    .resolve_expression_site(element, absolute_position, working_set)
                    .kind
                    .resolved(),
                element: Some(element),
                descent,
            })
            .collect();

        contexts.push(CompletionContext {
            element: site
                .kind
                .element()
                .or_else(|| chain.get(innermost).map(|&(element, _)| element)),
            ..site.own_context()
        });

        contexts
    }

    /// Fill `typed_prefix`/`cursor` from the final span so they never disagree.
    fn finalize_site<'a>(
        &self,
        mut site: CompletionSite<'a>,
        buffer: Buffer,
        contents: &'a str,
    ) -> CompletionSite<'a> {
        let token_start = site.span.start.saturating_sub(buffer.offset);
        site.typed_prefix = contents
            .get(token_start..buffer.cursor())
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed(""));
        site.cursor = buffer.cursor() + buffer.offset;
        site
    }

    fn resolve_expression_site<'a>(
        &self,
        expression: &'a Expression,
        absolute_position: usize,
        working_set: &'a StateWorkingSet,
    ) -> CompletionSite<'a> {
        // Cursor in whitespace after a completed value (`1 ⌶`) is an operator position.
        if absolute_position > expression.span.end && is_operator_lhs(&expression.expr) {
            return CompletionSite::new(
                SiteKind::Operator { lhs: expression },
                Span::point(absolute_position),
            );
        }

        // Default to file completion; overridden below where the expression warrants it.
        match &expression.expr {
            Expr::Call(call) => {
                self.resolve_call_site(call, expression, absolute_position, working_set)
            }
            Expr::ExternalCall(head, arguments) => {
                self.resolve_external_call_site(expression, head, arguments, absolute_position)
            }
            Expr::AttributeBlock(attribute_block) => {
                self.resolve_attribute_site(attribute_block, absolute_position)
            }
            Expr::Var(_) => CompletionSite::new(SiteKind::Variable, expression.span),
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

                CompletionSite::new(kind, expression.span)
            }
            Expr::BinaryOp(left_hand_side, operator, _) => CompletionSite::new(
                SiteKind::Operator {
                    lhs: left_hand_side.as_ref(),
                },
                operator.span,
            ),
            _ => CompletionSite::new(SiteKind::File, expression.span),
        }
    }

    /// Resolve a bare external call; the head completes as a command, else
    /// [`SiteKind::ExternalArg`].
    fn resolve_external_call_site<'a>(
        &self,
        expression: &'a Expression,
        head: &'a Expression,
        arguments: &'a [ExternalArgument],
        absolute_position: usize,
    ) -> CompletionSite<'a> {
        if absolute_position <= head.span.end {
            return CompletionSite::new(
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

        CompletionSite::new(
            SiteKind::ExternalArg {
                call: expression,
                index,
            },
            span,
        )
    }

    fn resolve_call_site<'a>(
        &self,
        call: &'a Call,
        expression: &'a Expression,
        absolute_position: usize,
        working_set: &'a StateWorkingSet,
    ) -> CompletionSite<'a> {
        // Cursor in (or right after) the command head: complete the command name.
        if absolute_position <= call.head.end {
            return CompletionSite::new(
                SiteKind::command(expression),
                command_name_span(call.head, expression.span),
            );
        }

        // Cursor on an existing argument.
        if let Some((argument_index, argument)) = call
            .arguments
            .iter()
            .enumerate()
            .find(|(_, argument)| touches(argument.span(), absolute_position))
        {
            return self.resolve_argument_site(
                call,
                expression,
                argument,
                argument_index,
                absolute_position,
                working_set,
            );
        }

        // A trailing gap after a row condition (`where name ⌶`) is an operator position.
        if let Some(operator_left_hand_side) =
            self.row_condition_operator_lhs(call, working_set, absolute_position)
        {
            return CompletionSite::new(
                SiteKind::Operator {
                    lhs: operator_left_hand_side,
                },
                Span::point(absolute_position),
            );
        }

        // Classify the trailing slot (flag value, flag name, or positional) by the
        // trailing non-whitespace token.
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

        if let Some(flag_ref) = self.pending_flag_value(call, working_set) {
            // `arg_slot` past the last argument: no node exists yet; `ArgValueCompletion` reads `None`.
            CompletionSite::new(
                SiteKind::FlagValue {
                    call,
                    element: expression,
                    flag: flag_ref,
                    arg_slot: call.arguments.len(),
                },
                point,
            )
        } else if token_is_flag {
            CompletionSite::new(
                SiteKind::FlagName {
                    call,
                    element: expression,
                },
                trailing_token,
            )
        } else {
            CompletionSite::new(
                SiteKind::Positional {
                    call,
                    element: expression,
                    sig_positional: count_positionals(call, call.arguments.len()),
                    arg_slot: call.arguments.len(),
                },
                point,
            )
        }
    }

    /// The last row-condition term when the cursor trails it: an operator's LHS.
    fn row_condition_operator_lhs<'a>(
        &self,
        call: &'a Call,
        working_set: &'a StateWorkingSet,
        absolute_position: usize,
    ) -> Option<&'a Expression> {
        let block_id = call
            .arguments
            .iter()
            .rev()
            .find_map(|argument| match argument {
                Argument::Positional(Expression {
                    expr: Expr::RowCondition(block_id),
                    ..
                }) => Some(*block_id),
                _ => None,
            })?;

        let last_term = &working_set
            .get_block(block_id)
            .pipelines
            .last()?
            .elements
            .last()?
            .expr;

        if absolute_position <= last_term.span.end || !is_operator_lhs(&last_term.expr) {
            return None;
        }

        let gap = working_set.get_span_contents(Span::new(last_term.span.end, absolute_position));
        gap.iter().all(u8::is_ascii_whitespace).then_some(last_term)
    }

    /// The [`FlagRef`] of a last-argument flag still awaiting its value.
    fn pending_flag_value<'a>(
        &self,
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
        &self,
        call: &'a Call,
        expression: &'a Expression,
        argument: &'a Argument,
        argument_index: usize,
        absolute_position: usize,
        working_set: &StateWorkingSet,
    ) -> CompletionSite<'a> {
        let flag_name = SiteKind::FlagName {
            call,
            element: expression,
        };

        let (kind, span) = match argument {
            Argument::Named((name, short, optional_value)) => {
                if let Some(value_expression) = optional_value
                    .as_ref()
                    .filter(|value| touches(value.span, absolute_position))
                {
                    (
                        SiteKind::FlagValue {
                            call,
                            element: expression,
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
                        call,
                        element: expression,
                        sig_positional: count_positionals(call, argument_index),
                        arg_slot: argument_index,
                    }
                };
                (kind, argument.span())
            }
            Argument::Spread(_) => (SiteKind::File, argument.span()),
        };

        CompletionSite::new(kind, span)
    }

    fn resolve_attribute_site<'a>(
        &self,
        attribute_block: &'a AttributeBlock,
        absolute_position: usize,
    ) -> CompletionSite<'a> {
        if let Some(attribute) = attribute_block
            .attributes
            .iter()
            .find(|attribute| touches(attribute.expr.span, absolute_position))
        {
            return CompletionSite::new(SiteKind::AttributeName, attribute.expr.span);
        }

        if touches(attribute_block.item.span, absolute_position) {
            return CompletionSite::new(SiteKind::AttributableItem, attribute_block.item.span);
        }

        // Past the last attribute is the item's slot; earlier gaps type another attribute.
        let kind = match attribute_block.attributes.last() {
            Some(last) if absolute_position >= last.expr.span.end => SiteKind::AttributableItem,
            _ => SiteKind::AttributeName,
        };

        CompletionSite::new(kind, Span::point(absolute_position))
    }

    fn resolve_fallback_site<'a>(
        &self,
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

        CompletionSite::new(kind, Span::point(absolute_position))
    }
    fn complete_argument_value(
        &self,
        custom: Option<Completion>,
        mut arg_value: ArgValueCompletion,
        context: &Context,
        signature: &Signature,
    ) -> Dispatched {
        let mut results = Dispatched::default();

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
                            Fetched::Cacheable(vec![])
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

        arg_value.need_fallback &= results.suggestions.is_empty();
        results.merge(arg_value.fetch(context).into());
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
    ) -> Dispatched {
        let mut results: Dispatched = FlagCompletion { decl_id }.fetch(context).into();
        results.merge(
            self.command_wide_completion_helper(signature, context)
                .into(),
        );
        results
    }

    fn suggestions_at<C: Completer>(&self, completer: &mut C, context: &Context) -> Dispatched {
        completer.fetch(context).into()
    }

    fn variable_names_completion_helper(&self, context: &Context) -> Dispatched {
        if !context.prefix.starts_with(b"$") {
            return Dispatched::default();
        }
        VariableCompletion.fetch(context).into()
    }

    fn command_completion_helper(
        &self,
        working_set: &StateWorkingSet,
        buffer: Buffer,
        span: Span,
        mut command_completion: CommandCompletion,
    ) -> Dispatched {
        let prefix = working_set.get_span_contents(span);
        let ctx = self.context(working_set, buffer, span, prefix);
        command_completion.fetch(&ctx).into()
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
    ) -> Dispatched {
        if cursor <= command_start {
            return Dispatched::default();
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
                .engine_state
                .get_config()
                .completions
                .external
                .completer
                .as_ref()
                .map(UserCompletion::closure),
            None => None,
        };

        match completion {
            // Answers for the whole call, never narrowed against the token prefix.
            Some(mut completion) => completion.fetch(&Context {
                prefix: b"",
                ..*context
            }),
            None => Fetched::Absent,
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
    /// Shared rather than borrowed: the background worker thread takes a handle.
    engine_state: Arc<EngineState>,
    stack: Arc<Stack>,
    options: CompletionOptions,
    cache: NarrowingCache,
    /// The [`CacheEnv`] of every entry this completer stores/reads; computed once per
    /// completer, not on [`CompletionEngine`] (which non-caching callers also build).
    cache_env: CacheEnv,
    worker: Option<CompletionWorker>,
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
        let cache_size = engine_state.get_config().completions.cache_size;
        cache.set_capacity(cache_size.try_into().unwrap_or(0));
        Self {
            options: configured_options(&engine_state),
            engine_state,
            stack,
            cache,
            cache_env,
            worker: None,
        }
    }

    fn fresh_for(&self, query: &CompletionQuery) -> Option<Suggestions> {
        if let Some(worker) = self.worker.as_ref()
            && let Some(latest) = &worker.latest
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
            latest: None,
        }
    }

    fn fold_completed(&mut self, done: Completed) -> bool {
        let Self {
            cache,
            cache_env,
            worker,
            ..
        } = self;
        let Some(worker) = worker.as_mut() else {
            return false;
        };
        let settled = worker.pending.as_ref() == Some(&done.query);
        if done.cacheable {
            cache.store(done.query.clone(), *cache_env, done.suggestions.clone());
        }
        worker.latest = Some(done);
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

    fn q(s: &str) -> CompletionQuery {
        CompletionQuery::new(s, s.len())
    }

    /// The token being extended starts at `start`; suggestions replace from there.
    fn token(start: usize) -> reedline::Span {
        reedline::Span::new(start, start)
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

    /// `cache_size = 0` must disable the cache entirely, even a carried-over one.
    #[test]
    fn cache_size_zero_disables_the_cache() {
        let mut engine = test_engine();
        {
            let state = Arc::make_mut(&mut engine);
            Arc::make_mut(&mut state.config).completions.cache_size = 0;
        }
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
}
