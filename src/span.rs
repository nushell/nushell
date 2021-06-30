#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file_id: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, file_id: usize) -> Span {
        Span {
            start,
            end,
            file_id,
        }
    }
}
