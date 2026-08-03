//! Module containing the internal representation of user configuration

use crate::FromValue;
use crate::{self as nu_protocol, Filesize, FilesizeUnit};
use helper::*;
use prelude::*;
use std::collections::HashMap;

pub use ansi_coloring::UseAnsiColoring;
pub use clip::ClipConfig;
pub use completions::{
    CompletionAlgorithm, CompletionConfig, CompletionSort, ExternalCompleterConfig,
};
pub use datetime_format::DatetimeFormatConfig;
pub use defaults::default_color_config;
pub use display_errors::DisplayErrors;
pub use duration_max_unit::DurationMaxUnit;
pub use filesize::FilesizeConfig;
pub use helper::extract_value;
pub use hinter::HinterConfig;
pub use history::{HistoryConfig, HistoryFileFormat, HistoryPath};
pub use hooks::Hooks;
pub use ls::LsConfig;
pub use output::{BannerKind, ErrorStyle};
pub use plugin_gc::{PluginGcConfig, PluginGcConfigs};
pub use reedline::{CursorShapeConfig, EditBindings, NuCursorShape, ParsedKeybinding, ParsedMenu};
pub use rm::RmConfig;
pub use shell_integration::ShellIntegrationConfig;
pub use table::{FooterMode, TableConfig, TableIndent, TableIndexMode, TableMode, TrimStrategy};

mod ansi_coloring;
mod clip;
mod completions;
mod datetime_format;
mod defaults;
mod display_errors;
mod duration_max_unit;
mod error;
mod filesize;
mod helper;
mod hinter;
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
    pub clip: ClipConfig,
    pub color_config: HashMap<String, Value>,
    pub footer_mode: FooterMode,
    pub float_precision: i64,
    pub recursion_limit: i64,
    pub use_ansi_coloring: UseAnsiColoring,
    pub completions: CompletionConfig,
    pub edit_mode: EditBindings,
    pub show_hints: bool,
    pub hinter: HinterConfig,
    pub history: HistoryConfig,
    pub keybindings: Vec<ParsedKeybinding>,
    pub abbreviations: HashMap<String, String>,
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
    pub error_lines: i64,
    pub display_errors: DisplayErrors,
    pub use_kitty_protocol: bool,
    pub highlight_resolved_externals: bool,
    pub auto_cd_implicit: bool,
    pub duration_max_unit: DurationMaxUnit,
    /// Maximum estimated memory size of the interactive last-result value (e.g. `$_`).
    ///
    /// Measured with [`Value::memory_size`]. `0` disables capture. Oversized results are truncated
    /// to fit this budget. The variable name itself is a code constant (`LAST_RESULT_VAR_NAME`),
    /// not a config option.
    pub last_result_size: Filesize,
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

            explore: defaults::default_explore(),

            history: HistoryConfig::default(),

            completions: CompletionConfig::default(),

            recursion_limit: 50,

            filesize: FilesizeConfig::default(),

            cursor_shape: CursorShapeConfig::default(),

            clip: ClipConfig::default(),

            color_config: defaults::default_color_config(),
            footer_mode: FooterMode::RowCount(25),
            float_precision: 2,
            buffer_editor: Value::nothing(Span::unknown()),
            use_ansi_coloring: UseAnsiColoring::default(),
            bracketed_paste: true,
            edit_mode: EditBindings::default(),
            show_hints: true,
            hinter: HinterConfig::default(),

            shell_integration: ShellIntegrationConfig::default(),

            render_right_prompt_on_last_line: false,

            hooks: Hooks::new(),

            menus: defaults::default_menus(),

            keybindings: defaults::default_keybindings(),
            abbreviations: HashMap::new(),

            error_style: ErrorStyle::default(),
            error_lines: 1,
            display_errors: DisplayErrors::default(),

            use_kitty_protocol: false,
            highlight_resolved_externals: false,

            auto_cd_implicit: false,
            duration_max_unit: DurationMaxUnit::default(),

            // 1 MiB default budget for interactive last-result storage
            last_result_size: Filesize::from_unit(1, FilesizeUnit::MiB)
                .expect("1 MiB fits in Filesize"),

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
            let current_path = &mut path.push(col);

            match col.as_str() {
                "ls" => self.ls.update(val, current_path, errors),
                "rm" => self.rm.update(val, current_path, errors),
                "history" => self.history.update(val, current_path, errors),
                "completions" => self.completions.update(val, current_path, errors),
                "cursor_shape" => self.cursor_shape.update(val, current_path, errors),
                "table" => self.table.update(val, current_path, errors),
                "filesize" => self.filesize.update(val, current_path, errors),
                "explore" => self.explore.update(val, current_path, errors),
                "color_config" => self.color_config.update(val, current_path, errors),
                "clip" => self.clip.update(val, current_path, errors),
                "footer_mode" => self.footer_mode.update(val, current_path, errors),
                "float_precision" => self.float_precision.update(val, current_path, errors),
                "use_ansi_coloring" => self.use_ansi_coloring.update(val, current_path, errors),
                "edit_mode" => self.edit_mode.update(val, current_path, errors),
                "show_hints" => self.show_hints.update(val, current_path, errors),
                "hinter" => self.hinter.update(val, current_path, errors),
                "shell_integration" => self.shell_integration.update(val, current_path, errors),
                "show_banner" => self.show_banner.update(val, current_path, errors),
                "display_errors" => self.display_errors.update(val, current_path, errors),
                "render_right_prompt_on_last_line" => {
                    self.render_right_prompt_on_last_line
                        .update(val, current_path, errors)
                }
                "bracketed_paste" => self.bracketed_paste.update(val, current_path, errors),
                "use_kitty_protocol" => self.use_kitty_protocol.update(val, current_path, errors),
                "highlight_resolved_externals" => {
                    self.highlight_resolved_externals
                        .update(val, current_path, errors)
                }
                "auto_cd_implicit" => self.auto_cd_implicit.update(val, current_path, errors),
                "duration_max_unit" => self.duration_max_unit.update(val, current_path, errors),
                "plugins" => self.plugins.update(val, current_path, errors),
                "plugin_gc" => self.plugin_gc.update(val, current_path, errors),
                "abbreviations" => self.abbreviations.update(val, current_path, errors),
                "hooks" => self.hooks.update(val, current_path, errors),
                "datetime_format" => self.datetime_format.update(val, current_path, errors),
                "error_style" => self.error_style.update(val, current_path, errors),

                "buffer_editor" => match val {
                    Value::Nothing { .. } | Value::String { .. } => {
                        self.buffer_editor = val.clone();
                    }
                    Value::List { vals: values, .. }
                        if values
                            .iter()
                            .all(|list_element| matches!(list_element, Value::String { .. })) =>
                    {
                        self.buffer_editor = val.clone();
                    }
                    _ => errors.type_mismatch(
                        current_path,
                        Type::custom("string, list<string>, or nothing"),
                        val,
                    ),
                },

                "last_result_size" => self.last_result_size.update(val, current_path, errors),

                "menus" => match Vec::<ParsedMenu>::from_value(val.clone()) {
                    Ok(menus) => {
                        for menu in menus {
                            let target_name = menu.name.to_expanded_string("", self);

                            let found_index = self.menus.iter().position(|existing_menu| {
                                existing_menu.name.to_expanded_string("", self) == target_name
                            });

                            if let Some(index) = found_index {
                                self.menus[index] = menu;
                            } else {
                                self.menus.push(menu);
                            }
                        }
                    }
                    Err(error) => errors.error(error.into()),
                },

                "keybindings" => match Vec::<ParsedKeybinding>::from_value(val.clone()) {
                    Ok(keybindings) => {
                        for keybinding in keybindings {
                            let target_name = keybinding
                                .name
                                .as_ref()
                                .map(|name| name.to_expanded_string("", self));

                            let found_index = target_name.as_ref().and_then(|name_to_match| {
                                self.keybindings.iter().position(|existing_keybinding| {
                                    existing_keybinding
                                        .name
                                        .as_ref()
                                        .map(|existing_name| {
                                            existing_name.to_expanded_string("", self)
                                        })
                                        .as_ref()
                                        == Some(name_to_match)
                                })
                            });

                            if let Some(index) = found_index {
                                self.keybindings[index] = keybinding;
                            } else {
                                self.keybindings.push(keybinding);
                            }
                        }
                    }
                    Err(error) => errors.error(error.into()),
                },

                "error_lines" => match val.as_int() {
                    Ok(integer) if integer >= 0 => self.error_lines = integer,
                    Ok(_) => {
                        errors.invalid_value(current_path, "an int greater than or equal to 0", val)
                    }
                    Err(_) => errors.type_mismatch(current_path, Type::Int, val),
                },

                "recursion_limit" => match val.as_int() {
                    Ok(integer) if integer > 1 => self.recursion_limit = integer,
                    Ok(_) => errors.invalid_value(current_path, "an int greater than 1", val),
                    Err(_) => errors.type_mismatch(current_path, Type::Int, val),
                },

                _ => errors.unknown_option(current_path, val),
            }
        }
    }
}

impl UpdateFromValue for Filesize {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        match value.as_filesize() {
            Ok(size) if !size.is_negative() => *self = size,
            Ok(_) => errors.invalid_value(path, "a non-negative filesize", value),
            Err(_) => errors.type_mismatch(path, Type::Filesize, value),
        }
    }
}

impl Config {
    /// Returns the configured last-result size budget in bytes (`0` disables capture).
    pub fn last_result_size_bytes(&self) -> usize {
        self.last_result_size.get().max(0) as usize
    }

    pub fn update_from_value(
        &mut self,
        old: &Config,
        value: &Value,
    ) -> Result<Option<ShellWarning>, ShellError> {
        self.update_from_value_with_options(old, value, false)
    }

    /// Like [`Config::update_from_value`], but allows callers to indicate that runtime-locked
    /// options should refuse to change.
    ///
    /// `history_locked_after_startup` should be set to `true` once the REPL has finished
    /// initializing reedline's history backend. After that point, changing any of the
    /// startup-only history fields (`path`, `max_size`, `file_format`, `isolation`) has no
    /// effect on the live history, so we reject the assignment with a clear error instead of
    /// silently ignoring it.
    pub fn update_from_value_with_options(
        &mut self,
        old: &Config,
        value: &Value,
        history_locked_after_startup: bool,
    ) -> Result<Option<ShellWarning>, ShellError> {
        // Current behaviour is that config errors are displayed, but do not prevent the rest
        // of the config from being updated (fields with errors are skipped/not updated).
        // Errors are simply collected one-by-one and wrapped into a ShellError variant at the end.
        let mut errors =
            ConfigErrors::new(old).with_history_locked_after_startup(history_locked_after_startup);
        let mut path = ConfigPath::new();

        self.update(value, &mut path, &mut errors);

        errors.check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record-valued config field is a full-record replace on assignment (e.g.
    /// `$env.config.keybindings = [...]`), but `update_from_value` must still merge
    /// named defaults into place rather than silently dropping ones the caller
    /// didn't mention.
    #[test]
    fn reassigning_a_named_list_field_keeps_unmentioned_defaults() {
        let old = Config::default();
        let mut new = old.clone();

        let mut extra_menu = old.menus[0].clone();
        extra_menu.name = Value::test_string("added_menu");
        let mut extra_keybinding = old.keybindings[0].clone();
        extra_keybinding.name = Some(Value::test_string("added_binding"));

        let value = Value::test_record(record! {
            "menus" => Value::test_list(vec![extra_menu.into_value(Span::test_data())]),
            "keybindings" => Value::test_list(vec![extra_keybinding.into_value(Span::test_data())]),
        });
        new.update_from_value(&old, &value)
            .expect("update should succeed");

        for default_menu in &old.menus {
            let name = default_menu.name.to_expanded_string("", &old);
            assert!(
                new.menus
                    .iter()
                    .any(|m| m.name.to_expanded_string("", &new) == name),
                "default menu {name:?} was lost after reassigning `menus`"
            );
        }
        for default_keybinding in &old.keybindings {
            let Some(name) = default_keybinding
                .name
                .as_ref()
                .map(|n| n.to_expanded_string("", &old))
            else {
                continue;
            };
            assert!(
                new.keybindings.iter().any(|k| k
                    .name
                    .as_ref()
                    .is_some_and(|n| n.to_expanded_string("", &new) == name)),
                "default keybinding {name:?} was lost after reassigning `keybindings`"
            );
        }
    }
}
