// todo: 3 cursor modes one for section

mod binary_widget;

use crossterm::event::KeyEvent;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use ratatui::layout::Rect;

use crate::{
    explore::ExploreConfig,
    nu_common::NuText,
    pager::{
        report::{Report, Severity},
        Frame, Transition, ViewInfo,
    },
    views::cursor::Position,
};

use self::binary_widget::{BinarySettings, BinaryStyle, BinaryWidget};

use super::{cursor::CursorMoveHandler, cursor::WindowCursor2D, Layout, View, ViewConfig};

/// An interactive view that displays binary data in a hex dump format.
/// Not finished; many aspects are still WIP.
#[derive(Debug, Clone)]
pub struct BinaryView {
    data: Vec<u8>,
    // HACK: we are only using the vertical dimension of the cursor, should we use a plain old WindowCursor?
    cursor: WindowCursor2D,
    settings: Settings,
}

#[derive(Debug, Default, Clone)]
struct Settings {
    opts: BinarySettings,
    style: BinaryStyle,
}

impl BinaryView {
    pub fn new(data: Vec<u8>, cfg: &ExploreConfig) -> Self {
        let settings = settings_from_config(cfg);
        // There's gotta be a nicer way of doing this than creating a widget just to count lines
        let count_rows = BinaryWidget::new(&data, settings.opts, Default::default()).count_lines();

        Self {
            data,
            cursor: WindowCursor2D::new(count_rows, 1).expect("Failed to create XYCursor"),
            settings,
        }
    }
}

impl View for BinaryView {
    fn draw(&mut self, f: &mut Frame, area: Rect, _cfg: ViewConfig<'_>, _layout: &mut Layout) {
        let widget = create_binary_widget(self);
        f.render_widget(widget, area);
    }

    fn handle_input(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: &Layout,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Transition {
        // currently only handle_enter() in crates/nu-explore/src/views/record/mod.rs raises an Err()
        if let Ok((Transition::Ok, ..)) = self.handle_input_key(&key) {
            let report = create_report(self.cursor);
            info.status = Some(report);
        }

        Transition::None
    }

    fn collect_data(&self) -> Vec<NuText> {
        // todo: impl to allow search
        vec![]
    }

    fn show_data(&mut self, _pos: usize) -> bool {
        // todo: impl to allow search
        false
    }

    fn exit(&mut self) -> Option<Value> {
        // todo: impl Cursor + peek of a value
        None
    }
}

impl CursorMoveHandler for BinaryView {
    fn get_cursor(&mut self) -> &mut WindowCursor2D {
        &mut self.cursor
    }
}

fn create_binary_widget(v: &BinaryView) -> BinaryWidget<'_> {
    let start_line = v.cursor.window_origin().row;
    let count_elements =
        BinaryWidget::new(&[], v.settings.opts, Default::default()).count_elements();
    let index = start_line * count_elements;
    let data = &v.data[index..];

    let mut w = BinaryWidget::new(data, v.settings.opts, v.settings.style.clone());
    w.set_row_offset(index);

    w
}

fn settings_from_config(config: &ExploreConfig) -> Settings {
    // Most of this is hardcoded for now, add it to the config later if needed
    Settings {
        opts: BinarySettings::new(2, 8),
        style: BinaryStyle::new(
            None,
            config.table.column_padding_left as u16,
            config.table.column_padding_right as u16,
        ),
    }
}

fn create_report(cursor: WindowCursor2D) -> Report {
    let covered_percent = report_row_position(cursor);
    let cursor = report_cursor_position(cursor);
    let mode = report_mode_name();
    let msg = String::new();

    Report::new(msg, Severity::Info, mode, cursor, covered_percent)
}

fn report_mode_name() -> String {
    String::from("VIEW")
}

fn report_row_position(cursor: WindowCursor2D) -> String {
    if cursor.window_origin().row == 0 {
        return String::from("Top");
    }

    // todo: there's some bug in XYCursor; when we hit PgDOWN/UP and general move it exceeds the limit
    //       not sure when it was introduced and if present in original view.
    //       but it just requires a refactoring as these method names are just ..... not perfect.
    let row = cursor.row().min(cursor.row_limit());
    let count_rows = cursor.row_limit();
    let percent_rows = get_percentage(row, count_rows);
    match percent_rows {
        100 => String::from("All"),
        value => format!("{value}%"),
    }
}

fn report_cursor_position(cursor: WindowCursor2D) -> String {
    let Position { row, column } = cursor.window_origin();
    format!("{row},{column}")
}

fn get_percentage(value: usize, max: usize) -> usize {
    debug_assert!(value <= max, "{value:?} {max:?}");

    ((value as f32 / max as f32) * 100.0).floor() as usize
}
