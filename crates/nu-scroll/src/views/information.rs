use crossterm::event::KeyEvent;
use nu_protocol::engine::{EngineState, Stack};
use nu_table::TextStyle;
use tui::{layout::Rect, widgets::Paragraph};

use crate::{
    nu_common::NuText,
    pager::{Frame, Transition, ViewConfig, ViewInfo},
};

use super::{Layout, View};

#[derive(Debug, Default)]
pub struct InformationView;

impl InformationView {
    const MESSAGE: [&'static str; 7] = [
        "Scroll",
        "",
        "Scroll helps you dynamically navigate through your data",
        "",
        "type :help<Enter> for help",
        "type :q<Enter> to exit",
        "type :try<Enter> to enter a REPL",
    ];
}

impl View for InformationView {
    fn draw(&mut self, f: &mut Frame, area: Rect, _: &ViewConfig, layout: &mut Layout) {
        let count_lines = Self::MESSAGE.len() as u16;

        if area.height < count_lines {
            return;
        }

        let centerh = area.height / 2;
        let centerw = area.width / 2;

        let mut y = centerh.saturating_sub(count_lines);
        for mut line in Self::MESSAGE {
            let mut line_width = line.len() as u16;
            if line_width > area.width {
                line_width = area.width;
                line = &line[..area.width as usize];
            }

            let x = centerw.saturating_sub(line_width / 2);
            let area = Rect::new(area.x + x, area.y + y, line_width, 1);

            let paragraph = Paragraph::new(line);
            f.render_widget(paragraph, area);

            layout.push(line, area.x, area.y, area.width, area.height);

            y += 1;
        }
    }

    fn handle_input(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: &Layout,
        _: &mut ViewInfo,
        _: KeyEvent,
    ) -> Option<Transition> {
        None
    }

    fn collect_data(&self) -> Vec<NuText> {
        Self::MESSAGE
            .into_iter()
            .map(|line| (line.to_owned(), TextStyle::default()))
            .collect::<Vec<_>>()
    }
}
