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
    Config, Record, Value,
};
use ratatui::{layout::Rect, widgets::Block};
use std::collections::HashMap;

pub use self::table_widget::Orientation;

#[derive(Debug, Clone)]
pub struct RecordView {
    layer_stack: Vec<RecordLayer>,
    mode: UIMode,
    orientation: Orientation,
    cfg: ExploreConfig,
}

impl RecordView {
    pub fn new(columns: Vec<String>, records: Vec<Vec<Value>>, cfg: ExploreConfig) -> Self {
        Self {
            layer_stack: vec![RecordLayer::new(columns, records)],
            mode: UIMode::View,
            orientation: Orientation::Top,
            cfg,
        }
    }

    pub fn tail(&mut self, width: u16, height: u16) {
        let page_size =
            estimate_page_size(Rect::new(0, 0, width, height), self.cfg.table.show_header);
        tail_data(self, page_size as usize);
    }

    pub fn transpose(&mut self) {
        let layer = self.get_top_layer_mut();
        transpose_table(layer);

        layer.reset_cursor();
    }

    pub fn get_top_layer(&self) -> &RecordLayer {
        self.layer_stack
            .last()
            .expect("we guarantee that 1 entry is always in a list")
    }

    pub fn get_top_layer_mut(&mut self) -> &mut RecordLayer {
        self.layer_stack
            .last_mut()
            .expect("we guarantee that 1 entry is always in a list")
    }

    pub fn set_top_layer_orientation(&mut self, orientation: Orientation) {
        let layer = self.get_top_layer_mut();
        layer.orientation = orientation;
        layer.reset_cursor();
    }

    /// Get the current position of the cursor in the table as a whole
    pub fn get_cursor_position(&self) -> Position {
        let layer = self.get_top_layer();
        layer.cursor.position()
    }

    /// Get the current position of the cursor in the window being shown
    pub fn get_cursor_position_in_window(&self) -> Position {
        let layer = self.get_top_layer();
        layer.cursor.window_relative_position()
    }

    /// Get the origin of the window being shown. (0,0), top left corner.
    pub fn get_window_origin(&self) -> Position {
        let layer = self.get_top_layer();
        layer.cursor.window_origin()
    }

    pub fn set_cursor_mode(&mut self) {
        self.mode = UIMode::Cursor;
    }

    pub fn set_view_mode(&mut self) {
        self.mode = UIMode::View;
    }

    pub fn get_current_value(&self) -> &Value {
        let Position { row, column } = self.get_cursor_position();
        let layer = self.get_top_layer();

        let (row, column) = match layer.orientation {
            Orientation::Top => (row, column),
            Orientation::Left => (column, row),
        };

        // These should never happen as long as the cursor is working correctly
        assert!(row < layer.record_values.len(), "row out of bounds");
        assert!(column < layer.column_names.len(), "column out of bounds");

        &layer.record_values[row][column]
    }

    fn create_table_widget<'a>(&'a mut self, cfg: ViewConfig<'a>) -> TableWidget<'a> {
        let style = self.cfg.table;
        let style_computer = cfg.style_computer;
        let Position { row, column } = self.get_window_origin();

        let layer = self.get_top_layer_mut();
        if layer.record_text.is_none() {
            let mut data =
                convert_records_to_string(&layer.record_values, cfg.nu_config, cfg.style_computer);
            lscolorize(&layer.column_names, &mut data, cfg.lscolors);

            layer.record_text = Some(data);
        }

        let headers = &layer.column_names;
        let data = layer.record_text.as_ref().expect("always ok");

        TableWidget::new(
            headers,
            data,
            style_computer,
            row,
            column,
            style,
            layer.orientation,
        )
    }

    fn update_cursors(&mut self, rows: usize, columns: usize) {
        match self.get_top_layer().orientation {
            Orientation::Top => {
                let _ = self
                    .get_top_layer_mut()
                    .cursor
                    .set_window_size(rows, columns);
            }

            Orientation::Left => {
                let _ = self
                    .get_top_layer_mut()
                    .cursor
                    .set_window_size(rows, columns);
            }
        }
    }

    fn create_records_report(&self) -> Report {
        let layer = self.get_top_layer();
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
                self.get_top_layer().orientation,
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
            &self.get_top_layer().record_values,
            &nu_protocol::Config::default(),
            &style_computer,
        );

        data.iter().flatten().cloned().collect()
    }

    fn show_data(&mut self, pos: usize) -> bool {
        let data = &self.get_top_layer().record_values;

        let mut i = 0;
        for (row, cells) in data.iter().enumerate() {
            if pos > i + cells.len() {
                i += cells.len();
                continue;
            }

            for (column, _) in cells.iter().enumerate() {
                if i == pos {
                    self.get_top_layer_mut()
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
    column_names: Vec<String>,
    // These are the raw records in the current layer. The sole reason we keep this around is so we can return the original value
    // if it's being peeked. Otherwise we could accept an iterator over it.
    // or if it could be Clonable we could do that anyway;
    // cause it would keep memory footprint lower while keep everything working
    // (yee would make return O(n); we would need to traverse iterator once again; but maybe worth it)
    record_values: Vec<Vec<Value>>,
    // This is the text representation of the record values (the actual text that will be displayed to users).
    // It's an Option because we need configuration to set it and we (currently) don't have access to configuration when things are created.
    record_text: Option<Vec<Vec<NuText>>>,
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

        let column_names = columns.iter().map(|s| strip_string(s)).collect();

        Self {
            column_names,
            record_values: records,
            record_text: None,
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
            Orientation::Top => self.record_values.len(),
            Orientation::Left => self.column_names.len(),
        }
    }

    fn count_columns(&self) -> usize {
        match self.orientation {
            Orientation::Top => self.column_names.len(),
            Orientation::Left => self.record_values.len(),
        }
    }

    fn get_column_header(&self) -> Option<String> {
        let col = self.cursor.column();
        self.column_names.get(col).map(|header| header.to_string())
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
            view.get_top_layer_mut().cursor.prev_row_page();

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
            view.get_top_layer_mut().cursor.next_row_page();

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
            view.get_top_layer_mut().cursor.prev_row_i();

            Some(Transition::Ok)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            view.get_top_layer_mut().cursor.next_row_i();

            Some(Transition::Ok)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            view.get_top_layer_mut().cursor.prev_column_i();

            Some(Transition::Ok)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            view.get_top_layer_mut().cursor.next_column_i();

            Some(Transition::Ok)
        }
        KeyCode::Home | KeyCode::Char('g') => {
            view.get_top_layer_mut().cursor.row_move_to_start();

            Some(Transition::Ok)
        }
        KeyCode::End | KeyCode::Char('G') => {
            view.get_top_layer_mut().cursor.row_move_to_end();

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
            view.get_top_layer_mut().cursor.prev_row_page();

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
            view.get_top_layer_mut().cursor.next_row_page();

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
            view.get_top_layer_mut().cursor.prev_row();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Down | KeyCode::Char('j') => {
            view.get_top_layer_mut().cursor.next_row();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Left | KeyCode::Char('h') => {
            view.get_top_layer_mut().cursor.prev_column();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Right | KeyCode::Char('l') => {
            view.get_top_layer_mut().cursor.next_column();

            Ok(Some(Transition::Ok))
        }
        KeyCode::Home | KeyCode::Char('g') => {
            view.get_top_layer_mut().cursor.row_move_to_start();

            Ok(Some(Transition::Ok))
        }
        KeyCode::End | KeyCode::Char('G') => {
            view.get_top_layer_mut().cursor.row_move_to_end();

            Ok(Some(Transition::Ok))
        }
        // Try to "drill down" into the selected value
        KeyCode::Enter => {
            let value = view.get_current_value();

            // ...but it only makes sense to drill down into a few types of values
            if !matches!(
                value,
                Value::Record { .. } | Value::List { .. } | Value::Custom { .. }
            ) {
                return Ok(None);
            }

            let is_record = matches!(value, Value::Record { .. });
            let next_layer = create_layer(value.clone())?;
            push_layer(view, next_layer);

            if is_record {
                view.set_top_layer_orientation(Orientation::Left);
            } else {
                view.set_top_layer_orientation(view.orientation);
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
    let layer = view.get_top_layer();
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

/// scroll to the end of the data
fn tail_data(state: &mut RecordView, page_size: usize) {
    let layer = state.get_top_layer_mut();
    let count_rows = layer.record_values.len();
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
        v.get_current_value().clone()
    } else if v.get_top_layer().count_rows() < 2 {
        build_table_as_record(v)
    } else {
        build_table_as_list(v)
    }
}

fn build_table_as_list(v: &RecordView) -> Value {
    let layer = v.get_top_layer();

    let vals = layer
        .record_values
        .iter()
        .map(|vals| {
            let record = layer
                .column_names
                .iter()
                .cloned()
                .zip(vals.iter().cloned())
                .collect();
            Value::record(record, NuSpan::unknown())
        })
        .collect();

    Value::list(vals, NuSpan::unknown())
}

fn build_table_as_record(v: &RecordView) -> Value {
    let layer = v.get_top_layer();

    let mut record = Record::new();
    if let Some(row) = layer.record_values.first() {
        record = layer
            .column_names
            .iter()
            .cloned()
            .zip(row.iter().cloned())
            .collect();
    }

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
    if layer.was_transposed {
        transpose_from(layer);
    } else {
        transpose_to(layer);
    }

    layer.was_transposed = !layer.was_transposed;
}

fn transpose_from(layer: &mut RecordLayer) {
    let count_rows = layer.record_values.len();
    let count_columns = layer.column_names.len();

    if let Some(data) = &mut layer.record_text {
        pop_first_column(data);
        *data = _transpose_table(data, count_rows, count_columns - 1);
    }

    let headers = pop_first_column(&mut layer.record_values);
    let headers = headers
        .into_iter()
        .map(|value| match value {
            Value::String { val, .. } => val,
            _ => unreachable!("must never happen"),
        })
        .collect();

    let data = _transpose_table(&layer.record_values, count_rows, count_columns - 1);

    layer.record_values = data;
    layer.column_names = headers;
}

fn transpose_to(layer: &mut RecordLayer) {
    let count_rows = layer.record_values.len();
    let count_columns = layer.column_names.len();

    if let Some(data) = &mut layer.record_text {
        *data = _transpose_table(data, count_rows, count_columns);
        for (column, column_name) in layer.column_names.iter().enumerate() {
            let value = (column_name.to_owned(), Default::default());
            data[column].insert(0, value);
        }
    }

    let mut data = _transpose_table(&layer.record_values, count_rows, count_columns);
    for (column, column_name) in layer.column_names.iter().enumerate() {
        let value = Value::string(column_name, NuSpan::unknown());
        data[column].insert(0, value);
    }

    layer.record_values = data;
    layer.column_names = (1..count_rows + 1 + 1).map(|i| i.to_string()).collect();
}

fn pop_first_column<T>(values: &mut [Vec<T>]) -> Vec<T>
where
    T: Default + Clone,
{
    let mut data = vec![T::default(); values.len()];
    for (row, values) in values.iter_mut().enumerate() {
        data[row] = values.remove(0);
    }

    data
}

fn _transpose_table<T>(values: &[Vec<T>], count_rows: usize, count_columns: usize) -> Vec<Vec<T>>
where
    T: Clone + Default,
{
    let mut data = vec![vec![T::default(); count_rows]; count_columns];
    for (row, values) in values.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            data[column][row].clone_from(value);
        }
    }

    data
}

fn strip_string(text: &str) -> String {
    String::from_utf8(strip_ansi_escapes::strip(text))
        .map_err(|_| ())
        .unwrap_or_else(|_| text.to_owned())
}
