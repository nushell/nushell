use super::Cursor;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowCursor {
    view: Cursor,
    window: Cursor,
}

impl WindowCursor {
    pub fn new(limit: usize, window: usize) -> Option<Self> {
        if window > limit {
            return None;
        }

        Some(Self {
            view: Cursor::new(limit),
            window: Cursor::new(window),
        })
    }

    pub fn index(&self) -> usize {
        self.view.index + self.window.index
    }

    pub fn offset(&self) -> usize {
        self.window.index
    }

    pub fn starts_at(&self) -> usize {
        self.view.index
    }

    pub fn cap(&self) -> usize {
        self.view.cap()
    }

    pub fn window(&self) -> usize {
        self.window.end()
    }

    pub fn end(&self) -> usize {
        self.view.end()
    }

    pub fn set_window_at(&mut self, i: usize) -> bool {
        self.view.set(i)
    }

    pub fn set_window(&mut self, i: usize) -> bool {
        if i > self.view.end() {
            return false;
        }

        self.window.limit(i)
    }

    pub fn next(&mut self, i: usize) -> bool {
        if i > self.cap() {
            return false;
        }

        let mut rest = 0;
        for y in 0..i {
            if !self.window.next(1) {
                rest = i - y;
                break;
            }
        }

        for _ in 0..rest {
            if self.index() + 1 == self.end() {
                return rest != i;
            }

            self.view.next(1);
        }

        true
    }

    pub fn next_window(&mut self) -> bool {
        let end_cursor = self.window() - self.offset();
        self.next(end_cursor);

        let mut index_move = self.window();
        if index_move + self.starts_at() >= self.end() {
            index_move = self.end() - self.starts_at();
        }

        self.next(index_move)
    }

    pub fn prev(&mut self, i: usize) -> bool {
        for _ in 0..i {
            if !self.window.prev(1) {
                self.view.prev(1);
            }
        }

        true
    }

    pub fn prev_window(&mut self) -> bool {
        self.prev(self.window() + self.offset())
    }
}
