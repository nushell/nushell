use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Widget},
};

use crate::{
    nu_common::{string_width, NuStyle},
    views::util::{nu_style_to_tui, set_span},
};

pub struct StatusBar {
    text: (String, Style),
    ctx1: (String, Style),
    ctx2: (String, Style),
    ctx3: (String, Style),
    back_s: Style,
}

impl StatusBar {
    pub fn new(text: String, ctx1: String, ctx2: String, ctx3: String) -> Self {
        Self {
            text: (text, Style::default()),
            ctx1: (ctx1, Style::default()),
            ctx2: (ctx2, Style::default()),
            ctx3: (ctx3, Style::default()),
            back_s: Style::default(),
        }
    }

    pub fn set_message_style(&mut self, style: NuStyle) {
        self.text.1 = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_ctx1_style(&mut self, style: NuStyle) {
        self.ctx1.1 = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_ctx2_style(&mut self, style: NuStyle) {
        self.ctx2.1 = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_ctx3_style(&mut self, style: NuStyle) {
        self.ctx3.1 = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_background_style(&mut self, style: NuStyle) {
        self.back_s = nu_style_to_tui(style);
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const MAX_CTX1_WIDTH: u16 = 12;
        const MAX_CTX2_WIDTH: u16 = 12;
        const MAX_CTX3_WIDTH: u16 = 12;

        // colorize the line
        let block = Block::default().style(self.back_s);
        block.render(area, buf);

        let mut used_width = 0;

        let (text, style) = self.ctx1;
        let text_width = (string_width(&text) as u16).min(MAX_CTX1_WIDTH);
        used_width +=
            try_render_text_from_right_most(area, buf, &text, style, used_width, text_width);

        let (text, style) = self.ctx2;
        used_width +=
            try_render_text_from_right_most(area, buf, &text, style, used_width, MAX_CTX2_WIDTH);

        let (text, style) = self.ctx3;
        used_width +=
            try_render_text_from_right_most(area, buf, &text, style, used_width, MAX_CTX3_WIDTH);

        let (text, style) = self.text;
        try_render_text_from_left(area, buf, &text, style, used_width);
    }
}

fn try_render_text_from_right_most(
    area: Rect,
    buf: &mut Buffer,
    text: &str,
    style: Style,
    used_width: u16,
    span_width: u16,
) -> u16 {
    let dis = span_width + used_width;
    try_render_text_from_right(area, buf, text, style, dis, used_width, span_width)
}

fn try_render_text_from_right(
    area: Rect,
    buf: &mut Buffer,
    text: &str,
    style: Style,
    distance_from_right: u16,
    used_width: u16,
    span_width: u16,
) -> u16 {
    let has_space = !text.is_empty() && area.width > used_width;
    if !has_space {
        return 0;
    }

    let x = area.right().saturating_sub(distance_from_right);
    set_span(buf, (x, area.y), text, style, span_width);

    span_width
}

fn try_render_text_from_left(
    area: Rect,
    buf: &mut Buffer,
    text: &str,
    style: Style,
    used_width: u16,
) -> u16 {
    let has_space = !text.is_empty() && area.width > used_width;
    if !has_space {
        return 0;
    }

    let rest_width = area.width - used_width;
    set_span(buf, (area.x, area.y), text, style, rest_width);

    rest_width
}
