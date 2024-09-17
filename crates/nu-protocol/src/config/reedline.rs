use super::{extract_value, prelude::*};
use crate as nu_protocol;
use crate::ShellError;

/// Definition of a parsed keybinding from the config object
#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct ParsedKeybinding {
    pub modifier: Value,
    pub keycode: Value,
    pub event: Value,
    pub mode: Value,
}

/// Definition of a parsed menu from the config object
#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct ParsedMenu {
    pub name: Value,
    pub marker: Value,
    pub only_buffer_difference: Value,
    pub style: Value,
    pub r#type: Value,
    pub source: Value,
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

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorShapeConfig {
    pub emacs: NuCursorShape,
    pub vi_insert: NuCursorShape,
    pub vi_normal: NuCursorShape,
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

/// Parses the config object to extract the strings that will compose a keybinding for reedline
pub(super) fn create_keybindings(value: &Value) -> Result<Vec<ParsedKeybinding>, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val, .. } => {
            // Finding the modifier value in the record
            let modifier = extract_value("modifier", val, span)?.clone();
            let keycode = extract_value("keycode", val, span)?.clone();
            let mode = extract_value("mode", val, span)?.clone();
            let event = extract_value("event", val, span)?.clone();

            let keybinding = ParsedKeybinding {
                modifier,
                keycode,
                mode,
                event,
            };

            // We return a menu to be able to do recursion on the same function
            Ok(vec![keybinding])
        }
        Value::List { vals, .. } => {
            let res = vals
                .iter()
                .map(create_keybindings)
                .collect::<Result<Vec<Vec<ParsedKeybinding>>, ShellError>>();

            let res = res?
                .into_iter()
                .flatten()
                .collect::<Vec<ParsedKeybinding>>();

            Ok(res)
        }
        _ => Ok(Vec::new()),
    }
}

/// Parses the config object to extract the strings that will compose a keybinding for reedline
pub fn create_menus(value: &Value) -> Result<Vec<ParsedMenu>, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val, .. } => {
            // Finding the modifier value in the record
            let name = extract_value("name", val, span)?.clone();
            let marker = extract_value("marker", val, span)?.clone();
            let only_buffer_difference =
                extract_value("only_buffer_difference", val, span)?.clone();
            let style = extract_value("style", val, span)?.clone();
            let r#type = extract_value("type", val, span)?.clone();

            // Source is an optional value
            let source = match extract_value("source", val, span) {
                Ok(source) => source.clone(),
                Err(_) => Value::nothing(span),
            };

            let menu = ParsedMenu {
                name,
                only_buffer_difference,
                marker,
                style,
                r#type,
                source,
            };

            Ok(vec![menu])
        }
        Value::List { vals, .. } => {
            let res = vals
                .iter()
                .map(create_menus)
                .collect::<Result<Vec<Vec<ParsedMenu>>, ShellError>>();

            let res = res?.into_iter().flatten().collect::<Vec<ParsedMenu>>();

            Ok(res)
        }
        _ => Ok(Vec::new()),
    }
}
