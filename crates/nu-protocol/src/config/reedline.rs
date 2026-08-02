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
}

impl FromStr for EditBindings {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "vi" => Ok(Self::Vi),
            "emacs" => Ok(Self::Emacs),
            _ => Err("'emacs' or 'vi'"),
        }
    }
}

impl UpdateFromValue for EditBindings {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}
