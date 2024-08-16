use std::cmp::min;

use super::Cursor;
use anyhow::{bail, Ok, Result};

/// WindowCursor provides a mechanism to navigate through a 1-dimensional range
/// using a smaller movable window within the view.
///
/// View: The larger context or total allowable range for navigation.
/// Window: The smaller, focused subset of the view.
///
/// Example:
/// ```plaintext
/// 1. Initial view of size 20 with a window of size 5. The absolute cursor position starts at 0.
///     View :
///     |--------------------|
///     Window :
///     |X====|
///
/// 2. After advancing the window by 3, the absolute cursor position becomes 3.
///     View :
///     |--------------------|
///     Window :
///        |X====|
///
/// 3. After advancing the cursor inside the window by 2, the absolute cursor position becomes 5.
///     View :
///     |--------------------|
///     Window :
///        |==X==|
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowCursor {
    view: Cursor,
    window: Cursor,
}

impl WindowCursor {
    pub fn new(view_size: usize, window_size: usize) -> Result<Self> {
        if window_size > view_size {
            bail!("Window size cannot be greater than view size");
        }

        Ok(Self {
            view: Cursor::new(view_size),
            window: Cursor::new(window_size),
        })
    }

    pub fn absolute_position(&self) -> usize {
        self.window_starts_at() + self.window.position
    }

    pub fn window_relative_position(&self) -> usize {
        self.window.position
    }

    pub fn window_starts_at(&self) -> usize {
        self.view.position
    }

    pub fn window_size(&self) -> usize {
        self.window.size
    }

    pub fn end(&self) -> usize {
        self.view.end()
    }

    pub fn set_window_start_position(&mut self, i: usize) {
        self.view.set_position(i)
    }

    pub fn move_window_to_end(&mut self) {
        self.view.set_position(self.end() - self.window_size() + 1);
    }

    pub fn set_window_size(&mut self, new_size: usize) -> Result<()> {
        if new_size > self.view.size {
            // TODO: should we return an error here or clamp? the Ok is copying existing behavior
            return Ok(());
        }

        self.window.set_size(new_size)?;
        Ok(())
    }

    pub fn next_n(&mut self, n: usize) {
        for _ in 0..n {
            self.next();
        }
    }

    pub fn next(&mut self) {
        if self.absolute_position() >= self.end() {
            return;
        }

        if self.window_relative_position() == self.window.end() {
            self.view.move_forward(1);
        } else {
            self.window.move_forward(1);
        }
    }

    pub fn next_window(&mut self) {
        self.move_cursor_to_end_of_window();

        // move window forward by window size, or less if that would send it off the end of the view
        let window_end = self.window_starts_at() + self.window_size() - 1;
        let distance_from_window_end_to_view_end = self.end() - window_end;
        self.view.move_forward(min(
            distance_from_window_end_to_view_end,
            self.window_size(),
        ));
    }

    pub fn prev_n(&mut self, n: usize) {
        for _ in 0..n {
            self.prev();
        }
    }

    pub fn prev(&mut self) {
        if self.window_relative_position() == 0 {
            self.view.move_backward(1);
        } else {
            self.window.move_backward(1);
        }
    }

    pub fn prev_window(&mut self) {
        self.move_cursor_to_start_of_window();

        // move the whole window back
        self.view.move_backward(self.window_size());
    }

    pub fn move_cursor_to_start_of_window(&mut self) {
        self.window.move_backward(self.window_size());
    }

    pub fn move_cursor_to_end_of_window(&mut self) {
        self.window.move_forward(self.window_size());
    }
}
