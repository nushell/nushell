//! Module containing the internal representation of user configuration

use crate::FromValue;
use crate::{self as nu_protocol};
use helper::*;
use prelude::*;
use std::collections::HashMap;

pub use ansi_coloring::UseAnsiColoring;
pub use completions::{
    CompletionAlgorithm, CompletionConfig, CompletionSort, ExternalCompleterConfig,
};
pub use datetime_format::DatetimeFormatConfig;
pub use display_errors::DisplayErrors;
pub use filesize::FilesizeConfig;
pub use helper::extract_value;
pub use history::{HistoryConfig, HistoryFileFormat};
pub use hooks::Hooks;
pub use ls::LsConfig;
pub use output::{BannerKind, ErrorStyle};
pub use plugin_gc::{PluginGcConfig, PluginGcConfigs};
pub use reedline::{CursorShapeConfig, EditBindings, NuCursorShape, ParsedKeybinding, ParsedMenu};
pub use rm::RmConfig;
pub use shell_integration::ShellIntegrationConfig;
pub use table::{FooterMode, TableConfig, TableIndent, TableIndexMode, TableMode, TrimStrategy};

mod ansi_coloring;
mod completions;
mod datetime_format;
mod display_errors;
mod error;
mod filesize;
mod helper;
mod history;
mod hooks;
mod ls;
mod output;
mod plugin_gc;
mod prelude;
mod reedline;
mod rm;
mod shell_integration;
mod table;

#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct Config {
    pub filesize: FilesizeConfig,
    pub table: TableConfig,
    pub ls: LsConfig,
    pub color_config: HashMap<String, Value>,
    pub footer_mode: FooterMode,
    pub float_precision: i64,
    pub recursion_limit: i64,
    pub use_ansi_coloring: UseAnsiColoring,
    pub completions: CompletionConfig,
    pub edit_mode: EditBindings,
    pub history: HistoryConfig,
    pub keybindings: Vec<ParsedKeybinding>,
    pub menus: Vec<ParsedMenu>,
    pub hooks: Hooks,
    pub rm: RmConfig,
    pub shell_integration: ShellIntegrationConfig,
    pub buffer_editor: Value,
    pub show_banner: BannerKind,
    pub bracketed_paste: bool,
    pub render_right_prompt_on_last_line: bool,
    pub explore: HashMap<String, Value>,
    pub cursor_shape: CursorShapeConfig,
    pub datetime_format: DatetimeFormatConfig,
    pub error_style: ErrorStyle,
    pub display_errors: DisplayErrors,
    pub use_kitty_protocol: bool,
    pub highlight_resolved_externals: bool,
    /// Configuration for plugins.
    ///
    /// Users can provide configuration for a plugin through this entry.  The entry name must
    /// match the registered plugin name so `plugin add nu_plugin_example` will be able to place
    /// its configuration under a `nu_plugin_example` column.
    pub plugins: HashMap<String, Value>,
    /// Configuration for plugin garbage collection.
    pub plugin_gc: PluginGcConfigs,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            show_banner: BannerKind::default(),

            table: TableConfig::default(),
            rm: RmConfig::default(),
            ls: LsConfig::default(),

            datetime_format: DatetimeFormatConfig::default(),

            explore: HashMap::new(),

            history: HistoryConfig::default(),

            completions: CompletionConfig::default(),

            recursion_limit: 50,

            filesize: FilesizeConfig::default(),

            cursor_shape: CursorShapeConfig::default(),

            color_config: HashMap::new(),
            footer_mode: FooterMode::RowCount(25),
            float_precision: 2,
            buffer_editor: Value::nothing(Span::unknown()),
            use_ansi_coloring: UseAnsiColoring::default(),
            bracketed_paste: true,
            edit_mode: EditBindings::default(),

            shell_integration: ShellIntegrationConfig::default(),

            render_right_prompt_on_last_line: false,

            hooks: Hooks::new(),

            menus: Vec::new(),

            keybindings: Vec::new(),

            error_style: ErrorStyle::Fancy,
            display_errors: DisplayErrors::default(),

            use_kitty_protocol: false,
            highlight_resolved_externals: false,

            plugins: HashMap::new(),
            plugin_gc: PluginGcConfigs::default(),
        }
    }
}

impl UpdateFromValue for Config {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "ls" => self.ls.update(val, path, errors),
                "rm" => self.rm.update(val, path, errors),
                "history" => self.history.update(val, path, errors),
                "completions" => self.completions.update(val, path, errors),
                "cursor_shape" => self.cursor_shape.update(val, path, errors),
                "table" => self.table.update(val, path, errors),
                "filesize" => self.filesize.update(val, path, errors),
                "explore" => self.explore.update(val, path, errors),
                "color_config" => self.color_config.update(val, path, errors),
                "footer_mode" => self.footer_mode.update(val, path, errors),
                "float_precision" => self.float_precision.update(val, path, errors),
                "use_ansi_coloring" => self.use_ansi_coloring.update(val, path, errors),
                "edit_mode" => self.edit_mode.update(val, path, errors),
                "shell_integration" => self.shell_integration.update(val, path, errors),
                "buffer_editor" => match val {
                    Value::Nothing { .. } | Value::String { .. } => {
                        self.buffer_editor = val.clone();
                    }
                    Value::List { vals, .. }
                        if vals.iter().all(|val| matches!(val, Value::String { .. })) =>
                    {
                        self.buffer_editor = val.clone();
                    }
                    _ => errors.type_mismatch(
                        path,
                        Type::custom("string, list<string>, or nothing"),
                        val,
                    ),
                },
                "show_banner" => self.show_banner.update(val, path, errors),
                "display_errors" => self.display_errors.update(val, path, errors),
                "render_right_prompt_on_last_line" => self
                    .render_right_prompt_on_last_line
                    .update(val, path, errors),
                "bracketed_paste" => self.bracketed_paste.update(val, path, errors),
                "use_kitty_protocol" => self.use_kitty_protocol.update(val, path, errors),
                "highlight_resolved_externals" => {
                    self.highlight_resolved_externals.update(val, path, errors)
                }
                "plugins" => self.plugins.update(val, path, errors),
                "plugin_gc" => self.plugin_gc.update(val, path, errors),
                "menus" => match Vec::from_value(val.clone()) {
                    Ok(menus) => self.menus = menus,
                    Err(err) => errors.error(err.into()),
                },
                "keybindings" => match Vec::from_value(val.clone()) {
                    Ok(keybindings) => self.keybindings = keybindings,
                    Err(err) => errors.error(err.into()),
                },
                "hooks" => self.hooks.update(val, path, errors),
                "datetime_format" => self.datetime_format.update(val, path, errors),
                "error_style" => self.error_style.update(val, path, errors),
                "recursion_limit" => {
                    if let Ok(limit) = val.as_int() {
                        if limit > 1 {
                            self.recursion_limit = limit;
                        } else {
                            errors.invalid_value(path, "an int greater than 1", val);
                        }
                    } else {
                        errors.type_mismatch(path, Type::Int, val);
                    }
                }
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

impl Config {
    pub fn update_from_value(
        &mut self,
        old: &Config,
        value: &Value,
    ) -> Result<Option<ShellWarning>, ShellError> {
        // Current behaviour is that config errors are displayed, but do not prevent the rest
        // of the config from being updated (fields with errors are skipped/not updated).
        // Errors are simply collected one-by-one and wrapped into a ShellError variant at the end.
        let mut errors = ConfigErrors::new(old);
        let mut path = ConfigPath::new();

        self.update(value, &mut path, &mut errors);

        errors.check()
    }
}
