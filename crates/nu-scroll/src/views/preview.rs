use std::cmp::max;

use crossterm::event::{KeyCode, KeyEvent};
use nu_protocol::engine::{EngineState, Stack};
use nu_table::TextStyle;
use tui::layout::Rect;

use crate::{
    nu_common::NuText,
    pager::{Frame, Transition, ViewConfig, ViewInfo},
};

use super::{coloredtextw::ColoredTextW, Layout, View};

// todo: Add wrap option
#[derive(Debug)]
pub struct Preview {
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
        }
    }
}

impl View for Preview {
    fn draw(&mut self, f: &mut Frame, area: Rect, _: &ViewConfig, _: &mut Layout) {
        if self.i_row >= self.lines.len() {
            f.render_widget(tui::widgets::Clear, area);
            return;
        }

        let lines = &self.lines[self.i_row..];
        for (i, line) in lines.iter().enumerate().take(area.height as usize) {
            let area = Rect::new(area.x, area.y + i as u16, area.width, 1);
            f.render_widget(ColoredTextW::new(line, self.i_col), area)
        }

        self.screen_size = area.width;
    }

    fn handle_input(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: &Layout,
        _: &mut ViewInfo,
        key: KeyEvent,
    ) -> Option<Transition> {
        match key.code {
            KeyCode::Right => {
                self.i_col += max(1, self.screen_size as usize / 2);

                Some(Transition::Ok)
            }
            KeyCode::Left => {
                if self.i_col > 0 {
                    self.i_col -= max(1, self.screen_size as usize / 2);
                }

                Some(Transition::Ok)
            }
            KeyCode::Down => {
                self.i_row += 1;

                Some(Transition::Ok)
            }
            KeyCode::Up => {
                self.i_row = self.i_row.saturating_sub(1);

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
}
