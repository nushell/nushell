use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Widget},
};

use crate::{
    nu_common::NuStyle,
    views::util::{nu_style_to_tui, set_span},
};

pub struct StatusBar {
    text: (String, Style),
    ctx1: (String, Style),
    ctx2: (String, Style),
    back_s: Style,
}

impl StatusBar {
    pub fn new(text: String, ctx: String, ctx2: String) -> Self {
        Self {
            text: (text, Style::default()),
            ctx1: (ctx, Style::default()),
            ctx2: (ctx2, Style::default()),
            back_s: Style::default(),
        }
    }

    pub fn set_message_style(&mut self, style: NuStyle) {
        self.text.1 = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_ctx_style(&mut self, style: NuStyle) {
        self.ctx1.1 = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_ctx2_style(&mut self, style: NuStyle) {
        self.ctx2.1 = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_background_style(&mut self, style: NuStyle) {
        self.back_s = nu_style_to_tui(style);
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const MAX_CONTEXT_WIDTH: u16 = 12;
        const MAX_CONTEXT2_WIDTH: u16 = 12;

        // colorize the line
        let block = Block::default().style(self.back_s);
        block.render(area, buf);

        let mut used_width = 0;

        let (text, style) = &self.ctx1;
        if !text.is_empty() && area.width > MAX_CONTEXT_WIDTH {
            let x = area.right().saturating_sub(MAX_CONTEXT_WIDTH);
            set_span(buf, (x, area.y), text, *style, MAX_CONTEXT_WIDTH);

            used_width += MAX_CONTEXT_WIDTH;
        }

        let (text, style) = &self.ctx2;
        if !text.is_empty() && area.width > MAX_CONTEXT2_WIDTH + used_width {
            let x = area.right().saturating_sub(MAX_CONTEXT2_WIDTH + used_width);
            set_span(buf, (x, area.y), text, *style, MAX_CONTEXT2_WIDTH);

            used_width += MAX_CONTEXT2_WIDTH;
        }

        let (text, style) = &self.text;
        if !text.is_empty() && area.width > used_width {
            let rest_width = area.width - used_width;
            set_span(buf, (area.x, area.y), text, *style, rest_width);
        }
    }
}
