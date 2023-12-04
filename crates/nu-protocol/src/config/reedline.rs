use std::str::FromStr;

use super::{extract_value, helper::ReconstructVal};
use crate::{record, Config, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

/// Definition of a parsed keybinding from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedKeybinding {
    pub modifier: Value,
    pub keycode: Value,
    pub event: Value,
    pub mode: Value,
}

/// Definition of a parsed menu from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedMenu {
    pub name: Value,
    pub marker: Value,
    pub only_buffer_difference: Value,
    pub style: Value,
    pub menu_type: Value,
    pub source: Value,
}

/// Definition of a Nushell CursorShape (to be mapped to crossterm::cursor::CursorShape)
#[derive(Serialize, Deserialize, Clone, Debug, Copy, Default)]
pub enum NuCursorShape {
    UnderScore,
    Line,
    Block,
    BlinkUnderScore,
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
        "underscore" => Ok(NuCursorShape::UnderScore),
        "blink_line" => Ok(NuCursorShape::BlinkLine),
        "blink_block" => Ok(NuCursorShape::BlinkBlock),
        "blink_underscore" => Ok(NuCursorShape::BlinkUnderScore),
        "inherit" => Ok(NuCursorShape::Inherit),
        _ => Err("expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', 'blink_underscore' or 'inherit'"),
        }
    }
}

impl ReconstructVal for NuCursorShape {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(
            match self {
                NuCursorShape::Line => "line",
                NuCursorShape::Block => "block",
                NuCursorShape::UnderScore => "underscore",
                NuCursorShape::BlinkLine => "blink_line",
                NuCursorShape::BlinkBlock => "blink_block",
                NuCursorShape::BlinkUnderScore => "blink_underscore",
                NuCursorShape::Inherit => "inherit",
            },
            span,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub enum HistoryFileFormat {
    /// Store history as an SQLite database with additional context
    Sqlite,
    /// store history as a plain text file where every line is one command (without any context such as timestamps)
    PlainText,
}

impl FromStr for HistoryFileFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "sqlite" => Ok(Self::Sqlite),
            "plaintext" => Ok(Self::PlainText),
            _ => Err("expected either 'sqlite' or 'plaintext'"),
        }
    }
}

impl ReconstructVal for HistoryFileFormat {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(
            match self {
                HistoryFileFormat::Sqlite => "sqlite",
                HistoryFileFormat::PlainText => "plaintext",
            },
            span,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy)]
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

impl ReconstructVal for EditBindings {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(
            match self {
                EditBindings::Vi => "vi",
                EditBindings::Emacs => "emacs",
            },
            span,
        )
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

pub(super) fn reconstruct_keybindings(config: &Config, span: Span) -> Value {
    Value::list(
        config
            .keybindings
            .iter()
            .map(
                |ParsedKeybinding {
                     modifier,
                     keycode,
                     mode,
                     event,
                 }| {
                    Value::record(
                        record! {
                            "modifier" => modifier.clone(),
                            "keycode" => keycode.clone(),
                            "mode" => mode.clone(),
                            "event" => event.clone(),
                        },
                        span,
                    )
                },
            )
            .collect(),
        span,
    )
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
            let menu_type = extract_value("type", val, span)?.clone();

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
                menu_type,
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

pub(super) fn reconstruct_menus(config: &Config, span: Span) -> Value {
    Value::list(
        config
            .menus
            .iter()
            .map(
                |ParsedMenu {
                     name,
                     only_buffer_difference,
                     marker,
                     style,
                     menu_type, // WARNING: this is not the same name as what is used in Config.nu! ("type")
                     source,
                 }| {
                    Value::record(
                        record! {
                            "name" => name.clone(),
                            "only_buffer_difference" => only_buffer_difference.clone(),
                            "marker" => marker.clone(),
                            "style" => style.clone(),
                            "type" => menu_type.clone(),
                            "source" => source.clone(),
                        },
                        span,
                    )
                },
            )
            .collect(),
        span,
    )
}
