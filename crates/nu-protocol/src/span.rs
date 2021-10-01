use miette::SourceSpan;
use serde::{Deserialize, Serialize};

pub struct Spanned<T> {
    pub item: T,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for SourceSpan {
    fn from(s: Span) -> Self {
        Self::new(s.start.into(), (s.end - s.start).into())
    }
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    pub fn unknown() -> Span {
        Span { start: 0, end: 0 }
    }

    pub fn offset(&self, offset: usize) -> Span {
        Span {
            start: self.start - offset,
            end: self.end - offset,
        }
    }
}

pub fn span(spans: &[Span]) -> Span {
    let length = spans.len();

    if length == 0 {
        Span::unknown()
    } else if length == 1 {
        spans[0]
    } else {
        Span {
            start: spans[0].start,
            end: spans[length - 1].end,
        }
    }
}
