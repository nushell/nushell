//! Maps nu `Span`s (global byte offsets across all files the engine has
//! seen) to (file path, 1-based line, 1-based column).
//!
//! `EngineState` keeps every parsed source in `files()` as `CachedFile`
//! entries, each with a `covered_span` giving its offset range in the
//! global span space. We index line starts per file once, lazily.

use nu_protocol::Span;
use nu_protocol::engine::EngineState;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct SourcePos {
    pub(crate) path: String,
    pub(crate) line: u64,   // 1-based
    pub(crate) column: u64, // 1-based
}

pub(crate) struct FileIndex {
    pub(crate) span_start: usize,
    pub(crate) span_end: usize,
    /// Byte offsets (relative to file start) where each line begins.
    pub(crate) line_starts: Vec<usize>,
}

#[derive(Default)]
pub(crate) struct SourceMap {
    pub(crate) files: HashMap<String, FileIndex>,
}

impl SourceMap {
    /// Refresh from the engine state. Cheap to call repeatedly; only
    /// indexes files it hasn't seen. Call after every merge_delta.
    pub(crate) fn refresh(&mut self, engine_state: &EngineState) {
        for cached in engine_state.files() {
            // Canonicalize so `source helper.nu` compares equal to the client's
            // absolute breakpoint paths (relative names resolve against the
            // launch cwd; non-file names fail and are kept as-is).
            let name = crate::paths::canonical_str(&cached.name);
            if self.files.contains_key(&name) {
                continue;
            }
            let content: &[u8] = &cached.content;
            let mut line_starts = vec![0usize];
            for (i, b) in content.iter().enumerate() {
                if *b == b'\n' {
                    line_starts.push(i + 1);
                }
            }
            self.files.insert(
                name,
                FileIndex {
                    span_start: cached.covered_span.start,
                    span_end: cached.covered_span.end,
                    line_starts,
                },
            );
        }
    }

    pub(crate) fn resolve(&self, span: Span) -> Option<SourcePos> {
        for (path, fi) in &self.files {
            if span.start < fi.span_start || span.start >= fi.span_end {
                continue;
            }
            let rel = span.start - fi.span_start;
            let idx = Self::line_index(&fi.line_starts, rel);
            let col = rel - fi.line_starts[idx] + 1;
            return Some(SourcePos {
                path: path.clone(),
                line: (idx + 1) as u64,
                column: col as u64,
            });
        }
        None
    }

    /// Like `resolve`, but only for valid *stop locations*: non-empty spans
    /// confined to a single source line. Structural glue (drain/load-empty/
    /// return) carries the whole block's span, which would resolve to line 1
    /// and make stepping jump around — such spans return None here.
    pub(crate) fn resolve_steppable(&self, span: Span) -> Option<SourcePos> {
        if span.end <= span.start {
            return None; // empty / synthetic (Span::unknown is 0..0)
        }
        for (path, fi) in &self.files {
            if span.start < fi.span_start || span.start >= fi.span_end {
                continue;
            }
            let rel_start = span.start - fi.span_start;
            // Last byte of the span, clamped to this file.
            let rel_end = (span.end - 1).min(fi.span_end - 1) - fi.span_start;
            let start_idx = Self::line_index(&fi.line_starts, rel_start);
            let end_idx = Self::line_index(&fi.line_starts, rel_end);
            if start_idx != end_idx {
                return None; // spans multiple lines: structural, not steppable
            }
            let col = rel_start - fi.line_starts[start_idx] + 1;
            return Some(SourcePos {
                path: path.clone(),
                line: (start_idx + 1) as u64,
                column: col as u64,
            });
        }
        None
    }

    /// Index of the last line start <= rel.
    fn line_index(line_starts: &[usize], rel: usize) -> usize {
        match line_starts.binary_search(&rel) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }
}

// Unit tests moved to `src/tests/source_map.rs`.
