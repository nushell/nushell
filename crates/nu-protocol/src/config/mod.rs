//! Module containing the internal representation of user configuration
use self::helper::*;
use self::hooks::*;

use crate::{IntoValue, ShellError, Span, Value};
use reedline::create_keybindings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use table::try_parse_trim_strategy;

pub use self::completer::{
    CompleterConfig, CompletionAlgorithm, CompletionSort, ExternalCompleterConfig,
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
    create_menus, CursorShapeConfig, EditBindings, NuCursorShape, ParsedKeybinding, ParsedMenu,
};
pub use self::rm::RmConfig;
pub use self::shell_integration::ShellIntegrationConfig;
pub use self::table::{FooterMode, TableConfig, TableIndexMode, TableMode, TrimStrategy};

mod completer;
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
    pub completions: CompleterConfig,
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

            completions: CompleterConfig::default(),

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
            let span = value.span();
            match key {
                "ls" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            match key2 {
                                "use_ls_colors" => {
                                    process_bool_config(value, &mut errors, &mut config.ls.use_ls_colors);
                                }
                                "clickable_links" => {
                                    process_bool_config(value, &mut errors, &mut config.ls.clickable_links);
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            }
                            true
                        });
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.ls.into_value(span);
                    }
                }
                "rm" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            match key2 {
                                "always_trash" => {
                                    process_bool_config(value, &mut errors, &mut config.rm.always_trash);
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            true
                        });
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.rm.into_value(span);
                    }
                }
                "history" => {
                    let history = &mut config.history;
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            match key2 {
                                "isolation" => {
                                    process_bool_config(value, &mut errors, &mut history.isolation);
                                }
                                "sync_on_enter" => {
                                    process_bool_config(value, &mut errors, &mut history.sync_on_enter);
                                }
                                "max_size" => {
                                    process_int_config(value, &mut errors, &mut history.max_size);
                                }
                                "file_format" => {
                                    process_string_enum(
                                        &mut history.file_format,
                                        &[key, key2],
                                        value,
                                        &mut errors
                                    );
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            true
                        });
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.history.into_value(span);
                    }
                }
                "completions" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            match key2 {
                                "quick" => {
                                    process_bool_config(value, &mut errors, &mut config.completions.quick);
                                }
                                "partial" => {
                                    process_bool_config(value, &mut errors, &mut config.completions.partial);
                                }
                                "algorithm" => {
                                    process_string_enum(
                                        &mut config.completions.algorithm,
                                        &[key, key2],
                                        value,
                                        &mut errors
                                    );
                                }
                                "case_sensitive" => {
                                    process_bool_config(value, &mut errors, &mut config.completions.case_sensitive);
                                }
                                "sort" => {
                                    process_string_enum(
                                        &mut config.completions.sort,
                                        &[key, key2],
                                        value,
                                        &mut errors
                                    );
                                }
                                "external" => {
                                    if let Value::Record { val, .. } = value {
                                        val.to_mut().retain_mut(|key3, value| {
                                            let span = value.span();
                                            match key3 {
                                                "max_results" => {
                                                    process_int_config(value, &mut errors, &mut config.completions.external.max_results);
                                                }
                                                "completer" => {
                                                    if let Ok(v) = value.as_closure() {
                                                        config.completions.external.completer = Some(v.clone())
                                                    } else {
                                                        match value {
                                                            Value::Nothing { .. } => {}
                                                            _ => {
                                                                report_invalid_value("should be a closure or null", span, &mut errors);
                                                                *value = config.completions.external.completer.clone().into_value(span);
                                                            }
                                                        }
                                                    }
                                                }
                                                "enable" => {
                                                    process_bool_config(value, &mut errors, &mut config.completions.external.enable);
                                                }
                                                _ => {
                                                    report_invalid_key(&[key, key2, key3], span, &mut errors);
                                                    return false;
                                                }
                                            };
                                            true
                                        });
                                    } else {
                                        report_invalid_value("should be a record", span, &mut errors);
                                        *value = config.completions.external.clone().into_value(span);
                                    }
                                }
                                "use_ls_colors" => {
                                    process_bool_config(value, &mut errors, &mut config.completions.use_ls_colors);
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            true
                        });
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.completions.clone().into_value(span);
                    }
                }
                "cursor_shape" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            let config_point = match key2 {
                                "vi_insert" => &mut config.cursor_shape.vi_insert,
                                "vi_normal" => &mut config.cursor_shape.vi_normal,
                                "emacs" => &mut config.cursor_shape.emacs,
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            process_string_enum(
                                config_point,
                                &[key, key2],
                                value,
                                &mut errors
                            );
                            true
                        });
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.cursor_shape.into_value(span);
                    }
                }
                "table" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            match key2 {
                                "mode" => {
                                    process_string_enum(
                                        &mut config.table.mode,
                                        &[key, key2],
                                        value,
                                        &mut errors
                                    );
                                }
                                "header_on_separator" => {
                                    process_bool_config(value, &mut errors, &mut config.table.header_on_separator);
                                }
                                "padding" => match value {
                                    Value::Int { val, .. } => {
                                        if *val < 0 {
                                            report_invalid_value("expected a unsigned integer", span, &mut errors);
                                            *value = config.table.padding.into_value(span);
                                        } else {
                                            config.table.padding.left = *val as usize;
                                            config.table.padding.right = *val as usize;
                                        }
                                    }
                                    Value::Record { val, .. } => {
                                        let mut invalid = false;
                                        val.to_mut().retain(|key3, value| {
                                            match key3 {
                                                "left" => {
                                                    match value.as_int() {
                                                        Ok(val) if val >= 0 => {
                                                            config.table.padding.left = val as usize;
                                                        }
                                                        _ => {
                                                            report_invalid_value("expected a unsigned integer >= 0", span, &mut errors);
                                                            invalid = true;
                                                        }
                                                    }
                                                }
                                                "right" => {
                                                    match value.as_int() {
                                                        Ok(val) if val >= 0 => {
                                                            config.table.padding.right = val as usize;
                                                        }
                                                        _ => {
                                                            report_invalid_value("expected a unsigned integer >= 0", span, &mut errors);
                                                            invalid = true;
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    report_invalid_key(&[key, key2, key3], span, &mut errors);
                                                    return false;
                                                }
                                            };
                                            true
                                        });
                                        if invalid {
                                            *value = config.table.padding.into_value(span);
                                        }
                                    }
                                    _ => {
                                        report_invalid_value("expected a unsigned integer or a record", span, &mut errors);
                                        *value = config.table.padding.into_value(span);
                                    }
                                },
                                "index_mode" => {
                                    process_string_enum(
                                        &mut config.table.index_mode,
                                        &[key, key2],
                                        value,
                                        &mut errors
                                    );
                                }
                                "trim" => {
                                    match try_parse_trim_strategy(value, &mut errors) {
                                        Ok(v) => config.table.trim = v,
                                        Err(e) => {
                                            // try_parse_trim_strategy() already adds its own errors
                                            errors.push(e);
                                            *value = config.table.trim.clone().into_value(span);
                                        }
                                    }
                                }
                                "show_empty" => {
                                    process_bool_config(value, &mut errors, &mut config.table.show_empty);
                                }
                                "abbreviated_row_count" => {
                                    match *value {
                                        Value::Int { val, .. } => {
                                            if val >= 0 {
                                                config.table.abbreviated_row_count = Some(val as usize);
                                            } else {
                                                report_invalid_value("should be an int unsigned", span, &mut errors);
                                                *value = config.table.abbreviated_row_count.map(|count| Value::int(count as i64, span)).unwrap_or(Value::nothing(span));
                                            }
                                        }
                                        Value::Nothing { .. } => {
                                            config.table.abbreviated_row_count = None;
                                        }
                                        _ => {
                                            report_invalid_value("should be an int", span, &mut errors);
                                            *value = config.table.abbreviated_row_count.map(|count| Value::int(count as i64, span)).unwrap_or(Value::nothing(span))
                                        }
                                    }
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            true
                        });
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.table.clone().into_value(span);
                    }
                }
                "filesize" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            match key2 {
                                "metric" => {
                                    process_bool_config(value, &mut errors, &mut config.filesize.metric);
                                }
                                "format" => {
                                    if let Ok(v) = value.coerce_str() {
                                        config.filesize.format = v.to_lowercase();
                                    } else {
                                        report_invalid_value("should be a string", span, &mut errors);
                                        *value = Value::string(config.filesize.format.clone(), span);
                                    }
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            true
                        })
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.filesize.clone().into_value(span);
                    }
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
                    process_string_enum(
                        &mut config.footer_mode,
                        &[key],
                        value,
                        &mut errors
                    );
                }
                "float_precision" => {
                    process_int_config(value, &mut errors, &mut config.float_precision);
                }
                "use_ansi_coloring" => {
                    process_bool_config(value, &mut errors, &mut config.use_ansi_coloring);
                }
                "edit_mode" => {
                    process_string_enum(
                        &mut config.edit_mode,
                        &[key],
                        value,
                        &mut errors
                    );
                }
                "shell_integration" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value| {
                            let span = value.span();
                            match key2 {
                                "osc2" => {
                                    process_bool_config(value, &mut errors, &mut config.shell_integration.osc2);
                                }
                                "osc7" => {
                                    process_bool_config(value, &mut errors, &mut config.shell_integration.osc7);
                                }
                                "osc8" => {
                                    process_bool_config(value, &mut errors, &mut config.shell_integration.osc8);
                                }
                                "osc9_9" => {
                                    process_bool_config(value, &mut errors, &mut config.shell_integration.osc9_9);
                                }
                                "osc133" => {
                                    process_bool_config(value, &mut errors, &mut config.shell_integration.osc133);
                                }
                                "osc633" => {
                                    process_bool_config(value, &mut errors, &mut config.shell_integration.osc633);
                                }
                                "reset_application_mode" => {
                                    process_bool_config(value, &mut errors, &mut config.shell_integration.reset_application_mode);
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            true
                        })
                    } else {
                        report_invalid_value("boolean value is deprecated, should be a record. see `config nu --default`.", span, &mut errors);
                        *value = config.shell_integration.into_value(span);
                    }
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
                        report_invalid_value("should be a string, list<string>, or null", span, &mut errors);
                        *value = config.buffer_editor.clone();
                    }
                },
                "show_banner" => {
                    process_bool_config(value, &mut errors, &mut config.show_banner);
                }
                "render_right_prompt_on_last_line" => {
                    process_bool_config(value, &mut errors, &mut config.render_right_prompt_on_last_line);
                }
                "bracketed_paste" => {
                    process_bool_config(value, &mut errors, &mut config.bracketed_paste);
                }
                "use_kitty_protocol" => {
                    process_bool_config(value, &mut errors, &mut config.use_kitty_protocol);
                }
                "highlight_resolved_externals" => {
                    process_bool_config(value, &mut errors, &mut config.highlight_resolved_externals);
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
                    config.plugin_gc.process(&[key], value, &mut errors);
                }
                "menus" => match create_menus(value) {
                    Ok(map) => config.menus = map,
                    Err(e) => {
                        report_invalid_value("should be a valid list of menus", span, &mut errors);
                        errors.push(e);
                        *value = config.menus.clone().into_value(span);
                    }
                },
                "keybindings" => match create_keybindings(value) {
                    Ok(keybindings) => config.keybindings = keybindings,
                    Err(e) => {
                        report_invalid_value("should be a valid keybindings list", span, &mut errors);
                        errors.push(e);
                        *value = config.keybindings.clone().into_value(span);
                    }
                },
                "hooks" => match create_hooks(value) {
                    Ok(hooks) => config.hooks = hooks,
                    Err(e) => {
                        report_invalid_value("should be a valid hooks list", span, &mut errors);
                        errors.push(e);
                        *value = config.hooks.clone().into_value(span);
                    }
                },
                "datetime_format" => {
                    if let Value::Record { val, .. } = value {
                        val.to_mut().retain_mut(|key2, value|
                            {
                            let span = value.span();
                            match key2 {
                                "normal" => {
                                    if let Ok(v) = value.coerce_string() {
                                        config.datetime_format.normal = Some(v);
                                    } else {
                                        report_invalid_value("should be a string", span, &mut errors);
                                    }
                                }
                                "table" => {
                                    if let Ok(v) = value.coerce_string() {
                                        config.datetime_format.table = Some(v);
                                    } else {
                                        report_invalid_value("should be a string", span, &mut errors);
                                    }
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            };
                            true
                        })
                    } else {
                        report_invalid_value("should be a record", span, &mut errors);
                        *value = config.datetime_format.clone().into_value(span);
                    }
                }
                "error_style" => {
                    process_string_enum(
                        &mut config.error_style,
                        &[key],
                        value,
                        &mut errors
                    );
                }
                "recursion_limit" => {
                    if let Value::Int { val, internal_span } = value {
                        if val > &mut 1 {
                            config.recursion_limit = *val;
                        } else {
                            report_invalid_value("should be a integer greater than 1", span, &mut errors);
                            *value = Value::Int { val: 50, internal_span: *internal_span };
                        }
                    } else {
                        report_invalid_value("should be a integer greater than 1", span, &mut errors);
                        *value = Value::Int { val: 50, internal_span: value.span() };
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
