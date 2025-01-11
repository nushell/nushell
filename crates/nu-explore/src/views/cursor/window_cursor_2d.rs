use crate::pager::{StatusTopOrEnd, Transition};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::WindowCursor;
use anyhow::Result;

/// `WindowCursor2D` manages a 2-dimensional "window" onto a grid of cells, with a cursor that can point to a specific cell.
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
/// Inside the window, the cursor can point to a specific cell.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowCursor2D {
    x: WindowCursor,
    y: WindowCursor,
}

pub struct Position {
    pub row: usize,
    pub column: usize,
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

    pub fn set_window_start_position(&mut self, row: usize, col: usize) {
        self.x.set_window_start_position(col);
        self.y.set_window_start_position(row);
    }

    /// The absolute position of the cursor in the grid (0-indexed, row only)
    pub fn row(&self) -> usize {
        self.y.absolute_position()
    }

    /// The absolute position of the cursor in the grid (0-indexed, column only)
    pub fn column(&self) -> usize {
        self.x.absolute_position()
    }

    /// The absolute position of the cursor in the grid (0-indexed)
    pub fn position(&self) -> Position {
        Position {
            row: self.row(),
            column: self.column(),
        }
    }

    pub fn row_limit(&self) -> usize {
        self.y.end()
    }

    pub fn window_origin(&self) -> Position {
        Position {
            row: self.y.window_starts_at(),
            column: self.x.window_starts_at(),
        }
    }

    pub fn window_relative_position(&self) -> Position {
        Position {
            row: self.y.window_relative_position(),
            column: self.x.window_relative_position(),
        }
    }

    pub fn window_width_in_columns(&self) -> usize {
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

pub trait CursorMoveHandler {
    fn get_cursor(&mut self) -> &mut WindowCursor2D;

    // standard handle_EVENT handlers that can be overwritten
    fn handle_enter(&mut self) -> Result<Transition> {
        Ok(Transition::None)
    }
    fn handle_esc(&mut self) -> Transition {
        Transition::Exit
    }
    fn handle_expand(&mut self) -> Transition {
        Transition::None
    }
    fn handle_left(&mut self) {
        self.get_cursor().prev_column_i()
    }
    fn handle_right(&mut self) {
        self.get_cursor().next_column_i()
    }
    fn handle_up(&mut self) {
        self.get_cursor().prev_row_i()
    }
    fn handle_down(&mut self) {
        self.get_cursor().next_row_i()
    }
    fn handle_transpose(&mut self) -> Transition {
        Transition::None
    }

    // top-level event handler should not be overwritten
    fn handle_input_key(&mut self, key: &KeyEvent) -> Result<(Transition, StatusTopOrEnd)> {
        let key_combo_status = match key {
            // PageUp supports Vi (Ctrl+b) and Emacs (Alt+v) keybindings
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('b'),
                ..
            }
            | KeyEvent {
                modifiers: KeyModifiers::ALT,
                code: KeyCode::Char('v'),
                ..
            }
            | KeyEvent {
                code: KeyCode::PageUp,
                ..
            } => {
                self.get_cursor().prev_row_page();
                StatusTopOrEnd::Top
            }
            // PageDown supports Vi (Ctrl+f) and Emacs (Ctrl+v) keybindings
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('f'),
                ..
            }
            | KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('v'),
                ..
            }
            | KeyEvent {
                code: KeyCode::PageDown,
                ..
            } => {
                self.get_cursor().next_row_page();
                self.get_cursor().prev_row();
                StatusTopOrEnd::End
            }
            // Up support Emacs (Ctrl+p) keybinding
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('p'),
                ..
            } => {
                self.handle_up();
                StatusTopOrEnd::Top
            }
            // Down support Emacs (Ctrl+n) keybinding
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('n'),
                ..
            } => {
                self.handle_down();
                StatusTopOrEnd::End
            }
            _ => StatusTopOrEnd::None,
        };
        match key_combo_status {
            StatusTopOrEnd::Top | StatusTopOrEnd::End => {
                return Ok((Transition::Ok, key_combo_status));
            }
            _ => {} // not page up or page down, so don't return; continue to next match block
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Ok((self.handle_esc(), StatusTopOrEnd::None)),
            KeyCode::Char('i') | KeyCode::Enter => Ok((self.handle_enter()?, StatusTopOrEnd::None)),
            KeyCode::Char('t') => Ok((self.handle_transpose(), StatusTopOrEnd::None)),
            KeyCode::Char('e') => Ok((self.handle_expand(), StatusTopOrEnd::None)),
            KeyCode::Up | KeyCode::Char('k') => {
                self.handle_up();
                Ok((Transition::Ok, StatusTopOrEnd::Top))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.handle_down();
                Ok((Transition::Ok, StatusTopOrEnd::End))
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.handle_left();
                Ok((Transition::Ok, StatusTopOrEnd::None))
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.handle_right();
                Ok((Transition::Ok, StatusTopOrEnd::None))
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.get_cursor().row_move_to_start();
                Ok((Transition::Ok, StatusTopOrEnd::Top))
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.get_cursor().row_move_to_end();
                Ok((Transition::Ok, StatusTopOrEnd::End))
            }
            _ => Ok((Transition::None, StatusTopOrEnd::None)),
        }
    }
}
