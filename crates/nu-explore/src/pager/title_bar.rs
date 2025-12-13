use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Widget},
};

use crate::{
    nu_common::{NuStyle, string_width},
    views::util::{nu_style_to_tui, set_span},
};

/// A title bar widget displayed at the top of the explore view
pub struct TitleBar {
    title: String,
    title_style: Style,
    info_left: String,
    info_right: String,
    info_style: Style,
    background_style: Style,
}

impl TitleBar {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            title_style: Style::default().add_modifier(Modifier::BOLD),
            info_left: String::new(),
            info_right: String::new(),
            info_style: Style::default().add_modifier(Modifier::DIM),
            background_style: Style::default(),
        }
    }

    pub fn with_info_left(mut self, info: impl Into<String>) -> Self {
        self.info_left = info.into();
        self
    }

    pub fn with_info_right(mut self, info: impl Into<String>) -> Self {
        self.info_right = info.into();
        self
    }

    pub fn set_background_style(&mut self, style: NuStyle) {
        self.background_style = nu_style_to_tui(style);
    }

    pub fn set_title_style(&mut self, style: NuStyle) {
        self.title_style = nu_style_to_tui(style).add_modifier(Modifier::BOLD);
    }

    pub fn set_info_style(&mut self, style: NuStyle) {
        self.info_style = nu_style_to_tui(style).add_modifier(Modifier::DIM);
    }
}

impl Widget for TitleBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width < 10 {
            return;
        }

        // Fill background
        let block = Block::default().style(self.background_style);
        block.render(area, buf);

        const PADDING: u16 = 1;
        const SEPARATOR_WIDTH: u16 = 3;

        let mut left_offset = PADDING;
        let mut right_offset = PADDING;

        // Render left info if present (with dimmed style for hints)
        if !self.info_left.is_empty() {
            let info_width = string_width(&self.info_left) as u16;
            let max_left_width = area.width / 3;
            if left_offset + info_width < max_left_width {
                set_span(
                    buf,
                    (area.x + left_offset, area.y),
                    &self.info_left,
                    self.info_style,
                    info_width,
                );
                left_offset += info_width + SEPARATOR_WIDTH;
            }
        }

        // Render right info if present (with dimmed style for hints)
        if !self.info_right.is_empty() {
            let info_width = string_width(&self.info_right) as u16;
            let max_right_width = area.width / 3;
            if right_offset + info_width < max_right_width {
                let x = area.right().saturating_sub(right_offset + info_width);
                set_span(
                    buf,
                    (x, area.y),
                    &self.info_right,
                    self.info_style,
                    info_width,
                );
                right_offset += info_width + SEPARATOR_WIDTH;
            }
        }

        // Calculate available space for title (centered)
        let title_width = string_width(&self.title) as u16;
        let available_center = area.width.saturating_sub(left_offset + right_offset);

        if title_width > 0 && available_center > 3 {
            // Try to center the title
            let ideal_x = area.x + (area.width.saturating_sub(title_width)) / 2;

            // Make sure title doesn't overlap with left/right info
            let min_x = area.x + left_offset;
            let max_x = area.right().saturating_sub(right_offset + title_width);

            let title_x = ideal_x.clamp(min_x, max_x.max(min_x));

            let actual_width = (area.right().saturating_sub(right_offset))
                .saturating_sub(title_x)
                .min(title_width);

            if actual_width > 0 {
                let title_to_render = if title_width > actual_width {
                    let mut truncated = self.title.clone();
                    while string_width(&truncated) as u16 > actual_width.saturating_sub(1)
                        && !truncated.is_empty()
                    {
                        truncated.pop();
                    }
                    if !truncated.is_empty() {
                        truncated.push('…');
                    }
                    truncated
                } else {
                    self.title
                };

                set_span(
                    buf,
                    (title_x, area.y),
                    &title_to_render,
                    self.title_style,
                    actual_width,
                );
            }
        }
    }
}
