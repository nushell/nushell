mod tablew;

use std::{borrow::Cow, cmp::min, collections::HashMap};

use crossterm::event::KeyEvent;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use reedline::KeyCode;
use tui::{layout::Rect, widgets::Block};

use crate::{
    nu_common::{collect_input, NuConfig, NuSpan, NuStyleTable, NuText},
    pager::{
        make_styled_string, nu_style_to_tui, Frame, Position, Report, Severentity, StyleConfig,
        TableConfig, Transition, ViewConfig, ViewInfo,
    },
    views::ElementInfo,
};

use self::tablew::{TableW, TableWState};

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
    pub fn get_layer_last(&self) -> &RecordLayer<'a> {
        self.layer_stack
            .last()
            .expect("we guarantee that 1 entry is always in a list")
    }

    pub fn get_layer_last_mut(&mut self) -> &mut RecordLayer<'a> {
        self.layer_stack
            .last_mut()
            .expect("we guarantee that 1 entry is always in a list")
    }

    fn create_tablew<'b>(&self, layer: &'b RecordLayer, view_cfg: &'b ViewConfig) -> TableW<'b> {
        let data = convert_records_to_string(&layer.records, view_cfg.config, view_cfg.color_hm);

        let style = tablew::TableStyle {
            show_index: self.cfg.show_index,
            show_header: self.cfg.show_head,
            splitline_style: view_cfg.theme.split_line,
            header_bottom: view_cfg.theme.split_lines.header_bottom,
            header_top: view_cfg.theme.split_lines.header_top,
            index_line: view_cfg.theme.split_lines.index_line,
            shift_line: view_cfg.theme.split_lines.shift_line,
        };

        let headers = layer.columns.as_ref();
        let color_hm = view_cfg.color_hm;
        let i_row = layer.index_row;
        let i_column = layer.index_column;

        TableW::new(headers, data, color_hm, i_row, i_column, style)
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

        if matches!(&result, Some(Transition::Ok) | Some(Transition::Cmd { .. })) {
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
pub struct RecordLayer<'a> {
    columns: Cow<'a, [String]>,
    records: Cow<'a, [Vec<Value>]>,
    pub(crate) index_row: usize,
    pub(crate) index_column: usize,
    name: Option<String>,
    was_transposed: bool,
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
            was_transposed: false,
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
        KeyCode::Char('t') => {
            let layer = view.get_layer_last_mut();
            layer.index_column = 0;
            layer.index_row = 0;

            transpose_table(layer);

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
            let next_layer = get_peeked_layer(view);
            push_layer(view, next_layer);
            Some(Transition::Ok)
        }
        _ => None,
    }
}

fn get_peeked_layer(view: &RecordView) -> RecordLayer<'static> {
    let layer = view.get_layer_last();

    let value = layer.get_current_value(view.cursor);

    let (columns, values) = collect_input(value);

    RecordLayer::new(columns, values)
}

fn push_layer(view: &mut RecordView<'_>, mut next_layer: RecordLayer<'static>) {
    let layer = view.get_layer_last();
    let header = layer.get_current_header(view.cursor);

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

fn transpose_table(layer: &mut RecordLayer<'_>) {
    let count_rows = layer.count_rows();
    let count_columns = layer.count_columns();

    if layer.was_transposed {
        let data = match &mut layer.records {
            Cow::Owned(data) => data,
            Cow::Borrowed(_) => unreachable!("must never happen"),
        };

        let headers = pop_first_column(data);
        let headers = headers
            .into_iter()
            .map(|value| match value {
                Value::String { val, .. } => val,
                _ => unreachable!("must never happen"),
            })
            .collect();

        let data = _transpose_table(data, count_rows, count_columns - 1);

        layer.records = Cow::Owned(data);
        layer.columns = Cow::Owned(headers);
    } else {
        let mut data = _transpose_table(&layer.records, count_rows, count_columns);

        for (column, column_name) in layer.columns.iter().enumerate() {
            let value = Value::String {
                val: column_name.to_string(),
                span: NuSpan::unknown(),
            };

            data[column].insert(0, value);
        }

        layer.records = Cow::Owned(data);
        layer.columns = (1..count_rows + 1 + 1).map(|i| i.to_string()).collect();
    }

    layer.was_transposed = !layer.was_transposed;
}

fn pop_first_column(values: &mut [Vec<Value>]) -> Vec<Value> {
    let mut data = vec![Value::default(); values.len()];
    for (row, values) in values.iter_mut().enumerate() {
        data[row] = values.remove(0);
    }

    data
}

fn _transpose_table(
    values: &[Vec<Value>],
    count_rows: usize,
    count_columns: usize,
) -> Vec<Vec<Value>> {
    let mut data = vec![vec![Value::default(); count_rows]; count_columns];
    for (row, values) in values.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            data[column][row] = value.to_owned();
        }
    }

    data
}
