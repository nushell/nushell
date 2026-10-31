//! Counters of what the winnow front end parsed and what it handed to the classic parser, to
//! tell how much of an input took which path. They are always kept, for the whole process, and
//! read with `nu_parser::winnow_stats()`: the `frontends` harness
//! (`crates/nu-parser/winnow/tools/nushell-harness`) prints them after `compare` and `bench`.
//!
//! Each hand-over to the classic parser is also logged at the debug level, with its reason, to
//! find what the lowering still lacks: a statement the lowering gives back, the rest of a block
//! from a statement the winnow parser reported a syntax error in, and each run parsed ahead
//! whose answers about command names changed. Every message starts with `winnow: `; to see
//! them, run `nu --log-level debug --log-include nu_parser::winnow`, or `frontends --log`.

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
static CLASSIC_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_STATEMENT_BYTES: AtomicU64 = AtomicU64::new(0);
static ERROR_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static AHEAD_RUNS: AtomicU64 = AtomicU64::new(0);
static AHEAD_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static RECOVERED_STATEMENTS: AtomicU64 = AtomicU64::new(0);

/// What the winnow front end has done in this process so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WinnowStats {
    /// Blocks and module bodies whose statements the winnow parser parsed.
    pub winnow_blocks: u64,
    /// Blocks handed to the classic parser whole (not UTF-8, or not taken by
    /// `BlockStatements::new`: they do not lex, or have an error about the whole block).
    pub classic_blocks: u64,
    /// Statements the winnow parser parsed and the lowering turned into the AST.
    pub lowered_statements: u64,
    /// Hand-overs to the classic parser, which parses from the source: a statement the
    /// lowering gave back or the winnow parser reported an error in, or the rest of a block,
    /// counted once.
    pub classic_statements: u64,
    /// The bytes of those hand-overs.
    pub classic_statement_bytes: u64,
    /// Of those, the statements the winnow parser reported an error in (each with the rest of
    /// its block where the winnow parser could not tell where it ends).
    pub error_statements: u64,
    /// Runs of statements a second thread parsed ahead of the lowering.
    pub ahead_runs: u64,
    /// Of those, the runs in which the live working set answered a question about command names
    /// (or about their longest length) differently: the classic parser took the rest of the
    /// block.
    pub ahead_fallbacks: u64,
    /// Statements with errors the winnow parser went on past (each left as a garbage node that
    /// the lowering reads again, reporting the error), lowered rather than handed over.
    pub recovered_statements: u64,
}

/// The counters so far.
pub fn winnow_stats() -> WinnowStats {
    WinnowStats {
        winnow_blocks: WINNOW_BLOCKS.load(Relaxed),
        classic_blocks: CLASSIC_BLOCKS.load(Relaxed),
        lowered_statements: LOWERED_STATEMENTS.load(Relaxed),
        classic_statements: CLASSIC_STATEMENTS.load(Relaxed),
        classic_statement_bytes: CLASSIC_STATEMENT_BYTES.load(Relaxed),
        error_statements: ERROR_STATEMENTS.load(Relaxed),
        ahead_runs: AHEAD_RUNS.load(Relaxed),
        ahead_fallbacks: AHEAD_FALLBACKS.load(Relaxed),
        recovered_statements: RECOVERED_STATEMENTS.load(Relaxed),
    }
}

/// Count a block whose statements the winnow parser parsed.
pub(super) fn record_winnow_block() {
    WINNOW_BLOCKS.fetch_add(1, Relaxed);
}

/// Count a block covering `span` handed to the classic parser whole, and log it with `why`.
pub(super) fn record_classic_block(why: impl Display, span: Span, working_set: &StateWorkingSet) {
    CLASSIC_BLOCKS.fetch_add(1, Relaxed);
    log_hand_over(format_args!("classic block ({why})"), span, working_set);
}

/// Count a run of statements parsed ahead on the second thread.
pub(super) fn record_ahead_run() {
    AHEAD_RUNS.fetch_add(1, Relaxed);
}

/// Count a run whose statement the classic parser had to take, because the live working set
/// answered a question about command names differently (`changed`), and log it.
pub(super) fn record_ahead_fallback(changed: &str) {
    AHEAD_FALLBACKS.fetch_add(1, Relaxed);
    debug!("winnow: parsed ahead with other command names: {changed}");
}

/// Count a statement with errors the winnow parser went on past, given to the lowering.
pub(super) fn record_recovered_statement() {
    RECOVERED_STATEMENTS.fetch_add(1, Relaxed);
}

/// Count a statement the lowering turned into the AST.
pub(super) fn record_lowered_statement() {
    LOWERED_STATEMENTS.fetch_add(1, Relaxed);
}

/// Count a hand-over of `bytes` of source to the classic parser: a statement, or the rest of a
/// block as one. `had_errors`: the winnow parser reported an error in its first statement.
pub(super) fn record_classic_statement(bytes: usize, had_errors: bool) {
    CLASSIC_STATEMENTS.fetch_add(1, Relaxed);
    CLASSIC_STATEMENT_BYTES.fetch_add(bytes as u64, Relaxed);
    if had_errors {
        ERROR_STATEMENTS.fetch_add(1, Relaxed);
    }
}

/// Count a statement the lowering gave back (it is then counted as a classic statement), and
/// log it.
pub(super) fn record_unlowered(span: Span, reason: &Unlowered, working_set: &StateWorkingSet) {
    record_classic_statement(span.len(), false);
    log_hand_over(reason.reason(), span, working_set);
}

/// Count a statement covering `span` that the winnow parser reported `error` in, handed to the
/// classic parser, and log it with the first error the classic parser reported for it (those
/// from `nu_errors_from` on).
pub(super) fn record_error_statement(
    span: Span,
    error: &Diagnostic,
    working_set: &StateWorkingSet,
    nu_errors_from: usize,
) {
    record_classic_statement(span.len(), true);
    if log_enabled!(Level::Debug) {
        // The variant of the first error the classic parser reported for the statement, for
        // telling its syntax errors from its semantic ones.
        let nu_error = working_set
            .parse_errors
            .get(nu_errors_from)
            .map(|error| {
                let debug = format!("{error:?}");
                debug
                    .split(['(', ' ', '{'])
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .unwrap_or_else(|| "none".to_owned());
        log_hand_over(
            format_args!("syntax error ({}) [nu: {nu_error}]", error.kind),
            span,
            working_set,
        );
    }
}

/// Log a hand-over of `span` to the classic parser: `why`, its length and its first line of
/// code.
fn log_hand_over(why: impl Display, span: Span, working_set: &StateWorkingSet) {
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
