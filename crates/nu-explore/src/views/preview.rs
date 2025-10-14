use super::{
    Layout, View, ViewConfig, colored_text_widget::ColoredTextWidget, cursor::CursorMoveHandler,
    cursor::WindowCursor2D,
};
use crate::{
    nu_common::NuText,
    pager::{Frame, StatusTopOrEnd, Transition, ViewInfo, report::Report},
};
use crossterm::event::KeyEvent;
use nu_color_config::TextStyle;
use nu_protocol::{
    Value,
    engine::{EngineState, Stack},
};
use ratatui::layout::Rect;
use std::cmp::max;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug)]
pub struct Preview {
    value: Option<Value>,
    cursor: WindowCursor2D,
    wrap_enabled: bool,
    is_editing: bool, // Track if we're expecting an edit result
}

impl Preview {
    pub fn new(value: Value) -> Self {
        let cursor = WindowCursor2D::new(1, usize::MAX).expect("Failed to create cursor");
        Self {
            value: Some(value),
            cursor,
            wrap_enabled: true, // Enable wrapping by default
            is_editing: false,
        }
    }

    pub fn empty() -> Self {
        let cursor = WindowCursor2D::new(1, usize::MAX).expect("Failed to create cursor");
        Self {
            value: None,
            cursor,
            wrap_enabled: false,
            is_editing: false,
        }
    }

    pub fn toggle_wrap(&mut self) {
        self.wrap_enabled = !self.wrap_enabled;
    }

    fn edit_value_direct(
        &mut self,
        current_value: &Value,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<Value, String> {
        use crate::nu_common::edit_value_with_editor;
        use nu_protocol::Span;

        // Call the editor
        match edit_value_with_editor(current_value, engine_state, stack, Span::unknown()) {
            Ok(new_value) => Ok(new_value),
            Err(e) => Err(e.to_string()),
        }
    }

}

impl View for Preview {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        // Generate lines on-demand from value
        // We regenerate text each draw to handle:
        // 1. Wrapping state changes (toggle with 'w' key)
        // 2. Terminal width changes (resize)
        // Draw is called ~4 times per second continuously (250ms tick_rate in pager/events.rs:19),
        // but Preview handles simple values, so performance is acceptable
        let lines = if let Some(ref value) = self.value {
            let config = &cfg.nu_config;
            let text = value.to_expanded_string(", ", config);
            let processed_lines = text
                .lines()
                .map(|line| line.replace('\t', "    "))
                .collect::<Vec<_>>();

            if self.wrap_enabled && area.width > 10 {
                wrap_lines(&processed_lines, area.width as usize)
            } else {
                processed_lines
            }
        } else {
            vec![String::new()]
        };

        // Update cursor with new line count, preserving position where possible
        let current_row = self.cursor.row();
        let current_col = self.cursor.column();

        self.cursor =
            WindowCursor2D::new(lines.len().max(1), usize::MAX).expect("Failed to create cursor");
        let _ = self
            .cursor
            .set_window_size(area.height as usize, area.width as usize);

        // Try to restore position, clamping to new bounds
        if current_row < lines.len() {
            self.cursor
                .set_window_start_position(current_row, current_col);
        }

        let visible_lines = &lines[self.cursor.window_origin().row..];
        for (i, line) in visible_lines.iter().enumerate().take(area.height as usize) {
            let text_widget = ColoredTextWidget::new(line, self.cursor.column());
            let plain_text = text_widget.get_plain_text(area.width as usize);

            let area = Rect::new(area.x, area.y + i as u16, area.width, 1);
            f.render_widget(text_widget, area);

            // push the plain text to layout so it can be searched
            layout.push(&plain_text, area.x, area.y, area.width, area.height);
        }
    }

    fn handle_input(
        &mut self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _: &Layout,
        info: &mut ViewInfo, // add this arg to draw too?
        key: KeyEvent,
    ) -> Transition {
        // Handle Ctrl+O for editing the value
        if let crossterm::event::KeyEvent {
            modifiers: crossterm::event::KeyModifiers::CONTROL,
            code: crossterm::event::KeyCode::Char('o'),
            ..
        } = key
        {
            if let Some(current_value) = self.value.clone() {
                match self.edit_value_direct(&current_value, _engine_state, _stack) {
                    Ok(new_value) => {
                        self.value = Some(new_value);
                        info.status = Some(Report::info("Value edited successfully"));
                        // Signal that we need a force redraw by returning a special transition
                        return Transition::Cmd("force_redraw".to_string());
                    }
                    Err(e) => {
                        info.status = Some(Report::message(format!("Edit failed: {}", e), crate::pager::report::Severity::Err));
                    }
                }
            } else {
                info.status = Some(Report::message("No value to edit", crate::pager::report::Severity::Err));
            }
            return Transition::None;
        }

        // Handle wrap toggle
        if let crossterm::event::KeyCode::Char('w') = key.code {
            self.toggle_wrap();
            let wrap_status = if self.wrap_enabled { "ON" } else { "OFF" };
            info.status = Some(Report::info(format!("Text wrapping: {wrap_status}")));
            return Transition::Ok;
        }

        match self.handle_input_key(&key) {
            Ok((transition, status_top_or_end)) => {
                match status_top_or_end {
                    StatusTopOrEnd::Top => set_status_top(self, info),
                    StatusTopOrEnd::End => set_status_end(self, info),
                    _ => {}
                }
                transition
            }
            _ => Transition::None, // currently only handle_enter() in crates/nu-explore/src/views/record/mod.rs raises an Err()
        }
    }

    fn collect_data(&self) -> Vec<NuText> {
        if let Some(ref value) = self.value {
            let config = nu_protocol::Config::default();
            let text = value.to_expanded_string(", ", &config);
            text.lines()
                .map(|line| (line.replace('\t', "    "), TextStyle::default()))
                .collect::<Vec<_>>()
        } else {
            vec![(String::new(), TextStyle::default())]
        }
    }

    fn show_data(&mut self, row: usize) -> bool {
        // we can only go to the appropriate line, but we can't target column
        //
        // todo: improve somehow?

        self.cursor.set_window_start_position(row, 0);
        true
    }

    fn exit(&mut self) -> Option<Value> {
        self.value.clone()
    }

    fn handle_child_result(&mut self, child_exit_value: Option<Value>) -> Result<(), String> {
        // If we were editing and got a result back, update our value
        if self.is_editing {
            if let Some(new_value) = child_exit_value {
                self.value = Some(new_value);
            }
            self.is_editing = false; // Clear the edit flag
        }
        Ok(())
    }
}

impl CursorMoveHandler for Preview {
    fn get_cursor(&mut self) -> &mut WindowCursor2D {
        &mut self.cursor
    }
    fn handle_left(&mut self) {
        self.cursor
            .prev_column_by(max(1, self.cursor.window_width_in_columns() / 2));
    }
    fn handle_right(&mut self) {
        self.cursor
            .next_column_by(max(1, self.cursor.window_width_in_columns() / 2));
    }
}

fn set_status_end(view: &Preview, info: &mut ViewInfo) {
    if view.cursor.row() + 1 == view.cursor.row_limit() {
        info.status = Some(Report::info("END"));
    } else {
        info.status = Some(Report::default());
    }
}

fn set_status_top(view: &Preview, info: &mut ViewInfo) {
    if view.cursor.window_origin().row == 0 {
        info.status = Some(Report::info("TOP"));
    } else {
        info.status = Some(Report::default());
    }
}

fn wrap_lines(lines: &[String], wrap_width: usize) -> Vec<String> {
    let mut wrapped_lines = Vec::new();

    for line in lines {
        if line.width() <= wrap_width {
            wrapped_lines.push(line.clone());
        } else {
            // Split long lines into multiple wrapped lines
            let mut remaining = line.as_str();

            while !remaining.is_empty() {
                let mut split_pos = 0;
                let mut current_width = 0;
                let mut last_space = None;

                // Find the best place to break the line
                for (byte_pos, ch) in remaining.char_indices() {
                    let char_width = ch.width().unwrap_or(0);

                    if current_width + char_width > wrap_width {
                        break;
                    }

                    current_width += char_width;
                    split_pos = byte_pos + ch.len_utf8();

                    // Remember the last space for word wrapping
                    if ch.is_whitespace() {
                        last_space = Some(split_pos);
                    }
                }

                // If we found a space and we're not at the beginning, break at the space
                let break_pos = if split_pos < remaining.len()
                    && let Some(space_pos) = last_space
                    && space_pos > wrap_width / 3
                {
                    space_pos
                } else {
                    split_pos
                };

                if break_pos == 0 {
                    // Single character wider than wrap_width, just take it
                    if let Some(ch) = remaining.chars().next() {
                        wrapped_lines.push(ch.to_string());
                        remaining = &remaining[ch.len_utf8()..];
                    } else {
                        break;
                    }
                } else {
                    wrapped_lines.push(remaining[..break_pos].trim_end().to_string());
                    remaining = remaining[break_pos..].trim_start();
                }
            }
        }
    }

    wrapped_lines
}
