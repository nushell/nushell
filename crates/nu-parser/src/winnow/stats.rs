//! Counters of what the winnow front end parsed and what it handed to the classic parser, to
//! tell how much of an input took which path. They are always kept, for the whole process, and
//! read with `nu_parser::winnow_stats()`: the `frontends` harness
//! (`crates/nu-winnow-parser/tools/nushell-harness`) prints them after `compare` and `bench`.
//!
//! With `NU_WINNOW_LOG` set to any value (`NU_WINNOW_LOG=1`), read once per process, each
//! statement the lowering gives back to the classic parser is written to standard error with
//! the reason, to find what the lowering still lacks, and so is each run parsed ahead whose
//! answers about command names changed.

use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering::Relaxed},
};

use nu_protocol::{Span, engine::StateWorkingSet};

use super::lower::Unlowered;

// One counter per field of `WinnowStats`.
static WINNOW_BLOCKS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_BLOCKS: AtomicU64 = AtomicU64::new(0);
static LOWERED_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_STATEMENT_BYTES: AtomicU64 = AtomicU64::new(0);
static ERROR_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static AHEAD_RUNS: AtomicU64 = AtomicU64::new(0);
static AHEAD_FALLBACKS: AtomicU64 = AtomicU64::new(0);

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
    /// lowering gave back, or the rest of a block, counted once.
    pub classic_statements: u64,
    /// The bytes of those hand-overs.
    pub classic_statement_bytes: u64,
    /// Of those, the rests of blocks from a statement the winnow parser reported an error in.
    pub error_statements: u64,
    /// Runs of statements a second thread parsed ahead of the lowering.
    pub ahead_runs: u64,
    /// Of those, the runs in which the live working set answered a question about command names
    /// (or about their longest length) differently: the classic parser took the rest of the
    /// block.
    pub ahead_fallbacks: u64,
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
    }
}

/// Count a block whose statements the winnow parser parsed.
pub(super) fn record_winnow_block() {
    WINNOW_BLOCKS.fetch_add(1, Relaxed);
}

/// Count a block handed to the classic parser whole.
pub(super) fn record_classic_block() {
    CLASSIC_BLOCKS.fetch_add(1, Relaxed);
}

/// Count a run of statements parsed ahead on the second thread.
pub(super) fn record_ahead_run() {
    AHEAD_RUNS.fetch_add(1, Relaxed);
}

/// Count a run whose statement the classic parser had to take, because the live working set
/// answered a question about command names differently (`changed`), and log it with
/// `NU_WINNOW_LOG`.
pub(super) fn record_ahead_fallback(changed: &str) {
    AHEAD_FALLBACKS.fetch_add(1, Relaxed);
    if log() {
        eprintln!("winnow: parsed ahead with other command names: {changed}");
    }
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
/// log it with `NU_WINNOW_LOG`.
pub(super) fn record_unlowered(span: Span, reason: &Unlowered, working_set: &StateWorkingSet) {
    record_classic_statement(span.len(), false);
    if log() {
        let text = String::from_utf8_lossy(working_set.get_span_contents(span));
        let first_line = text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .unwrap_or_default();
        eprintln!(
            "winnow: {}: {} bytes: {first_line}",
            reason.reason(),
            span.len()
        );
    }
}

/// Whether `NU_WINNOW_LOG` is set.
fn log() -> bool {
    static LOG: OnceLock<bool> = OnceLock::new();
    *LOG.get_or_init(|| std::env::var_os("NU_WINNOW_LOG").is_some())
}
