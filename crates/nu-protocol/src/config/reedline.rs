use super::extract_value;
use crate::{Config, ShellError, Span, Value};
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
#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub enum NuCursorShape {
    UnderScore,
    Line,
    Block,
    BlinkUnderScore,
    BlinkLine,
    BlinkBlock,
}

pub(super) fn parse_cursor_shape(s: &str) -> Result<Option<NuCursorShape>, &'static str> {
    match s.to_ascii_lowercase().as_str() {
        "line" => Ok(Some(NuCursorShape::Line)),
        "block" => Ok(Some(NuCursorShape::Block)),
        "underscore" => Ok(Some(NuCursorShape::UnderScore)),
        "blink_line" => Ok(Some(NuCursorShape::BlinkLine)),
        "blink_block" => Ok(Some(NuCursorShape::BlinkBlock)),
        "blink_underscore" => Ok(Some(NuCursorShape::BlinkUnderScore)),
        "inherit" => Ok(None),
        _ => Err("expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', 'blink_underscore' or 'inherit'"),
    }
}

pub(super) fn reconstruct_cursor_shape(name: Option<NuCursorShape>, span: Span) -> Value {
    Value::string(
        match name {
            Some(NuCursorShape::Line) => "line",
            Some(NuCursorShape::Block) => "block",
            Some(NuCursorShape::UnderScore) => "underscore",
            Some(NuCursorShape::BlinkLine) => "blink_line",
            Some(NuCursorShape::BlinkBlock) => "blink_block",
            Some(NuCursorShape::BlinkUnderScore) => "blink_underscore",
            None => "inherit",
        },
        span,
    )
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub enum HistoryFileFormat {
    /// Store history as an SQLite database with additional context
    Sqlite,
    /// store history as a plain text file where every line is one command (without any context such as timestamps)
    PlainText,
}

pub(super) fn reconstruct_history_file_format(config: &Config, span: Span) -> Value {
    Value::string(
        match config.history_file_format {
            HistoryFileFormat::Sqlite => "sqlite",
            HistoryFileFormat::PlainText => "plaintext",
        },
        span,
    )
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
