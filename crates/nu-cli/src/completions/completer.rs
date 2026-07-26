use crate::completions::{
    ArgValueCompletion, AttributableCompletion, AttributeCompletion, CellPathCompletion,
    CommandCompletion, CommandScope, Completer, CompletionOptions, CustomCompletion,
    DotNuCompletion, EnvVarCompletion, FileCompletion, FlagCompletion, NuMatcher,
    OperatorCompletion, VariableCompletion,
    base::{Fetched, SemanticSuggestion},
};
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
    Completer as ReedlineCompleter, CompletionResult, CompletionStatus, Partial, Suggestion,
    Suggestions,
};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use std::{borrow::Cow, ops::ControlFlow};
use std::{collections::HashMap, path::is_separator};

/// How long a cached completion stays usable.
///
/// Bounds staleness only against changes [`CacheEnv`] cannot observe — a file created in a
/// *subdirectory*, a binary installed into a directory already on `PATH`. It must outlive a
/// prompt, since the REPL rebuilds the completer per line but carries the cache across.
const CACHE_TTL: Duration = Duration::from_secs(60);

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
                    // `touches`, not `contains`: for a complicated external head like
                    // `^(ls | e⌶` the cursor sits at the head's trailing edge, which the
                    // end-exclusive `contains` would miss (issue #7648).
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
            // `use std/util [E, T⌶`: the bracketed import list parses as a `List` wrapped in
            // a `FullCellPath` positional. The cursor is inside the list, but completing an
            // import-list member is the enclosing *call*'s job (it knows the module), not a
            // cell-path/variable completion. Decline to claim the position so the parent
            // `Expr::Call` selects itself and routes to argument completion.
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
/// Results depend on cwd, `PATH`, and known declarations, which change between prompts
/// while the query text does not — so the query alone is not a sound cache key.
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
        // The directory's own mtime, so a command that adds or removes a file in it
        // (`touch`, `rm`, `mkdir`) invalidates the file completions it just made wrong,
        // rather than leaving them to age out. Only the working directory is stamped —
        // walking further would cost more than it saves — so a change inside a
        // *subdirectory* is left to [`CACHE_TTL`].
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
    at: Instant,
}

impl CacheEntry {
    /// Whether this entry may still answer a query: produced in the same environment, and
    /// recent enough that an unobservable change is unlikely. See [`CACHE_TTL`].
    fn is_usable(&self, env: CacheEnv) -> bool {
        self.env == env && self.at.elapsed() < CACHE_TTL
    }

    /// The span the cursor extends: the range the *last* suggestion replaces.
    ///
    /// `fetch_completions_by_block` keeps the cursor-anchored family last, so reading the
    /// last span is the correct one to extend.
    fn reference_span(&self) -> Option<reedline::Span> {
        self.suggestions.last().map(|suggestion| suggestion.span)
    }
}

#[derive(Clone, Default)]
pub(crate) struct NarrowingCache {
    entries: Arc<Mutex<HashMap<CompletionQuery, CacheEntry>>>,
}

impl NarrowingCache {
    pub(crate) fn fresh(&self, query: &CompletionQuery, env: CacheEnv) -> Option<Suggestions> {
        let entries = self.entries.lock().ok()?;
        let cache_entry = entries.get(query)?;

        cache_entry
            .is_usable(env)
            .then(|| cache_entry.suggestions.clone())
    }

    pub(crate) fn store(&self, query: CompletionQuery, env: CacheEnv, suggestions: Suggestions) {
        if let Ok(mut entries) = self.entries.lock() {
            // Also drops everything computed in a since-replaced environment, so a `cd`
            // clears the file completions it invalidated rather than leaving them to age out.
            entries.retain(|_, cache_entry| cache_entry.is_usable(env));
            entries.insert(
                query,
                CacheEntry {
                    suggestions,
                    env,
                    at: Instant::now(),
                },
            );
        }
    }

    pub(crate) fn narrowed_fallback(
        &self,
        query: &CompletionQuery,
        env: CacheEnv,
        options: &CompletionOptions,
    ) -> Suggestions {
        // Only the `Arc` is cloned under the lock; matching, and the handful of suggestion
        // clones that survive it, happen after the lock is dropped.
        let Some((base_suggestions, reference_span)) =
            self.entries.lock().ok().and_then(|entries| {
                entries
                    .iter()
                    .filter(|(_, cache_entry)| cache_entry.is_usable(env))
                    .filter_map(|(base_query, cache_entry)| {
                        // The entry's own replacement span anchors the comparison: it is
                        // where the in-progress token starts, which is what lets `narrows`
                        // compare the tokens themselves rather than just the query texts.
                        let reference_span = cache_entry.reference_span()?;
                        query.narrows(base_query, reference_span).then_some((
                            base_query.cursor(),
                            Arc::clone(&cache_entry.suggestions),
                            reference_span,
                        ))
                    })
                    .max_by_key(|(cursor, _, _)| *cursor)
                    .map(|(_, suggestions, span)| (suggestions, span))
            })
        else {
            return Suggestions::default();
        };

        let Some(search_token) = query.typed().get(reference_span.start..) else {
            return Suggestions::default();
        };

        let candidates: Vec<&Suggestion> = base_suggestions
            .iter()
            .filter(|suggestion| suggestion.span == reference_span)
            .collect();

        // The matcher carries indices rather than suggestions, so each candidate's text can
        // be borrowed for the match test instead of copied for every candidate considered.
        let mut matcher = NuMatcher::new(search_token, options, true);
        for (index, candidate) in candidates.iter().enumerate() {
            matcher.add(candidate.display_value(), index);
        }

        let updated_span = reedline::Span::new(reference_span.start, query.cursor());
        matcher
            .results()
            .into_iter()
            .map(|(index, match_indices)| {
                let mut suggestion = candidates[index].clone();
                suggestion.span = updated_span;
                suggestion.match_indices = Some(match_indices);
                suggestion
            })
            .collect()
    }
}
