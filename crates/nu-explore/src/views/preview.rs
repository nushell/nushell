use std::cmp::max;

use crossterm::event::{KeyCode, KeyEvent};
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use nu_table::TextStyle;
use tui::layout::Rect;

use crate::{
    nu_common::{NuSpan, NuText},
    pager::{report::Report, Frame, Transition, ViewInfo},
};

use super::{coloredtextw::ColoredTextW, cursorw::Cursor, Layout, View, ViewConfig};

// todo: Add wrap option
#[derive(Debug)]
pub struct Preview {
    underlaying_value: Option<Value>,
    lines: Vec<String>,
    row_c: Cursor,
    col_c: Cursor,
    area: Rect,
}

impl Preview {
    pub fn new(value: &str) -> Self {
        let lines: Vec<String> = value
            .lines()
            .map(|line| line.replace('\t', "    ")) // tui: doesn't support TAB
            .collect();
        let count_lines = lines.len();

        Self {
            lines,
            underlaying_value: None,
            col_c: Cursor::new(usize::MAX, usize::MAX),
            row_c: Cursor::new(0, count_lines),
            area: Rect::default(),
        }
    }

    pub fn set_value(&mut self, value: Value) {
        self.underlaying_value = Some(value);
    }
}

impl View for Preview {
    fn draw(&mut self, f: &mut Frame, area: Rect, _: ViewConfig<'_>, layout: &mut Layout) {
        self.row_c.reset(area.height as usize);
        self.col_c.reset(area.width as usize);
        self.area = area;

        let lines = &self.lines[self.row_c.current()..];
        for (i, line) in lines.iter().enumerate().take(area.height as usize) {
            let text = ColoredTextW::new(line, self.col_c.current());
            let s = text.what(area);

            let area = Rect::new(area.x, area.y + i as u16, area.width, 1);
            f.render_widget(text, area);

            layout.push(&s, area.x, area.y, area.width, area.height);
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
                self.col_c.prev(max(1, self.area.width as usize / 2));

                Some(Transition::Ok)
            }
            KeyCode::Right => {
                self.col_c.next(max(1, self.area.width as usize / 2));

                Some(Transition::Ok)
            }
            KeyCode::Up => {
                self.row_c.prev(1);

                if (self.row_c.page() + 1) * self.row_c.page_size() + self.row_c.relative()
                    != self.row_c.limit()
                {
                    info.status = Some(Report::default());
                }

                if self.row_c.current() == 0 {
                    info.status = Some(Report::info("TOP"));
                }

                Some(Transition::Ok)
            }
            KeyCode::Down => {
                if (self.row_c.page() + 1) * self.row_c.page_size() + self.row_c.relative()
                    == self.row_c.limit()
                {
                    return Some(Transition::Ok);
                }

                self.row_c.next(1);

                info.status = Some(Report::info(""));

                if (self.row_c.page() + 1) * self.row_c.page_size() + self.row_c.relative()
                    == self.row_c.limit()
                {
                    info.status = Some(Report::info("END"));
                }

                Some(Transition::Ok)
            }
            KeyCode::PageUp => {
                let page_size = self.area.height as usize;
                self.row_c.prev(page_size);

                if (self.row_c.page() + 1) * self.row_c.page_size() + self.row_c.relative()
                    != self.row_c.limit()
                {
                    info.status = Some(Report::default());
                }

                if self.row_c.current() == 0 {
                    info.status = Some(Report::info("TOP"));
                }

                Some(Transition::Ok)
            }
            KeyCode::PageDown => {
                if (self.row_c.page() + 1) * self.row_c.page_size() + self.row_c.relative()
                    == self.row_c.limit()
                {
                    return Some(Transition::Ok);
                }

                let page_size = self.area.height as usize;
                self.row_c.next(page_size);

                info.status = Some(Report::info(""));

                if (self.row_c.page() + 1) * self.row_c.page_size() + self.row_c.relative()
                    == self.row_c.limit()
                {
                    self.row_c.move_relative(0);
                    info.status = Some(Report::info("END"));
                }

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

        self.row_c.move_to(row);
        true
    }

    fn exit(&mut self) -> Option<Value> {
        match &self.underlaying_value {
            Some(value) => Some(value.clone()),
            None => {
                let text = self.lines.join("\n");
                Some(Value::string(text, NuSpan::unknown()))
            }
        }
    }
}
