//! Type definitions for the explore config TUI application.

use ratatui::style::Color;
use serde_json::Value;
use std::collections::HashMap;

/// Path through the JSON tree represented as a vector of keys/indices
pub type NodePath = Vec<String>;

/// Determines which pane has focus in the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Tree,
    Editor,
}

/// Mode for the editor pane
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorMode {
    Normal,
    Editing,
}

/// Information about a node in the tree
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub path: NodePath,
    pub value_type: ValueType,
    pub nu_type: Option<NuValueType>,
}

/// JSON value types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

/// Nushell-specific value types for display in config mode
#[derive(Debug, Clone, PartialEq)]
pub enum NuValueType {
    Nothing,
    Bool,
    Int,
    Float,
    String,
    List,
    Record,
    Closure,
    Filesize,
    Duration,
    Date,
    Glob,
    CellPath,
    Binary,
    Range,
    Custom(String),
    Error,
}

impl NuValueType {
    pub fn from_nu_value(value: &nu_protocol::Value) -> Self {
        match value {
            nu_protocol::Value::Nothing { .. } => NuValueType::Nothing,
            nu_protocol::Value::Bool { .. } => NuValueType::Bool,
            nu_protocol::Value::Int { .. } => NuValueType::Int,
            nu_protocol::Value::Float { .. } => NuValueType::Float,
            nu_protocol::Value::String { .. } => NuValueType::String,
            nu_protocol::Value::List { .. } => NuValueType::List,
            nu_protocol::Value::Record { .. } => NuValueType::Record,
            nu_protocol::Value::Closure { .. } => NuValueType::Closure,
            nu_protocol::Value::Filesize { .. } => NuValueType::Filesize,
            nu_protocol::Value::Duration { .. } => NuValueType::Duration,
            nu_protocol::Value::Date { .. } => NuValueType::Date,
            nu_protocol::Value::Glob { .. } => NuValueType::Glob,
            nu_protocol::Value::CellPath { .. } => NuValueType::CellPath,
            nu_protocol::Value::Binary { .. } => NuValueType::Binary,
            nu_protocol::Value::Range { .. } => NuValueType::Range,
            nu_protocol::Value::Custom { val, .. } => NuValueType::Custom(val.type_name()),
            nu_protocol::Value::Error { .. } => NuValueType::Error,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            NuValueType::Nothing => "nothing",
            NuValueType::Bool => "bool",
            NuValueType::Int => "int",
            NuValueType::Float => "float",
            NuValueType::String => "string",
            NuValueType::List => "list",
            NuValueType::Record => "record",
            NuValueType::Closure => "closure",
            NuValueType::Filesize => "filesize",
            NuValueType::Duration => "duration",
            NuValueType::Date => "date",
            NuValueType::Glob => "glob",
            NuValueType::CellPath => "cell-path",
            NuValueType::Binary => "binary",
            NuValueType::Range => "range",
            NuValueType::Custom(name) => name,
            NuValueType::Error => "error",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            NuValueType::Nothing => Color::DarkGray,
            NuValueType::Bool => Color::LightCyan,
            NuValueType::Int => Color::Magenta,
            NuValueType::Float => Color::Magenta,
            NuValueType::String => Color::Green,
            NuValueType::List => Color::Yellow,
            NuValueType::Record => Color::Blue,
            NuValueType::Closure => Color::Cyan,
            NuValueType::Filesize => Color::LightMagenta,
            NuValueType::Duration => Color::LightMagenta,
            NuValueType::Date => Color::LightYellow,
            NuValueType::Glob => Color::LightGreen,
            NuValueType::CellPath => Color::LightBlue,
            NuValueType::Binary => Color::Gray,
            NuValueType::Range => Color::Yellow,
            NuValueType::Custom(_) => Color::Rgb(255, 165, 0), // Orange
            NuValueType::Error => Color::Red,
        }
    }
}

impl ValueType {
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => ValueType::Null,
            Value::Bool(_) => ValueType::Bool,
            Value::Number(_) => ValueType::Number,
            Value::String(_) => ValueType::String,
            Value::Array(_) => ValueType::Array,
            Value::Object(_) => ValueType::Object,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ValueType::Null => "null",
            ValueType::Bool => "boolean",
            ValueType::Number => "number",
            ValueType::String => "string",
            ValueType::Array => "array",
            ValueType::Object => "object",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ValueType::Null => Color::DarkGray,
            ValueType::Bool => Color::Magenta,
            ValueType::Number => Color::Cyan,
            ValueType::String => Color::Green,
            ValueType::Array => Color::Yellow,
            ValueType::Object => Color::Blue,
        }
    }
}

/// Result from running the app - whether to continue or quit
pub enum AppResult {
    Continue,
    Quit,
}

/// The main application state for the TUI
pub struct App {
    pub tree_state: tui_tree_widget::TreeState<String>,
    pub json_data: Value,
    pub tree_items: Vec<tui_tree_widget::TreeItem<'static, String>>,
    pub node_map: HashMap<String, NodeInfo>,
    pub focus: Focus,
    pub editor_mode: EditorMode,
    pub editor_content: String,
    pub editor_cursor: usize,
    pub editor_scroll: usize,
    pub selected_identifier: String,
    pub status_message: String,
    pub modified: bool,
    /// In config mode, tracks whether user has confirmed they want to save (via Ctrl+S)
    pub confirmed_save: bool,
    pub output_file: Option<String>,
    pub config_mode: bool,
    pub doc_map: Option<HashMap<String, String>>,
}
