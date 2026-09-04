use crate::completions::{
    ArgValueCompletion, AttributableCompletion, AttributeCompletion, CellPathCompletion,
    CommandCompletion, CommandScope, Completer, CompletionOptions, CustomCompletion,
    DotNuCompletion, EnvVarCompletion, FileCompletion, FlagCompletion, MatchAlgorithm, NuMatcher,
    OperatorCompletion, VariableCompletion,
    base::{Fetched, SemanticSuggestion},
};
use lru::LruCache;
use nu_parser::{parse, parse_shorter_head_reading};
use nu_protocol::{
    BuiltinCompletion, CommandWideCompleter, Completion, DeclId, Flag, Signature, Span,
    SuggestionKind,
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

/// Max cache entries before evicting the least recently used; overridden per completer by
/// `$env.config.completions.cache_size` (`0` disables the cache).
const DEFAULT_CACHE_SIZE: usize = 100;

use super::{StaticCompletion, custom_completions::CommandWideCompletion};

/// Used as the function `f` in find_map Traverse
///
/// returns the inner-most pipeline_element of interest that reaches the given position
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
                    // `touches`, not `contains`: the cursor sits at the head's trailing
                    // edge (issue #7648).
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
            // `use std/util [E, T⌶`: the import list is a `List` in a `FullCellPath`; leave it
            // to the enclosing call, which knows the module, to complete its members.
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

/// Whether `position` is inside `span` or exactly at its trailing edge.
///
/// Completion happens at a token's trailing edge, which the end-exclusive
/// [`Span::contains`] would miss.
pub(crate) fn touches(span: Span, position: usize) -> bool {
    span.contains(position) || span.end == position
}

/// The last element when the cursor trails it over whitespace only (`ls ⌶`) — an empty
/// new slot for that element. Non-whitespace gaps fall through to
/// [`CompletionEngine::resolve_fallback_site`].
fn trailing_gap_element<'a>(
    block: &'a Block,
    working_set: &StateWorkingSet,
    absolute_position: usize,
) -> Option<&'a Expression> {
    let expression = &block.pipelines.last()?.elements.last()?.expr;
    let gap = working_set.get_span_contents(Span::new(expression.span.end, absolute_position));
    gap.iter()
        .all(u8::is_ascii_whitespace)
        .then_some(expression)
}

/// The span a command-name completion replaces, given the parsed `head` and the whole
/// `element` it heads.
fn command_name_span(head: Span, element: Span) -> Span {
    Span::new(head.start, head.end.max(element.end))
}

/// Whether `token` is a flag being typed — i.e. it begins with `-`.
///
/// The parser stores in-progress flags as positionals, so the leading dash is the only
/// reliable flag/positional test; the cache relies on it too.
fn is_flag_text(token: impl AsRef<[u8]>) -> bool {
    token.as_ref().starts_with(b"-")
}

/// [`is_flag_text`] for the token occupying `span`.
fn is_flag_token(working_set: &StateWorkingSet, span: Span) -> bool {
    is_flag_text(working_set.get_span_contents(span))
}

/// Whether `expr` is a value an operator can trail (`1 ⌶`, `'str' ⌶`). Exhaustive, so a
/// new [`Expr`] variant must be classified rather than defaulting.
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

/// Non-named arguments before `before_index` — the positional index of that slot.
fn count_positionals(call: &Call, before_index: usize) -> usize {
    call.arguments
        .iter()
        .take(before_index)
        .filter(|argument| !matches!(argument, Argument::Named(_)))
        .count()
}

/// Helper function to extract file-path expression from redirection target
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

/// For redirection target completion
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

/// Cache key and worker message identity: the text typed up to the cursor.
///
/// Excludes trailing text and derives `cursor()` from `typed.len()` so the two never
/// disagree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CompletionQuery {
    /// The prefix of the line buffer up to the (floored) cursor position.
    typed: Arc<str>,
}

impl CompletionQuery {
    fn new(line: &str, cursor: usize) -> Self {
        let floored = line.floor_char_boundary(cursor);
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

    /// Whether `self` is `base` with more characters typed into the same `token`. The
    /// appended text must stay within one token and must not turn it into a flag — a
    /// different completion site than the cached result came from.
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
            .map(|path| path.to_expanded_string(":", engine_state.get_config()))
            .hash(&mut hasher);

        let cwd = engine_state.cwd(Some(stack)).ok();
        // The cwd mtime, so adding/removing files invalidates stale file completions.
        cwd.as_ref()
            .and_then(|cwd| std::fs::metadata(cwd).ok()?.modified().ok())
            .hash(&mut hasher);
        cwd.hash(&mut hasher);

        engine_state.config_epoch().hash(&mut hasher);

        Self(hasher.finish())
    }
}

struct CacheEntry {
    suggestions: Suggestions,
    env: CacheEnv,
}

impl CacheEntry {
    /// Whether this entry may still answer a query: produced in the same environment.
    fn is_usable(&self, env: CacheEnv) -> bool {
        self.env == env
    }

    /// The span the cursor extends: the range the *last* suggestion replaces.
    ///
    /// `fetch_completions_by_block` keeps the cursor-anchored family last, so reading the
    /// last span is the correct one to extend.
    fn reference_span(&self) -> Option<reedline::Span> {
        self.suggestions.last().map(|suggestion| suggestion.span)
    }
}

/// Cross-prompt completion cache bounded by entry count (`$env.config.completions.cache_size`),
/// evicting least recently used entries. Capacity `0` disables the cache.
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

    /// Resizes the cache in place, dropping LRU entries when shrinking. Capacity `0`
    /// disables it. Called once per prompt so `cache_size` config changes take effect.
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
        // Fallback may discard fuzzy results that a longer query needs, so cached suggestions
        // cannot safely answer a narrowed query.
        if options.match_algorithm == MatchAlgorithm::Fallback {
            return Suggestions::default();
        }
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

        // Don't re-sort: the producing completer ranks a directory by its bare name and
        // appends the separator afterwards, so sorting here would rank it `config/` and
        // land it after `config.nu`. Filtering alone preserves the order it chose.
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
    latest: Option<Completed>,
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
    /// A command head. `node` is the whole call expression, used to detect a `^`/`%` sigil.
    Command { node: Option<&'a Expression> },
    /// A flag name being typed (`--`, `-x`).
    FlagName {
        call: &'a Call,
        element: &'a Expression,
    },
    /// The value of a flag (`--opt <tab>`). `flag` preserves long/short identity;
    /// `arg_slot` indexes `call.arguments`.
    FlagValue {
        call: &'a Call,
        element: &'a Expression,
        flag: FlagRef<'a>,
        arg_slot: usize,
    },
    /// A positional argument. `sig_positional` indexes the signature's positionals,
    /// `arg_slot` indexes `call.arguments`.
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
    /// An attribute name (`@<tab>`).
    AttributeName,
    /// The item an attribute block decorates (`def`, `extern`, …).
    AttributableItem,
    /// An argument of a bare external call; `index` is the argument slot.
    ExternalArg { call: &'a Expression, index: usize },
    /// A file path — the base/fallback completion.
    File,
}

impl<'a> SiteKind<'a> {
    /// A command head backed by an existing call expression (used for sigil detection).
    fn command(node: &'a Expression) -> Self {
        Self::Command { node: Some(node) }
    }
}

/// A fully resolved completion site: the span to replace, the typed text, the cursor, and
/// the [`SiteKind`].
///
/// `typed_prefix`/`cursor` are derived centrally in [`CompletionEngine::finalize_site`] so
/// they can never disagree with the span.
#[derive(Debug, Clone)]
pub(crate) struct CompletionSite<'a> {
    pub kind: SiteKind<'a>,
    pub span: Span,
    pub typed_prefix: Cow<'a, str>,
    /// The cursor, in absolute working-set (span) coordinates.
    pub cursor: usize,
}

impl<'a> CompletionSite<'a> {
    /// A site with the given kind and span; `typed_prefix`/`cursor` are filled later by
    /// [`CompletionEngine::finalize_site`].
    fn new(kind: SiteKind<'a>, span: Span) -> Self {
        Self {
            kind,
            span,
            typed_prefix: Cow::Borrowed(""),
            cursor: 0,
        }
    }
}

/// Engine dispatch output: suggestions plus whether an impure source ran (worth caching).
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
}

impl From<Fetched> for Dispatched {
    fn from(fetched: Fetched) -> Self {
        Self {
            // Read the cacheable flag before `into_suggestions` consumes the outcome.
            cacheable: fetched.is_cacheable(),
            suggestions: fetched.into_suggestions(),
        }
    }
}

pub struct CompletionEngine {
    engine_state: Arc<EngineState>,
    stack: Arc<Stack>,
    options: CompletionOptions,
}

#[derive(Clone, Copy)]
pub(crate) struct Context<'a> {
    pub working_set: &'a StateWorkingSet<'a>,
    pub stack: &'a Stack,
    pub options: &'a CompletionOptions,
    pub span: Span,
    pub prefix: &'a [u8],
    pub offset: usize,
}

impl Context<'_> {
    pub(crate) fn prefix_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.prefix)
    }
}

impl CompletionEngine {
    pub fn new(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self::with_stack(engine_state, isolated_stack(stack, false))
    }

    fn for_background(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self::with_stack(engine_state, isolated_stack(stack, true))
    }

    fn with_stack(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        let config = engine_state.get_config();
        let options = CompletionOptions {
            case_sensitive: config.completions.case_sensitive,
            match_algorithm: config.completions.algorithm.into(),
            sort: config.completions.sort,
            match_description: false,
        };
        Self {
            engine_state,
            stack,
            options,
        }
    }

    fn to_background(&self) -> Self {
        Self::for_background(Arc::clone(&self.engine_state), Arc::clone(&self.stack))
    }

    fn suggestions_for(&self, query: &CompletionQuery) -> (Suggestions, bool) {
        let dispatched = self.dispatch_completions_at(query.typed(), query.cursor());
        let suggestions = dispatched
            .suggestions
            .into_iter()
            .map(|semantic_suggestion| semantic_suggestion.suggestion)
            .collect();

        (suggestions, dispatched.cacheable)
    }

    /// Parse `query` once and run `f` with the resolved completion site.
    ///
    /// `CompletionQuery` is already the prefix up to the floored cursor, so this
    /// does not slice or floor again.
    fn with_completion_site<R>(
        &self,
        query: &CompletionQuery,
        f: impl FnOnce(&StateWorkingSet, &Arc<Block>, &CompletionSite, &str, usize) -> R,
    ) -> R {
        let line = query.typed();
        let position = query.cursor();
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let offset = working_set.next_span_start();
        let block = parse(&mut working_set, Some("completer"), line.as_bytes(), false);
        let site = self.resolve_completion_site(&block, &working_set, position, offset, line);
        f(&working_set, &block, &site, line, offset)
    }

    fn query_runs_user_closure(
        &self,
        working_set: &StateWorkingSet,
        site: &CompletionSite,
        line: &str,
        offset: usize,
    ) -> bool {
        self.site_runs_user_closure(site, working_set)
            || self.multiword_argument_runs_user_closure(site, working_set, line, offset)
    }

    /// User-closure completers (external completer, `@complete`, custom commands)
    /// take the TTY. They must run on the REPL thread with reedline blocked, not
    /// on the background worker.
    #[cfg(test)]
    fn should_complete_on_repl_thread(&self, query: &CompletionQuery) -> bool {
        self.with_completion_site(query, |working_set, _, site, line, offset| {
            self.query_runs_user_closure(working_set, site, line, offset)
        })
    }

    /// Parse once; if this site needs a user closure, dispatch on this thread.
    ///
    /// Results are not cached: a picker (`fzf`, `input list`, `carapace`) is not a function
    /// of the line, so a stored pick would skip the UI on the next Tab.
    fn complete_user_closure_on_repl_thread(&self, query: &CompletionQuery) -> Option<Suggestions> {
        self.with_completion_site(query, |working_set, block, site, line, offset| {
            if !self.query_runs_user_closure(working_set, site, line, offset) {
                return None;
            }
            let _tty = crate::util::ReplTerminalGuard::capture();
            let dispatched = self.fetch_completions_by_block(
                Arc::clone(block),
                working_set,
                query.cursor(),
                offset,
                line,
            );
            Some(
                dispatched
                    .suggestions
                    .into_iter()
                    .map(|semantic_suggestion| semantic_suggestion.suggestion)
                    .collect(),
            )
        })
    }

    fn has_external_completer(&self) -> bool {
        self.engine_state
            .get_config()
            .completions
            .external
            .completer
            .is_some()
    }

    fn signature_runs_user_closure(&self, signature: &Signature) -> bool {
        match signature.complete {
            Some(CommandWideCompleter::Command(_)) => true,
            Some(CommandWideCompleter::External) => self.has_external_completer(),
            None => false,
        }
    }

    fn site_runs_user_closure(&self, site: &CompletionSite, working_set: &StateWorkingSet) -> bool {
        match &site.kind {
            SiteKind::ExternalArg { .. } => self.has_external_completer(),
            SiteKind::FlagName { call, .. } => {
                self.signature_runs_user_closure(&working_set.get_decl(call.decl_id).signature())
            }
            SiteKind::FlagValue { call, flag, .. } => {
                let signature = working_set.get_decl(call.decl_id).signature();
                let resolved = find_flag(&signature, *flag);
                let custom = resolved.as_ref().and_then(|flag| flag.completion.as_ref());
                matches!(custom, Some(Completion::Command(_)))
                    || self.signature_runs_user_closure(&signature)
            }
            SiteKind::Positional {
                call,
                sig_positional,
                ..
            } => {
                let signature = working_set.get_decl(call.decl_id).signature();
                let custom = signature
                    .get_positional(*sig_positional)
                    .and_then(|positional| positional.completion.as_ref());
                matches!(custom, Some(Completion::Command(_)))
                    || self.signature_runs_user_closure(&signature)
            }
            _ => false,
        }
    }

    fn multiword_argument_runs_user_closure(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        contents: &str,
        offset: usize,
    ) -> bool {
        if !matches!(site.kind, SiteKind::Command { .. })
            || !working_set
                .get_span_contents(site.span)
                .iter()
                .any(u8::is_ascii_whitespace)
        {
            return false;
        }

        let mut parse_ws = StateWorkingSet::new(&self.engine_state);
        let _ = parse_ws.add_file("completer", contents.as_bytes());
        let Some(shorter) = parse_shorter_head_reading(&mut parse_ws, site.span, None) else {
            return false;
        };
        let position = site.cursor.saturating_sub(offset);
        let shorter_site = self.finalize_site(
            self.resolve_expression_site(&shorter, site.cursor, &parse_ws),
            contents,
            position,
            offset,
        );
        self.site_runs_user_closure(&shorter_site, &parse_ws)
    }

    pub fn fetch_completions_at(&self, line: &str, position: usize) -> Vec<SemanticSuggestion> {
        self.dispatch_completions_at(line, position).suggestions
    }

    fn dispatch_completions_at(&self, line: &str, position: usize) -> Dispatched {
        let safe_position = line.floor_char_boundary(position);
        // Parse only up to the cursor, so the last pipeline element is always the token (or
        // gap) being edited; trailing whitespace is kept to distinguish a gap from the token.
        let sliced_line = &line[..safe_position];

        let mut working_set = StateWorkingSet::new(&self.engine_state);
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
            safe_position,
            span_offset,
            sliced_line,
        )
    }

    pub fn fetch_completions_within_file(
        &self,
        filename: &str,
        position: usize,
        contents: &str,
    ) -> Vec<SemanticSuggestion> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);

        // `parse` must run first: it registers the file and its spans in `working_set`.
        let block = parse(&mut working_set, Some(filename), contents.as_bytes(), false);

        let Some(file_span) = working_set.get_span_for_filename(filename) else {
            return Vec::new();
        };

        self.fetch_completions_by_block(block, &working_set, position, file_span.start, contents)
            .suggestions
    }

    /// `position` is the cursor as a buffer-relative byte offset into `contents`.
    fn fetch_completions_by_block(
        &self,
        block: Arc<Block>,
        working_set: &StateWorkingSet,
        position: usize,
        offset: usize,
        contents: &str,
    ) -> Dispatched {
        let site = self.resolve_completion_site(&block, working_set, position, offset, contents);
        let mut dispatched = self.dispatch_completion_site(&site, working_set, offset);

        // A multi-word head is ambiguous: also recover the argument reading of the shorter
        // command and offer it before the subcommand name.
        let argument_reading =
            self.complete_multiword_head_as_argument(&site, working_set, offset, contents);
        dispatched.cacheable |= argument_reading.cacheable;
        dispatched
            .suggestions
            .splice(..0, argument_reading.suggestions);
        dispatched
    }

    /// A multi-word head is ambiguous: `baz --test bar` is also `bar`, the value of
    /// `baz --test`'s flag. Recover that argument reading via [`parse_shorter_head_reading`]
    /// over the real buffer spans (avoiding the stale-span hazard of #5127), dropping
    /// command-kind results the primary dispatch already offers.
    fn complete_multiword_head_as_argument(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        offset: usize,
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

        let mut parse_ws = StateWorkingSet::new(&self.engine_state);
        let _ = parse_ws.add_file("completer", contents.as_bytes());
        let Some(shorter) = parse_shorter_head_reading(&mut parse_ws, site.span, None) else {
            return Dispatched::default();
        };

        let position = site.cursor.saturating_sub(offset);
        let shorter_site = self.finalize_site(
            self.resolve_expression_site(&shorter, site.cursor, &parse_ws),
            contents,
            position,
            offset,
        );
        // A command-head result means no distinct argument; leave it to the primary dispatch.
        if matches!(shorter_site.kind, SiteKind::Command { .. }) {
            return Dispatched::default();
        }

        let mut dispatched = self.dispatch_completion_site(&shorter_site, &parse_ws, offset);
        // Drop command-kind results; only the argument value is contributed here.
        dispatched
            .suggestions
            .retain(|candidate| !matches!(candidate.kind, Some(SuggestionKind::Command(..))));
        dispatched
    }

    /// Dispatches the completion site to the appropriate specialized completer.
    fn dispatch_completion_site(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        offset: usize,
    ) -> Dispatched {
        let completion_context =
            self.context(working_set, site.span, site.typed_prefix.as_bytes(), offset);

        match &site.kind {
            SiteKind::Command { node } => {
                let completions = self.command_completion_helper(
                    working_set,
                    site.span,
                    offset,
                    self.command_completion_for_head(*node, site.span, working_set),
                );

                if completions.suggestions.is_empty() {
                    self.suggestions_at(&mut FileCompletion, working_set, site.span, offset)
                } else {
                    completions
                }
            }

            SiteKind::FlagName { .. }
            | SiteKind::FlagValue { .. }
            | SiteKind::Positional { .. } => {
                self.dispatch_call_completion_site(site, working_set, offset, &completion_context)
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

            SiteKind::Variable => {
                self.variable_names_completion_helper(working_set, site.span, offset)
            }

            SiteKind::AttributeName => AttributeCompletion.fetch(&completion_context).into(),

            SiteKind::AttributableItem => AttributableCompletion.fetch(&completion_context).into(),

            SiteKind::ExternalArg { .. } => {
                self.dispatch_external_arg(site, working_set, offset, &completion_context)
            }

            SiteKind::File => {
                self.suggestions_at(&mut FileCompletion, working_set, site.span, offset)
            }
        }
    }

    /// Complete an external call argument: `sudo`/`doas` special-case, the configured
    /// external completer, then file completion as a fallback.
    fn dispatch_external_arg(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        offset: usize,
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
                    site.span,
                    offset,
                    CommandCompletion::new(CommandScope::All),
                );
                if !commands.suggestions.is_empty() {
                    return commands;
                }
            }
        }

        let mut dispatched = Dispatched::default();
        let mut external_answered = false;

        // The user's configured external completer (`$env.config.completions.external.completer`).
        if let Some(closure) = self
            .engine_state
            .get_config()
            .completions
            .external
            .completer
            .as_ref()
        {
            let mut completion = CommandWideCompletion::closure(closure, external_call);
            let fetched = completion.fetch(completion_context);
            external_answered = !fetched.needs_fallback();
            dispatched.merge(fetched.into());
        }

        // Internal subcommands extending this call (e.g. `fod br` → `food bar`), which
        // suppress the file fallback like an internal call's arguments do.
        let subcommands =
            self.subcommand_suggestions(working_set, external_call.span.start, site.cursor, offset);

        // File completion for path arguments, only when nothing more specific answered.
        if !external_answered
            && dispatched.suggestions.is_empty()
            && subcommands.suggestions.is_empty()
        {
            dispatched.merge(self.suggestions_at(
                &mut FileCompletion,
                working_set,
                site.span,
                offset,
            ));
        }

        dispatched.merge(subcommands);
        dispatched
    }

    /// Dispatch completions for call-bound sites (FlagName, FlagValue, Positional).
    fn dispatch_call_completion_site(
        &self,
        site: &CompletionSite,
        working_set: &StateWorkingSet,
        offset: usize,
        completion_context: &Context,
    ) -> Dispatched {
        // Only call-bound kinds carry a call and element; anything else is an error here.
        let (call, element) = match &site.kind {
            SiteKind::FlagName { call, element }
            | SiteKind::FlagValue { call, element, .. }
            | SiteKind::Positional { call, element, .. } => (*call, *element),
            _ => return Dispatched::default(),
        };

        let signature = working_set.get_decl(call.decl_id).signature();

        // Subcommands extending this command line are always offered, and suppress the
        // file-path fallback: a matched subcommand shouldn't also dump the whole directory.
        let subcommands =
            self.subcommand_suggestions(working_set, call.head.start, site.cursor, offset);

        // The value kinds share one shape; only the `ArgType`, custom-completer, and declared
        // shape lookups differ.
        let argument_value = |engine: &Self, arg_type, custom, arg_slot, declared_shape| {
            engine.complete_argument_value(
                custom,
                ArgValueCompletion {
                    call,
                    arg_type,
                    need_fallback: subcommands.suggestions.is_empty(),
                    completer: engine,
                    arg_idx: arg_slot,
                    declared_shape,
                    cursor: site.cursor,
                },
                completion_context,
                &signature,
                element,
                site.cursor,
            )
        };

        let mut results = match &site.kind {
            SiteKind::FlagName { .. } => {
                self.complete_flag_names(call.decl_id, completion_context, &signature, element)
            }
            SiteKind::FlagValue { flag, arg_slot, .. } => {
                let resolved = find_flag(&signature, *flag);
                argument_value(
                    self,
                    ArgType::Flag(Cow::Borrowed(flag.name())),
                    resolved.as_ref().and_then(|flag| flag.completion.clone()),
                    *arg_slot,
                    resolved.and_then(|flag| flag.arg),
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
            _ => Dispatched::default(),
        };

        results.merge(subcommands);
        results
    }

    /// Resolves the contextual state and constraints at the cursor's location.
    pub(crate) fn resolve_completion_site<'a>(
        &self,
        block: &'a Block,
        working_set: &'a StateWorkingSet,
        position: usize,
        offset: usize,
        contents: &'a str,
    ) -> CompletionSite<'a> {
        let absolute_position = position + offset;

        // The token whose span the cursor is inside of, or at the trailing edge of.
        let touched_expression = block
            .find_map(working_set, &|expression: &Expression| {
                find_pipeline_element_by_position(expression, working_set, absolute_position)
            })
            .or_else(|| check_redirection_in_block(block, absolute_position))
            // Otherwise the cursor is in a whitespace gap after the element it trails.
            .or_else(|| trailing_gap_element(block, working_set, absolute_position));

        let site = match touched_expression {
            Some(expression) => {
                self.resolve_expression_site(expression, absolute_position, working_set)
            }
            None => self.resolve_fallback_site(block, working_set, absolute_position),
        };

        self.finalize_site(site, contents, position, offset)
    }

    /// Fill the centrally-derived `typed_prefix`/`cursor` fields from the final `site.span`,
    /// so the prefix and replacement span can never disagree. Point spans yield an empty
    /// prefix.
    fn finalize_site<'a>(
        &self,
        mut site: CompletionSite<'a>,
        contents: &'a str,
        position: usize,
        offset: usize,
    ) -> CompletionSite<'a> {
        let token_start = site.span.start.saturating_sub(offset);
        site.typed_prefix = contents
            .get(token_start..position)
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed(""));
        site.cursor = position + offset;
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

        // Base case: file completion; overridden below where the expression warrants it.
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
            _ => CompletionSite::new(SiteKind::File, expression.span), // The default `File` setup holds
        }
    }

    /// Resolve a bare external call (`git checkout`). The head completes as a command;
    /// other positions are [`SiteKind::ExternalArg`].
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

        // Classify the slot the cursor trails after the last argument: a pending flag value,
        // a new flag name, or a new positional. Looking only at the non-whitespace token
        // ending at the cursor keeps `cmd -f val ⌶` (positional) and `cmd --⌶` (flag name)
        // distinct, and its span preserves the `-`/`--` prefix.
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
            // Intentionally out-of-range `arg_slot`: there is no in-progress argument node
            // yet, and `ArgValueCompletion` reads `None` as exactly that.
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

    /// The last row-condition term when the cursor trails it (`where name ⌶`): the LHS of
    /// an operator the user is about to type.
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

    /// The [`FlagRef`] of a last-argument flag still awaiting its value (`cmd --opt ⌶`).
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
                    // Only the name is being completed: `Argument::span` would also cover
                    // the value written after it (`--endian big`), and `name.span` is the
                    // flag token itself for both spellings.
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

        // Past the last attribute is the decorated item's slot, even when the parser found
        // no item to give a span to (`@complete "c"⏎⌶`). Earlier gaps sit between two
        // attributes, where another attribute name is what's being typed.
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

        // A bare `@` opens an attribute name; trailing a completed attribute block completes
        // the attributable item itself; otherwise a fresh command position.
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
        element_expression: &Expression,
        cursor: usize,
    ) -> Dispatched {
        let mut results = Dispatched::default();

        if let Some(custom) = custom {
            let attempt = match custom {
                // A command declared an engine-provided completion for this argument.
                Completion::Builtin(kind) => self.complete_builtin(kind, &arg_value, context),
                // A custom completer receives the element text up to the cursor
                // (`my-command foobar`), so its spans are anchored to the element's start.
                other => {
                    let element_line = String::from_utf8_lossy(
                        context
                            .working_set
                            .get_span_contents(Span::new(element_expression.span.start, cursor)),
                    );
                    self.custom_completion_helper(other, element_line.as_ref(), context, cursor)
                }
            };
            let need_fallback = attempt.needs_fallback();
            results.merge(attempt.into());
            if !need_fallback {
                return results;
            }
        }

        let attempt = self.command_wide_completion_helper(signature, element_expression, context);
        let need_fallback = attempt.needs_fallback();
        results.merge(attempt.into());
        if !need_fallback {
            return results;
        }

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
        element_expression: &Expression,
    ) -> Dispatched {
        let mut results: Dispatched = FlagCompletion { decl_id }.fetch(context).into();
        results.merge(
            self.command_wide_completion_helper(signature, element_expression, context)
                .into(),
        );
        results
    }

    fn suggestions_at<C: Completer>(
        &self,
        completer: &mut C,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
    ) -> Dispatched {
        completer
            .fetch(&self.context(
                working_set,
                span,
                working_set.get_span_contents(span),
                offset,
            ))
            .into()
    }

    fn variable_names_completion_helper(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
    ) -> Dispatched {
        let prefix = working_set.get_span_contents(span);
        if !prefix.starts_with(b"$") {
            return Dispatched::default();
        }
        let ctx = self.context(working_set, span, prefix, offset);
        VariableCompletion.fetch(&ctx).into()
    }

    fn command_completion_helper(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
        mut command_completion: CommandCompletion,
    ) -> Dispatched {
        let prefix = working_set.get_span_contents(span);
        let ctx = self.context(working_set, span, prefix, offset);
        command_completion.fetch(&ctx).into()
    }

    /// Command-completion scope for a command head, honouring a leading sigil: `^` →
    /// externals only, `%` → built-ins only, otherwise everything. The sigil is the byte
    /// between the call's own span and its head span.
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

    /// Internal commands whose name extends the command line typed so far (`foo test⌶`
    /// also offers `foo test bar`). Externals are excluded: a multi-word line names only
    /// internal subcommands.
    fn subcommand_suggestions(
        &self,
        working_set: &StateWorkingSet,
        command_start: usize,
        cursor: usize,
        offset: usize,
    ) -> Dispatched {
        if cursor <= command_start {
            return Dispatched::default();
        }
        self.command_completion_helper(
            working_set,
            Span::new(command_start, cursor),
            offset,
            CommandCompletion::new(CommandScope::InternalsOnly),
        )
    }

    fn custom_completion_helper(
        &self,
        custom_completion: Completion,
        input: &str,
        context: &Context,
        pos: usize,
    ) -> Fetched {
        match custom_completion {
            Completion::Command(decl_id) => {
                let mut completer =
                    CustomCompletion::new(decl_id, input.into(), pos - context.offset);
                completer.fetch(context)
            }
            Completion::List(list) => {
                let mut completer = StaticCompletion::new(list);
                completer.fetch(context)
            }
            // Engine-provided completions are handled in `complete_argument_value`; decline
            // if one arrives by another path.
            Completion::Builtin(_) => Fetched::Absent,
        }
    }

    fn command_wide_completion_helper(
        &self,
        signature: &Signature,
        element_expression: &Expression,
        context: &Context,
    ) -> Fetched {
        let completion = match signature.complete {
            Some(CommandWideCompleter::Command(decl_id)) => {
                CommandWideCompletion::command(context.working_set, decl_id, element_expression)
            }
            Some(CommandWideCompleter::External) => self
                .engine_state
                .get_config()
                .completions
                .external
                .completer
                .as_ref()
                .map(|closure| CommandWideCompletion::closure(closure, element_expression)),
            None => None,
        };

        match completion {
            Some(mut completion) => {
                let context = Context {
                    prefix: b"",
                    ..*context
                };
                completion.fetch(&context)
            }
            None => Fetched::Absent,
        }
    }

    pub(crate) fn context<'a>(
        &'a self,
        working_set: &'a StateWorkingSet,
        span: Span,
        prefix: &'a [u8],
        offset: usize,
    ) -> Context<'a> {
        Context {
            working_set,
            stack: self.stack.as_ref(),
            options: &self.options,
            span,
            prefix,
            offset,
        }
    }

    pub(crate) fn options(&self) -> &CompletionOptions {
        &self.options
    }
}

pub struct NuCompleter {
    engine: CompletionEngine,
    cache: NarrowingCache,
    /// The [`CacheEnv`] of every entry this completer stores/reads; computed once per
    /// completer, not on [`CompletionEngine`] (which non-caching callers also build).
    cache_env: CacheEnv,
    worker: Option<CompletionWorker>,
    /// Whether [`complete`](ReedlineCompleter::complete) offloads to a worker.
    /// False only on the REPL path with `background-completions` disabled.
    background: bool,
}

impl NuCompleter {
    pub fn new(engine_state: Arc<EngineState>, stack: Arc<Stack>) -> Self {
        Self::with_cache(engine_state, stack, NarrowingCache::default())
    }

    /// The reedline completer; the only constructor that consults the
    /// `background-completions` experimental option.
    pub(crate) fn for_repl(
        engine_state: Arc<EngineState>,
        stack: Arc<Stack>,
        cache: NarrowingCache,
    ) -> Self {
        let mut completer = Self::with_cache(engine_state, stack, cache);
        completer.background = nu_experimental::BACKGROUND_COMPLETIONS.get();
        completer
    }

    pub(crate) fn with_cache(
        engine_state: Arc<EngineState>,
        stack: Arc<Stack>,
        cache: NarrowingCache,
    ) -> Self {
        let engine = CompletionEngine::new(engine_state, stack);
        let cache_env = CacheEnv::of(&engine.engine_state, &engine.stack);
        // Read fresh each prompt so `cache_size` config changes take effect.
        let cache_size = engine.engine_state.get_config().completions.cache_size;
        cache.set_capacity(cache_size.try_into().unwrap_or(0));
        Self {
            engine,
            cache,
            cache_env,
            worker: None,
            background: true,
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
            .narrowed_fallback(query, self.cache_env, self.engine.options())
    }

    fn spawn_worker(engine: &CompletionEngine) -> CompletionWorker {
        let (request_tx, request_rx) = mpsc::channel::<CompletionQuery>();
        let (result_tx, result_rx) = mpsc::channel::<Completed>();

        let engine = engine.to_background();
        thread::spawn(move || {
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

    // Narrow a window into the first value rather than allocating a `String`; runs every
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
        if !self.background {
            // Inline on this thread, skipping worker and cache: the pre-#18334
            // blocking behavior, which had neither. Blocking is the point, so
            // an interactive completer can own the terminal.
            let suggestions: Suggestions = self
                .engine
                .fetch_completions_at(line, pos)
                .into_iter()
                .map(|s| s.suggestion)
                .collect();
            let partial = partial_of(line, &suggestions);
            return CompletionResult::fresh(suggestions).with_partial(partial);
        }

        let query = CompletionQuery::new(line, pos);
        self.drain_completed();

        if let Some(suggestions) = self.fresh_for(&query) {
            self.settle_pending(&query);
            let partial = partial_of(line, &suggestions);
            return CompletionResult::fresh(suggestions).with_partial(partial);
        }

        if let Some(suggestions) = self.engine.complete_user_closure_on_repl_thread(&query) {
            self.settle_pending(&query);
            let partial = partial_of(line, &suggestions);
            return CompletionResult::fresh(suggestions).with_partial(partial);
        }

        let fallback = self.stale_fallback(&query);
        let partial = partial_of(line, &fallback);

        let worker = self
            .worker
            .get_or_insert_with(|| Self::spawn_worker(&self.engine));

        if worker.pending.as_ref() != Some(&query) {
            if worker.request_tx.send(query.clone()).is_ok() {
                worker.pending = Some(query);
            } else {
                // Worker died (a panic in a user completer closure kills it); drop it so the
                // next request spawns a replacement.
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
        // The empty positional slot after `from csv ` is answered with file names at a point
        // span; typing `--sep` appends no boundary, so only the flag check keeps them from
        // following it.
        let base = q("from csv ");
        assert!(!q("from csv --sep").narrows(&base, token(base.cursor())));

        // Extending a flag the user was already typing stays sound.
        assert!(q("from csv --sep").narrows(&q("from csv --s"), token(9)));
    }

    #[test]
    fn background_engine_suppresses_stdin() {
        let engine = test_engine();
        let stack = Arc::new(Stack::new());
        let foreground = CompletionEngine::new(engine, stack);
        assert!(!foreground.stack.suppress_stdin);
        let background = foreground.to_background();
        assert!(background.stack.suppress_stdin);
    }

    #[test]
    fn internal_command_completion_stays_on_the_worker() {
        let engine = CompletionEngine::new(test_engine(), Arc::new(Stack::new()));
        assert!(!engine.should_complete_on_repl_thread(&q("ls | c")));
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

    fn engine_with_external_completer() -> Arc<EngineState> {
        let mut engine = (*test_engine()).clone();
        let mut stack = Stack::new();
        apply_source(
            &mut engine,
            &mut stack,
            b"$env.config.completions.external.completer = {|spans| $spans}",
        );
        Arc::new(engine)
    }

    #[test]
    fn external_arg_with_completer_runs_on_the_repl_thread() {
        let engine =
            CompletionEngine::new(engine_with_external_completer(), Arc::new(Stack::new()));
        assert!(engine.should_complete_on_repl_thread(&q("nvim foo")));
        assert!(!engine.should_complete_on_repl_thread(&q("ls | c")));
    }

    /// Opted out: settles inline, spawns no worker, leaves the cache alone.
    #[test]
    fn opted_out_completer_settles_inline() {
        let mut completer = NuCompleter::new(test_engine(), Arc::new(Stack::new()));
        completer.background = false;

        let result = completer.complete("ls | c", 6);
        assert!(
            matches!(result, CompletionResult::Fresh { .. }),
            "expected a settled result, got {result:?}"
        );
        assert!(result.suggestions().iter().any(|s| s.value == "cd"));
        assert!(completer.worker.is_none(), "a worker was spawned anyway");
        assert_eq!(completer.poll_completion(), CompletionStatus::Idle);
    }

    /// Engine whose external completer reports what its evaluation environment
    /// allowed: `piped-N` if a piped external's stdout reached `lines`,
    /// `direct-S` if a final external's stdout was captured.
    fn probe_engine() -> (Arc<EngineState>, Arc<Stack>) {
        let mut engine =
            nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
        let mut stack = Stack::new();
        let cwd = std::env::temp_dir()
            .to_string_lossy()
            .trim_end_matches(['/', '\\'])
            .replace('\\', "/");
        stack.add_env_var(
            "PWD".to_string(),
            nu_protocol::Value::string(&cwd, Span::unknown()),
        );
        // External lookup needs a PATH; `Stack::new` starts with no env at all.
        stack.add_env_var(
            "PATH".to_string(),
            nu_protocol::Value::string(std::env::var("PATH").unwrap_or_default(), Span::unknown()),
        );

        let setup = r#"$env.config.completions.external = {
                enable: true
                completer: {|spans|
                    let piped = ("alpha\nbeta\n" | lines | length)
                    let direct = ('gamma' | str trim)
                    [$"piped-($piped)" $"direct-($direct)"]
                }
            }"#;
        let mut working_set = StateWorkingSet::new(&engine);
        let block = nu_parser::parse(&mut working_set, None, setup.as_bytes(), false);
        assert!(working_set.parse_errors.is_empty(), "setup failed to parse");
        engine.merge_delta(working_set.render()).expect("merge");
        nu_engine::eval_block::<nu_protocol::debugger::WithoutDebug>(
            &engine,
            &mut stack,
            &block,
            nu_protocol::PipelineData::empty(),
        )
        .expect("eval setup");
        engine.merge_env(&mut stack).expect("merge env");

        (Arc::new(engine), Arc::new(stack))
    }

    /// Externals inside a completer keep their stdout on both stacks:
    /// `suppress_output` only sets `out_dest.stdout` (final command), while
    /// `collect_value` sets `pipe_stdout` (piped stages). Thus the opt-out only
    /// has to stop offloading; the stack needs no changes.
    #[rstest::rstest]
    #[case::foreground(false)]
    #[case::background(true)]
    fn externals_in_a_completer_keep_their_stdout(#[case] suppress_stdin: bool) {
        let (engine, stack) = probe_engine();
        let engine = CompletionEngine::new(engine, isolated_stack(stack, suppress_stdin));

        let values: Vec<String> = engine
            .fetch_completions_at("somecmd x", 9)
            .into_iter()
            .map(|s| s.suggestion.value)
            .collect();

        assert!(
            values.iter().any(|v| v == "piped-2"),
            "a piped external lost its stdout: {values:?}"
        );
        assert!(
            values.iter().any(|v| v == "direct-gamma"),
            "a final external lost its stdout: {values:?}"
        );
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
            CompletionEngine::new(engine, Arc::new(Stack::new()))
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

    /// …but not across a `cd`/`$env.PATH` change — the reason [`CacheEnv`] exists.
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

    /// A cached answer stands in for the computed one, so the two must agree on order.
    /// Re-sorting the cache put `config/` after `config.nu`, inverting every keystroke.
    #[test]
    fn narrowing_cache_skips_fallback_matches() {
        let cache = NarrowingCache::default();
        let env = CacheEnv::of(&test_engine(), &Stack::new());
        let span = reedline::Span::new(3, 6);
        let cached = vec![Suggestion {
            value: "foobar".to_string(),
            span,
            ..Suggestion::default()
        }]
        .into();
        let options = CompletionOptions {
            match_algorithm: MatchAlgorithm::Fallback,
            ..Default::default()
        };

        cache.store(CompletionQuery::new("ls foo", 6), env, cached);

        let narrowed = cache.narrowed_fallback(&CompletionQuery::new("ls fooz", 7), env, &options);

        assert!(narrowed.is_empty());
    }

    #[test]
    fn a_narrowed_cache_answer_keeps_the_order_it_was_given() {
        let cache = NarrowingCache::default();
        let env = CacheEnv::of(&test_engine(), &Stack::new());
        let span = reedline::Span::new(3, 5);

        // The order file completion produces: the directory first, ranked as `config`.
        let cached: Suggestions = ["config/", "config.nu"]
            .iter()
            .map(|value| Suggestion {
                value: (*value).to_string(),
                span,
                ..Default::default()
            })
            .collect::<Vec<_>>()
            .into();

        cache.store(CompletionQuery::new("ls co", 5), env, cached);

        let narrowed = cache.narrowed_fallback(
            &CompletionQuery::new("ls con", 6),
            env,
            &CompletionOptions::default(),
        );

        let values: Vec<&str> = narrowed.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(
            values,
            ["config/", "config.nu"],
            "the cached answer must not reorder what it stands in for"
        );
    }
}
