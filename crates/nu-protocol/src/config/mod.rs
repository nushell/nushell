//! Module containing the internal representation of user configuration
use self::helper::*;

use crate::{FromValue, IntoValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use self::completions::{
    CompletionAlgorithm, CompletionConfig, CompletionSort, ExternalCompleterConfig,
};
pub use self::datetime_format::DatetimeFormatConfig;
pub use self::filesize::FilesizeConfig;
pub use self::helper::extract_value;
pub use self::history::{HistoryConfig, HistoryFileFormat};
pub use self::hooks::Hooks;
pub use self::ls::LsConfig;
pub use self::output::ErrorStyle;
pub use self::plugin_gc::{PluginGcConfig, PluginGcConfigs};
pub use self::reedline::{
    CursorShapeConfig, EditBindings, NuCursorShape, ParsedKeybinding, ParsedMenu,
};
pub use self::rm::RmConfig;
pub use self::shell_integration::ShellIntegrationConfig;
pub use self::table::{FooterMode, TableConfig, TableIndexMode, TableMode, TrimStrategy};

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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

impl Value {
    /// Parse the given [`Value`] as a configuration record, and recover encountered mistakes
    ///
    /// If any given (sub)value is detected as impossible, this value will be restored to the value
    /// in `existing_config`, thus mutates `self`.
    ///
    /// Returns a new [`Config`] (that is in a valid state) and if encountered the [`ShellError`]
    /// containing all observed inner errors.
    pub fn parse_as_config(&mut self, existing_config: &Config) -> (Config, Option<ShellError>) {
        // Clone the passed-in config rather than mutating it.
        let mut config = existing_config.clone();

        // Vec for storing errors. Current Nushell behaviour (Dec 2022) is that having some typo
        // like `"always_trash": tru` in your config.nu's `$env.config` record shouldn't abort all
        // config parsing there and then. Thus, errors are simply collected one-by-one and wrapped
        // in a GenericError at the end.
        let mut errors = vec![];

        // Config record (self) mutation rules:
        // * When parsing a config Record, if a config key error occurs, remove the key.
        // * When parsing a config Record, if a config value error occurs, replace the value
        // with a reconstructed Nu value for the current (unaltered) configuration for that setting.
        // For instance:
        // `$env.config.ls.use_ls_colors = 2` results in an error, so the current `use_ls_colors`
        // config setting is converted to a `Value::Boolean` and inserted in the record in place of
        // the `2`.

        let Value::Record { val, .. } = self else {
            return (
                config,
                Some(ShellError::GenericError {
                    error: "Error while applying config changes".into(),
                    msg: "$env.config is not a record".into(),
                    span: Some(self.span()),
                    help: None,
                    inner: vec![],
                }),
            );
        };

        val.to_mut().retain_mut(|key, value| {
            let mut path = ConfigPath::new();
            let path = &mut path.push(key);
            let span = value.span();
            match key {
                "ls" => {
                    config.ls.update(value, path, &mut errors);
                }
                "rm" => {
                    config.rm.update(value, path, &mut errors);
                }
                "history" => {
                    config.history.update(value, path, &mut errors);
                }
                "completions" => {
                    config.completions.update(value, path, &mut errors);
                }
                "cursor_shape" => {
                    config.cursor_shape.update(value, path, &mut errors);
                }
                "table" => {
                    config.table.update(value, path, &mut errors);
                }
                "filesize" => {
                    config.filesize.update(value, path, &mut errors);
                }
                "explore" => {
                    if let Ok(map) = create_map(value) {
                        config.explore = map;
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.explore.clone().into_value(span);
                    }
                }
                // Misc. options
                "color_config" => {
                    if let Ok(map) = create_map(value) {
                        config.color_config = map;
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.color_config.clone().into_value(span);
                    }
                }
                "use_grid_icons" => {
                    // TODO: delete it after 0.99
                    report_invalid_value(
                        "`use_grid_icons` is deleted, you should delete the key, and use `grid -i` in such case.",
                        span,
                        &mut errors
                    );
                }
                "footer_mode" => {
                    process_string_enum(&mut config.footer_mode, &[key], value, &mut errors);
                }
                "float_precision" => {
                    process_int_config(value, &mut errors, &mut config.float_precision);
                }
                "use_ansi_coloring" => {
                    process_bool_config(value, &mut errors, &mut config.use_ansi_coloring);
                }
                "edit_mode" => {
                    process_string_enum(&mut config.edit_mode, &[key], value, &mut errors);
                }
                "shell_integration" => {
                    config.shell_integration.update(value, path, &mut errors);
                }
                "buffer_editor" => match value {
                    Value::Nothing { .. } | Value::String { .. } => {
                        config.buffer_editor = value.clone();
                    }
                    Value::List { vals, .. }
                        if vals.iter().all(|val| matches!(val, Value::String { .. })) =>
                    {
                        config.buffer_editor = value.clone();
                    }
                    _ => {
                        report_invalid_value(
                            "should be a string, list<string>, or null",
                            span,
                            &mut errors,
                        );
                        *value = config.buffer_editor.clone();
                    }
                },
                "show_banner" => {
                    process_bool_config(value, &mut errors, &mut config.show_banner);
                }
                "render_right_prompt_on_last_line" => {
                    process_bool_config(
                        value,
                        &mut errors,
                        &mut config.render_right_prompt_on_last_line,
                    );
                }
                "bracketed_paste" => {
                    process_bool_config(value, &mut errors, &mut config.bracketed_paste);
                }
                "use_kitty_protocol" => {
                    process_bool_config(value, &mut errors, &mut config.use_kitty_protocol);
                }
                "highlight_resolved_externals" => {
                    process_bool_config(
                        value,
                        &mut errors,
                        &mut config.highlight_resolved_externals,
                    );
                }
                "plugins" => {
                    if let Ok(map) = create_map(value) {
                        config.plugins = map;
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.plugins.clone().into_value(span);
                    }
                }
                "plugin_gc" => {
                    config.plugin_gc.update(value, path, &mut errors);
                }
                "menus" => match Vec::from_value(value.clone()) {
                    Ok(menus) => config.menus = menus,
                    Err(e) => {
                        report_invalid_config_value(
                            "should be a valid list of menus",
                            span,
                            &path.push("menus"),
                            &mut errors,
                        );
                        errors.push(e);
                    }
                },
                "keybindings" => match Vec::from_value(value.clone()) {
                    Ok(keybindings) => config.keybindings = keybindings,
                    Err(e) => {
                        report_invalid_config_value(
                            "should be a valid keybindings list",
                            span,
                            &path.push("keybindings"),
                            &mut errors,
                        );
                        errors.push(e);
                    }
                },
                "hooks" => {
                    config.hooks.update(value, path, &mut errors);
                }
                "datetime_format" => {
                    config.datetime_format.update(value, path, &mut errors);
                }
                "error_style" => {
                    process_string_enum(&mut config.error_style, &[key], value, &mut errors);
                }
                "recursion_limit" => {
                    if let Value::Int { val, internal_span } = value {
                        if val > &mut 1 {
                            config.recursion_limit = *val;
                        } else {
                            report_invalid_value(
                                "should be a integer greater than 1",
                                span,
                                &mut errors,
                            );
                            *value = Value::Int {
                                val: 50,
                                internal_span: *internal_span,
                            };
                        }
                    } else {
                        report_invalid_value(
                            "should be a integer greater than 1",
                            span,
                            &mut errors,
                        );
                        *value = Value::Int {
                            val: 50,
                            internal_span: value.span(),
                        };
                    }
                }
                // Catch all
                _ => {
                    report_invalid_key(&[key], span, &mut errors);
                    return false;
                }
            };
            true
        });

        // Return the config and the vec of errors.
        (
            config,
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
            },
        )
    }
}
