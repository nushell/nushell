#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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
