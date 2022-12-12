use super::WindowCursor;

#[derive(Debug, Default, Clone, Copy)]
pub struct XYCursor {
    x: WindowCursor,
    y: WindowCursor,
}

impl XYCursor {
    pub fn new(count_rows: usize, count_columns: usize) -> Self {
        Self {
            x: WindowCursor::new(count_columns, count_columns).expect("..."),
            y: WindowCursor::new(count_rows, count_rows).expect("..."),
        }
    }

    pub fn set_window(&mut self, count_rows: usize, count_columns: usize) {
        self.x.set_window(count_columns);
        self.y.set_window(count_rows);
    }

    pub fn set_position(&mut self, row: usize, col: usize) {
        self.x.set_window_at(col);
        self.y.set_window_at(row);
    }

    pub fn row(&self) -> usize {
        self.y.index()
    }

    pub fn column(&self) -> usize {
        self.x.index()
    }

    #[allow(dead_code)]
    pub fn row_offset(&self) -> usize {
        self.y.offset()
    }

    #[allow(dead_code)]
    pub fn column_limit(&self) -> usize {
        self.x.end()
    }

    pub fn row_limit(&self) -> usize {
        self.y.end()
    }

    #[allow(dead_code)]
    pub fn column_offset(&self) -> usize {
        self.x.offset()
    }

    pub fn row_starts_at(&self) -> usize {
        self.y.starts_at()
    }

    pub fn column_starts_at(&self) -> usize {
        self.x.starts_at()
    }

    pub fn row_window(&self) -> usize {
        self.y.offset()
    }

    pub fn column_window(&self) -> usize {
        self.x.offset()
    }

    pub fn row_window_size(&self) -> usize {
        self.y.window()
    }

    pub fn column_window_size(&self) -> usize {
        self.x.window()
    }

    pub fn next_row(&mut self) -> bool {
        self.y.next(1)
    }

    #[allow(dead_code)]
    pub fn next_row_by(&mut self, i: usize) -> bool {
        self.y.next(i)
    }

    pub fn next_row_page(&mut self) -> bool {
        self.y.next_window()
    }

    pub fn prev_row(&mut self) -> bool {
        self.y.prev(1)
    }

    #[allow(dead_code)]
    pub fn prev_row_by(&mut self, i: usize) -> bool {
        self.y.prev(i)
    }

    pub fn prev_row_page(&mut self) -> bool {
        self.y.prev_window()
    }

    pub fn next_column(&mut self) -> bool {
        self.x.next(1)
    }

    pub fn next_column_by(&mut self, i: usize) -> bool {
        self.x.next(i)
    }

    pub fn prev_column(&mut self) -> bool {
        self.x.prev(1)
    }

    pub fn prev_column_by(&mut self, i: usize) -> bool {
        self.x.prev(i)
    }

    pub fn next_column_i(&mut self) -> bool {
        self.x.set_window_at(self.x.starts_at() + 1)
    }

    pub fn prev_column_i(&mut self) -> bool {
        if self.x.starts_at() == 0 {
            return false;
        }

        self.x.set_window_at(self.x.starts_at() - 1)
    }

    pub fn next_row_i(&mut self) -> bool {
        self.y.set_window_at(self.y.starts_at() + 1)
    }

    pub fn prev_row_i(&mut self) -> bool {
        if self.y.starts_at() == 0 {
            return false;
        }

        self.y.set_window_at(self.y.starts_at() - 1)
    }
}
