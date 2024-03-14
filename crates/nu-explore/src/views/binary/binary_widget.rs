use std::{
    borrow::Cow,
    cmp::{max, Ordering},
    fmt::Write,
};

use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_protocol::Value;
use nu_table::string_width;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    nu_common::{truncate_str, NuStyle, NuText},
    views::util::{nu_style_to_tui, text_style_to_tui_style},
};

use super::Layout;

type OptStyle = Option<NuStyle>;

#[derive(Debug, Clone)]
pub struct BinaryWidget<'a> {
    data: &'a [u8],
    opts: BinarySettings,
    style: BinaryStyle,
}

impl<'a> BinaryWidget<'a> {
    pub fn new(data: &'a [u8], opts: BinarySettings, style: BinaryStyle) -> Self {
        Self { data, opts, style }
    }

    pub fn count_lines(&self) -> usize {
        self.data.len() / self.count_elements()
    }

    pub fn count_elements(&self) -> usize {
        self.opts.count_segments * self.opts.segment_size
    }

    pub fn set_index_offset(&mut self, offset: usize) {
        self.opts.index_offset = offset;
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BinarySettings {
    disable_index: bool,
    disable_ascii: bool,
    disable_data: bool,
    segment_size: usize,
    count_segments: usize,
    index_offset: usize,
}

impl BinarySettings {
    pub fn new(
        disable_index: bool,
        disable_ascii: bool,
        disable_data: bool,
        segment_size: usize,
        count_segments: usize,
        index_offset: usize,
    ) -> Self {
        Self {
            disable_index,
            disable_ascii,
            disable_data,
            segment_size,
            count_segments,
            index_offset,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BinaryStyle {
    colors: BinaryStyleColors,
    indent_index: Indent,
    indent_data: Indent,
    indent_ascii: Indent,
    indent_segment: usize,
    show_split: bool,
}

impl BinaryStyle {
    pub fn new(
        colors: BinaryStyleColors,
        indent_index: Indent,
        indent_data: Indent,
        indent_ascii: Indent,
        indent_segment: usize,
        show_split: bool,
    ) -> Self {
        Self {
            colors,
            indent_index,
            indent_data,
            indent_ascii,
            indent_segment,
            show_split,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Indent {
    left: u16,
    right: u16,
}

impl Indent {
    pub fn new(left: u16, right: u16) -> Self {
        Self { left, right }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BinaryStyleColors {
    pub split_left: OptStyle,
    pub split_right: OptStyle,
    pub index: OptStyle,
    pub data: OptStyle,
    pub data_zero: OptStyle,
    pub data_unknown: OptStyle,
    pub ascii: OptStyle,
    pub ascii_zero: OptStyle,
    pub ascii_unknown: OptStyle,
}

impl BinaryStyleColors {
    pub fn new(
        split_left: OptStyle,
        split_right: OptStyle,
        index: OptStyle,
        data: OptStyle,
        data_zero: OptStyle,
        data_unknown: OptStyle,
        ascii: OptStyle,
        ascii_zero: OptStyle,
        ascii_unknown: OptStyle,
    ) -> Self {
        Self {
            split_left,
            split_right,
            index,
            data,
            data_zero,
            data_unknown,
            ascii,
            ascii_zero,
            ascii_unknown,
        }
    }
}

#[derive(Debug, Default)]
pub struct BinaryWidgetState {
    pub layout_index: Layout,
    pub layout_data: Layout,
    pub layout_ascii: Layout,
}

impl StatefulWidget for BinaryWidget<'_> {
    type State = BinaryWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let min_width = get_widget_width(&self);

        if (area.width as usize) < min_width {
            return;
        }

        if self.opts.disable_index && self.opts.disable_data && self.opts.disable_ascii {
            return;
        }

        render_hexdump(area, buf, state, self);
    }
}

// todo: indent color
fn render_hexdump(area: Rect, buf: &mut Buffer, state: &mut BinaryWidgetState, w: BinaryWidget) {
    const MIN_INDEX_SIZE: usize = 8;

    let show_index = !w.opts.disable_index;
    let show_data = !w.opts.disable_data;
    let show_ascii = !w.opts.disable_ascii;
    let show_split = w.style.show_split;

    let index_width = get_max_index_size(&w).max(MIN_INDEX_SIZE) as u16; // safe as it's checked before hand that we have enough space

    let mut last_line = None;

    for line in 0..area.height {
        let data_line_length = w.opts.count_segments * w.opts.segment_size;
        let start_index = line as usize * data_line_length;
        let address = w.opts.index_offset + start_index;

        if start_index > w.data.len() {
            last_line = Some(line);
            break;
        }

        let mut x = 0;
        let y = line;
        let line = &w.data[start_index..];

        if show_index {
            x += render_space(buf, x, y, 1, w.style.indent_index.left);
            x += render_hex_usize(buf, x, y, address, index_width, false, get_index_style(&w));
            x += render_space(buf, x, y, 1, w.style.indent_index.right);
        }

        if show_split {
            x += render_split(buf, x, y);
        }

        if show_data {
            x += render_space(buf, x, y, 1, w.style.indent_data.left);
            x += render_data_line(buf, x, y, line, &w);
            x += render_space(buf, x, y, 1, w.style.indent_data.right);
        }

        if show_split {
            x += render_split(buf, x, y);
        }

        if show_ascii {
            x += render_space(buf, x, y, 1, w.style.indent_ascii.left);
            x += render_ascii_line(buf, x, y, line, &w);
            render_space(buf, x, y, 1, w.style.indent_ascii.right);
        }
    }

    let data_line_size = (w.opts.count_segments * (w.opts.segment_size * 2)
        + w.opts.count_segments.saturating_sub(1)) as u16;
    let ascii_line_size = (w.opts.count_segments * w.opts.segment_size) as u16;

    if let Some(last_line) = last_line {
        for line in last_line..area.height {
            let data_line_length = w.opts.count_segments * w.opts.segment_size;
            let start_index = line as usize * data_line_length;
            let address = w.opts.index_offset + start_index;

            let mut x = 0;
            let y = line;

            if show_index {
                x += render_space(buf, x, y, 1, w.style.indent_index.left);
                x += render_hex_usize(buf, x, y, address, index_width, false, get_index_style(&w));
                x += render_space(buf, x, y, 1, w.style.indent_index.right);
            }

            if show_split {
                x += render_split(buf, x, y);
            }

            if show_data {
                x += render_space(buf, x, y, 1, w.style.indent_data.left);
                x += render_space(buf, x, y, 1, data_line_size);
                x += render_space(buf, x, y, 1, w.style.indent_data.right);
            }

            if show_split {
                x += render_split(buf, x, y);
            }

            if show_ascii {
                x += render_space(buf, x, y, 1, w.style.indent_ascii.left);
                x += render_space(buf, x, y, 1, ascii_line_size);
                render_space(buf, x, y, 1, w.style.indent_ascii.right);
            }
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
        size += render_space(buf, x + size, y, 1, w.style.indent_segment as u16);
        size += render_segment(buf, x + size, y, data, w);
        count += 1;
    }

    while count != count_max {
        size += render_space(buf, x + size, y, 1, w.style.indent_segment as u16);
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

        size += render_hex_u8(buf, x + size, y, n, false, get_segment_style(w, n));
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

        size += render_ascii_u8(buf, x + size, y, n, get_ascii_style(w, n));
        count += 1;
    }

    if count < length {
        size += render_space(buf, x + size, y, 1, (length - count) as u16);
    }

    size
}

fn render_ascii_u8(buf: &mut Buffer, x: u16, y: u16, n: u8, style: OptStyle) -> u16 {
    let c = u8_to_ascii(n);
    let text = c.to_string();

    let mut p = Paragraph::new(text);
    if let Some(style) = style {
        let style = nu_style_to_tui(style);
        p = p.style(style);
    }

    let area = Rect::new(x, y, 1, 1);

    p.render(area, buf);

    1
}

fn u8_to_ascii(n: u8) -> char {
    if n == b' ' {
        '.'
    } else if n.is_ascii() {
        n as char
    } else {
        ' '
    }
}

fn render_hex_u8(buf: &mut Buffer, x: u16, y: u16, n: u8, big: bool, style: OptStyle) -> u16 {
    render_hex_usize(buf, x, y, n as usize, 2, big, style)
}

fn render_hex_usize(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    n: usize,
    width: u16,
    big: bool,
    style: OptStyle,
) -> u16 {
    let text = usize_to_hex(n, width as usize, big);
    let mut p = Paragraph::new(text);
    if let Some(style) = style {
        let style = nu_style_to_tui(style);
        p = p.style(style);
    }

    let area = Rect::new(x, y, width, 1);

    p.render(area, buf);

    width
}

fn get_ascii_style(w: &BinaryWidget, n: u8) -> OptStyle {
    if n == 0 {
        w.style.colors.ascii_zero
    } else if n.is_ascii() {
        w.style.colors.ascii
    } else {
        w.style.colors.ascii_unknown
    }
}

fn get_segment_style(w: &BinaryWidget, n: u8) -> OptStyle {
    if n == 0 {
        w.style.colors.data_zero
    } else if n.is_ascii() {
        w.style.colors.data
    } else {
        w.style.colors.data_unknown
    }
}

fn get_index_style(w: &BinaryWidget) -> OptStyle {
    w.style.colors.index
}

#[allow(clippy::too_many_arguments)]
fn truncate_column_width(
    space: u16,
    min: u16,
    w: u16,
    pad: u16,
    is_last: bool,
    column: &mut [(String, TextStyle)],
    head: Option<&mut String>,
) -> (u16, bool, bool) {
    let result = check_column_width(space, min, w, pad, is_last);

    let (width, shift_column) = match result {
        Some(result) => result,
        None => return (w, true, false),
    };

    if width == 0 {
        return (0, false, shift_column);
    }

    truncate_list(column, width as usize);
    if let Some(head) = head {
        truncate_str(head, width as usize);
    }

    (width, false, shift_column)
}

fn check_column_width(
    space: u16,
    min: u16,
    w: u16,
    pad: u16,
    is_last: bool,
) -> Option<(u16, bool)> {
    if !is_space_available(space, pad) {
        return Some((0, false));
    }

    if is_last {
        if !is_space_available(space, w + pad) {
            return Some((space - pad, false));
        } else {
            return None;
        }
    }

    if !is_space_available(space, min + pad) {
        return Some((0, false));
    }

    if !is_space_available(space, w + pad + min + pad) {
        let left_space = space - (min + pad);

        if left_space > pad {
            let left = left_space - pad;
            return Some((left, true));
        } else {
            return Some((0, true));
        }
    }

    None
}

fn render_header_borders(buf: &mut Buffer, area: Rect, span: u16, style: NuStyle) -> (u16, u16) {
    let borders = Borders::TOP | Borders::BOTTOM;
    let block = Block::default()
        .borders(borders)
        .border_style(nu_style_to_tui(style));
    let height = span + 2;
    let area = Rect::new(area.x, area.y, area.width, height);
    block.render(area, buf);

    // y pos of header text and next line
    (height.saturating_sub(2), height)
}

fn render_vertical(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    top_slit: bool,
    bottom_slit: bool,
    style: NuStyle,
) -> u16 {
    render_vertical_split(buf, x, y, height, style);

    if top_slit && y > 0 {
        render_top_connector(buf, x, y - 1, style);
    }

    if bottom_slit {
        render_bottom_connector(buf, x, y + height, style);
    }

    1
}

fn render_vertical_split(buf: &mut Buffer, x: u16, y: u16, height: u16, style: NuStyle) {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(style),
    };

    repeat_vertical(buf, x, y, 1, height, '│', style);
}

fn render_space(buf: &mut Buffer, x: u16, y: u16, height: u16, padding: u16) -> u16 {
    repeat_vertical(buf, x, y, padding, height, ' ', TextStyle::default());
    padding
}

fn render_split(buf: &mut Buffer, x: u16, y: u16) -> u16 {
    repeat_vertical(buf, x, y, 1, 1, '│', TextStyle::default());
    1
}

fn create_column(data: &[Vec<NuText>], col: usize) -> Vec<NuText> {
    let mut column = vec![NuText::default(); data.len()];
    for (row, values) in data.iter().enumerate() {
        if values.is_empty() {
            debug_assert!(false, "must never happen?");
            continue;
        }

        let value = &values[col];

        let text = value.0.replace('\n', " ");

        column[row] = (text, value.1);
    }

    column
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
    let text = std::iter::repeat(c)
        .take(width as usize)
        .collect::<String>();
    let style = text_style_to_tui_style(style);
    let span = Span::styled(text, style);

    for row in 0..height {
        buf.set_span(x_offset, y_offset + row, &span, width);
    }
}

fn is_space_available(available: u16, got: u16) -> bool {
    match available.cmp(&got) {
        Ordering::Less => false,
        Ordering::Equal | Ordering::Greater => true,
    }
}

fn truncate_list(list: &mut [NuText], width: usize) {
    for (text, _) in list {
        truncate_str(text, width);
    }
}

fn render_shift_column(buf: &mut Buffer, x: u16, y: u16, height: u16, style: NuStyle) -> u16 {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: Some(style),
    };

    repeat_vertical(buf, x, y, 1, height, '…', style);

    1
}

fn render_top_connector(buf: &mut Buffer, x: u16, y: u16, style: NuStyle) {
    let style = nu_style_to_tui(style);
    let span = Span::styled("┬", style);
    buf.set_span(x, y, &span, 1);
}

fn render_bottom_connector(buf: &mut Buffer, x: u16, y: u16, style: NuStyle) {
    let style = nu_style_to_tui(style);
    let span = Span::styled("┴", style);
    buf.set_span(x, y, &span, 1);
}

fn calculate_column_width(column: &[NuText]) -> usize {
    column
        .iter()
        .map(|(text, _)| text)
        .map(|text| string_width(text))
        .max()
        .unwrap_or(0)
}

fn render_column(buf: &mut Buffer, x: u16, y: u16, available_width: u16, rows: &[NuText]) -> u16 {
    for (row, (text, style)) in rows.iter().enumerate() {
        let style = text_style_to_tui_style(*style);
        let text = strip_string(text);
        let span = Span::styled(text, style);
        buf.set_span(x, y + row as u16, &span, available_width);
    }

    available_width
}

fn strip_string(text: &str) -> String {
    String::from_utf8(strip_ansi_escapes::strip(text))
        .map_err(|_| ())
        .unwrap_or_else(|_| text.to_owned())
}

fn get_max_index_size(w: &BinaryWidget) -> usize {
    let line_size = w.opts.count_segments * (w.opts.segment_size * 2);
    let count_lines = w.data.len() / line_size;
    let max_index = w.opts.index_offset + count_lines * line_size;
    usize_to_hex(max_index, 0, false).len()
}

fn get_count_lines(w: &BinaryWidget) -> usize {
    let line_size = w.opts.count_segments * (w.opts.segment_size * 2);
    w.data.len() / line_size
}

fn get_widget_width(w: &BinaryWidget) -> usize {
    const MIN_INDEX_SIZE: usize = 8;

    let line_size = w.opts.count_segments * (w.opts.segment_size * 2);
    let count_lines = w.data.len() / line_size;

    let max_index = w.opts.index_offset + count_lines * line_size;
    let index_size = usize_to_hex(max_index, 0, false).len();
    let index_size = index_size.max(MIN_INDEX_SIZE);

    let data_split_size = w.opts.count_segments.saturating_sub(1) * w.style.indent_segment;
    let data_size = line_size + data_split_size;

    let ascii_size = w.opts.count_segments * w.opts.segment_size;

    let split = w.style.show_split as usize;
    #[allow(clippy::identity_op)]
    let min_width = 0
        + w.style.indent_index.left as usize
        + index_size
        + w.style.indent_index.right as usize
        + split
        + w.style.indent_data.left as usize
        + data_size
        + w.style.indent_data.right as usize
        + split
        + w.style.indent_ascii.left as usize
        + ascii_size
        + w.style.indent_ascii.right as usize;

    min_width
}

fn usize_to_hex(n: usize, width: usize, big: bool) -> String {
    if width == 0 {
        match big {
            true => format!("{:X}", n),
            false => format!("{:x}", n),
        }
    } else {
        match big {
            true => format!("{:0>width$X}", n, width = width),
            false => format!("{:0>width$x}", n, width = width),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::views::binary::binary_widget::usize_to_hex;

    #[test]
    fn test_to_hex() {
        assert_eq!(usize_to_hex(1, 2, false), "01");
        assert_eq!(usize_to_hex(16, 2, false), "10");
        assert_eq!(usize_to_hex(29, 2, false), "1d");
        assert_eq!(usize_to_hex(29, 2, true), "1D");
    }
}
