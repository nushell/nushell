use crate::Tag;
use derive_new::new;
use language_reporting::{FileName, Location};
use log::trace;
use uuid::Uuid;

#[derive(new, Debug, Clone)]
pub struct Files {
    snippet: String,
}

impl language_reporting::ReportingFiles for Files {
    type Span = Tag;
    type FileId = Uuid;

    fn byte_span(
        &self,
        file: Self::FileId,
        from_index: usize,
        to_index: usize,
    ) -> Option<Self::Span> {
        Some(Tag::new(file, (from_index, to_index).into()))
    }

    fn file_id(&self, tag: Self::Span) -> Self::FileId {
        tag.anchor
    }

    fn file_name(&self, _file: Self::FileId) -> FileName {
        FileName::Verbatim(format!("shell"))
    }

    fn byte_index(&self, _file: Self::FileId, _line: usize, _column: usize) -> Option<usize> {
        unimplemented!("byte_index")
    }

    fn location(&self, _file: Self::FileId, byte_index: usize) -> Option<Location> {
        let source = &self.snippet;
        let mut seen_lines = 0;
        let mut seen_bytes = 0;

        for (pos, slice) in source.match_indices('\n') {
            trace!(
                "SEARCH={} SEEN={} POS={} SLICE={:?} LEN={} ALL={:?}",
                byte_index,
                seen_bytes,
                pos,
                slice,
                source.len(),
                source
            );

            if pos >= byte_index {
                return Some(language_reporting::Location::new(
                    seen_lines,
                    byte_index - seen_bytes,
                ));
            } else {
                seen_lines += 1;
                seen_bytes = pos;
            }
        }

        if seen_lines == 0 {
            Some(language_reporting::Location::new(0, byte_index))
        } else {
            panic!("byte index {} wasn't valid", byte_index);
        }
    }

    fn line_span(&self, file: Self::FileId, lineno: usize) -> Option<Self::Span> {
        let source = &self.snippet;
        let mut seen_lines = 0;
        let mut seen_bytes = 0;

        for (pos, _) in source.match_indices('\n') {
            if seen_lines == lineno {
                return Some(Tag::new(file, (seen_bytes, pos + 1).into()));
            } else {
                seen_lines += 1;
                seen_bytes = pos + 1;
            }
        }

        if seen_lines == 0 {
            Some(Tag::new(file, (0, self.snippet.len() - 1).into()))
        } else {
            None
        }
    }

    fn source(&self, tag: Self::Span) -> Option<String> {
        trace!("source(tag={:?}) snippet={:?}", tag, self.snippet);

        if tag.span.start() > tag.span.end() {
            return None;
        } else if tag.span.end() > self.snippet.len() {
            return None;
        }
        Some(tag.slice(&self.snippet).to_string())
    }
}
