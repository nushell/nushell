//! Counters of what the winnow front end parsed, to tell how much of an input took which path.
//! They are always kept, for the whole process, and read with `nu_parser::winnow_stats()`: the
//! `frontends` harness (`crates/nu-parser/winnow/tools/nushell-harness`) prints them after
//! `compare` and `bench`.
//!
//! Each statement the front end could not read (one the lowering gives back, or one with a
//! syntax error the winnow parser did not go past), each block handed to nu-parser whole, each
//! run parsed ahead whose answers about command names changed, and each pipeline nu-parser's
//! own statement parser parsed is also logged at the debug level, with its reason, to find what
//! the front end still lacks. Every message starts with `winnow: `; to see them, run
//! `nu --log-level debug --log-include nu_parser::winnow`, or `frontends --log`.

use std::{
    fmt::Display,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
};

use log::{Level, debug, log_enabled};
use nu_protocol::{Span, engine::StateWorkingSet};

use super::lower::Unlowered;
use crate::winnow::syntax::Diagnostic;

// One counter per field of `WinnowStats`.
static WINNOW_BLOCKS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_BLOCKS: AtomicU64 = AtomicU64::new(0);
static LOWERED_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static UNLOWERED_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static ERROR_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static AHEAD_RUNS: AtomicU64 = AtomicU64::new(0);
static AHEAD_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static RECOVERED_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_PIPELINES: AtomicU64 = AtomicU64::new(0);

/// What the winnow front end has done in this process so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WinnowStats {
    /// Blocks and module bodies whose statements the winnow parser parsed.
    pub winnow_blocks: u64,
    /// Blocks handed to nu-parser whole (not UTF-8, or not taken by `BlockStatements::new`:
    /// they do not lex, or have an error about the whole block).
    pub classic_blocks: u64,
    /// Statements the winnow parser parsed and the lowering turned into the AST.
    pub lowered_statements: u64,
    /// Statements the lowering gave back: each is a garbage pipeline with an error.
    pub unlowered_statements: u64,
    /// Statements the winnow parser reported a syntax error in and did not go past: each is a
    /// garbage pipeline with an error.
    pub error_statements: u64,
    /// Runs of statements a second thread parsed ahead of the lowering.
    pub ahead_runs: u64,
    /// Of those, the runs in which the live working set answered a question about command names
    /// (or about their longest length) differently: the statements from there on were parsed
    /// again.
    pub ahead_fallbacks: u64,
    /// Statements with errors the winnow parser went on past (each left as a garbage node that
    /// the lowering reads again, reporting the error), lowered.
    pub recovered_statements: u64,
    /// Pipelines `nu-parser`'s own statement parser (`parse_pipeline`) parsed while the option
    /// was on: those of the blocks handed to it whole, and those of blocks the lowering had
    /// `nu-parser`'s value parsers read (a closure, a subexpression). With the option on there
    /// should be none.
    pub classic_pipelines: u64,
}

impl WinnowStats {
    /// The statements the front end could not read, each a garbage pipeline with an error:
    /// those the lowering gave back and those with a syntax error the winnow parser did not go
    /// past.
    pub fn unread_statements(&self) -> u64 {
        self.unlowered_statements + self.error_statements
    }
}

/// The counters so far.
pub fn winnow_stats() -> WinnowStats {
    WinnowStats {
        winnow_blocks: WINNOW_BLOCKS.load(Relaxed),
        classic_blocks: CLASSIC_BLOCKS.load(Relaxed),
        lowered_statements: LOWERED_STATEMENTS.load(Relaxed),
        unlowered_statements: UNLOWERED_STATEMENTS.load(Relaxed),
        error_statements: ERROR_STATEMENTS.load(Relaxed),
        ahead_runs: AHEAD_RUNS.load(Relaxed),
        ahead_fallbacks: AHEAD_FALLBACKS.load(Relaxed),
        recovered_statements: RECOVERED_STATEMENTS.load(Relaxed),
        classic_pipelines: CLASSIC_PIPELINES.load(Relaxed),
    }
}

/// Count a block whose statements the winnow parser parsed.
pub(super) fn record_winnow_block() {
    WINNOW_BLOCKS.fetch_add(1, Relaxed);
}

/// Count a block covering `span` handed to nu-parser whole, and log it with `why`.
pub(super) fn record_classic_block(why: impl Display, span: Span, working_set: &StateWorkingSet) {
    CLASSIC_BLOCKS.fetch_add(1, Relaxed);
    log_statement(format_args!("classic block ({why})"), span, working_set);
}

/// Count a run of statements parsed ahead on the second thread.
pub(super) fn record_ahead_run() {
    AHEAD_RUNS.fetch_add(1, Relaxed);
}

/// Count a run whose statements from one on were parsed again, because the live working set
/// answered a question about command names differently (`changed`), and log it.
pub(super) fn record_ahead_fallback(changed: &str) {
    AHEAD_FALLBACKS.fetch_add(1, Relaxed);
    debug!("winnow: parsed ahead with other command names: {changed}");
}

/// Count a statement with errors the winnow parser went on past, given to the lowering.
pub(super) fn record_recovered_statement() {
    RECOVERED_STATEMENTS.fetch_add(1, Relaxed);
}

/// Count a pipeline covering `span` that `nu-parser`'s statement parser parsed while the option
/// was on, and log it.
pub(crate) fn record_classic_pipeline(span: Span, working_set: &StateWorkingSet) {
    CLASSIC_PIPELINES.fetch_add(1, Relaxed);
    log_statement("nu-parser pipeline", span, working_set);
}

/// Count a statement the lowering turned into the AST.
pub(super) fn record_lowered_statement() {
    LOWERED_STATEMENTS.fetch_add(1, Relaxed);
}

/// Count a statement covering `span` the lowering gave back for `reason`, and log it.
pub(super) fn record_unlowered(span: Span, reason: &Unlowered, working_set: &StateWorkingSet) {
    UNLOWERED_STATEMENTS.fetch_add(1, Relaxed);
    log_statement(reason.reason(), span, working_set);
}

/// Count a statement covering `span` that the winnow parser reported `error` in and did not go
/// past, and log it.
pub(super) fn record_error_statement(
    span: Span,
    error: &Diagnostic,
    working_set: &StateWorkingSet,
) {
    ERROR_STATEMENTS.fetch_add(1, Relaxed);
    log_statement(
        format_args!("syntax error ({})", error.kind),
        span,
        working_set,
    );
}

/// Log `span`, which the winnow front end could not read or handed to nu-parser: `why`, its
/// length and its first line of code.
fn log_statement(why: impl Display, span: Span, working_set: &StateWorkingSet) {
    if log_enabled!(Level::Debug) {
        let text = String::from_utf8_lossy(working_set.get_span_contents(span));
        let first_line = text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .unwrap_or_default();
        debug!("winnow: {why}: {} bytes: {first_line}", span.len());
    }
}
