#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor {
    page_size: usize,
    page_shift: usize,
    page_pos: usize,
    max: usize,
}

impl Cursor {
    pub fn new(page_size: usize, max: usize) -> Self {
        Self {
            page_size,
            max,
            page_pos: 0,
            page_shift: 0,
        }
    }

    pub fn next(&mut self, step: usize) -> bool {
        let mut changed = false;
        for _ in 0..step {
            if !self._next() {
                return changed;
            }

            changed = true;
        }

        changed
    }

    pub fn prev(&mut self, step: usize) -> bool {
        let mut changed = false;
        for _ in 0..step {
            if !self._prev() {
                return changed;
            }

            changed = true;
        }

        changed
    }

    pub fn current(&self) -> usize {
        self.page_pos + self.page_shift * self.page_size
    }

    pub fn relative(&self) -> usize {
        self.page_pos
    }

    pub fn limit(&self) -> usize {
        self.max
    }

    pub fn page(&self) -> usize {
        self.page_shift
    }

    pub fn page_size(&self) -> usize {
        self.page_size
    }

    #[allow(dead_code)]
    pub fn count_pages(&self) -> usize {
        // todo: cover this case by tests
        if self.page_size > self.max {
            return 1;
        }

        if self.max % self.page_size == 0 {
            self.max / self.page_size
        } else {
            (self.max / self.page_size) + 1
        }
    }

    pub fn move_to(&mut self, i: usize) -> bool {
        use std::cmp::Ordering::*;

        if i > self.max {
            return false;
        }

        let current = self.current();
        match current.cmp(&i) {
            Less => {
                let i = i - current;
                self.next(i);
            }
            Greater => {
                let i = current - i;
                self.prev(i);
            }
            Equal => {}
        }

        true
    }

    pub fn move_relative(&mut self, i: usize) -> bool {
        if i > self.page_size {
            return false;
        }

        self.page_pos = i;

        true
    }

    #[allow(dead_code)]
    pub fn move_page(&mut self, i: usize) -> bool {
        if i > self.count_pages() {
            return false;
        }

        self.page_shift = i;

        true
    }

    pub fn reset(&mut self, page: usize) {
        let old_page = self.page_size;
        self.page_size = page;

        let is_page_decreased = old_page > self.page_size;
        let is_cursor_need_update = self.page_pos >= self.page_size;
        if is_page_decreased && is_cursor_need_update {
            // in such case we need to move the cursor position back
            // on a different bettween the pages cause we lost that space
            let i = old_page - self.page_size;
            if self.page_pos > i {
                self.page_pos -= i;
            }
        }
    }

    fn _next(&mut self) -> bool {
        let current = self.current();
        let next = current + 1;

        if next >= self.max {
            return false;
        }

        if self.page_pos + 1 == self.page_size {
            self.page_shift += 1;
            self.page_pos = 0;
        } else {
            self.page_pos += 1;
        }

        true
    }

    fn _prev(&mut self) -> bool {
        if self.page_pos == 0 {
            if self.page_shift == 0 {
                return false;
            }

            self.page_shift -= 1;
            self.page_pos = self.page_size - 1;
        } else {
            self.page_pos -= 1;
        }

        true
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor2 {
    view: Cursor,
    index: usize,
}

impl Cursor2 {
    pub fn new(hight: usize, max: usize) -> Self {
        Cursor2 {
            view: Cursor::new(hight, max),
            index: 0,
        }
    }

    pub fn current(&self) -> usize {
        self.view.relative() + self.index
    }

    pub fn relative(&self) -> usize {
        self.view.relative()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn next(&mut self) {
        if self.index + self.view.relative() + 1 == self.view.limit() {
            return;
        }

        if self.view.relative() + 1 == self.view.page_size() {
            self.index += 1;
        } else {
            self.view.move_relative(self.view.relative() + 1);
        }
    }

    pub fn prev(&mut self) {
        if self.view.relative() == 0 {
            if self.index > 0 {
                self.index -= 1;
            }
        } else {
            self.view.move_relative(self.view.relative() - 1);
        }
    }
}
