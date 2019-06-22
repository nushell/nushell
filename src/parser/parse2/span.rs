use derive_new::new;
use getset::Getters;

#[derive(new, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Getters)]
#[get = "crate"]
pub struct Spanned<T> {
    crate span: Span,
    crate item: T,
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.item
    }
}

impl<T> Spanned<T> {
    crate fn from_item(item: T, span: impl Into<Span>) -> Spanned<T> {
        Spanned {
            span: span.into(),
            item,
        }
    }

    crate fn map<U>(self, input: impl FnOnce(T) -> U) -> Spanned<U> {
        let Spanned { span, item } = self;

        let mapped = input(item);
        Spanned { span, item: mapped }
    }

    crate fn copy_span<U>(&self, output: U) -> Spanned<U> {
        let Spanned { span, .. } = self;

        Spanned {
            span: *span,
            item: output,
        }
    }

    pub fn source(&self, source: &'source str) -> &'source str {
        self.span().slice(source)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Span {
    crate start: usize,
    crate end: usize,
    // source: &'source str,
}

impl From<&Span> for Span {
    fn from(input: &Span) -> Span {
        *input
    }
}

impl From<nom_locate::LocatedSpan<&str>> for Span {
    fn from(input: nom_locate::LocatedSpan<&str>) -> Span {
        Span {
            start: input.offset,
            end: input.offset + input.fragment.len(),
        }
    }
}

impl<T> From<(nom_locate::LocatedSpan<T>, nom_locate::LocatedSpan<T>)> for Span {
    fn from(input: (nom_locate::LocatedSpan<T>, nom_locate::LocatedSpan<T>)) -> Span {
        Span {
            start: input.0.offset,
            end: input.1.offset,
        }
    }
}

impl From<(usize, usize)> for Span {
    fn from(input: (usize, usize)) -> Span {
        Span {
            start: input.0,
            end: input.1,
        }
    }
}

impl From<&std::ops::Range<usize>> for Span {
    fn from(input: &std::ops::Range<usize>) -> Span {
        Span {
            start: input.start,
            end: input.end,
        }
    }
}

impl Span {
    pub fn slice(&self, source: &'source str) -> &'source str {
        &source[self.start..self.end]
    }
}

impl language_reporting::ReportingSpan for Span {
    fn with_start(&self, start: usize) -> Self {
        Span {
            start,
            end: self.end,
        }
    }

    fn with_end(&self, end: usize) -> Self {
        Span {
            start: self.start,
            end,
        }
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}
