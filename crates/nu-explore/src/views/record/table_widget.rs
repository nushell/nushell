use super::Layout;
use crate::{
    explore::TableConfig,
    nu_common::{truncate_str, NuStyle, NuText},
    views::util::{nu_style_to_tui, text_style_to_tui_style},
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
use std::cmp::{max, Ordering};

#[derive(Debug, Clone)]
pub struct TableWidget<'a> {
    columns: &'a [String],
    data: &'a [Vec<NuText>],
    index_row: usize,
    index_column: usize,
    config: TableConfig,
    header_position: Orientation,
    style_computer: &'a StyleComputer<'a>,
}

// Basically: where's the header of the value being displayed? Usually at the top for tables, on the left for records
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Top,
    Left,
}

impl<'a> TableWidget<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        columns: &'a [String],
        data: &'a [Vec<NuText>],
        style_computer: &'a StyleComputer<'a>,
        index_row: usize,
        index_column: usize,
        config: TableConfig,
        header_position: Orientation,
    ) -> Self {
        Self {
            columns,
            data,
            style_computer,
            index_row,
            index_column,
            config,
            header_position,
        }
    }
}

#[derive(Debug, Default)]
pub struct TableWidgetState {
    pub layout: Layout,
    pub count_rows: usize,
    pub count_columns: usize,
    pub data_height: u16,
}

impl StatefulWidget for TableWidget<'_> {
    type State = TableWidgetState;

    fn render(
        self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        if area.width < 5 {
            return;
        }

        let is_horizontal = matches!(self.header_position, Orientation::Top);
        if is_horizontal {
            self.render_table_horizontal(area, buf, state);
        } else {
            self.render_table_vertical(area, buf, state);
        }
    }
}

// todo: refactoring these to methods as they have quite a bit in common.
impl TableWidget<'_> {
    // header at the top; header is always 1 line
    fn render_table_horizontal(self, area: Rect, buf: &mut Buffer, state: &mut TableWidgetState) {
        let padding_l = self.config.column_padding_left as u16;
        let padding_r = self.config.column_padding_right as u16;

        let show_index = self.config.show_index;
        let show_head = self.config.show_header;

        let separator_s = self.config.separator_style;

        let mut data_height = area.height;
        let mut data_y = area.y;
        let mut head_y = area.y;

        if show_head {
            data_y += 1;
            data_height -= 1;

            // top line
            data_y += 1;
            data_height -= 1;
            head_y += 1;

            // bottom line
            data_y += 1;
            data_height -= 1;
        }

        if area.width == 0 || area.height == 0 {
            return;
        }

        let mut width = area.x;
        let mut data = &self.data[self.index_row..];
        if data.len() > data_height as usize {
            data = &data[..data_height as usize];
        }

        if show_head {
            render_header_borders(buf, area, 1, separator_s);
        }

        if show_index {
            width += render_index(
                buf,
                Rect::new(width, data_y, area.width, data_height),
                self.style_computer,
                self.index_row,
                padding_l,
                padding_r,
            );

            width += render_split_line(buf, width, area.y, area.height, show_head, separator_s);
        }

        // if there is more data than we can show, add an ellipsis to the column headers to hint at that
        let mut show_overflow_indicator = false;
        state.count_rows = data.len();
        state.count_columns = 0;
        state.data_height = data_height;

        if width > area.width {
            return;
        }

        for col in self.index_column..self.columns.len() {
            let need_split_line = state.count_columns > 0 && width < area.width;
            if need_split_line {
                width += render_split_line(buf, width, area.y, area.height, show_head, separator_s);
            }

            let mut column = create_column(data, col);
            let column_width = calculate_column_width(&column);

            let mut head = String::from(&self.columns[col]);
            let head_width = string_width(&head);

            let mut use_space = column_width as u16;
            if show_head {
                use_space = max(head_width as u16, use_space);
            }

            if use_space > 0 {
                let is_last = col + 1 == self.columns.len();
                let space = area.width - width;

                let pad = padding_l + padding_r;
                let head = show_head.then_some(&mut head);
                let (w, ok, overflow) =
                    truncate_column_width(space, 1, use_space, pad, is_last, &mut column, head);

                if overflow {
                    show_overflow_indicator = true;
                }

                if w == 0 && !ok {
                    break;
                }

                use_space = w;
            }

            if show_head {
                let head_style = head_style(&head, self.style_computer);
                if head_width > use_space as usize {
                    truncate_str(&mut head, use_space as usize)
                }
                let head_iter = [(&head, head_style)].into_iter();

                // we don't change width here cause the whole column have the same width; so we add it when we print data
                let mut w = width;
                w += render_space(buf, w, head_y, 1, padding_l);
                w += render_column(buf, w, head_y, use_space, head_iter);
                w += render_space(buf, w, head_y, 1, padding_r);

                let x = w - padding_r - use_space;
                state.layout.push(&head, x, head_y, use_space, 1);
            }

            let column_rows = column.iter().map(|(t, s)| (t, *s));

            width += render_space(buf, width, data_y, data_height, padding_l);
            width += render_column(buf, width, data_y, use_space, column_rows);
            width += render_space(buf, width, data_y, data_height, padding_r);

            for (row, (text, _)) in column.iter().enumerate() {
                let x = width - padding_r - use_space;
                let y = data_y + row as u16;
                state.layout.push(text, x, y, use_space, 1);
            }

            state.count_columns += 1;

            if show_overflow_indicator {
                break;
            }
        }

        if show_overflow_indicator && show_head {
            width += render_space(buf, width, data_y, data_height, padding_l);
            width += render_overflow_column(buf, width, head_y, 1);
            width += render_space(buf, width, data_y, data_height, padding_r);
        }

        if width < area.width {
            width += render_split_line(buf, width, area.y, area.height, show_head, separator_s);
        }

        let rest = area.width.saturating_sub(width);
        if rest > 0 {
            render_space(buf, width, data_y, data_height, rest);
            if show_head {
                render_space(buf, width, head_y, 1, rest);
            }
        }
    }

    // header at the left; header is always 1 line
    fn render_table_vertical(self, area: Rect, buf: &mut Buffer, state: &mut TableWidgetState) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let padding_l = self.config.column_padding_left as u16;
        let padding_r = self.config.column_padding_right as u16;

        let show_index = self.config.show_index;
        let show_head = self.config.show_header;
        let separator_s = self.config.separator_style;

        let mut left_w = 0;

        if show_index {
            let area = Rect::new(area.x, area.y, area.width, area.height);
            left_w += render_index(
                buf,
                area,
                self.style_computer,
                self.index_row,
                padding_l,
                padding_r,
            );

            left_w += render_vertical_line_with_split(
                buf,
                area.x + left_w,
                area.y,
                area.height,
                false,
                false,
                separator_s,
            );
        }

        let mut columns = &self.columns[self.index_row..];
        if columns.len() > area.height as usize {
            columns = &columns[..area.height as usize];
        }

        if show_head {
            let columns_width = columns.iter().map(|s| string_width(s)).max().unwrap_or(0);

            let will_use_space =
                padding_l as usize + padding_r as usize + columns_width + left_w as usize;
            if will_use_space > area.width as usize {
                return;
            }

            let columns_iter = columns
                .iter()
                .map(|s| (s.clone(), head_style(s, self.style_computer)));

            if !show_index {
                let x = area.x + left_w;
                left_w += render_vertical_line_with_split(
                    buf,
                    x,
                    area.y,
                    area.height,
                    false,
                    false,
                    separator_s,
                );
            }

            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, 1, padding_l);
            let x = area.x + left_w;
            left_w += render_column(buf, x, area.y, columns_width as u16, columns_iter);
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, 1, padding_r);

            let layout_x = left_w - padding_r - columns_width as u16;
            for (i, text) in columns.iter().enumerate() {
                state
                    .layout
                    .push(text, layout_x, area.y + i as u16, columns_width as u16, 1);
            }

            left_w += render_vertical_line_with_split(
                buf,
                area.x + left_w,
                area.y,
                area.height,
                false,
                false,
                separator_s,
            );
        }

        // if there is more data than we can show, add an ellipsis to the column headers to hint at that
        let mut show_overflow_indicator = false;

        state.count_rows = columns.len();
        state.count_columns = 0;

        // note: is there a time where we would have more then 1 column?
        // seems like not really; cause it's literally KV table, or am I wrong?

        for col in self.index_column..self.data.len() {
            let mut column =
                self.data[col][self.index_row..self.index_row + columns.len()].to_vec();
            let column_width = calculate_column_width(&column);
            if column_width > u16::MAX as usize {
                break;
            }

            // see KV comment; this block might never got used
            let need_split_line = state.count_columns > 0 && left_w < area.width;
            if need_split_line {
                render_vertical_line(buf, area.x + left_w, area.y, area.height, separator_s);
                left_w += 1;
            }

            let column_width = column_width as u16;
            let available = area.width - left_w;
            let is_last = col + 1 == self.data.len();
            let pad = padding_l + padding_r;
            let (column_width, ok, overflow) =
                truncate_column_width(available, 1, column_width, pad, is_last, &mut column, None);

            if overflow {
                show_overflow_indicator = true;
            }

            if column_width == 0 && !ok {
                break;
            }

            let head_rows = column.iter().map(|(t, s)| (t, *s));

            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_l);
            let x = area.x + left_w;
            left_w += render_column(buf, x, area.y, column_width, head_rows);
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_r);

            {
                for (row, (text, _)) in column.iter().enumerate() {
                    let x = left_w - padding_r - column_width;
                    let y = area.y + row as u16;
                    state.layout.push(text, x, y, column_width, 1);
                }

                state.count_columns += 1;
            }

            if show_overflow_indicator {
                break;
            }
        }

        if show_overflow_indicator {
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_l);
            let x = area.x + left_w;
            left_w += render_overflow_column(buf, x, area.y, area.height);
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_r);
        }

        _ = left_w;
    }
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

    let (width, overflow) = match result {
        Some(result) => result,
        None => return (w, true, false),
    };

    if width == 0 {
        return (0, false, overflow);
    }

    truncate_list(column, width as usize);
    if let Some(head) = head {
        truncate_str(head, width as usize);
    }

    (width, false, overflow)
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

struct IndexColumn<'a> {
    style_computer: &'a StyleComputer<'a>,
    start: usize,
}

impl<'a> IndexColumn<'a> {
    fn new(style_computer: &'a StyleComputer, start: usize) -> Self {
        Self {
            style_computer,
            start,
        }
    }

    fn estimate_width(&self, height: u16) -> usize {
        let last_row = self.start + height as usize;
        last_row.to_string().len()
    }
}

impl Widget for IndexColumn<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for row in 0..area.height {
            let i = row as usize + self.start;
            let text = i.to_string();
            let style = nu_style_to_tui(self.style_computer.compute(
                "row_index",
                &Value::string(text.as_str(), nu_protocol::Span::unknown()),
            ));

            let p = Paragraph::new(text)
                .style(style)
                .alignment(ratatui::layout::Alignment::Right);
            let area = Rect::new(area.x, area.y + row, area.width, 1);

            p.render(area, buf);
        }
    }
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

fn render_index(
    buf: &mut Buffer,

    area: Rect,

    style_computer: &StyleComputer,
    start_index: usize,
    padding_left: u16,
    padding_right: u16,
) -> u16 {
    let mut width = render_space(buf, area.x, area.y, area.height, padding_left);

    let index = IndexColumn::new(style_computer, start_index);
    let w = index.estimate_width(area.height) as u16;
    let area = Rect::new(area.x + width, area.y, w, area.height);

    index.render(area, buf);

    width += w;
    width += render_space(buf, area.x + width, area.y, area.height, padding_right);

    width
}

fn render_split_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    has_head: bool,
    style: NuStyle,
) -> u16 {
    if has_head {
        render_vertical_split_line(buf, x, y, height, &[y], &[y + 2], &[], style);
    } else {
        render_vertical_split_line(buf, x, y, height, &[], &[], &[], style);
    }

    1
}

#[allow(clippy::too_many_arguments)]
fn render_vertical_split_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    top_slit: &[u16],
    inner_slit: &[u16],
    bottom_slit: &[u16],
    style: NuStyle,
) -> u16 {
    render_vertical_line(buf, x, y, height, style);

    for &y in top_slit {
        render_top_connector(buf, x, y, style);
    }

    for &y in inner_slit {
        render_inner_connector(buf, x, y, style);
    }

    for &y in bottom_slit {
        render_bottom_connector(buf, x, y, style);
    }

    1
}

fn render_vertical_line_with_split(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    top_slit: bool,
    bottom_slit: bool,
    style: NuStyle,
) -> u16 {
    render_vertical_line(buf, x, y, height, style);

    if top_slit && y > 0 {
        render_top_connector(buf, x, y - 1, style);
    }

    if bottom_slit {
        render_bottom_connector(buf, x, y + height, style);
    }

    1
}

fn render_vertical_line(buf: &mut Buffer, x: u16, y: u16, height: u16, style: NuStyle) {
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

/// Starting at cell [x, y]: paint `width` characters of `c` (left to right), move 1 row down, repeat
/// Repeat this `height` times
fn repeat_vertical(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    c: char,
    style: TextStyle,
) {
    let text = String::from(c);
    let style = text_style_to_tui_style(style);
    let span = Span::styled(&text, style);

    for row in 0..height {
        for col in 0..width {
            buf.set_span(x + col, y + row, &span, 1);
        }
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

/// Render a column with an ellipsis in the header to indicate that there is more data than can be displayed
fn render_overflow_column(buf: &mut Buffer, x: u16, y: u16, height: u16) -> u16 {
    let style = TextStyle {
        alignment: Alignment::Left,
        color_style: None,
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

fn render_inner_connector(buf: &mut Buffer, x: u16, y: u16, style: NuStyle) {
    let style = nu_style_to_tui(style);
    let span = Span::styled("┼", style);
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

fn render_column<T, S>(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    available_width: u16,
    rows: impl Iterator<Item = (T, S)>,
) -> u16
where
    T: AsRef<str>,
    S: Into<TextStyle>,
{
    for (row, (text, style)) in rows.enumerate() {
        let style = text_style_to_tui_style(style.into());
        let span = Span::styled(text.as_ref(), style);
        buf.set_span(x, y + row as u16, &span, available_width);
    }

    available_width
}

fn head_style(head: &str, style_computer: &StyleComputer) -> TextStyle {
    let style =
        style_computer.compute("header", &Value::string(head, nu_protocol::Span::unknown()));
    TextStyle::with_style(Alignment::Center, style)
}
