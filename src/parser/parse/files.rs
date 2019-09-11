use crate::Span;
use derive_new::new;
use language_reporting::{FileName, Location};

#[derive(new, Debug, Clone)]
pub struct Files {
    snippet: String,
}

impl language_reporting::ReportingFiles for Files {
    type Span = Span;
    type FileId = usize;

    fn byte_span(
        &self,
        _file: Self::FileId,
        from_index: usize,
        to_index: usize,
    ) -> Option<Self::Span> {
        Some(Span::from((from_index, to_index)))
    }
    fn file_id(&self, _span: Self::Span) -> Self::FileId {
        0
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

        for (pos, _) in source.match_indices('\n') {
            if pos > byte_index {
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
            None
        }
    }
    fn line_span(&self, _file: Self::FileId, lineno: usize) -> Option<Self::Span> {
        let source = &self.snippet;
        let mut seen_lines = 0;
        let mut seen_bytes = 0;

        for (pos, _) in source.match_indices('\n') {
            if seen_lines == lineno {
                return Some(Span::from((seen_bytes, pos)));
            } else {
                seen_lines += 1;
                seen_bytes = pos + 1;
            }
        }

        if seen_lines == 0 {
            Some(Span::from((0, self.snippet.len() - 1)))
        } else {
            None
        }
    }
    fn source(&self, span: Self::Span) -> Option<String> {
        if span.start > span.end {
            return None;
        } else if span.end >= self.snippet.len() {
            return None;
        }
        Some(self.snippet[span.start..span.end].to_string())
    }
}
