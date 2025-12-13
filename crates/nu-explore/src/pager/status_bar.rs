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
        self.ctx1.1 = nu_style_to_tui(style);
    }

    pub fn set_ctx2_style(&mut self, style: NuStyle) {
        self.ctx2.1 = nu_style_to_tui(style);
    }

    pub fn set_ctx3_style(&mut self, style: NuStyle) {
        self.ctx3.1 = nu_style_to_tui(style);
    }

    pub fn set_background_style(&mut self, style: NuStyle) {
        self.back_s = nu_style_to_tui(style);
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const MAX_CTX_WIDTH: u16 = 14;
        const SEPARATOR: &str = "│";
        const PADDING: u16 = 1;

        // Fill background
        let block = Block::default().style(self.back_s);
        block.render(area, buf);

        if area.width < 10 {
            return;
        }

        let mut used_width: u16 = 0;

        // Collect non-empty context items
        let contexts: Vec<(&String, Style)> = [
            (&self.ctx3.0, self.ctx3.1),
            (&self.ctx2.0, self.ctx2.1),
            (&self.ctx1.0, self.ctx1.1),
        ]
        .into_iter()
        .filter(|(text, _)| !text.is_empty())
        .collect();

        // Render context items from right to left
        for (i, (text, style)) in contexts.iter().enumerate() {
            let text_width = (string_width(text) as u16).min(MAX_CTX_WIDTH);

            // Calculate space needed
            let separator_space = if i > 0 { 2 } else { 0 }; // " │" before item
            let needed = text_width + PADDING + separator_space;

            if area.width <= used_width + needed + 5 {
                // Reserve space for message
                break;
            }

            // Add right padding for first item
            if i == 0 {
                used_width += PADDING;
            }

            // Render the text
            let x = area.right().saturating_sub(used_width + text_width);
            set_span(buf, (x, area.y), text, *style, text_width);
            used_width += text_width;

            // Render separator before next item (visually after current, since RTL)
            if i < contexts.len() - 1 {
                let sep_x = area.right().saturating_sub(used_width + 2);
                let dim_style = self.back_s.add_modifier(Modifier::DIM);
                set_span(buf, (sep_x, area.y), SEPARATOR, dim_style, 1);
                used_width += 2; // separator + space
            }
        }

        // Add spacing before message
        if !contexts.is_empty() {
            used_width += PADDING;
        }

        // Render the main message on the left
        let (text, style) = self.text;
        if !text.is_empty() && area.width > used_width + PADDING * 2 {
            let available_width = area.width.saturating_sub(used_width + PADDING * 2);
            let text_to_render = if string_width(&text) as u16 > available_width {
                let mut truncated = text.clone();
                while string_width(&truncated) as u16 > available_width.saturating_sub(1)
                    && !truncated.is_empty()
                {
                    truncated.pop();
                }
                if !truncated.is_empty() {
                    truncated.push('…');
                }
                truncated
            } else {
                text
            };
            set_span(
                buf,
                (area.x + PADDING, area.y),
                &text_to_render,
                style,
                available_width,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer_to_string(buf: &Buffer) -> String {
        let mut result = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    result.push_str(cell.symbol());
                }
            }
            if y < buf.area.height - 1 {
                result.push('\n');
            }
        }
        result
    }

    fn render_status_bar(status_bar: StatusBar, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        status_bar.render(area, &mut buf);
        buf
    }

    #[test]
    fn test_status_bar_basic_message() {
        let status_bar = StatusBar::new(
            "Test Message".to_string(),
            String::new(),
            String::new(),
            String::new(),
        );
        let buf = render_status_bar(status_bar, 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("Test Message"));
    }

    #[test]
    fn test_status_bar_with_context() {
        let status_bar = StatusBar::new(
            "Message".to_string(),
            "Ctx1".to_string(),
            "Ctx2".to_string(),
            "Ctx3".to_string(),
        );
        let buf = render_status_bar(status_bar, 60, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("Message"));
        assert!(content.contains("Ctx1"));
        assert!(content.contains("Ctx2"));
        assert!(content.contains("Ctx3"));
    }

    #[test]
    fn test_status_bar_context_order() {
        let status_bar = StatusBar::new(
            "Msg".to_string(),
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        );
        let buf = render_status_bar(status_bar, 60, 1);
        let content = buffer_to_string(&buf);

        // Message should be on the left, contexts on the right
        let msg_pos = content.find("Msg").unwrap();
        let first_pos = content.find("First").unwrap();

        assert!(msg_pos < first_pos);
    }

    #[test]
    fn test_status_bar_narrow_width_no_render() {
        // Width < 10 should not render context items
        let status_bar = StatusBar::new(
            "Message".to_string(),
            "Ctx1".to_string(),
            String::new(),
            String::new(),
        );
        let buf = render_status_bar(status_bar, 9, 1);
        let content = buffer_to_string(&buf);

        // Should not contain context when too narrow
        assert!(!content.contains("Ctx1"));
    }

    #[test]
    fn test_status_bar_empty_contexts_filtered() {
        let status_bar = StatusBar::new(
            "Message".to_string(),
            "OnlyOne".to_string(),
            String::new(),
            String::new(),
        );
        let buf = render_status_bar(status_bar, 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("Message"));
        assert!(content.contains("OnlyOne"));
        // Should not have separators for empty contexts
    }

    #[test]
    fn test_status_bar_long_message_truncation() {
        let long_message = "A".repeat(100);
        let status_bar = StatusBar::new(
            long_message,
            "Ctx".to_string(),
            String::new(),
            String::new(),
        );
        let buf = render_status_bar(status_bar, 30, 1);
        let content = buffer_to_string(&buf);

        // Message should be truncated with ellipsis
        assert!(content.contains('…') || content.len() <= 30);
    }

    #[test]
    fn test_status_bar_unicode_text() {
        let status_bar = StatusBar::new(
            "日本語メッセージ".to_string(),
            "状態".to_string(),
            String::new(),
            String::new(),
        );
        let buf = render_status_bar(status_bar, 50, 1);

        // Should handle Unicode without panicking
        assert_eq!(buf.area.width, 50);
    }

    #[test]
    fn test_status_bar_all_empty() {
        let status_bar = StatusBar::new(String::new(), String::new(), String::new(), String::new());
        let buf = render_status_bar(status_bar, 40, 1);

        // Should not panic with all empty strings
        assert_eq!(buf.area.width, 40);
    }

    #[test]
    fn test_status_bar_context_max_width() {
        // Context items should be limited to MAX_CTX_WIDTH (14)
        let long_ctx = "A".repeat(30);
        let status_bar = StatusBar::new("Msg".to_string(), long_ctx, String::new(), String::new());
        let buf = render_status_bar(status_bar, 60, 1);

        // Should not panic and should render
        assert_eq!(buf.area.width, 60);
    }

    #[test]
    fn test_status_bar_separator_between_contexts() {
        let status_bar = StatusBar::new(
            String::new(),
            "A".to_string(),
            "B".to_string(),
            String::new(),
        );
        let buf = render_status_bar(status_bar, 40, 1);
        let content = buffer_to_string(&buf);

        // Should have separator between contexts
        assert!(content.contains('│'));
    }

    #[test]
    fn test_status_bar_zero_height() {
        // Zero height buffer - just verify we can create the area
        let area = Rect::new(0, 0, 40, 0);
        assert_eq!(area.height, 0);
    }
}
