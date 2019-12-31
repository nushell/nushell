use derive_new::new;
use language_reporting::{FileName, Location};
use log::trace;
use nu_source::Span;

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
        Some(Span::new(from_index, to_index))
    }

    fn file_id(&self, _tag: Self::Span) -> Self::FileId {
        0
    }

    fn file_name(&self, _file: Self::FileId) -> FileName {
        FileName::Verbatim("shell".to_string())
    }

    fn byte_index(&self, _file: Self::FileId, _line: usize, _column: usize) -> Option<usize> {
        unimplemented!("byte_index")
    }

    fn location(&self, _file: Self::FileId, byte_index: usize) -> Option<Location> {
        trace!("finding location for {}", byte_index);

        let source = &self.snippet;
        let mut seen_lines = 0;
        let mut seen_bytes = 0;

        for (pos, slice) in source.match_indices('\n') {
            trace!(
                "searching byte_index={} seen_bytes={} pos={} slice={:?} slice.len={} source={:?}",
                byte_index,
                seen_bytes,
                pos,
                slice,
                source.len(),
                source
            );

            if pos >= byte_index {
                trace!(
                    "returning {}:{} seen_lines={} byte_index={} pos={} seen_bytes={}",
                    seen_lines,
                    byte_index,
                    pos,
                    seen_lines,
                    byte_index,
                    seen_bytes
                );

                return Some(language_reporting::Location::new(
                    seen_lines,
                    byte_index - pos,
                ));
            } else {
                seen_lines += 1;
                seen_bytes = pos;
            }
        }

        if seen_lines == 0 {
            trace!("seen_lines=0 end={}", source.len() - 1);

            // if we got here, there were no newlines in the source
            Some(language_reporting::Location::new(0, source.len() - 1))
        } else {
            trace!(
                "last line seen_lines={} end={}",
                seen_lines,
                source.len() - 1 - byte_index
            );

            // if we got here and we didn't return, it should mean that we're talking about
            // the last line
            Some(language_reporting::Location::new(
                seen_lines,
                source.len() - 1 - byte_index,
            ))
        }
    }

    fn line_span(&self, _file: Self::FileId, lineno: usize) -> Option<Self::Span> {
        trace!("finding line_span for {}", lineno);

        let source = &self.snippet;
        let mut seen_lines = 0;
        let mut seen_bytes = 0;

        for (pos, _) in source.match_indices('\n') {
            trace!(
                "lineno={} seen_lines={} seen_bytes={} pos={}",
                lineno,
                seen_lines,
                seen_bytes,
                pos
            );

            if seen_lines == lineno {
                trace!("returning start={} end={}", seen_bytes, pos);
                // If the number of seen lines is the lineno, seen_bytes is the start of the
                // line and pos is the end of the line
                return Some(Span::new(seen_bytes, pos));
            } else {
                // If it's not, increment seen_lines, and move seen_bytes to the beginning of
                // the next line
                seen_lines += 1;
                seen_bytes = pos + 1;
            }
        }

        if seen_lines == 0 {
            trace!("returning start={} end={}", 0, self.snippet.len() - 1);

            // if we got here, there were no newlines in the source
            Some(Span::new(0, self.snippet.len() - 1))
        } else {
            trace!(
                "returning start={} end={}",
                seen_bytes,
                self.snippet.len() - 1
            );

            // if we got here and we didn't return, it should mean that we're talking about
            // the last line
            Some(Span::new(seen_bytes, self.snippet.len() - 1))
        }
    }

    fn source(&self, span: Self::Span) -> Option<String> {
        trace!("source(tag={:?}) snippet={:?}", span, self.snippet);

        if span.start() > span.end() || span.end() > self.snippet.len() {
            return None;
        }
        Some(span.slice(&self.snippet).to_string())
    }
}
