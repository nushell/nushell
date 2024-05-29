use super::{
    colored_text_widget::ColoredTextWidget, cursor::WindowCursor2D, Layout, View, ViewConfig,
};
use crate::{
    nu_common::{NuSpan, NuText},
    pager::{report::Report, Frame, Transition, ViewInfo},
};
use crossterm::event::{KeyCode, KeyEvent};
use nu_color_config::TextStyle;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use ratatui::layout::Rect;
use std::cmp::max;

// todo: Add wrap option
#[derive(Debug)]
pub struct Preview {
    underlying_value: Option<Value>,
    lines: Vec<String>,
    cursor: WindowCursor2D,
}

impl Preview {
    pub fn new(value: &str) -> Self {
        let lines: Vec<String> = value
            .lines()
            .map(|line| line.replace('\t', "    ")) // tui: doesn't support TAB
            .collect();

        // TODO: refactor so this is fallible and returns a Result instead of panicking
        let cursor = WindowCursor2D::new(lines.len(), usize::MAX).expect("Failed to create cursor");
        Self {
            lines,
            cursor,
            underlying_value: None,
        }
    }
}

impl View for Preview {
    fn draw(&mut self, f: &mut Frame, area: Rect, _: ViewConfig<'_>, layout: &mut Layout) {
        let _ = self
            .cursor
            .set_window_size(area.height as usize, area.width as usize);

        let lines = &self.lines[self.cursor.window_origin().row..];
        for (i, line) in lines.iter().enumerate().take(area.height as usize) {
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
        _: &EngineState,
        _: &mut Stack,
        _: &Layout,
        info: &mut ViewInfo, // add this arg to draw too?
        key: KeyEvent,
    ) -> Option<Transition> {
        match key.code {
            KeyCode::Left => {
                self.cursor
                    .prev_column_by(max(1, self.cursor.window_width_in_columns() / 2));

                Some(Transition::Ok)
            }
            KeyCode::Right => {
                self.cursor
                    .next_column_by(max(1, self.cursor.window_width_in_columns() / 2));

                Some(Transition::Ok)
            }
            KeyCode::Up => {
                self.cursor.prev_row_i();
                set_status_top(self, info);

                Some(Transition::Ok)
            }
            KeyCode::Down => {
                self.cursor.next_row_i();
                set_status_end(self, info);

                Some(Transition::Ok)
            }
            KeyCode::PageUp => {
                self.cursor.prev_row_page();
                set_status_top(self, info);

                Some(Transition::Ok)
            }
            KeyCode::PageDown => {
                self.cursor.next_row_page();
                set_status_end(self, info);

                Some(Transition::Ok)
            }
            KeyCode::Home => {
                self.cursor.row_move_to_start();
                set_status_top(self, info);

                Some(Transition::Ok)
            }
            KeyCode::End => {
                self.cursor.row_move_to_end();
                set_status_end(self, info);

                Some(Transition::Ok)
            }
            KeyCode::Esc => Some(Transition::Exit),
            _ => None,
        }
    }

    fn collect_data(&self) -> Vec<NuText> {
        self.lines
            .iter()
            .map(|line| (line.to_owned(), TextStyle::default()))
            .collect::<Vec<_>>()
    }

    fn show_data(&mut self, row: usize) -> bool {
        // we can only go to the appropriate line, but we can't target column
        //
        // todo: improve somehow?

        self.cursor.set_window_start_position(row, 0);
        true
    }

    fn exit(&mut self) -> Option<Value> {
        match &self.underlying_value {
            Some(value) => Some(value.clone()),
            None => {
                let text = self.lines.join("\n");
                Some(Value::string(text, NuSpan::unknown()))
            }
        }
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
