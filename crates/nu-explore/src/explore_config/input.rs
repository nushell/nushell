//! Keyboard input handling for the explore config TUI.

use crate::explore_config::types::{App, AppResult, EditorMode, Focus, ValueType};
use crossterm::event::{KeyCode, KeyModifiers};

/// Handle keyboard input when the tree pane is focused
pub fn handle_tree_input(app: &mut App, key: KeyCode, _modifiers: KeyModifiers) -> AppResult {
    match key {
        KeyCode::Char('q') => {
            // In config mode, allow quit if user has confirmed save with Ctrl+S
            // In non-config mode, allow quit if there are no unsaved changes
            if app.modified && !app.confirmed_save {
                app.status_message =
                    String::from("Unsaved changes! Press Ctrl+C to force quit or Ctrl+S to save");
                return AppResult::Continue;
            } else {
                return AppResult::Quit;
            }
        }
        KeyCode::Up => {
            app.tree_state.key_up();
            app.force_update_editor();
        }
        KeyCode::Down => {
            app.tree_state.key_down();
            app.force_update_editor();
        }
        KeyCode::Left => {
            app.tree_state.key_left();
            app.force_update_editor();
        }
        KeyCode::Right => {
            app.tree_state.key_right();
            app.force_update_editor();
        }
        KeyCode::Char(' ') => {
            app.tree_state.toggle_selected();
        }
        KeyCode::Enter => {
            // Check if the selected node is a leaf (non-nested) value
            let is_leaf = app
                .get_current_node_info()
                .map(|info| !matches!(info.value_type, ValueType::Object | ValueType::Array))
                .unwrap_or(false);

            if is_leaf {
                // Switch to editor pane and enter edit mode
                app.focus = Focus::Editor;
                app.editor_mode = EditorMode::Editing;
                app.editor_cursor = 0;
                app.status_message = String::from("Editing - Ctrl+Enter to apply, Esc to cancel");
            } else {
                // Toggle tree expansion for nested values
                app.tree_state.toggle_selected();
            }
        }
        KeyCode::Home => {
            app.tree_state.select_first();
            app.force_update_editor();
        }
        KeyCode::End => {
            app.tree_state.select_last();
            app.force_update_editor();
        }
        KeyCode::Tab => {
            app.focus = Focus::Editor;
            app.status_message = String::from("Editor focused - press Enter or 'e' to edit value");
        }
        _ => {}
    }
    AppResult::Continue
}

/// Handle keyboard input when the editor pane is focused in normal mode
pub fn handle_editor_normal_input(
    app: &mut App,
    key: KeyCode,
    _modifiers: KeyModifiers,
) -> AppResult {
    match key {
        KeyCode::Tab => {
            app.focus = Focus::Tree;
            app.status_message = String::from(
                "↑↓ Navigate | ←→ Collapse/Expand | Tab Switch pane | Ctrl+S Save | q Quit",
            );
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            app.editor_mode = EditorMode::Editing;
            app.editor_cursor = 0;
            app.status_message = String::from("Editing - Ctrl+Enter to apply, Esc to cancel");
        }
        KeyCode::Up => {
            app.scroll_editor(-1);
        }
        KeyCode::Down => {
            app.scroll_editor(1);
        }
        KeyCode::PageUp => {
            app.scroll_editor(-10);
        }
        KeyCode::PageDown => {
            app.scroll_editor(10);
        }
        KeyCode::Char('q') => {
            // In config mode, allow quit if user has confirmed save with Ctrl+S
            // In non-config mode, allow quit if there are no unsaved changes
            if app.modified && !app.confirmed_save {
                app.status_message =
                    String::from("Unsaved changes! Press Ctrl+C to force quit or Ctrl+S to save");
            } else {
                return AppResult::Quit;
            }
        }
        _ => {}
    }
    AppResult::Continue
}

/// Handle keyboard input when the editor pane is focused in editing mode
pub fn handle_editor_editing_input(
    app: &mut App,
    key: KeyCode,
    modifiers: KeyModifiers,
) -> AppResult {
    match key {
        KeyCode::Esc => {
            app.editor_mode = EditorMode::Normal;
            app.force_update_editor(); // Restore original value
            app.status_message = String::from("Edit cancelled");
        }
        KeyCode::Enter if modifiers.contains(KeyModifiers::CONTROL) => {
            app.apply_edit();
            app.editor_mode = EditorMode::Normal;
        }
        KeyCode::Enter => {
            // Insert newline
            app.editor_content.insert(app.editor_cursor, '\n');
            app.editor_cursor += 1;
        }
        KeyCode::Backspace => {
            if app.editor_cursor > 0 {
                app.editor_cursor -= 1;
                app.editor_content.remove(app.editor_cursor);
            }
        }
        KeyCode::Delete => {
            if app.editor_cursor < app.editor_content.len() {
                app.editor_content.remove(app.editor_cursor);
            }
        }
        KeyCode::Left => {
            app.editor_cursor = app.editor_cursor.saturating_sub(1);
        }
        KeyCode::Right => {
            app.editor_cursor = (app.editor_cursor + 1).min(app.editor_content.len());
        }
        KeyCode::Up if modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_editor(-1);
        }
        KeyCode::Down if modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_editor(1);
        }
        KeyCode::Up => {
            // Move cursor up one line
            let lines: Vec<&str> = app.editor_content.lines().collect();
            let mut pos = 0;
            let mut cursor_line = 0;
            let mut cursor_col = 0;

            for (line_idx, line) in app.editor_content.lines().enumerate() {
                if pos + line.len() >= app.editor_cursor {
                    cursor_line = line_idx;
                    cursor_col = app.editor_cursor - pos;
                    break;
                }
                pos += line.len() + 1;
                cursor_line = line_idx + 1;
            }

            if cursor_line > 0 {
                let prev_line = lines.get(cursor_line - 1).unwrap_or(&"");
                let new_col = cursor_col.min(prev_line.len());
                let mut new_pos = 0;
                for (i, line) in lines.iter().enumerate() {
                    if i == cursor_line - 1 {
                        app.editor_cursor = new_pos + new_col;
                        break;
                    }
                    new_pos += line.len() + 1;
                }
            }
        }
        KeyCode::Down => {
            // Move cursor down one line
            let lines: Vec<&str> = app.editor_content.lines().collect();
            let mut pos = 0;
            let mut cursor_line = 0;
            let mut cursor_col = 0;

            for (line_idx, line) in app.editor_content.lines().enumerate() {
                if pos + line.len() >= app.editor_cursor {
                    cursor_line = line_idx;
                    cursor_col = app.editor_cursor - pos;
                    break;
                }
                pos += line.len() + 1;
                cursor_line = line_idx + 1;
            }

            if cursor_line < lines.len().saturating_sub(1) {
                let next_line = lines.get(cursor_line + 1).unwrap_or(&"");
                let new_col = cursor_col.min(next_line.len());
                let mut new_pos = 0;
                for (i, line) in lines.iter().enumerate() {
                    if i == cursor_line + 1 {
                        app.editor_cursor = new_pos + new_col;
                        break;
                    }
                    new_pos += line.len() + 1;
                }
            }
        }
        KeyCode::Home => {
            // Move to beginning of line
            let mut pos = 0;
            for line in app.editor_content.lines() {
                if pos + line.len() >= app.editor_cursor {
                    app.editor_cursor = pos;
                    break;
                }
                pos += line.len() + 1;
            }
        }
        KeyCode::End => {
            // Move to end of line
            let mut pos = 0;
            for line in app.editor_content.lines() {
                if pos + line.len() >= app.editor_cursor {
                    app.editor_cursor = pos + line.len();
                    break;
                }
                pos += line.len() + 1;
            }
        }
        KeyCode::Char(c) => {
            app.editor_content.insert(app.editor_cursor, c);
            app.editor_cursor += 1;
        }
        KeyCode::Tab => {
            // Insert 2 spaces for indentation
            app.editor_content.insert_str(app.editor_cursor, "  ");
            app.editor_cursor += 2;
        }
        _ => {}
    }
    AppResult::Continue
}
