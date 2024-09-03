use super::{
    config_update_string_enum, prelude::*, report_invalid_config_key, report_invalid_config_value,
};
use crate as nu_protocol;
use crate::{engine::Closure, FromValue};

/// Definition of a parsed keybinding from the config object
#[derive(Clone, Debug, FromValue, IntoValue, Serialize, Deserialize)]
pub struct ParsedKeybinding {
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
    pub only_buffer_difference: Value,
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
        _ => Err("expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', 'blink_underscore' or 'inherit'"),
        }
    }
}

impl UpdateFromValue for NuCursorShape {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
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
                "vi_insert" => self.vi_insert.update(val, path, errors),
                "vi_normal" => self.vi_normal.update(val, path, errors),
                "emacs" => self.emacs.update(val, path, errors),
                _ => report_invalid_config_key(span, path, errors),
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
            _ => Err("expected either 'emacs' or 'vi'"),
        }
    }
}

impl UpdateFromValue for EditBindings {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
        config_update_string_enum(self, value, path, errors)
    }
}
