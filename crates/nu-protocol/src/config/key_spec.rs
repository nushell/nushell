//! The text grammar of a keybinding's `modifier`, `keycode` and `mode`, and of a
//! menu's `layout`.
//!
//! The grammar lives here, and not next to reedline, so that an assignment to
//! `$env.config.keybindings` or `$env.config.menus` can be checked when it is
//! made: a list with an entry that can't be read is rejected and the previous
//! list stays, like for any other config value. `nu-cli` maps these types onto
//! `crossterm` and reedline, so both read an entry the same way.
//!
//! What is checked here is what the text names. An entry's `event` needs
//! reedline's own types, so it is still read when the prompt is built.

use super::{ParsedKeybinding, ParsedMenu, helper::extract_value, prelude::*};
use crate::Config;
use strum_macros::EnumString;

/// A modifier key of a keybinding, spelled in any case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum KeyModifier {
    Control,
    Shift,
    Alt,
    Super,
    Hyper,
    Meta,
}

/// The key of a keybinding, apart from its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCodeSpec {
    Backspace,
    Enter,
    Down,
    Up,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    /// A function key, `f1` to `f35`.
    F(u8),
    /// A character, including `space`.
    Char(char),
    Null,
    Esc,
}

/// The edit mode a keybinding is for, which is the table reedline keeps it in.
///
/// Each is spelled either `vi_normal` or `ViNormal`, in any case, like reedline's
/// `PromptEditModeDiscriminants`, which `switch_mode_target` in `nu-cli` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum EditModeName {
    Emacs,
    #[strum(serialize = "ViNormal", serialize = "vi_normal")]
    ViNormal,
    #[strum(serialize = "ViInsert", serialize = "vi_insert")]
    ViInsert,
    #[strum(serialize = "ViVisual", serialize = "vi_visual")]
    ViVisual,
    #[strum(serialize = "HelixNormal", serialize = "helix_normal")]
    HelixNormal,
    #[strum(serialize = "HelixInsert", serialize = "helix_insert")]
    HelixInsert,
    #[strum(serialize = "HelixSelect", serialize = "helix_select")]
    HelixSelect,
}

/// The layout a menu is shown with, spelled in lowercase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum MenuLayout {
    Columnar,
    List,
    Ide,
    Description,
}

const VALID_MODIFIERS: &str = "'control', 'shift', 'alt', 'super', 'hyper', 'meta', or 'none'";
/// The edit modes a keybinding can name, as the error says.
pub const VALID_KEYBINDING_MODES: &str = "'emacs', 'vi_insert', 'vi_normal', 'vi_visual', 'helix_insert', 'helix_normal', or 'helix_select'";
const VALID_LAYOUTS: &str = "'columnar', 'list', 'ide', or 'description'";

impl ParsedKeybinding {
    /// The modifiers of this entry: `none`, or `_`-joined names such as `control_shift`.
    pub fn modifiers(&self) -> Result<Vec<KeyModifier>, ShellError> {
        let Ok(modifier) = self.modifier.as_str() else {
            return Err(ShellError::RuntimeTypeMismatch {
                expected: Type::String,
                actual: self.modifier.get_type(),
                span: self.modifier.span(),
            });
        };

        if modifier.eq_ignore_ascii_case("none") {
            return Ok(Vec::new());
        }
        modifier
            .split('_')
            .map(|part| {
                part.parse().map_err(|_| ShellError::InvalidValue {
                    valid: VALID_MODIFIERS.into(),
                    actual: format!("'{part}'"),
                    span: self.modifier.span(),
                })
            })
            .collect()
    }

    /// The key of this entry, apart from its modifiers.
    pub fn key_code(&self) -> Result<KeyCodeSpec, ShellError> {
        let Ok(keycode) = self.keycode.as_str() else {
            return Err(ShellError::RuntimeTypeMismatch {
                expected: Type::String,
                actual: self.keycode.get_type(),
                span: self.keycode.span(),
            });
        };
        let span = self.keycode.span();
        let keycode_lower = keycode.to_ascii_lowercase();

        if let Some(rest) = keycode_lower.strip_prefix("char_") {
            let error = |valid: &str, actual: &str| ShellError::InvalidValue {
                valid: valid.into(),
                actual: actual.into(),
                span,
            };

            let mut chars = rest.chars();
            let char = match (chars.next(), chars.next()) {
                (Some(char), None) => char,
                (Some('u'), Some(_)) => {
                    // `rest` starts with the ASCII `u`, so slicing after it is safe.
                    let Ok(code_point) = u32::from_str_radix(&rest[1..], 16) else {
                        return Err(error("a valid hex code", keycode));
                    };
                    char::from_u32(code_point)
                        .ok_or_else(|| error("a valid Unicode code point", keycode))?
                }
                _ => return Err(error("'char_<char>' or 'char_u<hex code>'", keycode)),
            };
            return Ok(KeyCodeSpec::Char(char));
        }

        Ok(match keycode_lower.as_str() {
            "backspace" => KeyCodeSpec::Backspace,
            "enter" => KeyCodeSpec::Enter,
            "space" => KeyCodeSpec::Char(' '),
            "down" => KeyCodeSpec::Down,
            "up" => KeyCodeSpec::Up,
            "left" => KeyCodeSpec::Left,
            "right" => KeyCodeSpec::Right,
            "home" => KeyCodeSpec::Home,
            "end" => KeyCodeSpec::End,
            "pageup" => KeyCodeSpec::PageUp,
            "pagedown" => KeyCodeSpec::PageDown,
            "tab" => KeyCodeSpec::Tab,
            "backtab" => KeyCodeSpec::BackTab,
            "delete" => KeyCodeSpec::Delete,
            "insert" => KeyCodeSpec::Insert,
            name if name.starts_with('f') => name[1..]
                .parse()
                .ok()
                .filter(|num| (1..=35).contains(num))
                .map(KeyCodeSpec::F)
                .ok_or(ShellError::InvalidValue {
                    valid: "'f1', 'f2', ..., or 'f35'".into(),
                    actual: format!("'{keycode}'"),
                    span,
                })?,
            "null" => KeyCodeSpec::Null,
            "esc" | "escape" => KeyCodeSpec::Esc,
            _ => {
                return Err(ShellError::InvalidValue {
                    valid: "a crossterm KeyCode".into(),
                    actual: format!("'{keycode}'"),
                    span,
                });
            }
        })
    }

    /// The edit modes this entry binds the key in, in the order written.
    /// `mode` is a name or a list of names, and lists may nest.
    pub fn modes(&self) -> Result<Vec<EditModeName>, ShellError> {
        fn collect(mode: &Value, modes: &mut Vec<EditModeName>) -> Result<(), ShellError> {
            match mode {
                Value::String { val, .. } => {
                    modes.push(val.parse().map_err(|_| ShellError::InvalidValue {
                        valid: VALID_KEYBINDING_MODES.into(),
                        actual: format!("'{val}'"),
                        span: mode.span(),
                    })?);
                }
                Value::List { vals, .. } => {
                    for inner in vals {
                        collect(inner, modes)?;
                    }
                }
                v => {
                    return Err(ShellError::RuntimeTypeMismatch {
                        expected: Type::custom("string or list<string>"),
                        actual: v.get_type(),
                        span: v.span(),
                    });
                }
            }
            Ok(())
        }

        let mut modes = Vec::new();
        collect(&self.mode, &mut modes)?;
        Ok(modes)
    }

    /// Checks what the text of this entry names: its modes, modifier and
    /// keycode. The `event` is not checked, see the module documentation.
    pub(crate) fn validate(&self) -> Result<(), ShellError> {
        self.modes()?;
        self.modifiers()?;
        self.key_code()?;
        Ok(())
    }
}

impl ParsedMenu {
    /// The layout this menu is shown with. Its `type` must be a record with a
    /// `layout` that names one of the four layouts.
    pub fn layout(&self, config: &Config) -> Result<MenuLayout, ShellError> {
        let span = self.r#type.span();
        let Value::Record { val, .. } = &self.r#type else {
            return Err(ShellError::RuntimeTypeMismatch {
                expected: Type::record(),
                actual: self.r#type.get_type(),
                span,
            });
        };

        let layout = extract_value("layout", val, span)?.to_expanded_string("", config);
        layout.parse().map_err(|_| ShellError::InvalidValue {
            valid: VALID_LAYOUTS.into(),
            actual: format!("'{layout}'"),
            span,
        })
    }

    /// Checks what the text of this menu names: its layout.
    pub(crate) fn validate(&self, config: &Config) -> Result<(), ShellError> {
        self.layout(config).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(modifier: &str, keycode: &str, mode: Value) -> ParsedKeybinding {
        ParsedKeybinding {
            name: None,
            modifier: Value::test_string(modifier),
            keycode: Value::test_string(keycode),
            event: Value::test_nothing(),
            mode,
        }
    }

    fn emacs() -> Value {
        Value::test_string("emacs")
    }

    fn code(keycode: &str) -> Result<KeyCodeSpec, ShellError> {
        entry("none", keycode, emacs()).key_code()
    }

    fn modifiers(modifier: &str) -> Result<Vec<KeyModifier>, ShellError> {
        entry(modifier, "enter", emacs()).modifiers()
    }

    fn modes(mode: Value) -> Result<Vec<EditModeName>, ShellError> {
        entry("none", "enter", mode).modes()
    }

    /// What an `InvalidValue` error says was wrong with the text, or what came back
    /// instead, so that a test comparing it fails with that.
    fn rejected(result: Result<impl std::fmt::Debug, ShellError>) -> String {
        match result {
            Err(ShellError::InvalidValue { actual, .. }) => actual,
            other => format!("not an invalid value: {other:?}"),
        }
    }

    #[test]
    fn modifiers_are_an_unordered_set_in_any_case() -> Result<(), ShellError> {
        assert_eq!(modifiers("none")?, vec![]);
        assert_eq!(modifiers("None")?, vec![]);
        assert_eq!(
            modifiers("Control_shift")?,
            vec![KeyModifier::Control, KeyModifier::Shift]
        );
        assert_eq!(
            modifiers("super_hyper_meta_alt")?,
            vec![
                KeyModifier::Super,
                KeyModifier::Hyper,
                KeyModifier::Meta,
                KeyModifier::Alt
            ]
        );
        Ok(())
    }

    #[test]
    fn a_modifier_that_is_not_one_is_named_in_the_error() {
        assert_eq!(rejected(modifiers("control_bogus")), "'bogus'");
        assert_eq!(rejected(modifiers("")), "''");
    }

    #[test]
    fn a_value_that_is_not_text_is_a_type_mismatch() {
        let mut keybinding = entry("none", "enter", emacs());
        keybinding.modifier = Value::test_int(1);
        keybinding.keycode = Value::test_list(vec![]);
        keybinding.mode = Value::test_int(1);

        let results = [
            keybinding.modifiers().map(drop),
            keybinding.key_code().map(drop),
            keybinding.modes().map(drop),
        ];
        for result in results {
            assert!(
                matches!(result, Err(ShellError::RuntimeTypeMismatch { .. })),
                "{result:?}"
            );
        }
    }

    #[test]
    fn keycodes_name_characters_in_any_case() -> Result<(), ShellError> {
        assert_eq!(code("char_a")?, KeyCodeSpec::Char('a'));
        // The keycode is lowercased before it is read, so `char_A` is `char_a`.
        assert_eq!(code("Char_A")?, KeyCodeSpec::Char('a'));
        assert_eq!(code("char_u1F600")?, KeyCodeSpec::Char('\u{1F600}'));
        assert_eq!(code("char_u")?, KeyCodeSpec::Char('u'));
        assert_eq!(code("space")?, KeyCodeSpec::Char(' '));
        Ok(())
    }

    #[test]
    fn a_character_keycode_that_is_not_one_is_named_in_the_error() {
        assert_eq!(rejected(code("char_ab")), "char_ab");
        assert_eq!(rejected(code("char_")), "char_");
        assert_eq!(rejected(code("char_uzz")), "char_uzz");
        // A surrogate is not a character.
        assert_eq!(rejected(code("char_ud800")), "char_ud800");
    }

    #[test]
    fn keycodes_name_the_keys_nushell_knows() -> Result<(), ShellError> {
        let named = [
            ("backspace", KeyCodeSpec::Backspace),
            ("enter", KeyCodeSpec::Enter),
            ("down", KeyCodeSpec::Down),
            ("up", KeyCodeSpec::Up),
            ("left", KeyCodeSpec::Left),
            ("right", KeyCodeSpec::Right),
            ("home", KeyCodeSpec::Home),
            ("end", KeyCodeSpec::End),
            ("pageup", KeyCodeSpec::PageUp),
            ("pagedown", KeyCodeSpec::PageDown),
            ("tab", KeyCodeSpec::Tab),
            ("backtab", KeyCodeSpec::BackTab),
            ("delete", KeyCodeSpec::Delete),
            ("insert", KeyCodeSpec::Insert),
            ("null", KeyCodeSpec::Null),
            ("esc", KeyCodeSpec::Esc),
            ("Escape", KeyCodeSpec::Esc),
            ("PageUp", KeyCodeSpec::PageUp),
            ("f1", KeyCodeSpec::F(1)),
            ("F35", KeyCodeSpec::F(35)),
        ];
        for (name, expected) in named {
            assert_eq!(code(name)?, expected, "{name}");
        }
        Ok(())
    }

    #[test]
    fn a_keycode_that_is_not_one_is_named_in_the_error() {
        // The function keys run from f1 to f35, and any name starting with `f` is read as one.
        for bad in ["f0", "f36", "f", "fx", "f-1", "foo", "nonsense", ""] {
            assert_eq!(rejected(code(bad)), format!("'{bad}'"), "{bad}");
        }
    }

    #[test]
    fn a_mode_is_a_name_or_a_list_of_names_that_may_nest() -> Result<(), ShellError> {
        assert_eq!(modes(emacs())?, vec![EditModeName::Emacs]);
        assert_eq!(
            modes(Value::test_list(vec![
                Value::test_string("vi_normal"),
                Value::test_list(vec![Value::test_string("helix_select")]),
            ]))?,
            vec![EditModeName::ViNormal, EditModeName::HelixSelect]
        );
        // No mode binds nothing.
        assert_eq!(modes(Value::test_list(vec![]))?, vec![]);
        Ok(())
    }

    #[test]
    fn a_mode_may_be_spelled_either_way_in_any_case() -> Result<(), ShellError> {
        for (name, expected) in [
            ("ViNormal", EditModeName::ViNormal),
            ("vi_normal", EditModeName::ViNormal),
            ("VI_INSERT", EditModeName::ViInsert),
            ("ViVisual", EditModeName::ViVisual),
            ("helix_normal", EditModeName::HelixNormal),
            ("HelixInsert", EditModeName::HelixInsert),
            ("helix_select", EditModeName::HelixSelect),
            ("EMACS", EditModeName::Emacs),
        ] {
            assert_eq!(modes(Value::test_string(name))?, vec![expected], "{name}");
        }
        Ok(())
    }

    #[test]
    fn a_mode_that_is_not_one_is_named_in_the_error() {
        // `default` and `custom` are edit modes of the prompt, but no table to bind in.
        for bad in ["bogus", "default", "custom", "vi", ""] {
            assert_eq!(
                rejected(modes(Value::test_string(bad))),
                format!("'{bad}'"),
                "{bad}"
            );
        }
        assert_eq!(
            rejected(modes(Value::test_list(vec![
                emacs(),
                Value::test_string("nope")
            ]))),
            "'nope'"
        );
    }

    #[test]
    fn validation_covers_the_modes_the_modifier_and_the_keycode_but_not_the_event() {
        assert!(entry("control", "char_t", emacs()).validate().is_ok());
        assert!(entry("bogus", "char_t", emacs()).validate().is_err());
        assert!(entry("control", "bogus", emacs()).validate().is_err());
        assert!(
            entry("control", "char_t", Value::test_string("bogus"))
                .validate()
                .is_err()
        );

        let mut with_bad_event = entry("control", "char_t", emacs());
        with_bad_event.event =
            Value::test_record(record! { "send" => Value::test_string("bogus") });
        assert!(with_bad_event.validate().is_ok());
    }

    fn menu(layout: Value) -> ParsedMenu {
        ParsedMenu {
            name: Value::test_string("m"),
            marker: Value::test_string("| "),
            only_buffer_difference: None,
            input_mode: None,
            output_mode: None,
            style: Value::test_record(record! {}),
            r#type: layout,
            source: None,
        }
    }

    fn layout_of(layout: &str) -> Result<MenuLayout, ShellError> {
        let config = Config::default();
        menu(Value::test_record(
            record! { "layout" => Value::test_string(layout) },
        ))
        .layout(&config)
    }

    #[test]
    fn a_menu_layout_is_one_of_four_lowercase_names() -> Result<(), ShellError> {
        for (name, expected) in [
            ("columnar", MenuLayout::Columnar),
            ("list", MenuLayout::List),
            ("ide", MenuLayout::Ide),
            ("description", MenuLayout::Description),
        ] {
            assert_eq!(layout_of(name)?, expected, "{name}");
        }
        for bad in ["grid", "Columnar", ""] {
            assert_eq!(rejected(layout_of(bad)), format!("'{bad}'"), "{bad}");
        }
        Ok(())
    }

    #[test]
    fn a_menu_type_needs_a_layout_in_a_record() {
        let config = Config::default();
        assert!(matches!(
            menu(Value::test_record(record! {})).layout(&config),
            Err(ShellError::MissingRequiredColumn { .. })
        ));
        assert!(matches!(
            menu(Value::test_string("columnar")).layout(&config),
            Err(ShellError::RuntimeTypeMismatch { .. })
        ));
    }

    /// What `config default` and a fresh shell hold must be readable.
    #[test]
    fn the_defaults_are_valid() {
        let config = Config::default();
        for keybinding in &config.keybindings {
            assert!(keybinding.validate().is_ok(), "{:?}", keybinding.name);
        }
        for menu in &config.menus {
            assert!(menu.validate(&config).is_ok(), "{:?}", menu.name);
        }
    }
}
