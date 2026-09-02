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
    span_start: usize,
    span_end: usize,
    /// Byte offsets (relative to file start) where each line begins.
    line_starts: Vec<usize>,
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
            let name = crate::paths::canonical(&*cached.name);
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

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::source_map`].

    use super::{FileIndex, SourceMap};
    use nu_protocol::Span;
    use pretty_assertions::assert_eq;

    // A file whose global span is [100, 130); lines begin at relative offsets
    // 0 ("aaaa\n"), 5 ("bb\n"), 8 ("cccccc"). i.e. line 1 = 100..105,
    // line 2 = 105..108, line 3 = 108..130.
    fn map() -> SourceMap {
        let mut files = std::collections::HashMap::new();
        files.insert(
            "test.nu".to_string(),
            FileIndex {
                span_start: 100,
                span_end: 130,
                line_starts: vec![0, 5, 8],
            },
        );
        SourceMap { files }
    }

    #[test]
    fn resolve_maps_offset_to_line_and_column() {
        let m = map();
        let p = m.resolve(Span::new(100, 104)).expect("in file");
        assert_eq!((p.line, p.column), (1, 1));
        let p = m.resolve(Span::new(106, 107)).expect("in file");
        assert_eq!((p.line, p.column), (2, 2)); // 2nd line, 2nd column
        let p = m.resolve(Span::new(108, 110)).expect("in file");
        assert_eq!(p.line, 3);
        // Outside any file's span → None.
        assert!(m.resolve(Span::new(5, 6)).is_none());
    }

    #[test]
    fn resolve_steppable_rejects_multi_line_and_empty_spans() {
        let m = map();
        // Single line (within line 3): steppable.
        assert!(m.resolve_steppable(Span::new(108, 112)).is_some());
        // Spans two lines (line 1 into line 2): not a stop location.
        assert!(m.resolve_steppable(Span::new(100, 107)).is_none());
        // Empty / synthetic span: not steppable.
        assert!(m.resolve_steppable(Span::new(100, 100)).is_none());
    }
}
