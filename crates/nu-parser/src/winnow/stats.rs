//! Counters of what the winnow front end parsed and what it handed to the classic parser, for
//! benchmarks and tests that need to know how much of their input took which path.
//!
//! With `NU_WINNOW_LOG` set, each statement the lowering gives back to the classic parser is
//! written to standard error with the reason, to find what the lowering still lacks.

use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering::Relaxed},
};

use nu_protocol::{Span, engine::StateWorkingSet};

use super::lower::Unlowered;

static WINNOW_BLOCKS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_BLOCKS: AtomicU64 = AtomicU64::new(0);
static LOWERED_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_STATEMENTS: AtomicU64 = AtomicU64::new(0);
static CLASSIC_STATEMENT_BYTES: AtomicU64 = AtomicU64::new(0);
static ERROR_STATEMENTS: AtomicU64 = AtomicU64::new(0);

/// What the winnow front end has done in this process so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WinnowStats {
    /// Blocks and module bodies whose statements the winnow parser parsed.
    pub winnow_blocks: u64,
    /// Blocks handed to the classic parser whole (not UTF-8, or not lexable by winnow).
    pub classic_blocks: u64,
    /// Statements the winnow parser parsed and the lowering turned into the AST.
    pub lowered_statements: u64,
    /// Statements the classic parser parsed from their span.
    pub classic_statements: u64,
    /// The bytes of those statements.
    pub classic_statement_bytes: u64,
    /// Of those, the statements the winnow parser reported an error in.
    pub error_statements: u64,
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
    }
}

pub(super) fn record_winnow_block() {
    WINNOW_BLOCKS.fetch_add(1, Relaxed);
}

pub(super) fn record_classic_block() {
    CLASSIC_BLOCKS.fetch_add(1, Relaxed);
}

pub(super) fn record_lowered_statement() {
    LOWERED_STATEMENTS.fetch_add(1, Relaxed);
}

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
    static LOG: OnceLock<bool> = OnceLock::new();
    if *LOG.get_or_init(|| std::env::var_os("NU_WINNOW_LOG").is_some()) {
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
