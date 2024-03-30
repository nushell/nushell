mod tablew;

use self::tablew::{TableStyle, TableW, TableWState};
use super::{
    cursor::XYCursor,
    util::{make_styled_string, nu_style_to_tui},
    Layout, View, ViewConfig,
};
use crate::{
    nu_common::{collect_input, lscolorize, NuConfig, NuSpan, NuStyle, NuText},
    pager::{
        report::{Report, Severity},
        ConfigMap, Frame, Transition, ViewInfo,
    },
    util::create_map,
    views::ElementInfo,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nu_color_config::{get_color_map, StyleComputer};
use nu_protocol::{
    engine::{EngineState, Stack},
    Record, Value,
};
use ratatui::{layout::Rect, widgets::Block};
use std::{borrow::Cow, collections::HashMap};

pub use self::tablew::Orientation;

#[derive(Debug, Clone)]
pub struct RecordView<'a> {
    layer_stack: Vec<RecordLayer<'a>>,
    mode: UIMode,
    orientation: Orientation,
    theme: TableTheme,
}

impl<'a> RecordView<'a> {
    pub fn new(
        columns: impl Into<Cow<'a, [String]>>,
        records: impl Into<Cow<'a, [Vec<Value>]>>,
    ) -> Self {
        Self {
            layer_stack: vec![RecordLayer::new(columns, records)],
            mode: UIMode::View,
            orientation: Orientation::Top,
            theme: TableTheme::default(),
        }
    }

    pub fn reverse(&mut self, width: u16, height: u16) {
        let page_size =
            estimate_page_size(Rect::new(0, 0, width, height), self.theme.table.show_header);
        state_reverse_data(self, page_size as usize);
    }

    pub fn set_style_split_line(&mut self, style: NuStyle) {
        self.theme.table.splitline_style = style
    }

    pub fn set_style_selected_cell(&mut self, style: NuStyle) {
        self.theme.cursor.selected_cell = Some(style)
    }

    pub fn set_style_selected_row(&mut self, style: NuStyle) {
        self.theme.cursor.selected_row = Some(style)
    }

    pub fn set_style_selected_column(&mut self, style: NuStyle) {
        self.theme.cursor.selected_column = Some(style)
    }

    pub fn set_padding_column(&mut self, (left, right): (usize, usize)) {
        self.theme.table.padding_column_left = left;
        self.theme.table.padding_column_right = right;
    }

    pub fn set_padding_index(&mut self, (left, right): (usize, usize)) {
        self.theme.table.padding_index_left = left;
        self.theme.table.padding_index_right = right;
    }

    pub fn get_padding_column(&self) -> (usize, usize) {
        (
            self.theme.table.padding_column_left,
            self.theme.table.padding_column_right,
        )
    }

    pub fn get_padding_index(&self) -> (usize, usize) {
        (
            self.theme.table.padding_index_left,
            self.theme.table.padding_index_right,
        )
    }

    pub fn get_theme(&self) -> &TableTheme {
        &self.theme
    }

    pub fn set_theme(&mut self, theme: TableTheme) {
        self.theme = theme;
    }

    pub fn transpose(&mut self) {
        let layer = self.get_layer_last_mut();
        transpose_table(layer);

        layer.reset_cursor();
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

    pub fn get_orientation_current(&mut self) -> Orientation {
        self.get_layer_last().orientation
    }

    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;

        // we need to reset all indexes as we can't no more use them.
        self.reset_cursors();
    }

    fn reset_cursors(&mut self) {
        for layer in &mut self.layer_stack {
            layer.reset_cursor();
        }
    }

    pub fn set_orientation_current(&mut self, orientation: Orientation) {
        let layer = self.get_layer_last_mut();
        layer.orientation = orientation;
        layer.reset_cursor();
    }

    pub fn get_current_position(&self) -> (usize, usize) {
        let layer = self.get_layer_last();
        (layer.cursor.row(), layer.cursor.column())
    }

    pub fn get_current_window(&self) -> (usize, usize) {
        let layer = self.get_layer_last();
        (layer.cursor.row_window(), layer.cursor.column_window())
    }

    pub fn get_current_offset(&self) -> (usize, usize) {
        let layer = self.get_layer_last();
        (
            layer.cursor.row_starts_at(),
            layer.cursor.column_starts_at(),
        )
    }

    pub fn set_cursor_mode(&mut self) {
        self.mode = UIMode::Cursor;
    }

    pub fn set_view_mode(&mut self) {
        self.mode = UIMode::View;
    }

    pub fn get_current_value(&self) -> Value {
        let (row, column) = self.get_current_position();
        let layer = self.get_layer_last();

        let (row, column) = match layer.orientation {
            Orientation::Top => (row, column),
            Orientation::Left => (column, row),
        };

        layer.records[row][column].clone()
    }

    fn create_tablew(&'a self, cfg: ViewConfig<'a>) -> TableW<'a> {
        let layer = self.get_layer_last();
        let mut data = convert_records_to_string(&layer.records, cfg.nu_config, cfg.style_computer);

        lscolorize(&layer.columns, &mut data, cfg.lscolors);

        let headers = layer.columns.as_ref();
        let style_computer = cfg.style_computer;
        let (row, column) = self.get_current_offset();

        TableW::new(
            headers,
            data,
            style_computer,
            row,
            column,
            self.theme.table,
            layer.orientation,
        )
    }

    fn update_cursors(&mut self, rows: usize, columns: usize) {
        match self.get_layer_last().orientation {
            Orientation::Top => {
                self.get_layer_last_mut().cursor.set_window(rows, columns);
            }

            Orientation::Left => {
                self.get_layer_last_mut().cursor.set_window(rows, columns);
            }
        }
    }

    fn create_records_report(&self) -> Report {
        let layer = self.get_layer_last();
        let covered_percent = report_row_position(layer.cursor);
        let cursor = report_cursor_position(self.mode, layer.cursor);
        let message = layer.name.clone().unwrap_or_default();
        // note: maybe came up with a better short names? E/V/N?
        let mode = match self.mode {
            UIMode::Cursor => String::from("EDIT"),
            UIMode::View => String::from("VIEW"),
        };

        Report::new(message, Severity::Info, mode, cursor, covered_percent)
    }
}

impl View for RecordView<'_> {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        let mut table_layout = TableWState::default();
        let table = self.create_tablew(cfg);
        f.render_stateful_widget(table, area, &mut table_layout);

        *layout = table_layout.layout;

        self.update_cursors(table_layout.count_rows, table_layout.count_columns);

        if self.mode == UIMode::Cursor {
            let (row, column) = self.get_current_window();
            let info = get_element_info(
                layout,
                row,
                column,
                table_layout.count_rows,
                self.get_layer_last().orientation,
                self.theme.table.show_header,
            );

            if let Some(info) = info {
                highlight_cell(f, area, info.clone(), &self.theme.cursor);
            }
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
            UIMode::Cursor => handle_key_event_cursor_mode(self, &key),
        };

        if matches!(&result, Some(Transition::Ok) | Some(Transition::Cmd { .. })) {
            let report = self.create_records_report();
            info.status = Some(report);
        }

        result
    }

    fn collect_data(&self) -> Vec<NuText> {
        // Create a "dummy" style_computer.
        let dummy_engine_state = EngineState::new();
        let dummy_stack = Stack::new();
        let style_computer = StyleComputer::new(&dummy_engine_state, &dummy_stack, HashMap::new());

        let data = convert_records_to_string(
            &self.get_layer_last().records,
            &NuConfig::default(),
            &style_computer,
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
                    self.get_layer_last_mut().cursor.set_position(row, column);
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

    // todo: move the method to Command?
    fn setup(&mut self, cfg: ViewConfig<'_>) {
        if let Some(hm) = cfg.config.get("table").and_then(create_map) {
            self.theme = theme_from_config(&hm);

            if let Some(orientation) = hm.get("orientation").and_then(|v| v.coerce_str().ok()) {
                let orientation = match orientation.as_ref() {
                    "left" => Some(Orientation::Left),
                    "top" => Some(Orientation::Top),
                    _ => None,
                };

                if let Some(orientation) = orientation {
                    self.set_orientation(orientation);
                    self.set_orientation_current(orientation);
                }
            }
        }
    }
}

fn get_element_info(
    layout: &mut Layout,
    row: usize,
    column: usize,
    count_rows: usize,
    orientation: Orientation,
    with_head: bool,
) -> Option<&ElementInfo> {
    let with_head = with_head as usize;
    let index = match orientation {
        Orientation::Top => column * (count_rows + with_head) + row + 1,
        Orientation::Left => (column + with_head) * count_rows + row,
    };

    layout.data.get(index)
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
    orientation: Orientation,
    name: Option<String>,
    was_transposed: bool,
    cursor: XYCursor,
}

impl<'a> RecordLayer<'a> {
    fn new(
        columns: impl Into<Cow<'a, [String]>>,
        records: impl Into<Cow<'a, [Vec<Value>]>>,
    ) -> Self {
        let columns = columns.into();
        let records = records.into();
        let cursor = XYCursor::new(records.len(), columns.len());

        Self {
            columns,
            records,
            cursor,
            orientation: Orientation::Top,
            name: None,
            was_transposed: false,
        }
    }

    fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    fn count_rows(&self) -> usize {
        match self.orientation {
            Orientation::Top => self.records.len(),
            Orientation::Left => self.columns.len(),
        }
    }

    fn count_columns(&self) -> usize {
        match self.orientation {
            Orientation::Top => self.columns.len(),
            Orientation::Left => self.records.len(),
        }
    }

    fn get_column_header(&self) -> Option<String> {
        let col = self.cursor.column();
        self.columns.get(col).map(|header| header.to_string())
    }

    fn reset_cursor(&mut self) {
        self.cursor = XYCursor::new(self.count_rows(), self.count_columns());
    }
}

fn handle_key_event_view_mode(view: &mut RecordView, key: &KeyEvent) -> Option<Transition> {
    match key {
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => {
            view.get_layer_last_mut().cursor.prev_row_page();

            return Some(Transition::Ok);
        }
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => {
            view.get_layer_last_mut().cursor.next_row_page();

            return Some(Transition::Ok);
        }
        _ => {}
    }

    match key.code {
        KeyCode::Esc => {
            if view.layer_stack.len() > 1 {
                view.layer_stack.pop();
                view.mode = UIMode::Cursor;

                Some(Transition::Ok)
            } else {
                Some(Transition::Exit)
            }
        }
        KeyCode::Char('i') | KeyCode::Enter => {
            view.set_cursor_mode();

            Some(Transition::Ok)
        }
        KeyCode::Char('t') => {
            view.transpose();

            Some(Transition::Ok)
        }
        KeyCode::Char('e') => Some(Transition::Cmd(String::from("expand"))),
        KeyCode::Up | KeyCode::Char('k') => {
            view.get_layer_last_mut().cursor.prev_row_i();

            Some(Transition::Ok)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            view.get_layer_last_mut().cursor.next_row_i();

            Some(Transition::Ok)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            view.get_layer_last_mut().cursor.prev_column_i();

            Some(Transition::Ok)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            view.get_layer_last_mut().cursor.next_column_i();

            Some(Transition::Ok)
        }
        KeyCode::Home | KeyCode::Char('g') => {
            view.get_layer_last_mut().cursor.row_move_to_start();

            Some(Transition::Ok)
        }
        KeyCode::End | KeyCode::Char('G') => {
            view.get_layer_last_mut().cursor.row_move_to_end();

            Some(Transition::Ok)
        }
        _ => None,
    }
}

fn handle_key_event_cursor_mode(view: &mut RecordView, key: &KeyEvent) -> Option<Transition> {
    match key {
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => {
            view.get_layer_last_mut().cursor.prev_row_page();

            return Some(Transition::Ok);
        }
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => {
            view.get_layer_last_mut().cursor.next_row_page();

            return Some(Transition::Ok);
        }
        _ => {}
    }

    match key.code {
        KeyCode::Esc => {
            view.set_view_mode();

            Some(Transition::Ok)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            view.get_layer_last_mut().cursor.prev_row();

            Some(Transition::Ok)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            view.get_layer_last_mut().cursor.next_row();

            Some(Transition::Ok)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            view.get_layer_last_mut().cursor.prev_column();

            Some(Transition::Ok)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            view.get_layer_last_mut().cursor.next_column();

            Some(Transition::Ok)
        }
        KeyCode::Home | KeyCode::Char('g') => {
            view.get_layer_last_mut().cursor.row_move_to_start();

            Some(Transition::Ok)
        }
        KeyCode::End | KeyCode::Char('G') => {
            view.get_layer_last_mut().cursor.row_move_to_end();

            Some(Transition::Ok)
        }
        KeyCode::Enter => {
            let value = view.get_current_value();
            let is_record = matches!(value, Value::Record { .. });
            let next_layer = create_layer(value);

            push_layer(view, next_layer);

            if is_record {
                view.set_orientation_current(Orientation::Left);
            } else if view.orientation == view.get_layer_last().orientation {
                view.get_layer_last_mut().orientation = view.orientation;
            } else {
                view.set_orientation_current(view.orientation);
            }

            Some(Transition::Ok)
        }
        _ => None,
    }
}

fn create_layer(value: Value) -> RecordLayer<'static> {
    let (columns, values) = collect_input(value);

    RecordLayer::new(columns, values)
}

fn push_layer(view: &mut RecordView<'_>, mut next_layer: RecordLayer<'static>) {
    let layer = view.get_layer_last();
    let header = layer.get_column_header();

    if let Some(header) = header {
        next_layer.set_name(header);
    }

    view.layer_stack.push(next_layer);
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
    if count_rows > page_size {
        layer.cursor.set_position(count_rows - page_size, 0);
    }
}

fn convert_records_to_string(
    records: &[Vec<Value>],
    cfg: &NuConfig,
    style_computer: &StyleComputer,
) -> Vec<Vec<NuText>> {
    records
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| {
                    let text = value.clone().to_abbreviated_string(cfg);
                    let float_precision = cfg.float_precision as usize;

                    make_styled_string(style_computer, text, Some(value), float_precision)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

fn highlight_cell(f: &mut Frame, area: Rect, info: ElementInfo, theme: &CursorStyle) {
    // highlight selected column
    if let Some(style) = theme.selected_column {
        let highlight_block = Block::default().style(nu_style_to_tui(style));
        let area = Rect::new(info.area.x, area.y, info.area.width, area.height);
        f.render_widget(highlight_block.clone(), area);
    }

    // highlight selected row
    if let Some(style) = theme.selected_row {
        let highlight_block = Block::default().style(nu_style_to_tui(style));
        let area = Rect::new(area.x, info.area.y, area.width, 1);
        f.render_widget(highlight_block.clone(), area);
    }

    // highlight selected cell
    let cell_style = match theme.selected_cell {
        Some(s) => s,
        None => {
            let mut style = nu_ansi_term::Style::new();
            // light blue chosen somewhat arbitrarily, looks OK but I'm not set on it
            style.background = Some(nu_ansi_term::Color::LightBlue);
            style
        }
    };
    let highlight_block = Block::default().style(nu_style_to_tui(cell_style));
    let area = Rect::new(info.area.x, info.area.y, info.area.width, 1);
    f.render_widget(highlight_block.clone(), area)
}

fn build_last_value(v: &RecordView) -> Value {
    if v.mode == UIMode::Cursor {
        v.get_current_value()
    } else if v.get_layer_last().count_rows() < 2 {
        build_table_as_record(v)
    } else {
        build_table_as_list(v)
    }
}

fn build_table_as_list(v: &RecordView) -> Value {
    let layer = v.get_layer_last();

    let cols = &layer.columns;
    let vals = layer
        .records
        .iter()
        .map(|vals| {
            let record = cols.iter().cloned().zip(vals.iter().cloned()).collect();
            Value::record(record, NuSpan::unknown())
        })
        .collect();

    Value::list(vals, NuSpan::unknown())
}

fn build_table_as_record(v: &RecordView) -> Value {
    let layer = v.get_layer_last();

    let record = if let Some(row) = layer.records.first() {
        layer
            .columns
            .iter()
            .cloned()
            .zip(row.iter().cloned())
            .collect()
    } else {
        Record::new()
    };

    Value::record(record, NuSpan::unknown())
}

fn report_cursor_position(mode: UIMode, cursor: XYCursor) -> String {
    if mode == UIMode::Cursor {
        let row = cursor.row();
        let column = cursor.column();
        format!("{row},{column}")
    } else {
        let rows_seen = cursor.row_starts_at();
        let columns_seen = cursor.column_starts_at();
        format!("{rows_seen},{columns_seen}")
    }
}

fn report_row_position(cursor: XYCursor) -> String {
    if cursor.row_starts_at() == 0 {
        String::from("Top")
    } else {
        let percent_rows = get_percentage(cursor.row(), cursor.row_limit());

        match percent_rows {
            100 => String::from("All"),
            value => format!("{value}%"),
        }
    }
}

fn get_percentage(value: usize, max: usize) -> usize {
    debug_assert!(value <= max, "{value:?} {max:?}");

    ((value as f32 / max as f32) * 100.0).floor() as usize
}

fn transpose_table(layer: &mut RecordLayer<'_>) {
    let count_rows = layer.records.len();
    let count_columns = layer.columns.len();

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
            let value = Value::string(column_name, NuSpan::unknown());

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
            data[column][row].clone_from(value);
        }
    }

    data
}

fn theme_from_config(config: &ConfigMap) -> TableTheme {
    let mut theme = TableTheme::default();

    let colors = get_color_map(config);

    if let Some(s) = colors.get("split_line") {
        theme.table.splitline_style = *s;
    }

    theme.cursor.selected_cell = colors.get("selected_cell").cloned();
    theme.cursor.selected_row = colors.get("selected_row").cloned();
    theme.cursor.selected_column = colors.get("selected_column").cloned();

    theme.table.show_header = config_get_bool(config, "show_head", true);
    theme.table.show_index = config_get_bool(config, "show_index", false);

    theme.table.padding_index_left = config_get_usize(config, "padding_index_left", 2);
    theme.table.padding_index_right = config_get_usize(config, "padding_index_right", 1);
    theme.table.padding_column_left = config_get_usize(config, "padding_column_left", 2);
    theme.table.padding_column_right = config_get_usize(config, "padding_column_right", 2);

    theme
}

fn config_get_bool(config: &ConfigMap, key: &str, default: bool) -> bool {
    config
        .get(key)
        .and_then(|v| v.as_bool().ok())
        .unwrap_or(default)
}

fn config_get_usize(config: &ConfigMap, key: &str, default: usize) -> usize {
    config
        .get(key)
        .and_then(|v| v.coerce_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}

#[derive(Debug, Default, Clone)]
pub struct TableTheme {
    table: TableStyle,
    cursor: CursorStyle,
}

#[derive(Debug, Default, Clone)]
struct CursorStyle {
    selected_cell: Option<NuStyle>,
    selected_column: Option<NuStyle>,
    selected_row: Option<NuStyle>,
}
