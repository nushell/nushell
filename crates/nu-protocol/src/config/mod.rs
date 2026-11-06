//! Module containing the internal representation of user configuration

use crate::FromValue;
use crate::{self as nu_protocol, Filesize};
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
pub use key_spec::{
    EditModeName, KeyCodeSpec, KeyModifier, MenuInputMode, MenuLayout, MenuOutputMode,
    VALID_KEYBINDING_MODES,
};
pub use ls::LsConfig;
pub use output::{BannerKind, ErrorStyle};
pub use plugin_gc::{PluginGcConfig, PluginGcConfigs};
pub use reedline::{CursorShapeConfig, EditBindings, NuCursorShape, ParsedKeybinding, ParsedMenu};
pub use rm::RmConfig;
pub use shell_integration::ShellIntegrationConfig;
pub use status_line::StatusLineConfig;
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
mod key_spec;
mod ls;
mod output;
mod plugin_gc;
mod prelude;
mod reedline;
mod rm;
mod shell_integration;
mod status_line;
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
    pub status_line: StatusLineConfig,
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
    /// Styles for the `tui` command family (`$env.config.tui`).
    pub tui: HashMap<String, Value>,
    pub cursor_shape: CursorShapeConfig,
    pub datetime_format: DatetimeFormatConfig,
    pub error_style: ErrorStyle,
    pub error_lines: i64,
    pub display_errors: DisplayErrors,
    pub use_kitty_protocol: bool,
    pub highlight_resolved_externals: bool,
    pub auto_cd_implicit: bool,
    pub duration_max_unit: DurationMaxUnit,
    /// Maximum estimated memory size of the interactive last-result payload (`$ans.last`).
    ///
    /// Measured with [`Value::memory_size`]. Default is `0` (no `.last` payload; opt-in).
    /// Oversized results are truncated to fit this budget. The variable name itself is a code
    /// constant (`LAST_RESULT_VAR_NAME`), not a config option. With a positive budget, `$ans`
    /// is `{ last, exit_code, duration, command }`. With `0`, `$ans` still has `exit_code`,
    /// `duration`, and `command` but omits `last` entirely.
    pub max_last_result_size: Filesize,
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
            tui: defaults::default_tui(),

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
            status_line: StatusLineConfig::default(),

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

            // Opt-in for `.last` payload: 0 drops last, keeps exit_code/duration/command.
            max_last_result_size: Filesize::ZERO,

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
                "tui" => {
                    self.tui.update(val, current_path, errors);
                    check_tui_border_type(&mut self.tui, current_path, errors);
                }
                "color_config" => self.color_config.update(val, current_path, errors),
                "clip" => self.clip.update(val, current_path, errors),
                "footer_mode" => self.footer_mode.update(val, current_path, errors),
                "float_precision" => self.float_precision.update(val, current_path, errors),
                "use_ansi_coloring" => self.use_ansi_coloring.update(val, current_path, errors),
                "edit_mode" => self.edit_mode.update(val, current_path, errors),
                "show_hints" => self.show_hints.update(val, current_path, errors),
                "hinter" => self.hinter.update(val, current_path, errors),
                "status_line" => self.status_line.update(val, current_path, errors),
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

                "max_last_result_size" => {
                    self.max_last_result_size.update(val, current_path, errors)
                }

                // Stored as written, like any list. Reedline registers the
                // menus in order, so a later one wins on a name. A menu with a
                // layout, input mode or output mode that can't be read rejects
                // the whole list and the previous one stays, like for any other
                // config value.
                "menus" => match Vec::<ParsedMenu>::from_value(val.clone()).and_then(|menus| {
                    menus.iter().try_for_each(|menu| menu.validate(self))?;
                    Ok(menus)
                }) {
                    Ok(menus) => self.menus = menus,
                    Err(error) => errors.error(error.into()),
                },

                // Stored as written, like any list. Reedline applies the
                // entries in order, so a later one wins on a key. An entry
                // with a mode, modifier or keycode that can't be read rejects
                // the whole list and the previous one stays. Its `event` is
                // read when the prompt is built.
                "keybindings" => {
                    match Vec::<ParsedKeybinding>::from_value(val.clone()).and_then(|keybindings| {
                        keybindings
                            .iter()
                            .try_for_each(ParsedKeybinding::validate)?;
                        Ok(keybindings)
                    }) {
                        Ok(keybindings) => self.keybindings = keybindings,
                        Err(error) => errors.error(error.into()),
                    }
                }

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

/// `$env.config.tui` is a free-form map of colors, except `border_type`,
/// which must name a tui border (see [`TableMode::from_tui_border`]). A bad
/// value is reported here, where it is set, and the previous one is kept,
/// like a typed field that fails to update.
fn check_tui_border_type(
    tui: &mut HashMap<String, Value>,
    path: &mut ConfigPath,
    errors: &mut ConfigErrors,
) {
    const KEY: &str = "border_type";
    let Some(value) = tui.get(KEY) else {
        return;
    };
    let path = &mut path.push(KEY);
    match value.as_str() {
        Ok(name) if TableMode::from_tui_border(name).is_some() => return,
        Ok(_) => {
            let names = TableMode::tui_border_names().collect::<Vec<_>>().join(", ");
            errors.invalid_value(path, format!("one of {names}"), value);
        }
        Err(_) => errors.type_mismatch(path, Type::String, value),
    }
    match errors.config().tui.get(KEY).cloned() {
        Some(old) => tui.insert(KEY.into(), old),
        None => tui.remove(KEY),
    };
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
    /// Returns the configured last-result size budget in bytes (`0` disables `.last` only).
    pub fn max_last_result_size_bytes(&self) -> usize {
        self.max_last_result_size.get().max(0) as usize
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

    /// Run one `$env.config.menus = [...]` assignment.
    fn assign_menus(config: &mut Config, old: &Config, menus: Vec<ParsedMenu>) {
        try_assign_menus(config, old, menus).expect("update should succeed");
    }

    fn menu_names(config: &Config) -> Vec<String> {
        config
            .menus
            .iter()
            .map(|menu| menu.name.to_expanded_string("", config))
            .collect()
    }

    /// `=` replaces the whole list, defaults included.
    #[test]
    fn assigning_menus_replaces_the_list() {
        let old = Config::default();
        let mut new = old.clone();

        let mut extra = old.menus[0].clone();
        extra.name = Value::test_string("added_menu");
        assign_menus(&mut new, &old, vec![extra]);

        assert_eq!(menu_names(&new), ["added_menu"]);
    }

    /// `++=` keeps the defaults and adds the new menu after them.
    #[test]
    fn appending_a_menu_keeps_the_defaults() {
        let old = Config::default();
        let mut new = old.clone();

        let mut extra = old.menus[0].clone();
        extra.name = Value::test_string("added_menu");
        let mut menus = new.menus.clone();
        menus.push(extra);
        assign_menus(&mut new, &old, menus);

        let mut expected = menu_names(&old);
        expected.push("added_menu".into());
        assert_eq!(menu_names(&new), expected);
    }

    /// The list is stored as written, so two menus with one name both stay and
    /// keep their order. Reedline registers them in that order and the last
    /// one wins, which is what makes `++=` with a default's name override it.
    #[test]
    fn menus_are_stored_as_written() {
        let old = Config::default();
        let mut new = old.clone();

        let mut custom = old.menus[0].clone();
        custom.marker = Value::test_string("custom> ");
        let mut menus = new.menus.clone();
        menus.push(custom);
        assign_menus(&mut new, &old, menus);

        let name = old.menus[0].name.to_expanded_string("", &old);
        let same_name = |config: &Config| {
            menu_names(config)
                .iter()
                .filter(|menu| **menu == name)
                .count()
        };
        assert_eq!(same_name(&new), 2);
        let last = new.menus.last().expect("menus are not empty");
        assert_eq!(last.marker, Value::test_string("custom> "));
    }

    /// Assigning an empty list clears every menu.
    #[test]
    fn assigning_an_empty_list_clears_the_menus() {
        let old = Config::default();
        let mut new = old.clone();

        assign_menus(&mut new, &old, vec![]);
        assert!(new.menus.is_empty(), "`= []` should reset the list");
    }

    /// `tui.border_type` is checked where it is set: a name that is not a tui
    /// border is an error and the previous border stays.
    #[test]
    fn tui_border_type_rejects_non_borders_and_keeps_the_old_one() {
        let old = Config::default();
        let tui = |border_type: Value| {
            Value::test_record(record! {
                "tui" => Value::test_record(record! { "border_type" => border_type }),
            })
        };

        let mut new = old.clone();
        new.update_from_value(&old, &tui(Value::test_string("Double")))
            .expect("a table theme name is a border");
        assert_eq!(
            new.tui.get("border_type"),
            Some(&Value::test_string("Double"))
        );

        for bad in [
            Value::test_string("none"),
            Value::test_string("default"),
            Value::test_string("wavy"),
            Value::test_int(1),
        ] {
            let mut new = old.clone();
            let err = new
                .update_from_value(&old, &tui(bad.clone()))
                .expect_err("not a tui border");
            assert!(
                format!("{err:?}").contains("$env.config.tui.border_type"),
                "{bad:?}: {err:?}"
            );
            assert_eq!(new.tui.get("border_type"), old.tui.get("border_type"));
        }
    }

    /// `TableMode::NAMES` feeds `table --theme` and `tui --border`
    /// completions, so it must hold every name `from_str` accepts: the ones
    /// its error lists, plus `default`.
    #[test]
    fn table_mode_names_match_from_str() {
        for name in TableMode::NAMES {
            assert!(name.parse::<TableMode>().is_ok(), "{name}");
        }
        let listed = "wavy".parse::<TableMode>().expect_err("not a table mode");
        let listed: Vec<&str> = listed.split('\'').skip(1).step_by(2).collect();
        for name in &listed {
            assert!(TableMode::NAMES.contains(name), "{name} is not in NAMES");
        }
        assert_eq!(listed.len() + 1, TableMode::NAMES.len());
    }

    // --- keybindings: a plain list, applied in order by reedline ---

    fn keybinding(
        name: Option<&str>,
        modifier: &str,
        keycode: &str,
        mode: Value,
    ) -> ParsedKeybinding {
        ParsedKeybinding {
            name: name.map(Value::test_string),
            modifier: Value::test_string(modifier),
            keycode: Value::test_string(keycode),
            event: Value::test_nothing(),
            mode,
        }
    }

    fn atuin() -> ParsedKeybinding {
        keybinding(
            Some("atuin"),
            "control",
            "char_r",
            Value::test_string("emacs"),
        )
    }

    /// Run one `$env.config.keybindings = [...]` assignment.
    fn assign(config: &mut Config, old: &Config, keybindings: Vec<ParsedKeybinding>) {
        try_assign(config, old, keybindings).expect("update should succeed");
    }

    /// `$env.config.keybindings ++= [...]` is `= ($env.config.keybindings ++ [...])`.
    fn append(config: &mut Config, old: &Config, keybindings: Vec<ParsedKeybinding>) {
        let mut list = config.keybindings.clone();
        list.extend(keybindings);
        assign(config, old, list);
    }

    fn count_named(config: &Config, name: &str) -> usize {
        config
            .keybindings
            .iter()
            .filter(|kb| {
                kb.name
                    .as_ref()
                    .is_some_and(|n| n.to_expanded_string("", config) == name)
            })
            .count()
    }

    /// `=` replaces the whole list, defaults included.
    #[test]
    fn assigning_keybindings_replaces_the_list() {
        let old = Config::default();
        let mut new = old.clone();

        assign(&mut new, &old, vec![atuin()]);

        assert_eq!(new.keybindings.len(), 1);
        assert_eq!(count_named(&new, "atuin"), 1);
    }

    /// `++=` keeps the defaults and adds the new binding after them.
    #[test]
    fn appending_keeps_the_defaults() {
        let old = Config::default();
        let mut new = old.clone();

        append(&mut new, &old, vec![atuin()]);

        assert_eq!(new.keybindings.len(), old.keybindings.len() + 1);
        assert_eq!(count_named(&new, "atuin"), 1);
        assert_eq!(count_named(&new, "history_menu"), 1);
    }

    /// Filtering a binding out of the list unbinds it.
    #[test]
    fn assigning_a_filtered_list_drops_the_binding() {
        let old = Config::default();
        let mut new = old.clone();
        append(&mut new, &old, vec![atuin()]);
        let len = new.keybindings.len();

        let kept = new
            .keybindings
            .iter()
            .filter(|kb| {
                kb.name
                    .as_ref()
                    .is_none_or(|n| n.to_expanded_string("", &new) != "atuin")
            })
            .cloned()
            .collect();
        assign(&mut new, &old, kept);

        assert_eq!(count_named(&new, "atuin"), 0);
        assert_eq!(new.keybindings.len(), len - 1);
    }

    /// Assigning an empty list clears every binding.
    #[test]
    fn assigning_an_empty_list_clears_the_keybindings() {
        let old = Config::default();
        let mut new = old.clone();

        assign(&mut new, &old, vec![]);
        assert!(new.keybindings.is_empty(), "`= []` should reset the list");
    }

    /// The list is stored as written, so entries that bind the same key stay
    /// and keep their order. Reedline applies them in that order, thus the
    /// last one wins.
    #[test]
    fn the_list_is_stored_as_written() {
        let old = Config::default();
        let mut new = old.clone();

        let mut first = atuin();
        first.event = Value::test_string("first");
        let mut second = atuin();
        second.event = Value::test_string("second");
        assign(&mut new, &old, vec![first, second]);

        assert_eq!(new.keybindings.len(), 2);
        assert_eq!(new.keybindings[0].event, Value::test_string("first"));
        assert_eq!(new.keybindings[1].event, Value::test_string("second"));
    }

    /// The atuin case (nushell/nushell#18848): one name on two keys is two
    /// bindings.
    #[test]
    fn a_shared_name_on_different_keys_keeps_both_bindings() {
        let old = Config::default();
        let mut new = old.clone();

        append(
            &mut new,
            &old,
            vec![
                atuin(),
                keybinding(Some("atuin"), "none", "up", Value::test_string("emacs")),
            ],
        );

        assert_eq!(count_named(&new, "atuin"), 2);
    }

    // --- an entry that cannot be read rejects the whole list ---

    /// Run one `$env.config.<field> = [...]` assignment that may be rejected.
    fn try_assign_list<T: IntoValue>(
        config: &mut Config,
        old: &Config,
        field: &str,
        items: Vec<T>,
    ) -> Result<(), ShellError> {
        let list = Value::test_list(
            items
                .into_iter()
                .map(|item| item.into_value(Span::test_data()))
                .collect(),
        );
        let value = Value::test_record(record! { field => list });
        config.update_from_value(old, &value).map(drop)
    }

    /// Run one `$env.config.keybindings = [...]` assignment that may be rejected.
    fn try_assign(
        config: &mut Config,
        old: &Config,
        keybindings: Vec<ParsedKeybinding>,
    ) -> Result<(), ShellError> {
        try_assign_list(config, old, "keybindings", keybindings)
    }

    /// Run one `$env.config.menus = [...]` assignment that may be rejected.
    fn try_assign_menus(
        config: &mut Config,
        old: &Config,
        menus: Vec<ParsedMenu>,
    ) -> Result<(), ShellError> {
        try_assign_list(config, old, "menus", menus)
    }

    /// One bad entry among good ones: the list is rejected and the old one stays,
    /// like `table.mode = "bogus"` keeps the old mode.
    #[test]
    fn an_entry_that_cannot_be_read_rejects_the_whole_list() {
        let old = Config::default();
        let emacs = Value::test_string("emacs");
        let faulty = [
            keybinding(Some("bad_key"), "control", "nonsense", emacs.clone()),
            keybinding(Some("bad_modifier"), "bogus", "char_t", emacs),
            keybinding(
                Some("bad_mode"),
                "control",
                "char_t",
                Value::test_string("bogus"),
            ),
            keybinding(
                Some("bad_char"),
                "control",
                "char_ab",
                Value::test_string("emacs"),
            ),
        ];

        for bad in faulty {
            let name = bad
                .name
                .as_ref()
                .map(|name| name.to_expanded_string("", &old));
            let mut new = old.clone();
            let result = try_assign(&mut new, &old, vec![atuin(), bad]);

            assert!(result.is_err(), "{name:?} was accepted");
            assert_eq!(
                new.keybindings.len(),
                old.keybindings.len(),
                "{name:?} changed the list"
            );
            assert_eq!(
                count_named(&new, "atuin"),
                0,
                "{name:?} let a good entry through"
            );
        }
    }

    /// The scenario from a reload: a broken entry is appended, then the fixed
    /// config appends a good one. Only the good one ends up in the list.
    #[test]
    fn a_fixed_entry_appended_after_a_rejected_one_is_accepted() {
        let old = Config::default();
        let mut new = old.clone();
        let broken = || {
            keybinding(
                Some("mine"),
                "control",
                "nonsense",
                Value::test_string("emacs"),
            )
        };
        let fixed = || {
            keybinding(
                Some("mine"),
                "control",
                "char_t",
                Value::test_string("emacs"),
            )
        };

        let mut list = new.keybindings.clone();
        list.push(broken());
        assert!(try_assign(&mut new, &old, list).is_err());
        assert_eq!(new.keybindings.len(), old.keybindings.len());

        let mut list = new.keybindings.clone();
        list.push(fixed());
        assert!(try_assign(&mut new, &old, list).is_ok());
        assert_eq!(count_named(&new, "mine"), 1);
    }

    /// A rejected list is not a rejected config: other fields in the same
    /// record are still applied, as for any other value that can't be read.
    #[test]
    fn a_rejected_list_does_not_stop_other_fields() {
        let old = Config::default();
        let mut new = old.clone();
        let bad = keybinding(
            Some("bad"),
            "control",
            "nonsense",
            Value::test_string("emacs"),
        );
        let value = Value::test_record(record! {
            "keybindings" => Value::test_list(vec![bad.into_value(Span::test_data())]),
            "error_lines" => Value::test_int(old.error_lines + 3),
        });

        assert!(new.update_from_value(&old, &value).is_err());
        assert_eq!(new.error_lines, old.error_lines + 3);
        assert_eq!(new.keybindings.len(), old.keybindings.len());
    }

    /// An event needs reedline's types to be read, so it is read when the prompt
    /// is built and not when the list is assigned.
    #[test]
    fn an_event_is_not_checked_on_assignment() {
        let old = Config::default();
        let mut new = old.clone();
        let mut entry = atuin();
        entry.event = Value::test_record(record! { "send" => Value::test_string("bogus") });

        assert!(try_assign(&mut new, &old, vec![entry]).is_ok());
        assert_eq!(new.keybindings.len(), 1);
    }

    /// Entries without any mode bind nothing, but the key is checked all the same.
    #[test]
    fn an_entry_with_no_mode_still_needs_a_key_that_can_be_read() {
        let old = Config::default();
        let no_modes = Value::test_list(vec![]);

        let mut new = old.clone();
        let readable = keybinding(Some("idle"), "control", "char_t", no_modes.clone());
        assert!(try_assign(&mut new, &old, vec![readable]).is_ok());

        let mut new = old.clone();
        let unreadable = keybinding(Some("idle"), "control", "nonsense", no_modes);
        assert!(try_assign(&mut new, &old, vec![unreadable]).is_err());
        assert_eq!(new.keybindings.len(), old.keybindings.len());
    }

    /// A menu whose layout can't be read rejects the whole list too.
    #[test]
    fn a_menu_that_cannot_be_read_rejects_the_whole_list() {
        let old = Config::default();

        let mut grid = old.menus[0].clone();
        grid.name = Value::test_string("grid_menu");
        grid.r#type = Value::test_record(record! { "layout" => Value::test_string("grid") });
        let mut no_layout = grid.clone();
        no_layout.r#type = Value::test_record(record! {});
        let mut not_a_record = grid.clone();
        not_a_record.r#type = Value::test_string("columnar");

        // The modes are read on assignment too, not only the layout.
        let mut bad_input_mode = old.menus[0].clone();
        bad_input_mode.input_mode = Some(Value::test_string("bogus"));
        let mut no_input_mode = old.menus[0].clone();
        no_input_mode.input_mode = None;
        no_input_mode.only_buffer_difference = None;
        let mut bad_flag = no_input_mode.clone();
        bad_flag.only_buffer_difference = Some(Value::test_int(3));
        let mut bad_output_mode = old.menus[0].clone();
        bad_output_mode.output_mode = Some(Value::test_string("bogus"));

        for bad in [
            grid,
            no_layout,
            not_a_record,
            bad_input_mode,
            no_input_mode,
            bad_flag,
            bad_output_mode,
        ] {
            let mut new = old.clone();
            let good = old.menus[0].clone();
            let result = try_assign_menus(&mut new, &old, vec![good, bad]);

            assert!(result.is_err());
            assert_eq!(menu_names(&new), menu_names(&old));
        }
    }
}
