use self::completer::*;
use self::helper::*;
use self::hooks::*;
use self::output::*;
use self::reedline::*;
use self::table::*;

use crate::engine::Closure;
use crate::{record, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use self::completer::CompletionAlgorithm;
pub use self::helper::extract_value;
pub use self::hooks::Hooks;
pub use self::output::ErrorStyle;
pub use self::plugin_gc::{PluginGcConfig, PluginGcConfigs};
pub use self::reedline::{
    create_menus, EditBindings, HistoryFileFormat, NuCursorShape, ParsedKeybinding, ParsedMenu,
};
pub use self::table::{FooterMode, TableIndexMode, TableMode, TrimStrategy};

mod completer;
mod helper;
mod hooks;
mod output;
mod plugin_gc;
mod reedline;
mod table;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub max_size: i64,
    pub sync_on_enter: bool,
    pub file_format: HistoryFileFormat,
    pub isolation: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_size: 100_000,
            sync_on_enter: true,
            file_format: HistoryFileFormat::PlainText,
            isolation: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub external_completer: Option<Closure>,
    pub filesize_metric: bool,
    pub table_mode: TableMode,
    pub table_move_header: bool,
    pub table_show_empty: bool,
    pub table_indent: TableIndent,
    pub table_abbreviation_threshold: Option<usize>,
    pub use_ls_colors: bool,
    pub color_config: HashMap<String, Value>,
    pub use_grid_icons: bool,
    pub footer_mode: FooterMode,
    pub float_precision: i64,
    pub max_external_completion_results: i64,
    pub recursion_limit: i64,
    pub filesize_format: String,
    pub use_ansi_coloring: bool,
    pub quick_completions: bool,
    pub partial_completions: bool,
    pub completion_algorithm: CompletionAlgorithm,
    pub edit_mode: EditBindings,
    pub history: HistoryConfig,
    pub keybindings: Vec<ParsedKeybinding>,
    pub menus: Vec<ParsedMenu>,
    pub hooks: Hooks,
    pub rm_always_trash: bool,
    pub shell_integration: bool,
    pub buffer_editor: Value,
    pub table_index_mode: TableIndexMode,
    pub case_sensitive_completions: bool,
    pub enable_external_completion: bool,
    pub trim_strategy: TrimStrategy,
    pub show_banner: bool,
    pub bracketed_paste: bool,
    pub show_clickable_links_in_ls: bool,
    pub render_right_prompt_on_last_line: bool,
    pub explore: HashMap<String, Value>,
    pub cursor_shape_vi_insert: NuCursorShape,
    pub cursor_shape_vi_normal: NuCursorShape,
    pub cursor_shape_emacs: NuCursorShape,
    pub datetime_normal_format: Option<String>,
    pub datetime_table_format: Option<String>,
    pub error_style: ErrorStyle,
    pub use_kitty_protocol: bool,
    pub highlight_resolved_externals: bool,
    pub use_ls_colors_completions: bool,
    /// Configuration for plugins.
    ///
    /// Users can provide configuration for a plugin through this entry.  The entry name must
    /// match the registered plugin name so `register nu_plugin_example` will be able to place
    /// its configuration under a `nu_plugin_example` column.
    pub plugins: HashMap<String, Value>,
    /// Configuration for plugin garbage collection.
    pub plugin_gc: PluginGcConfigs,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            show_banner: true,

            use_ls_colors: true,
            show_clickable_links_in_ls: true,

            rm_always_trash: false,

            table_mode: TableMode::Rounded,
            table_index_mode: TableIndexMode::Always,
            table_show_empty: true,
            trim_strategy: TrimStrategy::default(),
            table_move_header: false,
            table_indent: TableIndent { left: 1, right: 1 },
            table_abbreviation_threshold: None,

            datetime_normal_format: None,
            datetime_table_format: None,

            explore: HashMap::new(),

            history: HistoryConfig::default(),

            case_sensitive_completions: false,
            quick_completions: true,
            partial_completions: true,
            completion_algorithm: CompletionAlgorithm::default(),
            enable_external_completion: true,
            max_external_completion_results: 100,
            recursion_limit: 50,
            external_completer: None,
            use_ls_colors_completions: true,

            filesize_metric: false,
            filesize_format: "auto".into(),

            cursor_shape_emacs: NuCursorShape::default(),
            cursor_shape_vi_insert: NuCursorShape::default(),
            cursor_shape_vi_normal: NuCursorShape::default(),

            color_config: HashMap::new(),
            use_grid_icons: true,
            footer_mode: FooterMode::RowCount(25),
            float_precision: 2,
            buffer_editor: Value::nothing(Span::unknown()),
            use_ansi_coloring: true,
            bracketed_paste: true,
            edit_mode: EditBindings::default(),
            shell_integration: false,
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

        if let Value::Record { val, .. } = self {
            val.to_mut().retain_mut(|key, value| {
                let span = value.span();
                match key {
                    // Grouped options
                    "ls" => {
                        if let Value::Record { val, .. } = value {
                            val.to_mut().retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "use_ls_colors" => {
                                        process_bool_config(value, &mut errors, &mut config.use_ls_colors);
                                    }
                                    "clickable_links" => {
                                        process_bool_config(value, &mut errors, &mut config.show_clickable_links_in_ls);
                                    }
                                    _ => {
                                        report_invalid_key(&[key, key2], span, &mut errors);
                                        return false;
                                    }
                                }; true
                            });
                        } else {
                            report_invalid_value("should be a record", span, &mut errors);
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "use_ls_colors" => Value::bool(config.use_ls_colors, span),
                                    "clickable_links" => Value::bool(config.show_clickable_links_in_ls, span),
                                },
                                span,
                            );
                        }
                    }
                    "rm" => {
                        if let Value::Record { val, .. } = value {
                            val.to_mut().retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "always_trash" => {
                                        process_bool_config(value, &mut errors, &mut config.rm_always_trash);
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
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "always_trash" => Value::bool(config.rm_always_trash, span),
                                },
                                span,
                            );
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
                                            &mut errors);
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
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "sync_on_enter" => Value::bool(history.sync_on_enter, span),
                                    "max_size" => Value::int(history.max_size, span),
                                    "file_format" => history.file_format.reconstruct_value(span),
                                    "isolation" => Value::bool(history.isolation, span),
                                },
                                span,
                            );
                        }
                    }
                    "completions" => {
                        if let Value::Record { val, .. } = value {
                            val.to_mut().retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "quick" => {
                                        process_bool_config(value, &mut errors, &mut config.quick_completions);
                                    }
                                    "partial" => {
                                        process_bool_config(value, &mut errors, &mut config.partial_completions);
                                    }
                                    "algorithm" => {
                                        process_string_enum(
                                            &mut config.completion_algorithm,
                                            &[key, key2],
                                            value,
                                            &mut errors);
                                    }
                                    "case_sensitive" => {
                                        process_bool_config(value, &mut errors, &mut config.case_sensitive_completions);
                                    }
                                    "external" => {
                                        if let Value::Record { val, .. } = value {
                                            val.to_mut().retain_mut(|key3, value|
                                                {
                                                    let span = value.span();
                                                    match key3 {
                                                        "max_results" => {
                                                            process_int_config(value, &mut errors, &mut config.max_external_completion_results);
                                                        }
                                                        "completer" => {
                                                            if let Ok(v) = value.as_closure() {
                                                                config.external_completer = Some(v.clone())
                                                            } else {
                                                                match value {
                                                                    Value::Nothing { .. } => {}
                                                                    _ => {
                                                                        report_invalid_value("should be a closure or null", span, &mut errors);
                                                                        // Reconstruct
                                                                        *value = reconstruct_external_completer(&config,
                                                                            span
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        "enable" => {
                                                            process_bool_config(value, &mut errors, &mut config.enable_external_completion);
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
                                            // Reconstruct
                                            *value = reconstruct_external(&config, span);
                                        }
                                    }
                                    "use_ls_colors" => {
                                        process_bool_config(value, &mut errors, &mut config.use_ls_colors_completions);
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
                            // Reconstruct record
                            *value = Value::record(
                                record! {
                                    "quick" => Value::bool(config.quick_completions, span),
                                    "partial" => Value::bool(config.partial_completions, span),
                                    "algorithm" => config.completion_algorithm.reconstruct_value(span),
                                    "case_sensitive" => Value::bool(config.case_sensitive_completions, span),
                                    "external" => reconstruct_external(&config, span),
                                    "use_ls_colors" => Value::bool(config.use_ls_colors_completions, span),
                                },
                                span,
                            );
                        }
                    }
                    "cursor_shape" => {
                        if let Value::Record { val, .. } = value {
                            val.to_mut().retain_mut(|key2, value| {
                                let span = value.span();
                                let config_point = match key2 {
                                    "vi_insert" => &mut config.cursor_shape_vi_insert,
                                    "vi_normal" => &mut config.cursor_shape_vi_normal,
                                    "emacs" => &mut config.cursor_shape_emacs,
                                    _ => {
                                        report_invalid_key(&[key, key2], span, &mut errors);
                                        return false;
                                    }
                                };
                                process_string_enum(
                                    config_point,
                                    &[key, key2],
                                    value,
                                    &mut errors);
                                true
                            });
                        } else {
                            report_invalid_value("should be a record", span, &mut errors);
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "vi_insert" => config.cursor_shape_vi_insert.reconstruct_value(span),
                                    "vi_normal" => config.cursor_shape_vi_normal.reconstruct_value(span),
                                    "emacs" => config.cursor_shape_emacs.reconstruct_value(span),
                                },
                                span,
                            );
                        }
                    }
                    "table" => {
                        if let Value::Record { val, .. } = value {
                            val.to_mut().retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "mode" => {
                                        process_string_enum(
                                            &mut config.table_mode,
                                    &[key, key2],
                                            value,
                                            &mut errors);
                                    }
                                    "header_on_separator" => {
                                        process_bool_config(value, &mut errors, &mut config.table_move_header);
                                    }
                                    "padding" => match value {
                                        Value::Int { val, .. } => {
                                            if *val < 0 {
                                                report_invalid_value("expected a unsigned integer", span, &mut errors);
                                                *value = reconstruct_padding(&config, span);
                                            } else {
                                                config.table_indent.left = *val as usize;
                                                config.table_indent.right = *val as usize;
                                            }
                                        }
                                        Value::Record { val, .. } => {
                                            let mut invalid = false;
                                            val.to_mut().retain(|key3, value| {
                                                match key3 {
                                                    "left" => {
                                                        match value.as_int() {
                                                            Ok(val) if val >= 0 => {
                                                                config.table_indent.left = val as usize;
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
                                                                config.table_indent.right = val as usize;
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
                                                *value = reconstruct_padding(&config, span);
                                            }
                                        }
                                        _ => {
                                            report_invalid_value("expected a unsigned integer or a record", span, &mut errors);
                                            *value = reconstruct_padding(&config, span);
                                        }
                                    },
                                    "index_mode" => {
                                        process_string_enum(
                                            &mut config.table_index_mode,
                                            &[key, key2],
                                            value,
                                            &mut errors);
                                    }
                                    "trim" => {
                                        match try_parse_trim_strategy(value, &mut errors) {
                                            Ok(v) => config.trim_strategy = v,
                                            Err(e) => {
                                                // try_parse_trim_strategy() already adds its own errors
                                                errors.push(e);
                                                *value =
                                                    reconstruct_trim_strategy(&config, span);
                                            }
                                        }
                                    }
                                    "show_empty" => {
                                        process_bool_config(value, &mut errors, &mut config.table_show_empty);
                                    }
                                    "abbreviated_row_count" => {
                                        if let Ok(b) = value.as_int() {
                                            if b < 0 {
                                                report_invalid_value("should be an int unsigned", span, &mut errors);
                                            }

                                            config.table_abbreviation_threshold = Some(b as usize);
                                        } else {
                                            report_invalid_value("should be an int", span, &mut errors);
                                        }
                                    }
                                    _ => {
                                        report_invalid_key(&[key, key2], span, &mut errors);
                                        return false
                                    }
                                };
                                true
                             });
                        } else {
                            report_invalid_value("should be a record", span, &mut errors);
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "mode" => config.table_mode.reconstruct_value(span),
                                    "index_mode" => config.table_index_mode.reconstruct_value(span),
                                    "trim" => reconstruct_trim_strategy(&config, span),
                                    "show_empty" => Value::bool(config.table_show_empty, span),
                                },
                                span,
                            );
                        }
                    }
                    "filesize" => {
                        if let Value::Record { val, .. } = value {
                            val.to_mut().retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                "metric" => {
                                    process_bool_config(value, &mut errors, &mut config.filesize_metric);
                                }
                                "format" => {
                                    if let Ok(v) = value.coerce_str() {
                                        config.filesize_format = v.to_lowercase();
                                    } else {
                                        report_invalid_value("should be a string", span, &mut errors);
                                        // Reconstruct
                                        *value =
                                            Value::string(config.filesize_format.clone(), span);
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
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "metric" => Value::bool(config.filesize_metric, span),
                                    "format" => Value::string(config.filesize_format.clone(), span),
                                },
                                span,
                            );
                        }
                    }
                    "explore" => {
                        if let Ok(map) = create_map(value) {
                            config.explore = map;
                        } else {
                            report_invalid_value("should be a record", span, &mut errors);
                            // Reconstruct
                            *value = Value::record(
                                config
                                    .explore
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                                span,
                            );
                        }
                    }
                    // Misc. options
                    "color_config" => {
                        if let Ok(map) = create_map(value) {
                            config.color_config = map;
                        } else {
                            report_invalid_value("should be a record", span, &mut errors);
                            // Reconstruct
                            *value = Value::record(
                                config
                                    .color_config
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                                span,
                            );
                        }
                    }
                    "use_grid_icons" => {
                        process_bool_config(value, &mut errors, &mut config.use_grid_icons);
                    }
                    "footer_mode" => {
                        process_string_enum(
                            &mut config.footer_mode,
                            &[key],
                            value,
                            &mut errors);
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
                            &mut errors);
                    }
                    "shell_integration" => {
                        process_bool_config(value, &mut errors, &mut config.shell_integration);
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
                            // Reconstruct
                            *value = Value::record(
                                config
                                    .explore
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                                span,
                            );
                        }
                    }
                    "plugin_gc" => {
                        config.plugin_gc.process(&[key], value, &mut errors);
                    }
                    // Menus
                    "menus" => match create_menus(value) {
                        Ok(map) => config.menus = map,
                        Err(e) => {
                            report_invalid_value("should be a valid list of menus", span, &mut errors);
                            errors.push(e);
                            // Reconstruct
                            *value = reconstruct_menus(&config, span);
                        }
                    },
                    // Keybindings
                    "keybindings" => match create_keybindings(value) {
                        Ok(keybindings) => config.keybindings = keybindings,
                        Err(e) => {
                            report_invalid_value("should be a valid keybindings list", span, &mut errors);
                            errors.push(e);
                            // Reconstruct
                            *value = reconstruct_keybindings(&config, span);
                        }
                    },
                    // Hooks
                    "hooks" => match create_hooks(value) {
                        Ok(hooks) => config.hooks = hooks,
                        Err(e) => {
                            report_invalid_value("should be a valid hooks list", span, &mut errors);
                            errors.push(e);
                            *value = reconstruct_hooks(&config, span);
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
                                        config.datetime_normal_format = Some(v);
                                    } else {
                                        report_invalid_value("should be a string", span, &mut errors);
                                    }
                                }
                                "table" => {
                                    if let Ok(v) = value.coerce_string() {
                                        config.datetime_table_format = Some(v);
                                    } else {
                                        report_invalid_value("should be a string", span, &mut errors);
                                    }
                                }
                                _ => {
                                    report_invalid_key(&[key, key2], span, &mut errors);
                                    return false;
                                }
                            }; true})
                        } else {
                            report_invalid_value("should be a record", span, &mut errors);
                            // Reconstruct
                            *value = reconstruct_datetime_format(&config, span);
                        }
                    }
                    "error_style" => {
                        process_string_enum(
                            &mut config.error_style,
                            &[key],
                            value,
                            &mut errors);
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
        } else {
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
        }

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
