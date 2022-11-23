use crossterm::event::KeyEvent;
use nu_protocol::engine::{EngineState, Stack};
use tui::{
    layout::Rect,
    text::{Span, Spans},
    widgets::Paragraph,
};

use crate::pager::{Frame, Transition, ViewConfig, ViewInfo};

use super::{Layout, View};

#[derive(Debug, Default)]
pub struct InformationView;

impl View for InformationView {
    fn draw(&mut self, f: &mut Frame, area: Rect, _: &ViewConfig, _: &mut Layout) {
        let message = [
            "Scroll",
            "",
            "Scroll helps you dynamically navigate through your data",
            "",
            "type :help<Enter> for help",
            "type :q<Enter> to exit",
            "type :try<Enter> to enter a REPL",
        ];
        let count_lines = message.len() as u16;

        if area.height < count_lines {
            return;
        }

        let spans = message
            .into_iter()
            .map(|line| Spans::from(vec![Span::raw(line)]))
            .collect::<Vec<_>>();

        let paragraph = Paragraph::new(spans).alignment(tui::layout::Alignment::Center);

        let y = (area.height / 2).saturating_sub(count_lines);
        let area = Rect::new(area.x, y, area.width, count_lines);

        f.render_widget(paragraph, area);
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
}
