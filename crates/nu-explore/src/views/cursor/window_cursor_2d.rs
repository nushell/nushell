use super::WindowCursor;
use anyhow::Result;

/// `WindowCursor2D` manages a 2-dimensional "window" onto a grid of cells.
/// For example, consider a 3x3 grid of cells:
///
/// +---+---+---+
/// | a | b | c |
/// |---|---|---|
/// | d | e | f |
/// |---|---|---|
/// | g | h | i |
/// +---+---+---+
///
/// A `WindowCursor2D` can be used to track the currently visible section of this grid.
/// For example, a 2x2 window onto this grid could initially show the top left 2x2 section:
///
/// +---+---+
/// | a | b |
/// |---|---|
/// | d | e |
/// +---+---+
///
/// Moving the window down 1 row:
///
/// +---+---+
/// | d | e |
/// |---|---|
/// | g | h |
/// +---+---+
///
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowCursor2D {
    x: WindowCursor,
    y: WindowCursor,
}

impl WindowCursor2D {
    pub fn new(count_rows: usize, count_columns: usize) -> Result<Self> {
        Ok(Self {
            x: WindowCursor::new(count_columns, count_columns)?,
            y: WindowCursor::new(count_rows, count_rows)?,
        })
    }

    pub fn set_window_size(&mut self, count_rows: usize, count_columns: usize) -> Result<()> {
        self.x.set_window_size(count_columns)?;
        self.y.set_window_size(count_rows)?;
        Ok(())
    }

    pub fn set_start_position(&mut self, row: usize, col: usize) {
        self.x.set_window_start_position(col);
        self.y.set_window_start_position(row);
    }

    pub fn row(&self) -> usize {
        self.y.absolute_position()
    }

    pub fn column(&self) -> usize {
        self.x.absolute_position()
    }

    pub fn row_limit(&self) -> usize {
        self.y.end()
    }

    pub fn row_starts_at(&self) -> usize {
        self.y.window_starts_at()
    }

    pub fn column_starts_at(&self) -> usize {
        self.x.window_starts_at()
    }

    pub fn row_window_position(&self) -> usize {
        self.y.window_relative_position()
    }

    pub fn column_window_position(&self) -> usize {
        self.x.window_relative_position()
    }

    pub fn column_window_size(&self) -> usize {
        self.x.window_size()
    }

    pub fn next_row(&mut self) {
        self.y.next_n(1)
    }

    pub fn next_row_page(&mut self) {
        self.y.next_window()
    }

    pub fn row_move_to_end(&mut self) {
        self.y.move_window_to_end();
        self.y.move_cursor_to_end_of_window();
    }

    pub fn row_move_to_start(&mut self) {
        self.y.move_cursor_to_start_of_window();
        self.y.set_window_start_position(0);
    }

    pub fn prev_row(&mut self) {
        self.y.prev()
    }

    pub fn prev_row_page(&mut self) {
        self.y.prev_window()
    }

    pub fn next_column(&mut self) {
        self.x.next()
    }

    pub fn next_column_by(&mut self, i: usize) {
        self.x.next_n(i)
    }

    pub fn prev_column(&mut self) {
        self.x.prev()
    }

    pub fn prev_column_by(&mut self, i: usize) {
        self.x.prev_n(i)
    }

    pub fn next_column_i(&mut self) {
        self.x
            .set_window_start_position(self.x.window_starts_at() + 1)
    }

    pub fn prev_column_i(&mut self) {
        if self.x.window_starts_at() == 0 {
            return;
        }

        self.x
            .set_window_start_position(self.x.window_starts_at() - 1)
    }

    pub fn next_row_i(&mut self) {
        self.y
            .set_window_start_position(self.y.window_starts_at() + 1)
    }

    pub fn prev_row_i(&mut self) {
        if self.y.window_starts_at() == 0 {
            return;
        }

        self.y
            .set_window_start_position(self.y.window_starts_at() - 1)
    }
}
