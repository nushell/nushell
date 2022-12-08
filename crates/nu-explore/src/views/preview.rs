use std::cmp::{max, min};

use crossterm::event::{KeyCode, KeyEvent};
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use nu_table::TextStyle;
use tui::layout::Rect;

use crate::{
    nu_common::{NuSpan, NuText},
    pager::{Frame, Report, Severity, Transition, ViewInfo},
};

use super::{coloredtextw::ColoredTextW, Layout, View, ViewConfig};

// todo: Add wrap option
#[derive(Debug)]
pub struct Preview {
    underlaying_value: Option<Value>,
    lines: Vec<String>,
    i_row: usize,
    i_col: usize,
    screen_size: u16,
}

impl Preview {
    pub fn new(value: &str) -> Self {
        let lines: Vec<String> = value
            .lines()
            .map(|line| line.replace('\t', "    ")) // tui: doesn't support TAB
            .collect();

        Self {
            lines,
            i_col: 0,
            i_row: 0,
            screen_size: 0,
            underlaying_value: None,
        }
    }

    pub fn set_value(&mut self, value: Value) {
        self.underlaying_value = Some(value);
    }
}

impl View for Preview {
    fn draw(&mut self, f: &mut Frame, area: Rect, _: ViewConfig<'_>, layout: &mut Layout) {
        if self.i_row >= self.lines.len() {
            f.render_widget(tui::widgets::Clear, area);
            return;
        }

        let lines = &self.lines[self.i_row..];
        for (i, line) in lines.iter().enumerate().take(area.height as usize) {
            let text = ColoredTextW::new(line, self.i_col);
            let s = text.what(area);

            let area = Rect::new(area.x, area.y + i as u16, area.width, 1);
            f.render_widget(text, area);

            layout.push(&s, area.x, area.y, area.width, area.height);
        }

        self.screen_size = area.width;
    }

    fn handle_input(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        layout: &Layout,
        info: &mut ViewInfo, // add this arg to draw too?
        key: KeyEvent,
    ) -> Option<Transition> {
        match key.code {
            KeyCode::Left => {
                if self.i_col > 0 {
                    self.i_col -= max(1, self.screen_size as usize / 2);
                }

                Some(Transition::Ok)
            }
            KeyCode::Right => {
                self.i_col += max(1, self.screen_size as usize / 2);

                Some(Transition::Ok)
            }
            KeyCode::Up => {
                let is_start = self.i_row == 0;
                if is_start {
                    // noop
                    return Some(Transition::Ok);
                }

                let page_size = layout.data.len();
                let max = self.lines.len().saturating_sub(page_size);
                let was_end = self.i_row == max;

                if max != 0 && was_end {
                    info.status = Some(Report::default());
                }

                self.i_row = self.i_row.saturating_sub(1);

                Some(Transition::Ok)
            }
            KeyCode::Down => {
                let page_size = layout.data.len();
                let max = self.lines.len().saturating_sub(page_size);

                let is_end = self.i_row == max;
                if is_end {
                    // noop
                    return Some(Transition::Ok);
                }

                self.i_row = min(self.i_row + 1, max);

                let is_end = self.i_row == max;
                if is_end {
                    let report = Report::new(
                        String::from("END"),
                        Severity::Info,
                        String::new(),
                        String::new(),
                    );

                    info.status = Some(report);
                }

                Some(Transition::Ok)
            }
            KeyCode::PageUp => {
                let page_size = layout.data.len();
                let max = self.lines.len().saturating_sub(page_size);
                let was_end = self.i_row == max;

                if max != 0 && was_end {
                    info.status = Some(Report::default());
                }

                self.i_row = self.i_row.saturating_sub(page_size);

                Some(Transition::Ok)
            }
            KeyCode::PageDown => {
                let page_size = layout.data.len();
                let max = self.lines.len().saturating_sub(page_size);
                self.i_row = min(self.i_row + page_size, max);

                let is_end = self.i_row == max;
                if is_end {
                    let report = Report::new(
                        String::from("END"),
                        Severity::Info,
                        String::new(),
                        String::new(),
                    );

                    info.status = Some(report);
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

        self.i_row = row;
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
