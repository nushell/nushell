//! Keyboard input handling for the explore config TUI.

use crate::explore_config::types::{
    App, AppResult, EditorMode, Focus, ValueType, calculate_cursor_position,
};
use crossterm::event::{KeyCode, KeyModifiers};

/// Handle keyboard input when search mode is active
pub fn handle_search_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> AppResult {
    match key {
        KeyCode::Esc => {
            // Cancel search and restore full tree
            app.clear_search();
            app.focus = Focus::Tree;
            app.status_message = get_tree_status_message(app);
        }
        KeyCode::Enter => {
            // Confirm search and return to tree navigation
            app.search_active = !app.search_query.is_empty();
            app.focus = Focus::Tree;
            app.status_message = get_tree_status_message(app);
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_search_filter();
        }
        KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_query.push(c);
            app.apply_search_filter();
        }
        _ => {}
    }
    AppResult::Continue
}

/// Handle keyboard input when the tree pane is focused
pub fn handle_tree_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> AppResult {
    match key {
        // Enter search mode with / or Ctrl+F
        KeyCode::Char('/') => {
            app.focus = Focus::Search;
            // Don't clear existing search query - allow refining
            app.status_message =
                String::from("Search: Type to filter tree | Enter to confirm | Esc to cancel");
            return AppResult::Continue;
        }
        KeyCode::Char('f') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.focus = Focus::Search;
            app.status_message =
                String::from("Search: Type to filter tree | Enter to confirm | Esc to cancel");
            return AppResult::Continue;
        }
        // Clear search filter with Escape when search is active
        KeyCode::Esc if app.search_active => {
            app.clear_search();
            app.status_message = get_tree_status_message(app);
            return AppResult::Continue;
        }
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
                app.status_message = String::from("Editing - Ctrl+S to apply, Esc to cancel");
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
        KeyCode::PageUp => {
            app.tree_state
                .select_relative(|current| current.map_or(0, |current| current.saturating_sub(10)));
            app.force_update_editor();
        }
        KeyCode::PageDown => {
            app.tree_state
                .select_relative(|current| current.map_or(0, |current| current.saturating_add(10)));
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

/// Get the default status message for tree focus
pub fn get_tree_status_message(app: &App) -> String {
    let save_action = if app.config_mode { "Apply" } else { "Save" };
    if app.search_active {
        format!(
            "Filter: \"{}\" | Esc to clear | / to modify",
            app.search_query
        )
    } else {
        format!(
            "↑↓ Navigate | / Search | ←→ Collapse/Expand | Tab Switch pane | Ctrl+S {} | q Quit",
            save_action
        )
    }
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
            app.status_message = get_tree_status_message(app);
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            app.editor_mode = EditorMode::Editing;
            app.editor_cursor = 0;
            app.status_message = String::from("Editing - Ctrl+S to apply, Esc to cancel");
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
        // Alt+Enter to apply edit
        KeyCode::Enter if modifiers.contains(KeyModifiers::ALT) => {
            app.apply_edit();
            app.editor_mode = EditorMode::Normal;
        }
        // Ctrl+S to apply edit (most reliable across platforms)
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
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
            let cursor_pos = calculate_cursor_position(&app.editor_content, app.editor_cursor);

            if cursor_pos.line > 0 {
                let prev_line = lines.get(cursor_pos.line - 1).unwrap_or(&"");
                let new_col = cursor_pos.col.min(prev_line.len());
                let mut new_pos = 0;
                for (i, line) in lines.iter().enumerate() {
                    if i == cursor_pos.line - 1 {
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
            let cursor_pos = calculate_cursor_position(&app.editor_content, app.editor_cursor);

            if cursor_pos.line < lines.len().saturating_sub(1) {
                let next_line = lines.get(cursor_pos.line + 1).unwrap_or(&"");
                let new_col = cursor_pos.col.min(next_line.len());
                let mut new_pos = 0;
                for (i, line) in lines.iter().enumerate() {
                    if i == cursor_pos.line + 1 {
                        app.editor_cursor = new_pos + new_col;
                        break;
                    }
                    new_pos += line.len() + 1;
                }
            }
        }
        KeyCode::Home => {
            // Move to beginning of line
            // Handle edge cases: empty content or cursor at position 0
            if app.editor_content.is_empty() || app.editor_cursor == 0 {
                app.editor_cursor = 0;
            } else {
                let cursor_pos = calculate_cursor_position(&app.editor_content, app.editor_cursor);
                // Calculate the start position of the current line
                let mut line_start = 0;
                for (idx, line) in app.editor_content.lines().enumerate() {
                    if idx == cursor_pos.line {
                        app.editor_cursor = line_start;
                        break;
                    }
                    line_start += line.len() + 1;
                }
            }
        }
        KeyCode::End => {
            // Move to end of line
            // Handle edge case: empty content
            if app.editor_content.is_empty() {
                app.editor_cursor = 0;
            } else {
                let cursor_pos = calculate_cursor_position(&app.editor_content, app.editor_cursor);
                let lines: Vec<&str> = app.editor_content.lines().collect();
                // Calculate the start position of the current line, then add line length
                let mut line_start = 0;
                for (idx, line) in lines.iter().enumerate() {
                    if idx == cursor_pos.line {
                        app.editor_cursor = line_start + line.len();
                        break;
                    }
                    line_start += line.len() + 1;
                }
                // Handle edge case: cursor is past the last line (e.g., after trailing newline)
                // In this case, cursor_pos.line might be beyond the lines vector
                if cursor_pos.line >= lines.len() {
                    app.editor_cursor = app.editor_content.len();
                }
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
