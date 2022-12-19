use std::io::Result;

use nu_ansi_term::Style;
use nu_color_config::lookup_ansi_color_style;
use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{
    nu_common::collect_input,
    views::{Orientation, RecordView},
};

use super::{
    default_color_list, default_int_list, ConfigOption, HelpExample, HelpManual, Shortcode,
    ViewCommand,
};

#[derive(Debug, Default, Clone)]
pub struct TableCmd {
    // todo: add arguments to override config right from CMD
    settings: TableSettings,
}

#[derive(Debug, Default, Clone)]
struct TableSettings {
    orientation: Option<Orientation>,
    line_head_top: Option<bool>,
    line_head_bottom: Option<bool>,
    line_shift: Option<bool>,
    line_index: Option<bool>,
    split_line_s: Option<Style>,
    selected_cell_s: Option<Style>,
    selected_row_s: Option<Style>,
    selected_column_s: Option<Style>,
    show_cursor: Option<bool>,
    padding_column_left: Option<usize>,
    padding_column_right: Option<usize>,
    padding_index_left: Option<usize>,
    padding_index_right: Option<usize>,
    turn_on_cursor_mode: bool,
}

impl TableCmd {
    pub fn new() -> Self {
        Self::default()
    }

    pub const NAME: &'static str = "table";
}

impl ViewCommand for TableCmd {
    type View = RecordView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        #[rustfmt::skip]
        let shortcuts = vec![
            Shortcode::new("Up",     "",        "Moves the cursor or viewport one row up"),
            Shortcode::new("Down",   "",        "Moves the cursor or viewport one row down"),
            Shortcode::new("Left",   "",        "Moves the cursor or viewport one column left"),
            Shortcode::new("Right",  "",        "Moves the cursor or viewport one column right"),
            Shortcode::new("PgDown", "view",    "Moves the cursor or viewport one page of rows down"),
            Shortcode::new("PgUp",   "view",    "Moves the cursor or viewport one page of rows up"),
            Shortcode::new("Esc",    "",        "Exits cursor mode. Exits the just explored dataset."),
            Shortcode::new("i",      "view",    "Enters cursor mode to inspect individual cells"),
            Shortcode::new("t",      "view",    "Transpose table, so that columns become rows and vice versa"),
            Shortcode::new("e",      "view",    "Open expand view (equvalent of :expand)"),
            Shortcode::new("Enter",  "cursor",  "In cursor mode, explore the data of the selected cell"),
        ];

        #[rustfmt::skip]
        let config_options = vec![
            ConfigOption::new(
                ":table group",
                "Used to move column header",
                "table.orientation",
                vec![
                    HelpExample::new("top", "Sticks column header to the top"),
                    HelpExample::new("bottom", "Sticks column header to the bottom"),
                    HelpExample::new("left", "Sticks column header to the left"),
                    HelpExample::new("right", "Sticks column header to the right"),
                ],
            ),
            ConfigOption::boolean(":table group", "Show index", "table.show_index"),
            ConfigOption::boolean(":table group", "Show header", "table.show_head"),

            ConfigOption::boolean(":table group", "Lines are lines", "table.line_head_top"),
            ConfigOption::boolean(":table group", "Lines are lines", "table.line_head_bottom"),
            ConfigOption::boolean(":table group", "Lines are lines", "table.line_shift"),
            ConfigOption::boolean(":table group", "Lines are lines", "table.line_index"),

            ConfigOption::boolean(":table group", "Show cursor", "table.show_cursor"),

            ConfigOption::new(":table group", "Color of selected cell", "table.selected_cell", default_color_list()),
            ConfigOption::new(":table group", "Color of selected row", "table.selected_row", default_color_list()),
            ConfigOption::new(":table group", "Color of selected column", "table.selected_column", default_color_list()),

            ConfigOption::new(":table group", "Color of split line", "table.split_line", default_color_list()),

            ConfigOption::new(":table group", "Padding column left", "table.padding_column_left", default_int_list()),
            ConfigOption::new(":table group", "Padding column right", "table.padding_column_right", default_int_list()),
            ConfigOption::new(":table group", "Padding index left", "table.padding_index_left", default_int_list()),
            ConfigOption::new(":table group", "Padding index right", "table.padding_index_right", default_int_list()),
        ];

        Some(HelpManual {
            name: "table",
            description: "Display a table view",
            arguments: vec![],
            examples: vec![],
            config_options,
            input: shortcuts,
        })
    }

    fn display_config_option(&mut self, _group: String, key: String, value: String) -> bool {
        match key.as_str() {
            "table.orientation" => self.settings.orientation = orientation_from_str(&value),
            "table.line_head_top" => self.settings.line_head_top = bool_from_str(&value),
            "table.line_head_bottom" => self.settings.line_head_bottom = bool_from_str(&value),
            "table.line_shift" => self.settings.line_shift = bool_from_str(&value),
            "table.line_index" => self.settings.line_index = bool_from_str(&value),
            "table.show_cursor" => {
                self.settings.show_cursor = bool_from_str(&value);
                self.settings.turn_on_cursor_mode = true;
            }
            "table.split_line" => {
                self.settings.split_line_s = Some(lookup_ansi_color_style(&value));
                self.settings.turn_on_cursor_mode = true;
            }
            "table.selected_cell" => {
                self.settings.selected_cell_s = Some(lookup_ansi_color_style(&value));
                self.settings.turn_on_cursor_mode = true;
            }
            "table.selected_row" => {
                self.settings.selected_row_s = Some(lookup_ansi_color_style(&value));
                self.settings.turn_on_cursor_mode = true;
            }
            "table.selected_column" => {
                self.settings.selected_column_s = Some(lookup_ansi_color_style(&value));
                self.settings.turn_on_cursor_mode = true;
            }
            "table.padding_column_left" => {
                self.settings.padding_column_left = usize_from_str(&value);
            }
            "table.padding_column_right" => {
                self.settings.padding_column_right = usize_from_str(&value);
            }
            "table.padding_index_left" => {
                self.settings.padding_index_left = usize_from_str(&value);
            }
            "table.padding_index_right" => {
                self.settings.padding_index_right = usize_from_str(&value);
            }
            _ => return false,
        }

        true
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let is_record = matches!(value, Value::Record { .. });

        let (columns, data) = collect_input(value);

        let mut view = RecordView::new(columns, data);

        // todo: use setup instead ????

        if is_record {
            view.set_orientation_current(Orientation::Left);
        }

        if let Some(o) = self.settings.orientation {
            view.set_orientation_current(o);
        }

        if self.settings.line_head_bottom.unwrap_or(false) {
            view.set_line_head_bottom(true);
        }

        if self.settings.line_head_top.unwrap_or(false) {
            view.set_line_head_top(true);
        }

        if self.settings.line_index.unwrap_or(false) {
            view.set_line_index(true);
        }

        if self.settings.line_shift.unwrap_or(false) {
            view.set_line_trailing(true);
        }

        if self.settings.show_cursor.unwrap_or(false) {
            view.show_cursor(true);
        }

        if let Some(style) = self.settings.selected_cell_s {
            view.set_style_selected_cell(style);
        }

        if let Some(style) = self.settings.selected_column_s {
            view.set_style_selected_column(style);
        }

        if let Some(style) = self.settings.selected_row_s {
            view.set_style_selected_row(style);
        }

        if let Some(style) = self.settings.split_line_s {
            view.set_style_split_line(style);
        }

        if let Some(p) = self.settings.padding_column_left {
            let c = view.get_padding_column();
            view.set_padding_column((p, c.1))
        }

        if let Some(p) = self.settings.padding_column_right {
            let c = view.get_padding_column();
            view.set_padding_column((c.0, p))
        }

        if let Some(p) = self.settings.padding_index_left {
            let c = view.get_padding_index();
            view.set_padding_index((p, c.1))
        }

        if let Some(p) = self.settings.padding_index_right {
            let c = view.get_padding_index();
            view.set_padding_index((c.0, p))
        }

        if self.settings.turn_on_cursor_mode {
            view.set_cursor_mode();
        }

        Ok(view)
    }
}

fn bool_from_str(s: &str) -> Option<bool> {
    match s {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn usize_from_str(s: &str) -> Option<usize> {
    s.parse::<usize>().ok()
}

fn orientation_from_str(s: &str) -> Option<Orientation> {
    match s {
        "left" => Some(Orientation::Left),
        "right" => Some(Orientation::Right),
        "top" => Some(Orientation::Top),
        "bottom" => Some(Orientation::Bottom),
        _ => None,
    }
}
