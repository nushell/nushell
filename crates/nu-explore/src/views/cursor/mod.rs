mod window_cursor;
mod window_cursor_2d;

use anyhow::{bail, Result};
pub use window_cursor::WindowCursor;
pub use window_cursor_2d::{CursorMoveHandler, Position, WindowCursor2D};

/// A 1-dimensional cursor to track a position from 0 to N
///
/// Say we have a cursor with size=9, at position 3:
/// 0  1  2  3  4  5  6  7  8  9
/// |  |  |  C  |  |  |  |  |  |
///
/// After moving forward by 2 steps:
/// 0  1  2  3  4  5  6  7  8  9
/// |  |  |  |  |  C  |  |  |  |
///
/// After moving backward by 6 steps (clamped to 0):
/// 0  1  2  3  4  5  6  7  8  9
/// C  |  |  |  |  |  |  |  |  |
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cursor {
    /// The current position of the cursor
    position: usize,
    /// The number of distinct positions the cursor can be at
    size: usize,
}

impl Cursor {
    /// Constructor to create a new Cursor
    pub fn new(size: usize) -> Self {
        // In theory we should not be able to create a cursor with size 0, but in practice
        // it's easier to allow that for empty lists etc. instead of propagating errors
        Cursor { position: 0, size }
    }

    /// The max position the cursor can be at
    pub fn end(&self) -> usize {
        self.size - 1
    }

    /// Set the position to a specific value within the bounds [0, end]
    pub fn set_position(&mut self, pos: usize) {
        if pos <= self.end() {
            self.position = pos;
        } else {
            // Clamp the position to end if out of bounds
            self.position = self.end();
        }
    }

    /// Set the size of the cursor. The position is clamped if it exceeds the new size
    pub fn set_size(&mut self, size: usize) -> Result<()> {
        if size == 0 {
            bail!("Size cannot be zero");
        }
        self.size = size;
        if self.position > self.end() {
            self.position = self.end();
        }
        Ok(())
    }

    /// Move the cursor forward by a specified number of steps
    pub fn move_forward(&mut self, steps: usize) {
        if self.position + steps <= self.end() {
            self.position += steps;
        } else {
            self.position = self.end();
        }
    }

    /// Move the cursor backward by a specified number of steps
    pub fn move_backward(&mut self, steps: usize) {
        if self.position >= steps {
            self.position -= steps;
        } else {
            self.position = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_set_position() {
        // from 0 to 9
        let mut cursor = Cursor::new(10);
        cursor.set_position(5);
        assert_eq!(cursor.position, 5);

        cursor.set_position(15);
        assert_eq!(cursor.position, 9);
    }

    #[test]
    fn test_cursor_move_forward() {
        // from 0 to 9
        let mut cursor = Cursor::new(10);
        assert_eq!(cursor.position, 0);
        cursor.move_forward(3);
        assert_eq!(cursor.position, 3);

        cursor.move_forward(10);
        assert_eq!(cursor.position, 9);
    }

    #[test]
    fn test_cursor_move_backward() {
        // from 0 to 9
        let mut cursor = Cursor::new(10);
        cursor.move_backward(3);
        assert_eq!(cursor.position, 0);

        cursor.move_forward(5);
        assert_eq!(cursor.position, 5);
        cursor.move_backward(3);
        assert_eq!(cursor.position, 2);
        cursor.move_backward(3);
        assert_eq!(cursor.position, 0);
    }
}
