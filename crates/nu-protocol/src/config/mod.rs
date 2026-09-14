//! Module containing the internal representation of user configuration

use crate::config::reedline::{KeyIdentity, name_of};
use crate::{self as nu_protocol, Filesize};
use crate::{ConfigWarning, FromValue};
use helper::*;
use prelude::*;
use std::collections::{BTreeSet, HashMap};

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
pub use prompt::PromptConfig;
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
mod prompt;
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
    pub prompt: PromptConfig,
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
            prompt: PromptConfig::default(),
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
                // Not aliased onto the new key: `$env.config` is rebuilt from
                // `Config` after every update, so an alias would keep writes
                // working while reads of the old name failed.
                "render_right_prompt_on_last_line" => errors.deprecated_option(
                    current_path,
                    "use $env.config.prompt.render_right_on_last_line",
                    val.span(),
                ),
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
                "prompt" => self.prompt.update(val, current_path, errors),
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
                    Ok(keybindings) => self.merge_keybindings(keybindings, val.span(), errors),
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

    fn merge_keybindings(
        &mut self,
        incoming: Vec<ParsedKeybinding>,
        span: Span,
        errors: &mut ConfigErrors,
    ) {
        if incoming.is_empty() {
            self.keybindings.clear();
            return;
        }

        let mut shared_names = BTreeSet::new();
        let identities = incoming.into_iter().map(|kb| {
            let name = name_of(&kb);
            let id = KeyIdentity::of(&kb);
            (kb, name, id)
        });

        // Snapshot of existing identities, kept in lockstep with the list.
        // `claimed` marks entries already spoken for by this assignment, so a
        // second incoming binding with the same name starts a new entry
        // instead of re-keying its sibling.
        struct Existing {
            name: Option<String>,
            id: KeyIdentity,
            claimed: bool,
        }
        let mut ex_kbs: Vec<Existing> = self
            .keybindings
            .iter()
            .map(|ex| Existing {
                name: name_of(ex),
                id: KeyIdentity::of(ex),
                claimed: false,
            })
            .collect();

        for (kb, name, id) in identities {
            // The same binding (name and key): replace, claimed or not, so a
            // re-sourced config stays idempotent and event updates land.
            if let Some(i) = ex_kbs.iter().position(|ex| ex.name == name && ex.id == id) {
                ex_kbs[i].claimed = true;
                self.keybindings[i] = kb;
                continue;
            }

            // A named binding with a new key re-keys the unclaimed entry of
            // that name in place, keeping its position in the list.
            if name.is_some()
                && let Some(i) = ex_kbs.iter().position(|ex| ex.name == name && !ex.claimed)
            {
                ex_kbs[i].id = id;
                ex_kbs[i].claimed = true;
                self.keybindings[i] = kb;
                continue;
            }

            // A new binding. If its name is already taken (necessarily by a
            // claimed entry), both stay active; say so once.
            if let Some(name) = &name
                && ex_kbs.iter().any(|ex| ex.name.as_ref() == Some(name))
            {
                shared_names.insert(name.clone());
            }
            ex_kbs.push(Existing {
                name,
                id,
                claimed: true,
            });
            self.keybindings.push(kb);
        }
        if !shared_names.is_empty() {
            errors.warn(ConfigWarning::SharedKeybindingName {
                names: shared_names.into_iter().collect::<Vec<_>>().join(", "),
                span,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Aliasing instead would leave a config that writes fine and fails on
    /// read, since `$env.config` is rebuilt from `Config` after every update.
    #[test]
    fn the_old_render_right_prompt_key_is_reported_as_deprecated() {
        let old = Config::default();
        let mut new = old.clone();

        let result = new.update_from_value(
            &old,
            &Value::test_record(record! {
                "render_right_prompt_on_last_line" => Value::test_bool(true),
            }),
        );

        assert!(result.is_err(), "the moved key should report as deprecated");
        assert!(!new.prompt.render_right_on_last_line);
    }

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

    /// Guards the unnamed case: with no `name` to merge on, every reassignment
    /// used to append another copy.
    #[test]
    fn reassigning_an_unnamed_keybinding_does_not_duplicate_it() {
        let old = Config::default();
        let mut new = old.clone();

        let mut unnamed = old.keybindings[0].clone();
        unnamed.name = None;
        unnamed.modifier = Value::test_string("alt");
        unnamed.keycode = Value::test_string("char_j");
        new.keybindings.push(unnamed);

        let expected = new.keybindings.len();

        // Feed the list back through `update_from_value` the way re-sourcing a
        // config (or any `$env.config.keybindings = ...`) does.
        for _ in 0..2 {
            let value = Value::test_record(record! {
                "keybindings" => Value::test_list(
                    new.keybindings
                        .iter()
                        .map(|keybinding| keybinding.clone().into_value(Span::test_data()))
                        .collect(),
                ),
            });
            new.update_from_value(&old, &value)
                .expect("update should succeed");
        }

        assert_eq!(
            new.keybindings.len(),
            expected,
            "reassigning `keybindings` duplicated the unnamed binding"
        );
    }

    // --- merge semantics: replace on same name+key, append+warn on shared name ---

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

    /// Run one `$env.config.keybindings = [...]` assignment; returns the warning.
    fn assign(
        config: &mut Config,
        old: &Config,
        keybindings: Vec<ParsedKeybinding>,
    ) -> Option<ShellWarning> {
        let value = Value::test_record(record! {
            "keybindings" => Value::test_list(
                keybindings
                    .into_iter()
                    .map(|kb| kb.into_value(Span::test_data()))
                    .collect(),
            ),
        });
        config
            .update_from_value(old, &value)
            .expect("update should succeed")
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

    /// The atuin regression (nushell/nushell#18848): two bindings sharing a name
    /// on different keys must both survive, with one warning.
    #[test]
    fn a_shared_name_on_different_keys_keeps_both_bindings_and_warns() {
        let old = Config::default();
        let mut new = old.clone();

        let warning = assign(
            &mut new,
            &old,
            vec![
                keybinding(
                    Some("atuin"),
                    "control",
                    "char_r",
                    Value::test_string("emacs"),
                ),
                keybinding(Some("atuin"), "none", "up", Value::test_string("emacs")),
            ],
        );

        assert_eq!(
            count_named(&new, "atuin"),
            2,
            "one of the bindings was dropped"
        );
        assert!(warning.is_some(), "sharing a name should warn");
    }

    /// Re-sourcing the exact same binding is idempotent and silent.
    #[test]
    fn reassigning_the_same_binding_replaces_it_without_warning() {
        let old = Config::default();
        let mut new = old.clone();

        let atuin = || {
            keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_string("emacs"),
            )
        };
        assign(&mut new, &old, vec![atuin()]);
        let len = new.keybindings.len();

        let warning = assign(&mut new, &old, vec![atuin()]);
        assert_eq!(
            new.keybindings.len(),
            len,
            "re-sourcing duplicated the binding"
        );
        assert!(
            warning.is_none(),
            "an identical re-assignment must not warn"
        );
    }

    /// Same name and key with a new event is the update case: replaced in place.
    #[test]
    fn a_new_event_on_the_same_key_replaces_the_binding() {
        let old = Config::default();
        let mut new = old.clone();

        assign(
            &mut new,
            &old,
            vec![keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_string("emacs"),
            )],
        );
        let len = new.keybindings.len();

        let mut updated = keybinding(
            Some("atuin"),
            "control",
            "char_r",
            Value::test_string("emacs"),
        );
        updated.event = Value::test_string("marker");
        assign(&mut new, &old, vec![updated]);

        assert_eq!(new.keybindings.len(), len);
        let event = new
            .keybindings
            .iter()
            .rev()
            .find(|kb| {
                kb.name
                    .as_ref()
                    .is_some_and(|n| n.to_expanded_string("", &new) == "atuin")
            })
            .map(|kb| kb.event.clone());
        assert_eq!(
            event,
            Some(Value::test_string("marker")),
            "event was not updated"
        );
    }

    /// `emacs` and `[emacs]` spell the same key, so the second assignment replaces.
    #[test]
    fn a_bare_mode_and_its_singleton_list_merge_into_one_binding() {
        let old = Config::default();
        let mut new = old.clone();

        assign(
            &mut new,
            &old,
            vec![keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_string("emacs"),
            )],
        );
        let warning = assign(
            &mut new,
            &old,
            vec![keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_list(vec![Value::test_string("emacs")]),
            )],
        );

        assert_eq!(
            count_named(&new, "atuin"),
            1,
            "the mode spellings did not merge"
        );
        assert!(warning.is_none());
    }

    /// The same new binding twice in one assignment collapses to one entry
    /// (guards the identity snapshot staying in sync with the list).
    #[test]
    fn the_same_binding_twice_in_one_assignment_is_stored_once() {
        let old = Config::default();
        let mut new = old.clone();

        let atuin = || {
            keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_string("emacs"),
            )
        };
        assign(&mut new, &old, vec![atuin(), atuin()]);

        assert_eq!(count_named(&new, "atuin"), 1, "the duplicate was appended");
    }

    /// Re-keying by name: assigning a named binding with a new key replaces the
    /// existing binding of that name in place, keeping its list position
    /// (`$env.config.keybindings.0.keycode = ...` depends on this).
    #[test]
    fn a_named_binding_with_a_new_key_replaces_in_place() {
        let old = Config::default();
        let mut new = old.clone();

        assign(
            &mut new,
            &old,
            vec![keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_string("emacs"),
            )],
        );
        let len = new.keybindings.len();
        let index = new
            .keybindings
            .iter()
            .position(|kb| {
                kb.name
                    .as_ref()
                    .is_some_and(|n| n.to_expanded_string("", &new) == "atuin")
            })
            .expect("binding was added");

        let warning = assign(
            &mut new,
            &old,
            vec![keybinding(
                Some("atuin"),
                "none",
                "up",
                Value::test_string("emacs"),
            )],
        );

        assert_eq!(new.keybindings.len(), len, "re-keying must not append");
        assert_eq!(
            new.keybindings[index].keycode,
            Value::test_string("up"),
            "the binding was not re-keyed in place"
        );
        assert!(warning.is_none(), "re-keying a lone name must not warn");
    }

    /// A changed mode set is a re-key too, not a sibling binding.
    #[test]
    fn a_named_binding_with_a_changed_mode_replaces_instead_of_appending() {
        let old = Config::default();
        let mut new = old.clone();

        assign(
            &mut new,
            &old,
            vec![keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_string("emacs"),
            )],
        );
        let warning = assign(
            &mut new,
            &old,
            vec![keybinding(
                Some("atuin"),
                "control",
                "char_r",
                Value::test_list(vec![
                    Value::test_string("vi_normal"),
                    Value::test_string("vi_insert"),
                ]),
            )],
        );

        assert_eq!(count_named(&new, "atuin"), 1, "the mode change appended");
        assert!(warning.is_none());
    }

    /// Assigning an empty list is the reset escape hatch.
    #[test]
    fn assigning_an_empty_list_clears_the_keybindings() {
        let old = Config::default();
        let mut new = old.clone();

        assign(&mut new, &old, vec![]);
        assert!(new.keybindings.is_empty(), "`= []` should reset the list");
    }
}
