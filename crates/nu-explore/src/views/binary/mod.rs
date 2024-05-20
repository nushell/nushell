// todo: 3 cursor modes one for section

mod binary_widget;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nu_color_config::get_color_map;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};
use ratatui::layout::Rect;

use crate::{
    nu_common::NuText,
    pager::{
        report::{Report, Severity},
        ConfigMap, Frame, Transition, ViewInfo,
    },
    util::create_map,
};

use self::binary_widget::{BinarySettings, BinaryStyle, BinaryWidget};

use super::{cursor::XYCursor, Layout, View, ViewConfig};

#[derive(Debug, Clone)]
pub struct BinaryView {
    data: Vec<u8>,
    cursor: XYCursor,
    settings: Settings,
}

#[derive(Debug, Default, Clone)]
struct Settings {
    opts: BinarySettings,
    style: BinaryStyle,
}

impl BinaryView {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            cursor: XYCursor::default(),
            settings: Settings::default(),
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
    ) -> Option<Transition> {
        let result = handle_event_view_mode(self, &key);

        if matches!(&result, Some(Transition::Ok)) {
            let report = create_report(self.cursor);
            info.status = Some(report);
        }

        None
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

    fn setup(&mut self, cfg: ViewConfig<'_>) {
        let hm = match cfg.config.get("hex-dump").and_then(create_map) {
            Some(hm) => hm,
            None => return,
        };

        self.settings = settings_from_config(&hm);

        let count_rows =
            BinaryWidget::new(&self.data, self.settings.opts, Default::default()).count_lines();
        self.cursor = XYCursor::new(count_rows, 0);
    }
}

fn create_binary_widget(v: &BinaryView) -> BinaryWidget<'_> {
    let start_line = v.cursor.row_starts_at();
    let count_elements =
        BinaryWidget::new(&[], v.settings.opts, Default::default()).count_elements();
    let index = start_line * count_elements;
    let data = &v.data[index..];

    let mut w = BinaryWidget::new(data, v.settings.opts, v.settings.style.clone());
    w.set_row_offset(index);

    w
}

fn handle_event_view_mode(view: &mut BinaryView, key: &KeyEvent) -> Option<Transition> {
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
            view.cursor.prev_row_page();

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
            view.cursor.next_row_page();

            return Some(Transition::Ok);
        }
        _ => {}
    }

    match key.code {
        KeyCode::Esc => Some(Transition::Exit),
        KeyCode::Up | KeyCode::Char('k') => {
            view.cursor.prev_row_i();

            Some(Transition::Ok)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            view.cursor.next_row_i();

            Some(Transition::Ok)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            view.cursor.prev_column_i();

            Some(Transition::Ok)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            view.cursor.next_column_i();

            Some(Transition::Ok)
        }
        KeyCode::Home | KeyCode::Char('g') => {
            view.cursor.row_move_to_start();

            Some(Transition::Ok)
        }
        KeyCode::End | KeyCode::Char('G') => {
            view.cursor.row_move_to_end();

            Some(Transition::Ok)
        }
        _ => None,
    }
}

fn settings_from_config(config: &ConfigMap) -> Settings {
    let colors = get_color_map(config);

    Settings {
        opts: BinarySettings::new(
            config_get_usize(config, "segment_size", 2),
            config_get_usize(config, "count_segments", 8),
        ),
        style: BinaryStyle::new(
            colors.get("color_index").cloned(),
            config_get_usize(config, "column_padding_left", 1) as u16,
            config_get_usize(config, "column_padding_right", 1) as u16,
        ),
    }
}

fn config_get_usize(config: &ConfigMap, key: &str, default: usize) -> usize {
    config
        .get(key)
        .and_then(|v| v.coerce_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}

fn create_report(cursor: XYCursor) -> Report {
    let covered_percent = report_row_position(cursor);
    let cursor = report_cursor_position(cursor);
    let mode = report_mode_name();
    let msg = String::new();

    Report::new(msg, Severity::Info, mode, cursor, covered_percent)
}

fn report_mode_name() -> String {
    String::from("VIEW")
}

fn report_row_position(cursor: XYCursor) -> String {
    if cursor.row_starts_at() == 0 {
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

fn report_cursor_position(cursor: XYCursor) -> String {
    let rows_seen = cursor.row_starts_at();
    let columns_seen = cursor.column_starts_at();
    format!("{rows_seen},{columns_seen}")
}

fn get_percentage(value: usize, max: usize) -> usize {
    debug_assert!(value <= max, "{value:?} {max:?}");

    ((value as f32 / max as f32) * 100.0).floor() as usize
}
