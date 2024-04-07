use super::Layout;
use crate::{
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
use std::{
    borrow::Cow,
    cmp::{max, Ordering},
};

#[derive(Debug, Clone)]
pub struct TableW<'a> {
    columns: Cow<'a, [String]>,
    data: Cow<'a, [Vec<NuText>]>,
    index_row: usize,
    index_column: usize,
    style: TableStyle,
    head_position: Orientation,
    style_computer: &'a StyleComputer<'a>,
}

// Basically: where's the header of the value being displayed? Usually at the top for tables, on the left for records
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Top,
    Left,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TableStyle {
    pub splitline_style: NuStyle,
    pub shift_line_style: NuStyle,
    pub show_index: bool,
    pub show_header: bool,
    pub padding_index_left: usize,
    pub padding_index_right: usize,
    pub padding_column_left: usize,
    pub padding_column_right: usize,
}

impl<'a> TableW<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        columns: impl Into<Cow<'a, [String]>>,
        data: impl Into<Cow<'a, [Vec<NuText>]>>,
        style_computer: &'a StyleComputer<'a>,
        index_row: usize,
        index_column: usize,
        style: TableStyle,
        head_position: Orientation,
    ) -> Self {
        Self {
            columns: columns.into(),
            data: data.into(),
            style_computer,
            index_row,
            index_column,
            style,
            head_position,
        }
    }
}

#[derive(Debug, Default)]
pub struct TableWState {
    pub layout: Layout,
    pub count_rows: usize,
    pub count_columns: usize,
    pub data_height: u16,
}

impl StatefulWidget for TableW<'_> {
    type State = TableWState;

    fn render(
        self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        if area.width < 5 {
            return;
        }

        let is_horizontal = matches!(self.head_position, Orientation::Top);
        if is_horizontal {
            self.render_table_horizontal(area, buf, state);
        } else {
            self.render_table_vertical(area, buf, state);
        }
    }
}

// todo: refactoring these to methods as they have quite a bit in common.
impl<'a> TableW<'a> {
    fn render_table_horizontal(self, area: Rect, buf: &mut Buffer, state: &mut TableWState) {
        let padding_cell_l = self.style.padding_column_left as u16;
        let padding_cell_r = self.style.padding_column_right as u16;
        let padding_index_l = self.style.padding_index_left as u16;
        let padding_index_r = self.style.padding_index_right as u16;

        let show_index = self.style.show_index;
        let show_head = self.style.show_header;

        let splitline_s = self.style.splitline_style;
        let shift_column_s = self.style.shift_line_style;

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
            render_header_borders(buf, area, 1, splitline_s);
        }

        if show_index {
            let area = Rect::new(width, data_y, area.width, data_height);
            width += render_index(
                buf,
                area,
                self.style_computer,
                self.index_row,
                padding_index_l,
                padding_index_r,
            );

            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                false,
                splitline_s,
            );
        }

        let mut do_render_shift_column = false;
        state.count_rows = data.len();
        state.count_columns = 0;
        state.data_height = data_height;

        if width > area.width {
            return;
        }

        for col in self.index_column..self.columns.len() {
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

                let pad = padding_cell_l + padding_cell_r;
                let head = show_head.then_some(&mut head);
                let (w, ok, shift) =
                    truncate_column_width(space, 1, use_space, pad, is_last, &mut column, head);

                if shift {
                    do_render_shift_column = true;
                }

                if w == 0 && !ok {
                    break;
                }

                use_space = w;
            }

            if show_head {
                let mut header = [head_row_text(&head, self.style_computer)];
                if head_width > use_space as usize {
                    truncate_str(&mut header[0].0, use_space as usize)
                }

                let mut w = width;
                w += render_space(buf, w, head_y, 1, padding_cell_l);
                w += render_column(buf, w, head_y, use_space, &header);
                w += render_space(buf, w, head_y, 1, padding_cell_r);

                let x = w - padding_cell_r - use_space;
                state.layout.push(&header[0].0, x, head_y, use_space, 1);
            }

            width += render_space(buf, width, data_y, data_height, padding_cell_l);
            width += render_column(buf, width, data_y, use_space, &column);
            width += render_space(buf, width, data_y, data_height, padding_cell_r);

            for (row, (text, _)) in column.iter().enumerate() {
                let x = width - padding_cell_r - use_space;
                let y = data_y + row as u16;
                state.layout.push(text, x, y, use_space, 1);
            }

            state.count_columns += 1;

            if do_render_shift_column {
                break;
            }
        }

        if do_render_shift_column && show_head {
            width += render_space(buf, width, data_y, data_height, padding_cell_l);
            width += render_shift_column(buf, width, head_y, 1, shift_column_s);
            width += render_space(buf, width, data_y, data_height, padding_cell_r);
        }

        if width < area.width {
            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                false,
                splitline_s,
            );
        }

        let rest = area.width.saturating_sub(width);
        if rest > 0 {
            render_space(buf, width, data_y, data_height, rest);
            if show_head {
                render_space(buf, width, head_y, 1, rest);
            }
        }
    }

    fn render_table_vertical(self, area: Rect, buf: &mut Buffer, state: &mut TableWState) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let padding_cell_l = self.style.padding_column_left as u16;
        let padding_cell_r = self.style.padding_column_right as u16;
        let padding_index_l = self.style.padding_index_left as u16;
        let padding_index_r = self.style.padding_index_right as u16;

        let show_index = self.style.show_index;
        let show_head = self.style.show_header;
        let splitline_s = self.style.splitline_style;
        let shift_column_s = self.style.shift_line_style;

        let mut left_w = 0;

        if show_index {
            let area = Rect::new(area.x, area.y, area.width, area.height);
            left_w += render_index(
                buf,
                area,
                self.style_computer,
                self.index_row,
                padding_index_l,
                padding_index_r,
            );

            left_w += render_vertical(
                buf,
                area.x + left_w,
                area.y,
                area.height,
                false,
                false,
                splitline_s,
            );
        }

        let mut columns = &self.columns[self.index_row..];
        if columns.len() > area.height as usize {
            columns = &columns[..area.height as usize];
        }

        if show_head {
            let columns_width = columns.iter().map(|s| string_width(s)).max().unwrap_or(0);

            let will_use_space =
                padding_cell_l as usize + padding_cell_r as usize + columns_width + left_w as usize;
            if will_use_space > area.width as usize {
                return;
            }

            let columns = columns
                .iter()
                .map(|s| head_row_text(s, self.style_computer))
                .collect::<Vec<_>>();

            if !show_index {
                let x = area.x + left_w;
                left_w += render_vertical(buf, x, area.y, area.height, false, false, splitline_s);
            }

            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, 1, padding_cell_l);
            let x = area.x + left_w;
            left_w += render_column(buf, x, area.y, columns_width as u16, &columns);
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, 1, padding_cell_r);

            let layout_x = left_w - padding_cell_r - columns_width as u16;
            for (i, (text, _)) in columns.iter().enumerate() {
                state
                    .layout
                    .push(text, layout_x, area.y + i as u16, columns_width as u16, 1);
            }

            left_w += render_vertical(
                buf,
                area.x + left_w,
                area.y,
                area.height,
                false,
                false,
                splitline_s,
            );
        }

        let mut do_render_shift_column = false;

        state.count_rows = columns.len();
        state.count_columns = 0;

        for col in self.index_column..self.data.len() {
            let mut column =
                self.data[col][self.index_row..self.index_row + columns.len()].to_vec();
            let column_width = calculate_column_width(&column);
            if column_width > u16::MAX as usize {
                break;
            }

            let column_width = column_width as u16;
            let available = area.width - left_w;
            let is_last = col + 1 == self.data.len();
            let pad = padding_cell_l + padding_cell_r;
            let (column_width, ok, shift) =
                truncate_column_width(available, 1, column_width, pad, is_last, &mut column, None);

            if shift {
                do_render_shift_column = true;
            }

            if column_width == 0 && !ok {
                break;
            }

            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_cell_l);
            let x = area.x + left_w;
            left_w += render_column(buf, x, area.y, column_width, &column);
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_cell_r);

            {
                for (row, (text, _)) in column.iter().enumerate() {
                    let x = left_w - padding_cell_r - column_width;
                    let y = area.y + row as u16;
                    state.layout.push(text, x, y, column_width, 1);
                }

                state.count_columns += 1;
            }

            if do_render_shift_column {
                break;
            }
        }

        if do_render_shift_column {
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_cell_l);
            let x = area.x + left_w;
            left_w += render_shift_column(buf, x, area.y, area.height, shift_column_s);
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_cell_r);
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
            let i = 1 + row as usize + self.start;
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
    buf: &mut ratatui::buffer::Buffer,
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

fn render_column(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    available_width: u16,
    rows: &[NuText],
) -> u16 {
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

fn head_row_text(head: &str, style_computer: &StyleComputer) -> NuText {
    (
        String::from(head),
        TextStyle::with_style(
            Alignment::Center,
            style_computer.compute("header", &Value::string(head, nu_protocol::Span::unknown())),
        ),
    )
}
