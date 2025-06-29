use nu_color_config::TextStyle;
use nu_pretty_hex::categorize_byte;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Span,
    widgets::{Paragraph, Widget},
};

use crate::{
    nu_common::NuStyle,
    views::util::{nu_style_to_tui, text_style_to_tui_style},
};

/// Padding between segments in the hex view
const SEGMENT_PADDING: u16 = 1;

#[derive(Debug, Clone)]
pub struct BinaryWidget<'a> {
    data: &'a [u8],
    opts: BinarySettings,
    style: BinaryStyle,
    row_offset: usize,
}

impl<'a> BinaryWidget<'a> {
    pub fn new(data: &'a [u8], opts: BinarySettings, style: BinaryStyle) -> Self {
        Self {
            data,
            opts,
            style,
            row_offset: 0,
        }
    }

    pub fn count_lines(&self) -> usize {
        self.data.len() / self.count_elements()
    }

    pub fn count_elements(&self) -> usize {
        self.opts.count_segments * self.opts.segment_size
    }

    pub fn set_row_offset(&mut self, offset: usize) {
        self.row_offset = offset;
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BinarySettings {
    segment_size: usize,
    count_segments: usize,
}

impl BinarySettings {
    pub fn new(segment_size: usize, count_segments: usize) -> Self {
        Self {
            segment_size,
            count_segments,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BinaryStyle {
    color_index: Option<NuStyle>,
    column_padding_left: u16,
    column_padding_right: u16,
}

impl BinaryStyle {
    pub fn new(
        color_index: Option<NuStyle>,
        column_padding_left: u16,
        column_padding_right: u16,
    ) -> Self {
        Self {
            color_index,
            column_padding_left,
            column_padding_right,
        }
    }
}

impl Widget for BinaryWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let min_width = get_widget_width(&self);

        if (area.width as usize) < min_width {
            return;
        }

        render_hexdump(area, buf, self);
    }
}

// todo: indent color
fn render_hexdump(area: Rect, buf: &mut Buffer, w: BinaryWidget) {
    const MIN_INDEX_SIZE: usize = 8;

    let index_width = get_max_index_size(&w).max(MIN_INDEX_SIZE) as u16; // safe as it's checked before hand that we have enough space

    let mut last_line = None;

    for line in 0..area.height {
        let data_line_length = w.opts.count_segments * w.opts.segment_size;
        let start_index = line as usize * data_line_length;
        let address = w.row_offset + start_index;

        if start_index > w.data.len() {
            last_line = Some(line);
            break;
        }

        let mut x = 0;
        let y = line;
        let line = &w.data[start_index..];

        // index column
        x += render_space(buf, x, y, 1, w.style.column_padding_left);
        x += render_hex_usize(buf, x, y, address, index_width, get_index_style(&w));
        x += render_space(buf, x, y, 1, w.style.column_padding_right);

        x += render_vertical_split(buf, x, y);

        // data/hex column
        x += render_space(buf, x, y, 1, w.style.column_padding_left);
        x += render_data_line(buf, x, y, line, &w);
        x += render_space(buf, x, y, 1, w.style.column_padding_right);

        x += render_vertical_split(buf, x, y);

        // ASCII column
        x += render_space(buf, x, y, 1, w.style.column_padding_left);
        x += render_ascii_line(buf, x, y, line, &w);
        render_space(buf, x, y, 1, w.style.column_padding_right);
    }

    let data_line_size = (w.opts.count_segments * (w.opts.segment_size * 2)
        + w.opts.count_segments.saturating_sub(1)) as u16;
    let ascii_line_size = (w.opts.count_segments * w.opts.segment_size) as u16;

    if let Some(last_line) = last_line {
        for line in last_line..area.height {
            let data_line_length = w.opts.count_segments * w.opts.segment_size;
            let start_index = line as usize * data_line_length;
            let address = w.row_offset + start_index;

            let mut x = 0;
            let y = line;

            // index column
            x += render_space(buf, x, y, 1, w.style.column_padding_left);
            x += render_hex_usize(buf, x, y, address, index_width, get_index_style(&w));
            x += render_space(buf, x, y, 1, w.style.column_padding_right);

            x += render_vertical_split(buf, x, y);

            // data/hex column
            x += render_space(buf, x, y, 1, w.style.column_padding_left);
            x += render_space(buf, x, y, 1, data_line_size);
            x += render_space(buf, x, y, 1, w.style.column_padding_right);

            x += render_vertical_split(buf, x, y);

            // ASCII column
            x += render_space(buf, x, y, 1, w.style.column_padding_left);
            x += render_space(buf, x, y, 1, ascii_line_size);
            render_space(buf, x, y, 1, w.style.column_padding_right);
        }
    }
}

fn render_data_line(buf: &mut Buffer, x: u16, y: u16, line: &[u8], w: &BinaryWidget) -> u16 {
    let mut size = 0;
    let mut count = 0;
    let count_max = w.opts.count_segments;
    let segment_size = w.opts.segment_size;

    size += render_segment(buf, x, y, line, w);
    count += 1;

    while count != count_max && count * segment_size < line.len() {
        let data = &line[count * segment_size..];
        size += render_space(buf, x + size, y, 1, SEGMENT_PADDING);
        size += render_segment(buf, x + size, y, data, w);
        count += 1;
    }

    while count != count_max {
        size += render_space(buf, x + size, y, 1, SEGMENT_PADDING);
        size += render_space(buf, x + size, y, 1, w.opts.segment_size as u16 * 2);
        count += 1;
    }

    size
}

fn render_segment(buf: &mut Buffer, x: u16, y: u16, line: &[u8], w: &BinaryWidget) -> u16 {
    let mut count = w.opts.segment_size;
    let mut size = 0;

    for &n in line {
        if count == 0 {
            break;
        }

        let (_, style) = get_segment_char(w, n);
        size += render_hex_u8(buf, x + size, y, n, style);
        count -= 1;
    }

    if count > 0 {
        size += render_space(buf, x + size, y, 1, (count * 2) as u16);
    }

    size
}

fn render_ascii_line(buf: &mut Buffer, x: u16, y: u16, line: &[u8], w: &BinaryWidget) -> u16 {
    let mut size = 0;
    let mut count = 0;
    let length = w.count_elements();

    for &n in line {
        if count == length {
            break;
        }

        let (c, style) = get_ascii_char(w, n);
        size += render_ascii_char(buf, x + size, y, c, style);
        count += 1;
    }

    if count < length {
        size += render_space(buf, x + size, y, 1, (length - count) as u16);
    }

    size
}

fn render_ascii_char(buf: &mut Buffer, x: u16, y: u16, n: char, style: Option<NuStyle>) -> u16 {
    let text = n.to_string();

    let mut p = Paragraph::new(text);
    if let Some(style) = style {
        let style = nu_style_to_tui(style);
        p = p.style(style);
    }

    let area = Rect::new(x, y, 1, 1);

    p.render(area, buf);

    1
}

fn render_hex_u8(buf: &mut Buffer, x: u16, y: u16, n: u8, style: Option<NuStyle>) -> u16 {
    render_hex_usize(buf, x, y, n as usize, 2, style)
}

fn render_hex_usize(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    n: usize,
    width: u16,
    style: Option<NuStyle>,
) -> u16 {
    let text = usize_to_hex(n, width as usize);
    let mut p = Paragraph::new(text);
    if let Some(style) = style {
        let style = nu_style_to_tui(style);
        p = p.style(style);
    }

    let area = Rect::new(x, y, width, 1);

    p.render(area, buf);

    width
}

fn get_ascii_char(_w: &BinaryWidget, n: u8) -> (char, Option<NuStyle>) {
    let (style, c) = categorize_byte(&n);
    let c = c.unwrap_or(n as char);
    let style = if style.is_plain() { None } else { Some(style) };

    (c, style)
}

fn get_segment_char(_w: &BinaryWidget, n: u8) -> (char, Option<NuStyle>) {
    let (style, c) = categorize_byte(&n);
    let c = c.unwrap_or(n as char);
    let style = if style.is_plain() { None } else { Some(style) };

    (c, style)
}

fn get_index_style(w: &BinaryWidget) -> Option<NuStyle> {
    w.style.color_index
}

/// Render blank characters starting at the given position, going right `width` characters and down `height` characters.
/// Returns `width` for convenience.
fn render_space(buf: &mut Buffer, x: u16, y: u16, height: u16, width: u16) -> u16 {
    repeat_vertical(buf, x, y, width, height, ' ', TextStyle::default());
    width
}

/// Render a vertical split (│) at the given position. Returns the width of the split (always 1) for convenience.
fn render_vertical_split(buf: &mut Buffer, x: u16, y: u16) -> u16 {
    repeat_vertical(buf, x, y, 1, 1, '│', TextStyle::default());
    1
}

fn repeat_vertical(
    buf: &mut Buffer,
    x_offset: u16,
    y_offset: u16,
    width: u16,
    height: u16,
    c: char,
    style: TextStyle,
) {
    let text = std::iter::repeat_n(c, width as usize).collect::<String>();
    let style = text_style_to_tui_style(style);
    let span = Span::styled(text, style);

    for row in 0..height {
        buf.set_span(x_offset, y_offset + row, &span, width);
    }
}

fn get_max_index_size(w: &BinaryWidget) -> usize {
    let line_size = w.opts.count_segments * (w.opts.segment_size * 2);
    let count_lines = w.data.len() / line_size;
    let max_index = w.row_offset + count_lines * line_size;
    usize_to_hex(max_index, 0).len()
}

fn get_widget_width(w: &BinaryWidget) -> usize {
    const MIN_INDEX_SIZE: usize = 8;

    let line_size = w.opts.count_segments * (w.opts.segment_size * 2);
    let count_lines = w.data.len() / line_size;

    let max_index = w.row_offset + count_lines * line_size;
    let index_size = usize_to_hex(max_index, 0).len();
    let index_size = index_size.max(MIN_INDEX_SIZE);

    let data_split_size = w.opts.count_segments.saturating_sub(1) * (SEGMENT_PADDING as usize);
    let data_size = line_size + data_split_size;

    let ascii_size = w.opts.count_segments * w.opts.segment_size;

    #[allow(clippy::identity_op)]
    let min_width = 0
        + w.style.column_padding_left as usize
        + index_size
        + w.style.column_padding_right as usize
        + 1 // split
        + w.style.column_padding_left as usize
        + data_size
        + w.style.column_padding_right as usize
        + 1 //split
        + w.style.column_padding_left as usize
        + ascii_size
        + w.style.column_padding_right as usize;

    min_width
}

fn usize_to_hex(n: usize, width: usize) -> String {
    if width == 0 {
        format!("{n:x}")
    } else {
        format!("{n:0>width$x}")
    }
}

#[cfg(test)]
mod tests {
    use crate::views::binary::binary_widget::usize_to_hex;

    #[test]
    fn test_to_hex() {
        assert_eq!(usize_to_hex(1, 2), "01");
        assert_eq!(usize_to_hex(16, 2), "10");
        assert_eq!(usize_to_hex(29, 2), "1d");
    }
}
