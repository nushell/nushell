use super::{Layout, View, ViewConfig};
use crate::{
    nu_common::NuText,
    pager::{Frame, Transition, ViewInfo},
};
use crossterm::event::KeyEvent;
use nu_color_config::TextStyle;
use nu_protocol::engine::{EngineState, Stack};
use ratatui::{layout::Rect, widgets::Paragraph};

#[derive(Debug, Default)]
pub struct InformationView;

impl InformationView {
    const MESSAGE: [&'static str; 7] = [
        "Explore",
        "",
        "Explore helps you dynamically navigate through your data",
        "",
        "type :help<Enter> for help",
        "type :q<Enter> to exit",
        "type :try<Enter> to enter a REPL",
    ];
}

impl View for InformationView {
    fn draw(&mut self, f: &mut Frame, area: Rect, _: ViewConfig<'_>, layout: &mut Layout) {
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
        event: KeyEvent,
    ) -> Option<Transition> {
        match event.code {
            crossterm::event::KeyCode::Esc => Some(Transition::Exit),
            _ => None,
        }
    }

    fn collect_data(&self) -> Vec<NuText> {
        Self::MESSAGE
            .into_iter()
            .map(|line| (line.to_owned(), TextStyle::default()))
            .collect::<Vec<_>>()
    }
}
