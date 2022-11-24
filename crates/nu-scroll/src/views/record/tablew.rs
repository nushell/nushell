use std::{borrow::Cow, cmp::max, collections::HashMap};

use nu_table::{string_width, Alignment, TextStyle};
use tui::{
    buffer::Buffer,
    layout::Rect,
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    nu_common::{NuStyle, NuStyleTable, NuText},
    pager::{nu_style_to_tui, text_style_to_tui_style},
    views::ElementInfo,
};

use super::Layout;

pub struct TableW<'a> {
    columns: Cow<'a, [String]>,
    data: Cow<'a, [Vec<NuText>]>,
    show_index: bool,
    show_header: bool,
    index_row: usize,
    index_column: usize,
    splitline_style: NuStyle,
    color_hm: &'a NuStyleTable,
}

impl<'a> TableW<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        columns: impl Into<Cow<'a, [String]>>,
        data: impl Into<Cow<'a, [Vec<NuText>]>>,
        show_index: bool,
        show_header: bool,
        splitline_style: NuStyle,
        color_hm: &'a NuStyleTable,
        index_row: usize,
        index_column: usize,
    ) -> Self {
        Self {
            columns: columns.into(),
            data: data.into(),
            color_hm,
            show_index,
            show_header,
            splitline_style,
            index_row,
            index_column,
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
        const CELL_PADDING_LEFT: u16 = 2;
        const CELL_PADDING_RIGHT: u16 = 2;

        let show_index = self.show_index;
        let show_head = self.show_header;

        let mut data_y = area.y;
        if show_head {
            data_y += 3;
        }

        let head_y = area.y + 1;

        if area.width == 0 || area.height == 0 {
            return;
        }

        let mut data_height = area.height;
        if show_head {
            data_height -= 3;
        }

        let mut width = area.x;

        let mut data = &self.data[self.index_row..];
        if data.len() > data_height as usize {
            data = &data[..data_height as usize];
        }

        // header lines
        if show_head {
            // fixme: color from config
            render_header_borders(buf, area, 0, 1, self.splitline_style);
        }

        if show_index {
            let area = Rect::new(width, data_y, area.width, data_height);
            width += render_index(buf, area, self.color_hm, self.index_row);
            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                self.splitline_style,
            );
        }

        let mut do_render_split_line = true;
        let mut do_render_shift_column = false;

        state.count_rows = data.len();
        state.count_columns = 0;

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
                let available_space = area.width - width;
                let head = show_head.then(|| &mut head);
                let control = truncate_column(
                    &mut column,
                    head,
                    available_space,
                    col + 1 == self.columns.len(),
                    PrintControl {
                        break_everything: false,
                        print_shift_column: false,
                        print_split_line: true,
                        width: use_space,
                    },
                );

                use_space = control.width;
                do_render_split_line = control.print_split_line;
                do_render_shift_column = control.print_shift_column;

                if control.break_everything {
                    break;
                }
            }

            if show_head {
                let header = &[head_row_text(&head, self.color_hm)];

                let mut w = width;
                w += render_space(buf, w, head_y, 1, CELL_PADDING_LEFT);
                w += render_column(buf, w, head_y, use_space, header);
                render_space(buf, w, head_y, 1, CELL_PADDING_RIGHT);

                let x = w - CELL_PADDING_RIGHT - use_space;
                state.layout.push(&header[0].0, x, head_y, use_space, 1);

                // it would be nice to add it so it would be available on search
                // state.state.data_index.insert((i, col), ElementInfo::new(text, x, data_y, use_space, 1));
            }

            width += render_space(buf, width, data_y, data_height, CELL_PADDING_LEFT);
            width += render_column(buf, width, data_y, use_space, &column);
            width += render_space(buf, width, data_y, data_height, CELL_PADDING_RIGHT);

            for (row, (text, _)) in column.iter().enumerate() {
                let x = width - CELL_PADDING_RIGHT - use_space;
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
                width += render_space(buf, width, data_y, data_height, CELL_PADDING_LEFT);
                width += render_shift_column(buf, width, head_y, 1, self.splitline_style);
                width += render_space(buf, width, data_y, data_height, CELL_PADDING_RIGHT);
            }
        }

        if do_render_split_line {
            width += render_vertical(
                buf,
                width,
                data_y,
                data_height,
                show_head,
                self.splitline_style,
            );
        }

        // we try out best to cleanup the rest of the space cause it could be meassed.
        let rest = area.width.saturating_sub(width);
        if rest > 0 {
            render_space(buf, width, data_y, data_height, rest);
            if show_head {
                render_space(buf, width, head_y, 1, rest);
            }
        }
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
    y: u16,
    span: u16,
    style: NuStyle,
) -> (u16, u16) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(nu_style_to_tui(style));
    let height = 2 + span;
    let area = Rect::new(area.x, area.y + y, area.width, height);
    block.render(area, buf);
    // y pos of header text and next line
    (height.saturating_sub(2), height)
}

fn render_index(buf: &mut Buffer, area: Rect, color_hm: &NuStyleTable, start_index: usize) -> u16 {
    const PADDING_LEFT: u16 = 2;
    const PADDING_RIGHT: u16 = 1;

    let mut width = render_space(buf, area.x, area.y, area.height, PADDING_LEFT);

    let index = IndexColumn::new(color_hm, start_index);
    let w = index.estimate_width(area.height) as u16;
    let area = Rect::new(area.x + width, area.y, w, area.height);

    index.render(area, buf);

    width += w;
    width += render_space(buf, area.x + width, area.y, area.height, PADDING_RIGHT);

    width
}

fn render_vertical(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    show_header: bool,
    style: NuStyle,
) -> u16 {
    render_vertical_split(buf, x, y, height, style);

    if show_header && y > 0 {
        render_top_connector(buf, x, y - 1, style);
    }

    // render_bottom_connector(buf, x, height + y);

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

#[derive(Debug, Default, Copy, Clone)]
struct PrintControl {
    width: u16,
    break_everything: bool,
    print_split_line: bool,
    print_shift_column: bool,
}

fn truncate_column(
    column: &mut [NuText],
    head: Option<&mut String>,
    available_space: u16,
    is_column_last: bool,
    mut control: PrintControl,
) -> PrintControl {
    const CELL_PADDING_LEFT: u16 = 2;
    const CELL_PADDING_RIGHT: u16 = 2;
    const VERTICAL_LINE_WIDTH: u16 = 1;
    const CELL_MIN_WIDTH: u16 = 1;

    let min_space_cell = CELL_PADDING_LEFT + CELL_PADDING_RIGHT + CELL_MIN_WIDTH;
    let min_space = min_space_cell + VERTICAL_LINE_WIDTH;
    if available_space < min_space {
        // if there's not enough space at all just return; doing our best
        if available_space < VERTICAL_LINE_WIDTH {
            control.print_split_line = false;
        }

        control.break_everything = true;
        return control;
    }

    let column_taking_space =
        control.width + CELL_PADDING_LEFT + CELL_PADDING_RIGHT + VERTICAL_LINE_WIDTH;
    let is_enough_space = available_space > column_taking_space;
    if !is_enough_space {
        if is_column_last {
            // we can do nothing about it we need to truncate.
            // we assume that there's always at least space for padding and 1 symbol. (5 chars)

            let width = available_space
                .saturating_sub(CELL_PADDING_LEFT + CELL_PADDING_RIGHT + VERTICAL_LINE_WIDTH);
            if width == 0 {
                control.break_everything = true;
                return control;
            }

            if let Some(head) = head {
                truncate_str(head, width as usize);
            }

            truncate_list(column, width as usize);

            control.width = width;
        } else {
            let min_space_2cells = min_space + min_space_cell;
            if available_space > min_space_2cells {
                let width = available_space.saturating_sub(min_space_2cells);
                if width == 0 {
                    control.break_everything = true;
                    return control;
                }

                truncate_list(column, width as usize);

                if let Some(head) = head {
                    truncate_str(head, width as usize);
                }

                control.width = width;
                control.print_shift_column = true;
            } else {
                control.break_everything = true;
                control.print_shift_column = true;
            }
        }
    } else if !is_column_last {
        // even though we can safely render current column,
        // we need to check whether there's enough space for AT LEAST a shift column
        // (2 padding + 2 padding + 1 a char)
        let left_space = available_space - column_taking_space;
        if left_space < min_space {
            let need_space = min_space_cell - left_space;
            let min_left_width = 1;
            let is_column_big_enough = control.width > need_space + min_left_width;

            if is_column_big_enough {
                let width = control.width.saturating_sub(need_space);
                if width == 0 {
                    control.break_everything = true;
                    return control;
                }

                truncate_list(column, width as usize);

                if let Some(head) = head {
                    truncate_str(head, width as usize);
                }

                control.width = width;
                control.print_shift_column = true;
            }
        }
    }

    control
}

fn truncate_list(list: &mut [NuText], width: usize) {
    for (text, _) in list {
        truncate_str(text, width);
    }
}

fn truncate_str(text: &mut String, width: usize) {
    if width == 0 {
        text.clear();
    } else {
        *text = nu_table::string_truncate(text, width - 1);
        text.push('…');
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
        let text = strip_string(text);
        let style = text_style_to_tui_style(*style);
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
