use std::{
    borrow::Cow,
    cmp::{max, min},
    collections::HashMap,
};

use crossterm::event::KeyEvent;
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use nu_table::{string_width, Alignment, TextStyle};
use reedline::KeyCode;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::viewers::scroll::{
    collect_input, collect_pipeline,
    pager::{
        make_styled_string, nu_ansi_color_to_tui_color, nu_style_to_tui, run_nu_command,
        text_style_to_tui_style, Frame, NuConfig, NuSpan, NuStyle, NuStyleTable, NuText, Position,
        Report, Severentity, StyleConfig, TableConfig, Transition, ViewConfig, ViewInfo,
    },
    views::ElementInfo,
};

use super::{Layout, View};

#[derive(Debug, Clone)]
pub struct RecordView<'a> {
    layer_stack: Vec<RecordLayer<'a>>,
    mode: UIMode,
    cfg: TableConfig,
    cursor: Position,
    state: RecordViewState,
}

impl<'a> RecordView<'a> {
    pub fn new(
        columns: impl Into<Cow<'a, [String]>>,
        records: impl Into<Cow<'a, [Vec<Value>]>>,
        table_cfg: TableConfig,
    ) -> Self {
        Self {
            layer_stack: vec![RecordLayer::new(columns, records)],
            mode: UIMode::View,
            cursor: Position::new(0, 0),
            cfg: table_cfg,
            state: RecordViewState::default(),
        }
    }

    pub fn reverse(&mut self, width: u16, height: u16) {
        let page_size = estimate_page_size(Rect::new(0, 0, width, height), self.cfg.show_head);
        state_reverse_data(self, page_size as usize);
    }

    // todo: rename to get_layer
    fn get_layer_last(&self) -> &RecordLayer<'a> {
        self.layer_stack.last().unwrap()
    }

    fn get_layer_last_mut(&mut self) -> &mut RecordLayer<'a> {
        self.layer_stack.last_mut().unwrap()
    }

    fn create_tablew<'b>(&self, layer: &'b RecordLayer, view_cfg: &'b ViewConfig) -> TableW<'b> {
        let data = convert_records_to_string(&layer.records, view_cfg.config, view_cfg.color_hm);

        TableW::new(
            layer.columns.as_ref(),
            data,
            self.cfg.show_index,
            self.cfg.show_head,
            view_cfg.theme.split_line,
            view_cfg.color_hm,
            layer.index_row,
            layer.index_column,
        )
    }
}

impl View for RecordView<'_> {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: &ViewConfig, layout: &mut Layout) {
        let layer = self.get_layer_last();
        let table = self.create_tablew(layer, cfg);

        let mut table_layout = TableWState::default();
        f.render_stateful_widget(table, area, &mut table_layout);

        *layout = table_layout.layout;
        self.state = RecordViewState {
            count_rows: table_layout.count_rows,
            count_columns: table_layout.count_columns,
            data_index: table_layout.data_index,
        };

        if self.mode == UIMode::Cursor {
            let cursor = get_cursor(self);
            highlight_cell(f, area, &self.state, cursor, cfg.theme);
        }
    }

    fn handle_input(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: &Layout,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Option<Transition> {
        let result = match self.mode {
            UIMode::View => handle_key_event_view_mode(self, &key),
            UIMode::Cursor => {
                // we handle a situation where we got resized and the old cursor is no longer valid
                self.cursor = get_cursor(self);

                handle_key_event_cursor_mode(self, &key)
            }
        };

        if matches!(&result, Some(Transition::Ok) | Some(Transition::Cmd(..))) {
            // update status bar
            let report =
                create_records_report(self.get_layer_last(), &self.state, self.mode, self.cursor);

            info.status = Some(report);
        }

        result
    }

    fn collect_data(&self) -> Vec<NuText> {
        let data = convert_records_to_string(
            &self.get_layer_last().records,
            &NuConfig::default(),
            &HashMap::default(),
        );

        data.iter().flatten().cloned().collect()
    }

    fn show_data(&mut self, pos: usize) -> bool {
        let data = &self.get_layer_last().records;

        let mut i = 0;
        for (row, cells) in data.iter().enumerate() {
            if pos > i + cells.len() {
                i += cells.len();
                continue;
            }

            for (column, _) in cells.iter().enumerate() {
                if i == pos {
                    let layer = self.get_layer_last_mut();
                    layer.index_column = column;
                    layer.index_row = row;

                    return true;
                }

                i += 1;
            }
        }

        false
    }

    fn exit(&mut self) -> Option<Value> {
        Some(build_last_value(self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum UIMode {
    Cursor,
    View,
}

#[derive(Debug, Clone)]
struct RecordLayer<'a> {
    columns: Cow<'a, [String]>,
    records: Cow<'a, [Vec<Value>]>,
    index_row: usize,
    index_column: usize,
    name: Option<String>,
}

impl<'a> RecordLayer<'a> {
    fn new(
        columns: impl Into<Cow<'a, [String]>>,
        records: impl Into<Cow<'a, [Vec<Value>]>>,
    ) -> Self {
        Self {
            columns: columns.into(),
            records: records.into(),
            index_row: 0,
            index_column: 0,
            name: None,
        }
    }

    fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    fn count_rows(&self) -> usize {
        self.records.len()
    }

    fn count_columns(&self) -> usize {
        self.columns.len()
    }

    fn get_current_value(&self, Position { x, y }: Position) -> Value {
        let current_row = y as usize + self.index_row;
        let current_column = x as usize + self.index_column;

        let row = self.records[current_row].clone();
        row[current_column].clone()
    }

    fn get_current_header(&self, Position { x, .. }: Position) -> Option<String> {
        let col = x as usize + self.index_column;

        self.columns.get(col).map(|header| header.to_string())
    }
}

#[derive(Debug, Default, Clone)]
pub struct RecordViewState {
    count_rows: usize,
    count_columns: usize,
    data_index: HashMap<(usize, usize), ElementInfo>,
}

fn handle_key_event_view_mode(view: &mut RecordView, key: &KeyEvent) -> Option<Transition> {
    match key.code {
        KeyCode::Esc => {
            if view.layer_stack.len() > 1 {
                view.layer_stack.pop();
                Some(Transition::Ok)
            } else {
                Some(Transition::Exit)
            }
        }
        KeyCode::Char('i') => {
            view.mode = UIMode::Cursor;
            view.cursor = Position::default();

            Some(Transition::Ok)
        }
        KeyCode::Up => {
            let layer = view.get_layer_last_mut();
            layer.index_row = layer.index_row.saturating_sub(1);

            Some(Transition::Ok)
        }
        KeyCode::Down => {
            let layer = view.get_layer_last_mut();
            let max_index = layer.count_rows().saturating_sub(1);
            layer.index_row = min(layer.index_row + 1, max_index);

            Some(Transition::Ok)
        }
        KeyCode::Left => {
            let layer = view.get_layer_last_mut();
            layer.index_column = layer.index_column.saturating_sub(1);

            Some(Transition::Ok)
        }
        KeyCode::Right => {
            let layer = view.get_layer_last_mut();
            let max_index = layer.count_columns().saturating_sub(1);
            layer.index_column = min(layer.index_column + 1, max_index);

            Some(Transition::Ok)
        }
        KeyCode::PageUp => {
            let count_rows = view.state.count_rows;
            let layer = view.get_layer_last_mut();
            layer.index_row = layer.index_row.saturating_sub(count_rows as usize);

            Some(Transition::Ok)
        }
        KeyCode::PageDown => {
            let count_rows = view.state.count_rows;
            let layer = view.get_layer_last_mut();
            let max_index = layer.count_rows().saturating_sub(1);
            layer.index_row = min(layer.index_row + count_rows as usize, max_index);

            Some(Transition::Ok)
        }
        _ => None,
    }
}

fn handle_key_event_cursor_mode(view: &mut RecordView, key: &KeyEvent) -> Option<Transition> {
    match key.code {
        KeyCode::Esc => {
            view.mode = UIMode::View;
            view.cursor = Position::default();

            Some(Transition::Ok)
        }
        KeyCode::Up => {
            if view.cursor.y == 0 {
                let layer = view.get_layer_last_mut();
                layer.index_row = layer.index_row.saturating_sub(1);
            } else {
                view.cursor.y -= 1
            }

            Some(Transition::Ok)
        }
        KeyCode::Down => {
            let cursor = view.cursor;
            let showed_rows = view.state.count_rows;
            let layer = view.get_layer_last_mut();

            let total_rows = layer.count_rows();
            let row_index = layer.index_row + cursor.y as usize + 1;

            if row_index < total_rows {
                if cursor.y as usize + 1 == showed_rows {
                    layer.index_row += 1;
                } else {
                    view.cursor.y += 1;
                }
            }

            Some(Transition::Ok)
        }
        KeyCode::Left => {
            let cursor = view.cursor;
            let layer = view.get_layer_last_mut();

            if cursor.x == 0 {
                layer.index_column = layer.index_column.saturating_sub(1);
            } else {
                view.cursor.x -= 1
            }

            Some(Transition::Ok)
        }
        KeyCode::Right => {
            let cursor = view.cursor;
            let showed_columns = view.state.count_columns;
            let layer = view.get_layer_last_mut();

            let total_columns = layer.count_columns();
            let column_index = layer.index_column + cursor.x as usize + 1;

            if column_index < total_columns {
                if cursor.x as usize + 1 == showed_columns {
                    layer.index_column += 1;
                } else {
                    view.cursor.x += 1;
                }
            }

            Some(Transition::Ok)
        }
        KeyCode::Enter => {
            push_current_value_to_layer(view);
            Some(Transition::Ok)
        }
        _ => None,
    }
}

struct TableW<'a> {
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
    fn new(
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
struct TableWState {
    layout: Layout,
    count_rows: usize,
    count_columns: usize,
    data_index: HashMap<(usize, usize), ElementInfo>,
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
            render_header_borders(buf, area, 0, 1);
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

fn push_current_value_to_layer(view: &mut RecordView) {
    let layer = view.get_layer_last();

    let value = layer.get_current_value(view.cursor);
    let header = layer.get_current_header(view.cursor);

    let (columns, values) = collect_input(value);

    let mut next_layer = RecordLayer::new(columns, values);
    if let Some(header) = header {
        next_layer.set_name(header);
    }

    view.layer_stack.push(next_layer);

    view.mode = UIMode::View;
    view.cursor = Position::default();
}

fn estimate_page_size(area: Rect, show_head: bool) -> u16 {
    let mut available_height = area.height;
    available_height -= 3; // status_bar

    if show_head {
        available_height -= 3; // head
    }

    available_height
}

fn state_reverse_data(state: &mut RecordView<'_>, page_size: usize) {
    let layer = state.get_layer_last_mut();
    let count_rows = layer.records.len();
    if count_rows > page_size as usize {
        layer.index_row = count_rows - page_size as usize;
    }
}

fn convert_records_to_string(
    records: &[Vec<Value>],
    cfg: &NuConfig,
    color_hm: &NuStyleTable,
) -> Vec<Vec<NuText>> {
    records
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| {
                    let text = value.clone().into_abbreviated_string(cfg);
                    let tp = value.get_type().to_string();
                    let float_precision = cfg.float_precision as usize;

                    make_styled_string(text, &tp, 0, false, color_hm, float_precision)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

fn highlight_cell(
    f: &mut Frame,
    area: Rect,
    state: &RecordViewState,
    cursor: Position,
    theme: &StyleConfig,
) {
    let Position { x: column, y: row } = cursor;

    let info = state.data_index.get(&(row as usize, column as usize));

    if let Some(info) = info {
        if let Some(style) = theme.selected_column {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(info.area.x, area.y, info.area.width, area.height);
            f.render_widget(hightlight_block.clone(), area);
        }

        if let Some(style) = theme.selected_row {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(area.x, info.area.y, area.width, 1);
            f.render_widget(hightlight_block.clone(), area);
        }

        if let Some(style) = theme.selected_cell {
            let hightlight_block = Block::default().style(nu_style_to_tui(style));
            let area = Rect::new(info.area.x, info.area.y, info.area.width, 1);
            f.render_widget(hightlight_block.clone(), area);
        }

        if theme.show_cursow {
            f.set_cursor(info.area.x, info.area.y);
        }
    }
}

fn get_cursor(v: &RecordView<'_>) -> Position {
    let count_rows = v.state.count_rows as u16;
    let count_columns = v.state.count_columns as u16;

    let mut cursor = v.cursor;
    cursor.y = min(cursor.y, count_rows.saturating_sub(1) as u16);
    cursor.x = min(cursor.x, count_columns.saturating_sub(1) as u16);

    cursor
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

fn render_space(buf: &mut Buffer, x: u16, y: u16, height: u16, padding: u16) -> u16 {
    repeat_vertical(buf, x, y, padding, height, ' ', TextStyle::default());
    padding
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

fn render_header_borders(buf: &mut Buffer, area: Rect, y: u16, span: u16) -> (u16, u16) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(64, 64, 64)));
    let height = 2 + span;
    let area = Rect::new(area.x, area.y + y, area.width, height);
    block.render(area, buf);
    // y pos of header text and next line
    (height.saturating_sub(2), height)
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

fn build_last_value(v: &RecordView) -> Value {
    if v.mode == UIMode::Cursor {
        peak_current_value(v)
    } else if v.get_layer_last().count_rows() < 2 {
        build_table_as_record(v)
    } else {
        build_table_as_list(v)
    }
}

fn peak_current_value(v: &RecordView) -> Value {
    let layer = v.get_layer_last();
    let Position { x: column, y: row } = v.cursor;
    let row = row as usize + layer.index_row;
    let column = column as usize + layer.index_column;
    let value = &layer.records[row][column];
    value.clone()
}

fn build_table_as_list(v: &RecordView) -> Value {
    let layer = v.get_layer_last();

    let headers = layer.columns.to_vec();
    let vals = layer
        .records
        .iter()
        .cloned()
        .map(|vals| Value::Record {
            cols: headers.clone(),
            vals,
            span: NuSpan::unknown(),
        })
        .collect();

    Value::List {
        vals,
        span: NuSpan::unknown(),
    }
}

fn build_table_as_record(v: &RecordView) -> Value {
    let layer = v.get_layer_last();

    let cols = layer.columns.to_vec();
    let vals = layer.records.get(0).map_or(Vec::new(), |row| row.clone());

    Value::Record {
        cols,
        vals,
        span: NuSpan::unknown(),
    }
}

fn create_records_report(
    layer: &RecordLayer,
    state: &RecordViewState,
    mode: UIMode,
    cursor: Position,
) -> Report {
    let seen_rows = layer.index_row + state.count_rows;
    let seen_rows = min(seen_rows, layer.count_rows());
    let percent_rows = get_percentage(seen_rows, layer.count_rows());
    let covered_percent = match percent_rows {
        100 => String::from("All"),
        _ if layer.index_row == 0 => String::from("Top"),
        value => format!("{}%", value),
    };
    let title = if let Some(name) = &layer.name {
        name.clone()
    } else {
        String::new()
    };
    let cursor = {
        if mode == UIMode::Cursor {
            let row = layer.index_row + cursor.y as usize;
            let column = layer.index_column + cursor.x as usize;
            format!("{},{}", row, column)
        } else {
            format!("{},{}", layer.index_row, layer.index_column)
        }
    };

    Report {
        message: title,
        context: covered_percent,
        context2: cursor,
        level: Severentity::Info,
    }
}

fn get_percentage(value: usize, max: usize) -> usize {
    debug_assert!(value <= max, "{:?} {:?}", value, max);

    ((value as f32 / max as f32) * 100.0).floor() as usize
}
