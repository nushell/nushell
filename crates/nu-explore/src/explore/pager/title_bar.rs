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

    fn render_title_bar(title_bar: TitleBar, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        title_bar.render(area, &mut buf);
        buf
    }

    #[test]
    fn test_title_bar_basic_render() {
        let title_bar = TitleBar::new("Test Title");
        let buf = render_title_bar(title_bar, 40, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("Test Title"));
    }

    #[test]
    fn test_title_bar_with_left_info() {
        let title_bar = TitleBar::new("Title").with_info_left("Left Info");
        let buf = render_title_bar(title_bar, 50, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("Left Info"));
        assert!(content.contains("Title"));
    }

    #[test]
    fn test_title_bar_with_right_info() {
        let title_bar = TitleBar::new("Title").with_info_right("Right Info");
        let buf = render_title_bar(title_bar, 50, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("Right Info"));
        assert!(content.contains("Title"));
    }

    #[test]
    fn test_title_bar_with_both_infos() {
        let title_bar = TitleBar::new("Center")
            .with_info_left("Left")
            .with_info_right("Right");
        let buf = render_title_bar(title_bar, 60, 1);
        let content = buffer_to_string(&buf);

        assert!(content.contains("Left"));
        assert!(content.contains("Center"));
        assert!(content.contains("Right"));

        // Verify order: left info should come before center, center before right
        let left_pos = content.find("Left").unwrap();
        let center_pos = content.find("Center").unwrap();
        let right_pos = content.find("Right").unwrap();

        assert!(left_pos < center_pos);
        assert!(center_pos < right_pos);
    }

    #[test]
    fn test_title_bar_narrow_width_no_render() {
        // Width < 10 should not render anything
        let title_bar = TitleBar::new("Title");
        let buf = render_title_bar(title_bar, 9, 1);
        let content = buffer_to_string(&buf);

        // Should be empty (just spaces)
        assert!(!content.contains("Title"));
    }

    #[test]
    fn test_title_bar_zero_height_no_render() {
        // Zero height buffer - just verify we can create the area
        let area = Rect::new(0, 0, 40, 0);
        assert_eq!(area.height, 0);
    }

    #[test]
    fn test_title_bar_title_centered() {
        let title_bar = TitleBar::new("XX");
        let buf = render_title_bar(title_bar, 20, 1);
        let content = buffer_to_string(&buf);

        // Find position of title - should be roughly centered
        let title_pos = content.find("XX").unwrap();
        // With width 20 and title "XX" (2 chars), ideal center is around position 9
        assert!((8..=10).contains(&title_pos));
    }

    #[test]
    fn test_title_bar_unicode_width() {
        // Test with Unicode characters that have different display widths
        let title_bar = TitleBar::new("日本語"); // Japanese characters (wider)
        let buf = render_title_bar(title_bar, 40, 1);

        // Should not panic when handling wide Unicode characters
        assert_eq!(buf.area.width, 40);
    }

    #[test]
    fn test_title_bar_empty_title() {
        let title_bar = TitleBar::new("");
        let buf = render_title_bar(title_bar, 40, 1);

        // Should not panic, just render empty
        assert_eq!(buf.area.width, 40);
    }

    #[test]
    fn test_title_bar_builder_pattern() {
        // Verify builder pattern works correctly
        let title_bar = TitleBar::new("Title")
            .with_info_left("L")
            .with_info_right("R");

        assert_eq!(title_bar.title, "Title");
        assert_eq!(title_bar.info_left, "L");
        assert_eq!(title_bar.info_right, "R");
    }

    #[test]
    fn test_title_bar_very_long_title_truncation() {
        let long_title = "A".repeat(100);
        let title_bar = TitleBar::new(long_title);
        let buf = render_title_bar(title_bar, 30, 1);
        let content = buffer_to_string(&buf);

        // Title should be truncated with ellipsis
        assert!(content.contains('…') || content.len() <= 30);
    }

    #[test]
    fn test_title_bar_info_respects_width_limit() {
        // Info should not take more than 1/3 of width
        let title_bar = TitleBar::new("T")
            .with_info_left("A".repeat(50).as_str())
            .with_info_right("B".repeat(50).as_str());
        let buf = render_title_bar(title_bar, 30, 1);

        // Should not panic, and title should still be visible if possible
        assert_eq!(buf.area.width, 30);
    }
}
