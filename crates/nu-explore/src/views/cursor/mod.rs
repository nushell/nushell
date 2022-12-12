mod windowcursor;
mod xycursor;

pub use windowcursor::WindowCursor;
pub use xycursor::XYCursor;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor {
    index: usize,
    limit: usize,
}

impl Cursor {
    pub fn new(limit: usize) -> Self {
        Self { index: 0, limit }
    }

    #[allow(dead_code)]
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn cap(&self) -> usize {
        self.limit - self.index
    }

    pub fn set(&mut self, i: usize) -> bool {
        if i >= self.limit {
            return false;
        }

        self.index = i;
        true
    }

    pub fn limit(&mut self, i: usize) -> bool {
        if self.index > self.limit {
            self.index = self.limit.saturating_sub(1);
            return false;
        }

        self.limit = i;
        if self.index >= self.limit {
            self.index = self.limit.saturating_sub(1);
        }

        true
    }

    pub fn end(&self) -> usize {
        self.limit
    }

    pub fn next(&mut self, i: usize) -> bool {
        if self.index + i == self.limit {
            return false;
        }

        self.index += i;
        true
    }

    pub fn prev(&mut self, i: usize) -> bool {
        if self.index < i {
            return false;
        }

        self.index -= i;
        true
    }
}
