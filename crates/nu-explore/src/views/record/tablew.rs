use std::{borrow::Cow, cmp::max, collections::HashMap};

use nu_table::{string_width, Alignment, TextStyle};
use tui::{
    buffer::Buffer,
    layout::Rect,
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    nu_common::{truncate_str, NuStyle, NuStyleTable, NuText},
    pager::{nu_style_to_tui, text_style_to_tui_style},
    views::ElementInfo,
};

use super::Layout;

#[derive(Debug, Clone)]
pub struct TableW<'a> {
    columns: Cow<'a, [String]>,
    data: Cow<'a, [Vec<NuText>]>,
    index_row: usize,
    index_column: usize,
    style: TableStyle,
    head_position: Orientation,
    color_hm: &'a NuStyleTable,
}

#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TableStyle {
    pub splitline_style: NuStyle,
    pub show_index: bool,
    pub show_header: bool,
    pub header_top: bool,
    pub header_bottom: bool,
    pub shift_line: bool,
    pub index_line: bool,
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
        color_hm: &'a NuStyleTable,
        index_row: usize,
        index_column: usize,
        style: TableStyle,
        head_position: Orientation,
    ) -> Self {
        Self {
            columns: columns.into(),
            data: data.into(),
            index_row,
            index_column,
            style,
            head_position,
            color_hm,
        }
    }
}

#[derive(Debug, Default)]
pub struct TableWState {
    pub layout: Layout,
    pub count_rows: usize,
    pub count_columns: usize,
    pub data_index: HashMap<(usize, usize), ElementInfo>,
}

impl StatefulWidget for TableW<'_> {
    type State = TableWState;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        let is_horizontal = matches!(self.head_position, Orientation::Top | Orientation::Bottom);
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

        let shift_column_w = 1 + padding_cell_l + padding_cell_r;

        let show_index = self.style.show_index;
        let show_head = self.style.show_header;
        let splitline_s = self.style.splitline_style;

        let mut data_height = area.height;
        let mut data_y = area.y;
        let mut head_y = area.y;

        let is_head_top = matches!(self.head_position, Orientation::Top);
        let is_head_bottom = matches!(self.head_position, Orientation::Bottom);

        if show_head {
            if is_head_top {
                data_y += 1;
                data_height -= 1;

                if self.style.header_top {
                    data_y += 1;
                    data_height -= 1;
                    head_y += 1
                }

                if self.style.header_bottom {
                    data_y += 1;
                    data_height -= 1;
                }
            }

            if is_head_bottom {
                data_height -= 1;
                head_y = area.y + data_height;

                if self.style.header_top && self.style.header_bottom {
                    data_height -= 2;
                    head_y -= 1
                } else if self.style.header_top {
                    data_height -= 1;
                } else if self.style.header_bottom {
                    data_height -= 1;
                    head_y -= 1
                }
            }
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
            // fixme: color from config
            let top = self.style.header_top;
            let bottom = self.style.header_bottom;

            if top || bottom {
                if is_head_top {
                    render_header_borders(buf, area, 1, splitline_s, top, bottom);
                } else if is_head_bottom {
                    let area = Rect::new(area.x, area.y + data_height, area.width, area.height);
                    render_header_borders(buf, area, 1, splitline_s, top, bottom);
                }
            }
        }

        if show_index {
            let area = Rect::new(width, data_y, area.width, data_height);
            width += render_index(
                buf,
                area,
                self.color_hm,
                self.index_row,
                padding_index_l,
                padding_index_r,
            );

            if self.style.index_line {
                let head_t = show_head && is_head_top && self.style.header_bottom;
                let head_b = show_head && is_head_bottom && self.style.header_top;
                width +=
                    render_vertical(buf, width, data_y, data_height, head_t, head_b, splitline_s);
            }
        }

        let mut do_render_shift_column = false;
        state.count_rows = data.len();
        state.count_columns = 0;

        if width > area.width {
            return;
        }

        for (i, col) in (self.index_column..self.columns.len()).enumerate() {
            let mut head = String::from(&self.columns[col]);

            let mut column = create_column(data, col);

            let column_width = calculate_column_width(&column);
            let mut use_space = column_width as u16;

            if show_head {
                let head_width = string_width(&head);
                use_space = max(head_width as u16, use_space);
            }

            {
                let available = area.width - width;
                let column_space = use_space + padding_cell_l + padding_cell_r;

                let is_last = col + 1 == self.columns.len();
                let w = truncate_column_width(available, column_space, shift_column_w, is_last);

                if w == 0 {
                    break;
                } else if w <= shift_column_w {
                    do_render_shift_column = true;
                    break;
                } else if w < column_space {
                    use_space = w - (padding_cell_l + padding_cell_r);

                    truncate_list(&mut column, use_space as usize);
                    if show_head {
                        truncate_str(&mut head, use_space as usize);
                    }
                    if !is_last {
                        do_render_shift_column = true;
                    }
                }
            }

            if show_head {
                let header = &[head_row_text(&head, self.color_hm)];

                let mut w = width;
                w += render_space(buf, w, head_y, 1, padding_cell_l);
                w += render_column(buf, w, head_y, use_space, header);
                render_space(buf, w, head_y, 1, padding_cell_r);

                let x = w - padding_cell_r - use_space;
                state.layout.push(&header[0].0, x, head_y, use_space, 1);

                // it would be nice to add it so it would be available on search
                // state.state.data_index.insert((i, col), ElementInfo::new(text, x, data_y, use_space, 1));
            }

            width += render_space(buf, width, data_y, data_height, padding_cell_l);
            width += render_column(buf, width, data_y, use_space, &column);
            width += render_space(buf, width, data_y, data_height, padding_cell_r);

            for (row, (text, _)) in column.iter().enumerate() {
                let x = width - padding_cell_r - use_space;
                let y = data_y + row as u16;
                state.layout.push(text, x, y, use_space, 1);

                let e = ElementInfo::new(text, x, y, use_space, 1);
                state.data_index.insert((row, i), e);
            }

            state.count_columns += 1;

            if do_render_shift_column {
                break;
            }
        }

        if do_render_shift_column {
            // we actually want to show a shift only in header.
            //
            // render_shift_column(buf, used_width, head_offset, available_height);

            if show_head {
                width += render_space(buf, width, data_y, data_height, padding_cell_l);
                width += render_shift_column(buf, width, head_y, 1, splitline_s);
                width += render_space(buf, width, data_y, data_height, padding_cell_r);
            }
        }

        if self.style.shift_line && width < area.width {
            let head_t = show_head && is_head_top && self.style.header_bottom;
            let head_b = show_head && is_head_bottom && self.style.header_top;
            width += render_vertical(buf, width, data_y, data_height, head_t, head_b, splitline_s);
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
        let padding_cell_l = self.style.padding_column_left as u16;
        let padding_cell_r = self.style.padding_column_right as u16;
        let padding_index_l = self.style.padding_index_left as u16;
        let padding_index_r = self.style.padding_index_right as u16;

        let shift_column_w = 1 + padding_cell_l + padding_cell_r;

        if area.width == 0 || area.height == 0 {
            return;
        }

        let show_index = self.style.show_index;
        let show_head = self.style.show_header;
        let splitline_s = self.style.splitline_style;

        let is_head_left = matches!(self.head_position, Orientation::Left);
        let is_head_right = matches!(self.head_position, Orientation::Right);

        let mut left_w = 0;
        let mut right_w = 0;

        if show_index {
            let area = Rect::new(area.x, area.y, area.width, area.height);
            left_w += render_index(
                buf,
                area,
                self.color_hm,
                self.index_row,
                padding_index_l,
                padding_index_r,
            );

            if self.style.index_line {
                let x = area.x + left_w;
                left_w += render_vertical(buf, x, area.y, area.height, false, false, splitline_s);
            }
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
                .map(|s| head_row_text(s, self.color_hm))
                .collect::<Vec<_>>();

            if is_head_left {
                let have_index_line = show_index && self.style.index_line;
                if !have_index_line && self.style.header_top {
                    let x = area.x + left_w;
                    left_w +=
                        render_vertical(buf, x, area.y, area.height, false, false, splitline_s);
                }

                let x = area.x + left_w;
                left_w += render_space(buf, x, area.y, 1, padding_cell_l);
                let x = area.x + left_w;
                left_w += render_column(buf, x, area.y, columns_width as u16, &columns);
                let x = area.x + left_w;
                left_w += render_space(buf, x, area.y, 1, padding_cell_r);

                if self.style.header_bottom {
                    let x = area.x + left_w;
                    left_w +=
                        render_vertical(buf, x, area.y, area.height, false, false, splitline_s);
                }
            } else if is_head_right {
                if self.style.header_bottom {
                    let x = area.x + area.width - 1;
                    right_w +=
                        render_vertical(buf, x, area.y, area.height, false, false, splitline_s);
                }

                let x = area.x + area.width - right_w - padding_cell_r;
                right_w += render_space(buf, x, area.y, 1, padding_cell_r);
                let x = area.x + area.width - right_w - columns_width as u16;
                right_w += render_column(buf, x, area.y, columns_width as u16, &columns);
                let x = area.x + area.width - right_w - padding_cell_l;
                right_w += render_space(buf, x, area.y, 1, padding_cell_l);

                if self.style.header_top {
                    let x = area.x + area.width - right_w - 1;
                    right_w +=
                        render_vertical(buf, x, area.y, area.height, false, false, splitline_s);
                }
            }
        }

        let mut do_render_shift_column = false;

        state.count_rows = columns.len();
        state.count_columns = 0;

        for (i, col) in (self.index_column..self.data.len()).enumerate() {
            let mut column =
                self.data[col][self.index_row..self.index_row + columns.len()].to_vec();
            let column_width = calculate_column_width(&column);
            if column_width > u16::MAX as usize {
                break;
            }

            let mut column_width = column_width as u16;

            let available = area.width - left_w - right_w;
            let column_w = column_width + padding_cell_l + padding_cell_r;
            let is_last = col + 1 == self.data.len();
            let w = truncate_column_width(available, column_w, shift_column_w, is_last);

            if w == 0 {
                break;
            } else if w <= shift_column_w {
                do_render_shift_column = true;
                break;
            } else if w < column_w {
                column_width = w - (padding_cell_l + padding_cell_r);
                truncate_list(&mut column, column_width as usize);

                if !is_last {
                    do_render_shift_column = true;
                }
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

                    let e = ElementInfo::new(text, x, y, column_width, 1);
                    state.data_index.insert((row, i), e);
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
            left_w += render_shift_column(buf, x, area.y, area.height, splitline_s);
            let x = area.x + left_w;
            left_w += render_space(buf, x, area.y, area.height, padding_cell_r);
        }

        _ = left_w;

        // let rest = area.width.saturating_sub(left_w + right_w);
        // if rest > 0 {
        //     render_space(buf, left_w, area.y, area.height, rest);
        // }
    }
}

struct IndexColumn<'a> {
    color_hm: &'a NuStyleTable,
    start: usize,
}

impl<'a> IndexColumn<'a> {
    fn new(color_hm: &'a NuStyleTable, start: usize) -> Self {
        Self { color_hm, start }
    }

    fn estimate_width(&self, height: u16) -> usize {
        let last_row = self.start + height as usize;
        last_row.to_string().len()
    }
}

impl Widget for IndexColumn<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = nu_style_to_tui(self.color_hm["row_index"]);

        for row in 0..area.height {
            let i = 1 + row as usize + self.start;
            let text = i.to_string();

            let p = Paragraph::new(text)
                .style(style)
                .alignment(tui::layout::Alignment::Right);
            let area = Rect::new(area.x, area.y + row, area.width, 1);

            p.render(area, buf);
        }
    }
}

fn render_header_borders(
    buf: &mut Buffer,
    area: Rect,
    span: u16,
    style: NuStyle,
    top: bool,
    bottom: bool,
) -> (u16, u16) {
    let mut i = 0;
    let mut borders = Borders::NONE;
    if top {
        borders |= Borders::TOP;
        i += 1;
    }

    if bottom {
        borders |= Borders::BOTTOM;
        i += 1;
    }

    let block = Block::default()
        .borders(borders)
        .border_style(nu_style_to_tui(style));
    let height = i + span;
    let area = Rect::new(area.x, area.y, area.width, height);
    block.render(area, buf);

    // y pos of header text and next line
    (height.saturating_sub(2), height)
}

fn render_index(
    buf: &mut Buffer,
    area: Rect,
    color_hm: &NuStyleTable,
    start_index: usize,
    padding_left: u16,
    padding_right: u16,
) -> u16 {
    let mut width = render_space(buf, area.x, area.y, area.height, padding_left);

    let index = IndexColumn::new(color_hm, start_index);
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
        column[row] = value.clone();
    }

    column
}

fn repeat_vertical(
    buf: &mut tui::buffer::Buffer,
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
        buf.set_span(x_offset, y_offset + row as u16, &span, width);
    }
}

fn truncate_column_width(
    available_w: u16,
    column_w: u16,
    min_column_w: u16,
    is_column_last: bool,
) -> u16 {
    if available_w < min_column_w {
        return 0;
    }

    if available_w < column_w {
        if is_column_last {
            return available_w;
        }

        if available_w > min_column_w + min_column_w {
            return available_w - min_column_w;
        }

        return min_column_w;
    }

    // check whether we will have a enough space just in case...
    let is_enough_space_for_shift = available_w > column_w + min_column_w;
    if !is_column_last && !is_enough_space_for_shift {
        return min_column_w;
    }

    column_w
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
    buf: &mut tui::buffer::Buffer,
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
    strip_ansi_escapes::strip(text)
        .ok()
        .and_then(|s| String::from_utf8(s).ok())
        .unwrap_or_else(|| text.to_owned())
}

fn head_row_text(head: &str, color_hm: &NuStyleTable) -> NuText {
    (
        String::from(head),
        TextStyle {
            alignment: Alignment::Center,
            color_style: Some(color_hm["header"]),
        },
    )
}
