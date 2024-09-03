//! Module containing the internal representation of user configuration

use crate as nu_protocol;
use crate::FromValue;
use helper::*;
use prelude::*;
use std::collections::HashMap;

pub use completions::{
    CompletionAlgorithm, CompletionConfig, CompletionSort, ExternalCompleterConfig,
};
pub use datetime_format::DatetimeFormatConfig;
pub use filesize::FilesizeConfig;
pub use helper::extract_value;
pub use history::{HistoryConfig, HistoryFileFormat};
pub use hooks::Hooks;
pub use ls::LsConfig;
pub use output::ErrorStyle;
pub use plugin_gc::{PluginGcConfig, PluginGcConfigs};
pub use reedline::{CursorShapeConfig, EditBindings, NuCursorShape, ParsedKeybinding, ParsedMenu};
pub use rm::RmConfig;
pub use shell_integration::ShellIntegrationConfig;
pub use table::{FooterMode, TableConfig, TableIndexMode, TableMode, TrimStrategy};

mod completions;
mod datetime_format;
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
    pub use_ansi_coloring: bool,
    pub completions: CompletionConfig,
    pub edit_mode: EditBindings,
    pub history: HistoryConfig,
    pub keybindings: Vec<ParsedKeybinding>,
    pub menus: Vec<ParsedMenu>,
    pub hooks: Hooks,
    pub rm: RmConfig,
    pub shell_integration: ShellIntegrationConfig,
    pub buffer_editor: Value,
    pub show_banner: bool,
    pub bracketed_paste: bool,
    pub render_right_prompt_on_last_line: bool,
    pub explore: HashMap<String, Value>,
    pub cursor_shape: CursorShapeConfig,
    pub datetime_format: DatetimeFormatConfig,
    pub error_style: ErrorStyle,
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
            show_banner: true,

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
            use_ansi_coloring: true,
            bracketed_paste: true,
            edit_mode: EditBindings::default(),

            shell_integration: ShellIntegrationConfig::default(),

            render_right_prompt_on_last_line: false,

            hooks: Hooks::new(),

            menus: Vec::new(),

            keybindings: Vec::new(),

            error_style: ErrorStyle::Fancy,

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
        errors: &mut Vec<ShellError>,
    ) {
        let span = value.span();
        let Value::Record { val: record, .. } = value else {
            report_invalid_config_value("should be a record", span, path, errors);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let span = val.span();
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
                "use_grid_icons" => {
                    // TODO: delete it after 0.99
                    report_invalid_value(
                        "`use_grid_icons` is deleted, you should delete the key, and use `grid -i` in such case.",
                        span,
                        &mut errors
                    );
                }
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
                    _ => {
                        report_invalid_config_value(
                            "should be a string, list<string>, or null",
                            span,
                            path,
                            errors,
                        );
                    }
                },
                "show_banner" => self.show_banner.update(val, path, errors),
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
                    Err(e) => {
                        report_invalid_config_value(
                            "should be a valid list of menus",
                            span,
                            path,
                            errors,
                        );
                        errors.push(e);
                    }
                },
                "keybindings" => match Vec::from_value(val.clone()) {
                    Ok(keybindings) => self.keybindings = keybindings,
                    Err(e) => {
                        report_invalid_config_value(
                            "should be a valid keybindings list",
                            span,
                            path,
                            errors,
                        );
                        errors.push(e);
                    }
                },
                "hooks" => self.hooks.update(val, path, errors),
                "datetime_format" => self.datetime_format.update(val, path, errors),
                "error_style" => self.error_style.update(val, path, errors),
                "recursion_limit" => {
                    if let &Value::Int { val, .. } = val {
                        if val > 1 {
                            self.recursion_limit = val;
                        } else {
                            report_invalid_config_value(
                                "should be an integer greater than 1",
                                span,
                                path,
                                errors,
                            );
                        }
                    } else {
                        report_invalid_config_value(
                            "should be an integer greater than 1",
                            span,
                            path,
                            errors,
                        );
                    }
                }
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}

impl Config {
    pub fn update_from_value(&mut self, value: &Value) -> Option<ShellError> {
        // Vec for storing errors. Current Nushell behaviour (Dec 2022) is that having some typo
        // like `"always_trash": tru` in your config.nu's `$env.config` record shouldn't abort all
        // config parsing there and then. Thus, errors are simply collected one-by-one and wrapped
        // in a GenericError at the end.
        let mut errors = Vec::new();
        let mut path = ConfigPath::new();

        self.update(value, &mut path, &mut errors);

        if !errors.is_empty() {
            Some(ShellError::GenericError {
                error: "Config record contains invalid values or unknown settings".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: errors,
            })
        } else {
            None
        }
    }
}
