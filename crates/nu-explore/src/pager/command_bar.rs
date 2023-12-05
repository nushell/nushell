use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Widget},
};

use crate::{
    nu_common::NuStyle,
    views::util::{nu_style_to_tui, set_span},
};

#[derive(Debug)]
pub struct CommandBar<'a> {
    text: &'a str,
    information: &'a str,
    text_s: Style,
    back_s: Style,
}

impl<'a> CommandBar<'a> {
    pub fn new(text: &'a str, information: &'a str, text_s: NuStyle, back_s: NuStyle) -> Self {
        let text_s = nu_style_to_tui(text_s).add_modifier(Modifier::BOLD);
        let back_s = nu_style_to_tui(back_s);

        Self {
            text,
            information,
            text_s,
            back_s,
        }
    }
}

impl Widget for CommandBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const INFO_WIDTH: u16 = 12;
        const INFO_PADDING: u16 = 12;

        // colorize the line
        let block = Block::default().style(self.back_s);
        block.render(area, buf);

        let text_width = set_span(buf, (area.x, area.y), self.text, self.text_s, area.width);
        let available_width = area.width.saturating_sub(text_width);

        if available_width <= INFO_WIDTH + INFO_PADDING {
            return;
        }

        let x = area.right().saturating_sub(INFO_WIDTH + INFO_PADDING);
        set_span(buf, (x, area.y), self.information, self.text_s, INFO_WIDTH);
    }
}
