//! Unit tests for [`crate::source_map`].

use crate::source_map::{FileIndex, SourceMap};
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
