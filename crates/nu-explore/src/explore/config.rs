//! Configuration types for the explore command.

use crate::explore::nu_common::create_map;
use nu_ansi_term::{Color, Style};
use nu_color_config::get_color_map;
use nu_protocol::Config;

#[derive(Debug, Clone)]
pub struct ExploreConfig {
    pub table: TableConfig,
    pub selected_cell: Style,
    pub status_info: Style,
    pub status_success: Style,
    pub status_warn: Style,
    pub status_error: Style,
    pub status_bar_background: Style,
    pub status_bar_text: Style,
    pub cmd_bar_text: Style,
    pub cmd_bar_background: Style,
    pub highlight: Style,
    pub title_bar_background: Style,
    pub title_bar_text: Style,
    /// if true, the explore view will immediately try to run the command as it is typed
    pub try_reactive: bool,
}

impl Default for ExploreConfig {
    fn default() -> Self {
        Self {
            table: TableConfig::default(),
            selected_cell: color(None, Some(Color::LightBlue)),
            status_info: color(None, None),
            status_success: color(Some(Color::Black), Some(Color::Green)),
            status_warn: color(None, None),
            status_error: color(Some(Color::White), Some(Color::Red)),
            // Use None to inherit from terminal/nushell theme
            status_bar_background: color(None, None),
            status_bar_text: color(None, None),
            cmd_bar_text: color(None, None),
            cmd_bar_background: color(None, None),
            highlight: color(Some(Color::Black), Some(Color::Yellow)),
            // Use None to inherit from terminal/nushell theme
            title_bar_background: color(None, None),
            title_bar_text: color(None, None),
            try_reactive: false,
        }
    }
}

impl ExploreConfig {
    /// take the default explore config and update it with relevant values from the nu config
    pub fn from_nu_config(config: &Config) -> Self {
        let mut ret = Self::default();

        ret.table.column_padding_left = config.table.padding.left;
        ret.table.column_padding_right = config.table.padding.right;

        let explore_cfg_hash_map = config.explore.clone();
        let colors = get_color_map(&explore_cfg_hash_map);

        if let Some(s) = colors.get("status_bar_text") {
            ret.status_bar_text = *s;
        }

        if let Some(s) = colors.get("status_bar_background") {
            ret.status_bar_background = *s;
        }

        if let Some(s) = colors.get("command_bar_text") {
            ret.cmd_bar_text = *s;
        }

        if let Some(s) = colors.get("command_bar_background") {
            ret.cmd_bar_background = *s;
        }

        if let Some(s) = colors.get("command_bar_background") {
            ret.cmd_bar_background = *s;
        }

        if let Some(s) = colors.get("selected_cell") {
            ret.selected_cell = *s;
        }

        if let Some(s) = colors.get("title_bar_text") {
            ret.title_bar_text = *s;
        }

        if let Some(s) = colors.get("title_bar_background") {
            ret.title_bar_background = *s;
        }

        if let Some(hm) = explore_cfg_hash_map.get("status").and_then(create_map) {
            let colors = get_color_map(&hm);

            if let Some(s) = colors.get("info") {
                ret.status_info = *s;
            }

            if let Some(s) = colors.get("success") {
                ret.status_success = *s;
            }

            if let Some(s) = colors.get("warn") {
                ret.status_warn = *s;
            }

            if let Some(s) = colors.get("error") {
                ret.status_error = *s;
            }
        }

        if let Some(hm) = explore_cfg_hash_map.get("try").and_then(create_map)
            && let Some(reactive) = hm.get("reactive")
            && let Ok(b) = reactive.as_bool()
        {
            ret.try_reactive = b;
        }

        ret
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TableConfig {
    pub separator_style: Style,
    pub show_index: bool,
    pub show_header: bool,
    pub column_padding_left: usize,
    pub column_padding_right: usize,
}

const fn color(foreground: Option<Color>, background: Option<Color>) -> Style {
    Style {
        background,
        foreground,
        is_blink: false,
        is_bold: false,
        is_dimmed: false,
        is_hidden: false,
        is_italic: false,
        is_reverse: false,
        is_strikethrough: false,
        is_underline: false,
        prefix_with_reset: false,
    }
}
