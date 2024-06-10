mod table_widget;

use self::table_widget::{TableWidget, TableWidgetState};
use super::{
    cursor::{Position, WindowCursor2D},
    util::{make_styled_string, nu_style_to_tui},
    Layout, View, ViewConfig,
};
use crate::{
    explore::ExploreConfig,
    nu_common::{collect_input, lscolorize, NuSpan, NuText},
    pager::{
        report::{Report, Severity},
        Frame, Transition, ViewInfo,
    },
    views::ElementInfo,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nu_color_config::StyleComputer;
use nu_protocol::{
    engine::{EngineState, Stack},
    Config, Record, Span, Value,
};
use ratatui::{layout::Rect, widgets::Block};
use std::collections::HashMap;

pub use self::table_widget::Orientation;

#[derive(Debug, Clone)]
pub struct RecordView {
    layer_stack: Vec<LayerData>,
    mode: UIMode,
    orientation: Orientation,
    cfg: ExploreConfig,
}

#[derive(Debug, Clone)]
struct LayerData {
    record: RecordLayer,
    widget_data: Option<RecordData>,
}

#[derive(Debug, Clone)]
struct RecordData {
    columns: Vec<String>,
    records: Vec<Vec<NuText>>,
}

impl RecordView {
    pub fn new(columns: Vec<String>, records: Vec<Vec<Value>>) -> Self {
        let layer = LayerData {
            record: RecordLayer::new(columns, records),
            widget_data: None,
        };

        Self {
            layer_stack: vec![layer],
            mode: UIMode::View,
            orientation: Orientation::Top,
            // TODO: It's kind of gross how this temporarily has an incorrect/default config.
            // See if we can pass correct config in through the constructor
            cfg: ExploreConfig::default(),
        }
    }

    pub fn tail(&mut self, width: u16, height: u16) {
        let page_size =
            estimate_page_size(Rect::new(0, 0, width, height), self.cfg.table.show_header);
        tail_data(self, page_size as usize);
    }

    pub fn transpose(&mut self) {
        let layer = self.get_layer_last_mut();
        transpose_table(layer);

        layer.reset_cursor();
    }

    // todo: rename to get_layer
    pub fn get_layer_last(&self) -> &RecordLayer {
        &self.get_layer2().record
    }

    pub fn get_layer_last_mut(&mut self) -> &mut RecordLayer {
        &mut self.get_layer2_mut().record
    }

    // todo: rename to get_layer
    fn get_layer2(&self) -> &LayerData {
        self.layer_stack
            .last()
            .expect("we guarantee that 1 entry is always in a list")
    }

    fn get_layer2_mut(&mut self) -> &mut LayerData {
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
            layer.record.reset_cursor();
        }
    }

    pub fn set_orientation_current(&mut self, orientation: Orientation) {
        let layer = self.get_layer_last_mut();
        layer.orientation = orientation;
        layer.reset_cursor();
    }

    /// Get the current position of the cursor in the table as a whole
    pub fn get_cursor_position(&self) -> Position {
        let layer = self.get_layer_last();
        layer.cursor.position()
    }

    /// Get the current position of the cursor in the window being shown
    pub fn get_cursor_position_in_window(&self) -> Position {
        let layer = self.get_layer_last();
        layer.cursor.window_relative_position()
    }

    /// Get the origin of the window being shown. (0,0), top left corner.
    pub fn get_window_origin(&self) -> Position {
        let layer = self.get_layer_last();
        layer.cursor.window_origin()
    }

    pub fn set_cursor_mode(&mut self) {
        self.mode = UIMode::Cursor;
    }

    pub fn set_view_mode(&mut self) {
        self.mode = UIMode::View;
    }

    pub fn get_current_value(&self) -> Value {
        let Position { row, column } = self.get_cursor_position();
        let layer = self.get_layer_last();

        let (row, column) = match layer.orientation {
            Orientation::Top => (row, column),
            Orientation::Left => (column, row),
        };

        if layer.records.len() > row && layer.records[row].len() > column {
            layer.records[row][column].clone()
        } else {
            Value::nothing(Span::unknown())
        }
    }

    fn create_table_widget<'a>(&'a mut self, cfg: ViewConfig<'a>) -> TableWidget<'a> {
        let style = self.cfg.table;
        let style_computer = cfg.style_computer;
        let Position { row, column } = self.get_window_origin();

        let layer = self.get_layer2_mut();
        if layer.widget_data.is_none() {
            let mut data =
                convert_records_to_string(&layer.record.records, cfg.nu_config, cfg.style_computer);
            lscolorize(&layer.record.columns, &mut data, cfg.lscolors);

            let columns = layer
                .record
                .columns
                .iter()
                .map(|s| make_head_text(s))
                .collect();

            layer.widget_data = Some(RecordData {
                records: data,
                columns,
            })
        }

        let headers = &layer.widget_data.as_ref().expect("ok").columns;
        let data = &layer.widget_data.as_ref().expect("ok").records;

        TableWidget::new(
            headers,
            data,
            style_computer,
            row,
            column,
            style,
            layer.record.orientation,
        )
    }

    fn update_cursors(&mut self, rows: usize, columns: usize) {
        match self.get_layer_last().orientation {
            Orientation::Top => {
                let _ = self
                    .get_layer_last_mut()
                    .cursor
                    .set_window_size(rows, columns);
            }

            Orientation::Left => {
                let _ = self
                    .get_layer_last_mut()
                    .cursor
                    .set_window_size(rows, columns);
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

impl View for RecordView {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        let mut table_layout = TableWidgetState::default();
        // TODO: creating the table widget is O(N) where N is the number of cells in the grid.
        // Way too slow to do on every draw call!
        // To make explore work for larger data sets, this needs to be improved.
        let table = self.create_table_widget(cfg);
        f.render_stateful_widget(table, area, &mut table_layout);

        *layout = table_layout.layout;

        self.update_cursors(table_layout.count_rows, table_layout.count_columns);

        if self.mode == UIMode::Cursor {
            let Position { row, column } = self.get_cursor_position_in_window();
            let info = get_element_info(
                layout,
                row,
                column,
                table_layout.count_rows,
                self.get_layer_last().orientation,
                self.cfg.table.show_header,
            );

            if let Some(info) = info {
                highlight_selected_cell(f, info.clone(), &self.cfg);
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
            UIMode::View => Ok(handle_key_event_view_mode(self, &key)),
            UIMode::Cursor => handle_key_event_cursor_mode(self, &key),
        };

        match result {
            Ok(result) => {
                if matches!(&result, Some(Transition::Ok) | Some(Transition::Cmd { .. })) {
                    let report = self.create_records_report();
                    info.status = Some(report);
                }

                result
            }
            Err(e) => {
                log::error!("Error handling input in RecordView: {e}");
                let report = Report::message(e.to_string(), Severity::Err);
                info.status = Some(report);
                None
            }
        }
    }

    fn collect_data(&self) -> Vec<NuText> {
        // Create a "dummy" style_computer.
        let dummy_engine_state = EngineState::new();
        let dummy_stack = Stack::new();
        let style_computer = StyleComputer::new(&dummy_engine_state, &dummy_stack, HashMap::new());

        let data = convert_records_to_string(
            &self.get_layer_last().records,
            &nu_protocol::Config::default(),
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
                    self.get_layer_last_mut()
                        .cursor
                        .set_window_start_position(row, column);
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
        self.cfg = cfg.explore_config.clone();
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
pub struct RecordLayer {
    columns: Vec<String>,
    records: Vec<Vec<Value>>,
    orientation: Orientation,
    name: Option<String>,
    was_transposed: bool,
    cursor: WindowCursor2D,
}

impl RecordLayer {
    fn new(columns: Vec<String>, records: Vec<Vec<Value>>) -> Self {
        // TODO: refactor so this is fallible and returns a Result instead of panicking
        let cursor =
            WindowCursor2D::new(records.len(), columns.len()).expect("Failed to create cursor");

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
        // TODO: refactor so this is fallible and returns a Result instead of panicking
        self.cursor = WindowCursor2D::new(self.count_rows(), self.count_columns())
            .expect("Failed to create cursor");
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

fn handle_key_event_cursor_mode(
    view: &mut RecordView,
    key: &KeyEvent,
) -> Result<Option<Transition>> {
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

            return Ok(Some(Transition::Ok));
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

            return Ok(Some(Transition::Ok));
        }
        _ => {}
    }

    match key.code {
        KeyCode::Esc => {
            view.set_view_mode();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Up | KeyCode::Char('k') => {
            view.get_layer_last_mut().cursor.prev_row();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Down | KeyCode::Char('j') => {
            view.get_layer_last_mut().cursor.next_row();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Left | KeyCode::Char('h') => {
            view.get_layer_last_mut().cursor.prev_column();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Right | KeyCode::Char('l') => {
            view.get_layer_last_mut().cursor.next_column();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Home | KeyCode::Char('g') => {
            view.get_layer_last_mut().cursor.row_move_to_start();

            Ok(Some(Transition::Ok))
        }
        KeyCode::End | KeyCode::Char('G') => {
            view.get_layer_last_mut().cursor.row_move_to_end();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Enter => {
            let value = view.get_current_value();
            let is_record = matches!(value, Value::Record { .. });
            let next_layer = create_layer(value)?;
            push_layer(view, next_layer);

            if is_record {
                view.set_orientation_current(Orientation::Left);
            } else if view.orientation == view.get_layer_last().orientation {
                view.get_layer_last_mut().orientation = view.orientation;
            } else {
                view.set_orientation_current(view.orientation);
            }

            Ok(Some(Transition::Ok))
        }
        _ => Ok(None),
    }
}

fn create_layer(value: Value) -> Result<RecordLayer> {
    let (columns, values) = collect_input(value)?;

    Ok(RecordLayer::new(columns, values))
}

fn push_layer(view: &mut RecordView, mut next_layer: RecordLayer) {
    let layer = view.get_layer_last();
    let header = layer.get_column_header();

    if let Some(header) = header {
        next_layer.set_name(header);
    }

    let layer = LayerData {
        record: next_layer,
        widget_data: None,
    };
    view.layer_stack.push(layer);
}

fn estimate_page_size(area: Rect, show_head: bool) -> u16 {
    let mut available_height = area.height;
    available_height -= 3; // status_bar

    if show_head {
        available_height -= 3; // head
    }

    available_height
}

/// scroll to the end of the data
fn tail_data(state: &mut RecordView, page_size: usize) {
    let layer = state.get_layer_last_mut();
    let count_rows = layer.records.len();
    if count_rows > page_size {
        layer
            .cursor
            .set_window_start_position(count_rows - page_size, 0);
    }
}

fn convert_records_to_string(
    records: &[Vec<Value>],
    cfg: &Config,
    style_computer: &StyleComputer,
) -> Vec<Vec<NuText>> {
    records
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| {
                    let text = value.clone().to_abbreviated_string(cfg);
                    let text = strip_string(&text);
                    let float_precision = cfg.float_precision as usize;

                    make_styled_string(style_computer, text, Some(value), float_precision)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

fn highlight_selected_cell(f: &mut Frame, info: ElementInfo, cfg: &ExploreConfig) {
    let cell_style = cfg.selected_cell;
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

fn report_cursor_position(mode: UIMode, cursor: WindowCursor2D) -> String {
    if mode == UIMode::Cursor {
        let Position { row, column } = cursor.position();
        format!("{row},{column}")
    } else {
        let Position { row, column } = cursor.window_origin();
        format!("{row},{column}")
    }
}

fn report_row_position(cursor: WindowCursor2D) -> String {
    if cursor.window_origin().row == 0 {
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

fn transpose_table(layer: &mut RecordLayer) {
    let count_rows = layer.records.len();
    let count_columns = layer.columns.len();

    if layer.was_transposed {
        let data = &mut layer.records;

        let headers = pop_first_column(data);
        let headers = headers
            .into_iter()
            .map(|value| match value {
                Value::String { val, .. } => val,
                _ => unreachable!("must never happen"),
            })
            .collect();

        let data = _transpose_table(data, count_rows, count_columns - 1);

        layer.records = data;
        layer.columns = headers;
    } else {
        let mut data = _transpose_table(&layer.records, count_rows, count_columns);

        for (column, column_name) in layer.columns.iter().enumerate() {
            let value = Value::string(column_name, NuSpan::unknown());

            data[column].insert(0, value);
        }

        layer.records = data;
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

fn make_head_text(head: &str) -> String {
    strip_string(head)
}

fn strip_string(text: &str) -> String {
    String::from_utf8(strip_ansi_escapes::strip(text))
        .map_err(|_| ())
        .unwrap_or_else(|_| text.to_owned())
}
