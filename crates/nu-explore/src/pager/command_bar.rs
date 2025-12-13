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
        const INFO_PADDING_RIGHT: u16 = 2;
        const TEXT_PADDING_LEFT: u16 = 1;

        // colorize the entire line background
        let block = Block::default().style(self.back_s);
        block.render(area, buf);

        // Render the command/search text on the left with padding
        let text_x = area.x + TEXT_PADDING_LEFT;
        let info_width = self.information.len() as u16;
        let max_text_width = area
            .width
            .saturating_sub(TEXT_PADDING_LEFT + info_width + INFO_PADDING_RIGHT + 2);

        let text_width = set_span(
            buf,
            (text_x, area.y),
            self.text,
            self.text_s,
            max_text_width,
        );

        // Calculate available space for info
        let available_width = area.width.saturating_sub(text_x - area.x + text_width);

        if available_width <= INFO_PADDING_RIGHT + 2 || self.information.is_empty() {
            return;
        }

        // Render info on the right with padding
        let info_x = area.right().saturating_sub(info_width + INFO_PADDING_RIGHT);
        if info_x > text_x + text_width + 1 {
            set_span(
                buf,
                (info_x, area.y),
                self.information,
                self.text_s,
                info_width,
            );
        }
    }
}
