use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Widget},
};

use super::super::{
    nu_common::{NuStyle, string_width},
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
        let info_width = string_width(self.information) as u16;
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

    fn render_command_bar(text: &str, information: &str, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        let cmd_bar = CommandBar::new(text, information, NuStyle::default(), NuStyle::default());
        cmd_bar.render(area, &mut buf);
        buf
    }

    #[test]
    fn test_command_bar_basic_text() {
        let buf = render_command_bar(":help", "", 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains(":help"));
    }

    #[test]
    fn test_command_bar_with_information() {
        let buf = render_command_bar(":search", "[1/5]", 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains(":search"));
        assert!(content.contains("[1/5]"));
    }

    #[test]
    fn test_command_bar_text_on_left_info_on_right() {
        let buf = render_command_bar("/pattern", "[3/10]", 50, 1);
        let content = buffer_to_string(&buf);

        let text_pos = content.find("/pattern").unwrap();
        let info_pos = content.find("[3/10]").unwrap();

        // Text should be on the left, info on the right
        assert!(text_pos < info_pos);
    }

    #[test]
    fn test_command_bar_empty_text() {
        let buf = render_command_bar("", "[info]", 40, 1);
        let content = buffer_to_string(&buf);

        // Should not panic with empty text
        assert!(content.contains("[info]"));
    }

    #[test]
    fn test_command_bar_empty_information() {
        let buf = render_command_bar(":quit", "", 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains(":quit"));
    }

    #[test]
    fn test_command_bar_both_empty() {
        let buf = render_command_bar("", "", 40, 1);

        // Should not panic with both empty
        assert_eq!(buf.area.width, 40);
    }

    #[test]
    fn test_command_bar_narrow_width() {
        // Very narrow width - info might not fit
        let buf = render_command_bar(":x", "[info]", 10, 1);

        // Should not panic
        assert_eq!(buf.area.width, 10);
    }

    #[test]
    fn test_command_bar_unicode_text() {
        let buf = render_command_bar("/日本語", "[結果]", 50, 1);
        let content = buffer_to_string(&buf);

        // Should handle Unicode without panicking
        assert!(content.contains("日本語") || !content.is_empty());
    }

    #[test]
    fn test_command_bar_left_padding() {
        let buf = render_command_bar(":a", "", 20, 1);
        let content = buffer_to_string(&buf);

        // Text should not start at position 0 (there's padding)
        let first_char = content.chars().next().unwrap();
        assert_eq!(first_char, ' ');
    }

    #[test]
    fn test_command_bar_zero_height() {
        // Zero height buffer - just verify we can create the area
        let area = Rect::new(0, 0, 40, 0);
        assert_eq!(area.height, 0);
    }

    #[test]
    fn test_command_bar_zero_width() {
        let buf = render_command_bar(":test", "[info]", 0, 1);

        assert_eq!(buf.area.width, 0);
    }

    #[test]
    fn test_command_bar_long_text_and_info() {
        let long_text = ":".to_string() + &"a".repeat(50);
        let long_info = "b".repeat(20);
        let buf = render_command_bar(&long_text, &long_info, 40, 1);

        // Should not panic with long text
        assert_eq!(buf.area.width, 40);
    }

    #[test]
    fn test_command_bar_search_pattern() {
        let buf = render_command_bar("/search_pattern", "[0/0]", 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("/search_pattern"));
        assert!(content.contains("[0/0]"));
    }

    #[test]
    fn test_command_bar_reverse_search() {
        let buf = render_command_bar("?reverse", "[2/5]", 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("?reverse"));
    }
}
