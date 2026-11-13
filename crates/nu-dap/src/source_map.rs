//! Maps nu `Span`s to (file, 1-based line, column).
//!
//! `EngineState::files()` holds every parsed source with a `covered_span`
//! locating it in nu's global span space. Line starts are indexed per file
//! once, lazily; positions are reported against the [`FileId`] the path
//! interned to (see [`crate::file_table`]).

use crate::file_table::{FileId, FileTable};
use nu_protocol::Span;
use nu_protocol::engine::EngineState;

#[derive(Debug, Clone)]
pub(crate) struct SourcePos {
    pub file: FileId,
    pub line: u64,   // 1-based
    pub column: u64, // 1-based
}

struct FileIndex {
    file: FileId,
    span_start: usize,
    span_end: usize,
    /// Byte offsets, relative to file start, where each line begins.
    line_starts: Vec<usize>,
}

pub(crate) struct SourceMap {
    files: FileTable,
    indexed: Vec<FileIndex>,
    /// Number of `EngineState::files()` entries already indexed. That list only
    /// grows within a run, so `refresh` skips straight to the new ones.
    seen: usize,
}

impl SourceMap {
    /// `files` must be the session's table, so ids agree with the ones
    /// breakpoints were interned under.
    pub(crate) fn new(files: FileTable) -> Self {
        Self {
            files,
            indexed: Vec::new(),
            seen: 0,
        }
    }

    /// Index any files added since the last call. Runs per instruction, so it
    /// must stay cheap.
    pub(crate) fn refresh(&mut self, engine_state: &EngineState) {
        for cached in engine_state.files().skip(self.seen) {
            self.seen += 1;
            let file = self.files.intern(&*cached.name);
            let content: &[u8] = &cached.content;
            let mut line_starts = vec![0usize];
            for (i, b) in content.iter().enumerate() {
                if *b == b'\n' {
                    line_starts.push(i + 1);
                }
            }
            self.indexed.push(FileIndex {
                file,
                span_start: cached.covered_span.start,
                span_end: cached.covered_span.end,
                line_starts,
            });
        }
    }

    /// Canonical path of `id`, for display and for the DAP `Source` a client
    /// needs in order to open the file.
    pub(crate) fn path(&self, id: FileId) -> String {
        self.files.path(id)
    }

    pub(crate) fn resolve(&self, span: Span) -> Option<SourcePos> {
        let fi = self.containing(span)?;
        let rel = span.start - fi.span_start;
        let idx = Self::line_index(&fi.line_starts, rel);
        Some(SourcePos {
            file: fi.file,
            line: (idx + 1) as u64,
            column: (rel - fi.line_starts[idx] + 1) as u64,
        })
    }

    /// Like `resolve`, but only for valid *stop locations*: non-empty spans
    /// confined to one line. Structural glue (drain/load-empty/return) carries
    /// the whole block's span, which would make stepping jump around.
    pub(crate) fn resolve_steppable(&self, span: Span) -> Option<SourcePos> {
        if span.end <= span.start {
            return None; // empty / synthetic (Span::unknown is 0..0)
        }
        let fi = self.containing(span)?;
        let rel_start = span.start - fi.span_start;
        // Last byte of the span, clamped to this file.
        let rel_end = (span.end - 1).min(fi.span_end - 1) - fi.span_start;
        let start_idx = Self::line_index(&fi.line_starts, rel_start);
        if start_idx != Self::line_index(&fi.line_starts, rel_end) {
            return None; // multi-line: structural, not steppable
        }
        Some(SourcePos {
            file: fi.file,
            line: (start_idx + 1) as u64,
            column: (rel_start - fi.line_starts[start_idx] + 1) as u64,
        })
    }

    /// The indexed file whose global span range covers `span`.
    fn containing(&self, span: Span) -> Option<&FileIndex> {
        self.indexed
            .iter()
            .find(|fi| span.start >= fi.span_start && span.start < fi.span_end)
    }

    /// Index of the last line start <= `rel`.
    fn line_index(line_starts: &[usize], rel: usize) -> usize {
        match line_starts.binary_search(&rel) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }
}
