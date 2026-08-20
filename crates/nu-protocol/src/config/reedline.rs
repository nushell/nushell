use std::collections::BTreeSet;

use super::{config_update_string_enum, prelude::*};
use crate as nu_protocol;
use crate::{FromValue, engine::Closure};

/// Definition of a parsed keybinding from the config object
#[derive(Clone, Debug, FromValue, IntoValue, Serialize, Deserialize)]
pub struct ParsedKeybinding {
    pub name: Option<Value>,
    pub modifier: Value,
    pub keycode: Value,
    pub event: Value,
    pub mode: Value,
}

pub(crate) fn name_of(kb: &ParsedKeybinding) -> Option<String> {
    kb.name
        .as_ref()
        .and_then(|v| v.coerce_str().ok())
        .map(|s| s.to_string())
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct KeyIdentity {
    modifier: BTreeSet<String>,
    keycode: String,
    modes: BTreeSet<String>,
}

impl KeyIdentity {
    pub(crate) fn of(kb: &ParsedKeybinding) -> Self {
        let lower = |v: &Value| {
            v.coerce_str()
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default()
        };
        let modes = match &kb.mode {
            Value::List { vals, .. } => vals.iter().map(lower).collect(),
            v => BTreeSet::from([lower(v)]),
        };

        Self {
            // Best-effort mirror of `add_parsed_keybinding`'s reading: modifiers
            // are an unordered `_`-joined set and `esc`/`escape` are aliases. The
            // exotic overlaps (`space` vs `char_ `, `char_u<hex>` vs `char_<c>`)
            // are deliberately not canonicalized; a mismatch only costs an
            // append plus a warning, never a lost binding.
            modifier: lower(&kb.modifier).split('_').map(|s| s.into()).collect(),
            keycode: match lower(&kb.keycode).as_str() {
                "esc" => "escape".into(),
                other => other.into(),
            },
            modes,
        }
    }
}

/// Definition of a parsed menu from the config object
#[derive(Clone, Debug, FromValue, IntoValue, Serialize, Deserialize)]
pub struct ParsedMenu {
    pub name: Value,
    pub marker: Value,
    /// Legacy two-state input behavior. Required unless `input_mode` is set,
    /// which supersedes it.
    pub only_buffer_difference: Option<Value>,
    /// Optional reedline `InputMode` ("diff" / "cursor_prefix" / "full_buffer").
    /// Supersedes `only_buffer_difference` when set; absent keeps current behavior.
    pub input_mode: Option<Value>,
    /// Optional reedline `OutputMode` ("suggested_span" / "full_buffer" / "extend_to_end").
    pub output_mode: Option<Value>,
    pub style: Value,
    pub r#type: Value,
    pub source: Option<Closure>,
}

/// Definition of a Nushell CursorShape (to be mapped to crossterm::cursor::CursorShape)
#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum NuCursorShape {
    Underscore,
    Line,
    Block,
    BlinkUnderscore,
    BlinkLine,
    BlinkBlock,
    #[default]
    Inherit,
}

impl FromStr for NuCursorShape {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<NuCursorShape, &'static str> {
        match s.to_ascii_lowercase().as_str() {
            "line" => Ok(NuCursorShape::Line),
            "block" => Ok(NuCursorShape::Block),
            "underscore" => Ok(NuCursorShape::Underscore),
            "blink_line" => Ok(NuCursorShape::BlinkLine),
            "blink_block" => Ok(NuCursorShape::BlinkBlock),
            "blink_underscore" => Ok(NuCursorShape::BlinkUnderscore),
            "inherit" => Ok(NuCursorShape::Inherit),
            _ => Err(
                "'line', 'block', 'underscore', 'blink_line', 'blink_block', 'blink_underscore' or 'inherit'",
            ),
        }
    }
}

impl UpdateFromValue for NuCursorShape {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorShapeConfig {
    pub emacs: NuCursorShape,
    pub vi_insert: NuCursorShape,
    pub vi_normal: NuCursorShape,
    /// Only takes effect in builds with the `helix` feature.
    pub helix_normal: NuCursorShape,
    pub helix_select: NuCursorShape,
    pub helix_insert: NuCursorShape,
}

impl UpdateFromValue for CursorShapeConfig {
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
                "vi_insert" => self.vi_insert.update(val, path, errors),
                "vi_normal" => self.vi_normal.update(val, path, errors),
                "emacs" => self.emacs.update(val, path, errors),
                "helix_normal" => self.helix_normal.update(val, path, errors),
                "helix_select" => self.helix_select.update(val, path, errors),
                "helix_insert" => self.helix_insert.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditBindings {
    Vi,
    #[default]
    Emacs,
    /// Only usable in builds with the `helix` feature; selecting it elsewhere
    /// reports an error when the keybindings are constructed.
    Helix,
}

impl FromStr for EditBindings {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "vi" => Ok(Self::Vi),
            "emacs" => Ok(Self::Emacs),
            "helix" => Ok(Self::Helix),
            _ => Err("'emacs', 'vi' or 'helix'"),
        }
    }
}

impl UpdateFromValue for EditBindings {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kb(modifier: &str, keycode: &str, mode: Value) -> ParsedKeybinding {
        ParsedKeybinding {
            name: None,
            modifier: Value::test_string(modifier),
            keycode: Value::test_string(keycode),
            event: Value::test_nothing(),
            mode,
        }
    }

    #[test]
    fn a_bare_mode_and_its_singleton_list_are_the_same_key() {
        let bare = kb("control", "char_r", Value::test_string("emacs"));
        let listed = kb(
            "control",
            "char_r",
            Value::test_list(vec![Value::test_string("emacs")]),
        );
        assert_eq!(KeyIdentity::of(&bare), KeyIdentity::of(&listed));
    }

    #[test]
    fn mode_list_order_does_not_matter() {
        let forward = kb(
            "control",
            "char_r",
            Value::test_list(vec![
                Value::test_string("emacs"),
                Value::test_string("vi_insert"),
            ]),
        );
        let reversed = kb(
            "control",
            "char_r",
            Value::test_list(vec![
                Value::test_string("vi_insert"),
                Value::test_string("emacs"),
            ]),
        );
        assert_eq!(KeyIdentity::of(&forward), KeyIdentity::of(&reversed));
    }

    #[test]
    fn spelling_case_does_not_matter() {
        let lower = kb("control", "char_r", Value::test_string("emacs"));
        let upper = kb("Control", "Char_R", Value::test_string("Emacs"));
        assert_eq!(KeyIdentity::of(&lower), KeyIdentity::of(&upper));
    }

    // Canonicalization to match `add_parsed_keybinding`'s reading of the fields:
    // modifiers are an unordered `_`-joined set, `esc`/`escape` are aliases,
    // and mode names keep their underscores.

    #[test]
    fn modifier_component_order_does_not_matter() {
        let cs = kb("control_shift", "char_r", Value::test_string("emacs"));
        let sc = kb("shift_control", "char_r", Value::test_string("emacs"));
        assert_eq!(KeyIdentity::of(&cs), KeyIdentity::of(&sc));
    }

    #[test]
    fn esc_and_escape_are_the_same_key() {
        let esc = kb("none", "esc", Value::test_string("emacs"));
        let escape = kb("none", "escape", Value::test_string("emacs"));
        assert_eq!(KeyIdentity::of(&esc), KeyIdentity::of(&escape));
    }

    #[test]
    fn mode_names_are_not_split_on_underscores() {
        // Guards the tokenizer split: `vi_normal` is one mode, not `vi` + `normal`.
        let whole = kb("none", "char_r", Value::test_string("vi_normal"));
        let parts = kb(
            "none",
            "char_r",
            Value::test_list(vec![Value::test_string("vi"), Value::test_string("normal")]),
        );
        assert_ne!(KeyIdentity::of(&whole), KeyIdentity::of(&parts));
    }

    #[test]
    fn a_different_key_is_a_different_identity() {
        let ctrl_r = kb("control", "char_r", Value::test_string("emacs"));
        let up = kb("none", "up", Value::test_string("emacs"));
        assert_ne!(KeyIdentity::of(&ctrl_r), KeyIdentity::of(&up));
    }
}
