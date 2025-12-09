use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use nu_engine::command_prelude::*;
use nu_protocol::{IntoValue, PipelineData};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation};
use ratatui::{Frame, Terminal};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{self, Write};
use tui_tree_widget::{Tree, TreeItem, TreeState};

// ==================== CLI Functions (kept from original) ====================

/// A `regular expression explorer` program.
#[derive(Clone)]
pub struct ExploreConfigCommand;

impl Command for ExploreConfigCommand {
    fn name(&self) -> &str {
        "explore config"
    }

    fn description(&self) -> &str {
        "Launch a TUI to view and edit the nushell configuration interactively."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("explore config")
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::String, Type::String),
            ])
            .switch(
                "use-example-data",
                "Show the nushell configuration TUI using example data",
                Some('e'),
            )
            .switch(
                "tree",
                "Do not show the TUI, just show a tree structure of the data",
                Some('t'),
            )
            .named(
                "output",
                SyntaxShape::String,
                "Optional output file to save changes to (default: output.json)",
                Some('o'),
            )
            .category(Category::Viewers)
    }

    fn extra_description(&self) -> &str {
        r#"By default, opens the current nushell configuration ($env.config) in the TUI.
Changes made in config mode are applied to the running session when you quit.

You can also pipe JSON data to explore arbitrary data structures, or use
--use-example-data to see sample configuration data.

TUI Keybindings:
  Tab           Switch between tree and editor panes
  ↑↓            Navigate tree / scroll editor
  ←→            Collapse/Expand tree nodes
  Enter/Space   Toggle tree node expansion
  Enter/e       Start editing (in editor pane)
  Ctrl+Enter    Apply edit
  Esc           Cancel edit
  Ctrl+S        Save/Apply changes
  q             Quit (applies config changes if modified)
  Ctrl+C        Force quit without saving"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input_span = input.span().unwrap_or(call.head);
        let (string_input, _span, _metadata) = input.collect_string_strict(input_span)?;
        let use_example = call.has_flag(engine_state, stack, "use-example-data")?;
        let cli_mode = call.has_flag(engine_state, stack, "tree")?;
        let output_file: Option<String> = call.get_flag(engine_state, stack, "output")?;

        // Determine the data source and mode
        let (json_data, config_mode): (Value, bool) = if use_example {
            // Use example data
            (get_example_json(), false)
        } else if !string_input.trim().is_empty() {
            // Use piped input data
            let data =
                serde_json::from_str(&string_input).map_err(|e| ShellError::GenericError {
                    error: "Could not parse JSON from input".into(),
                    msg: format!("JSON parse error: {e}"),
                    span: Some(call.head),
                    help: Some("Make sure the input is valid JSON".into()),
                    inner: vec![],
                })?;
            (data, false)
        } else {
            // Default: use nushell configuration
            // First convert Config to nu_protocol::Value, then to serde_json::Value
            // This properly handles closures by converting them to their string representation
            let config = stack.get_config(engine_state);
            let nu_value = config.as_ref().clone().into_value(call.head);
            let json_data = nu_value_to_json(engine_state, &nu_value, call.head)?;
            (json_data, true)
        };

        if cli_mode {
            // Original CLI behavior
            print_json_tree(&json_data, "", true, None);
        } else {
            // TUI mode
            let result = run_config_tui(json_data, output_file, config_mode)?;

            // If in config mode and data was modified, apply changes to the config
            if config_mode {
                if let Some(modified_json) = result {
                    // Convert JSON back to nu_protocol::Value
                    let nu_value = json_to_nu_value(&modified_json, call.head).map_err(|e| {
                        ShellError::GenericError {
                            error: "Could not convert JSON to nu Value".into(),
                            msg: format!("conversion error: {e}"),
                            span: Some(call.head),
                            help: None,
                            inner: vec![],
                        }
                    })?;

                    // Update $env.config with the new value
                    stack.add_env_var("config".into(), nu_value);

                    // Apply the config update
                    stack.update_config(engine_state)?;
                }
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Open the nushell configuration in an interactive TUI editor",
                example: r#"explore config"#,
                result: None,
            },
            Example {
                description: "Explore JSON data interactively",
                example: r#"open data.json | explore config"#,
                result: None,
            },
            Example {
                description: "Explore with example data to see TUI features",
                example: r#"explore config --use-example-data"#,
                result: None,
            },
        ]
    }
}

/// Convert a nu_protocol::Value to a serde_json::Value
/// This properly handles closures by converting them to their string representation
fn nu_value_to_json(
    engine_state: &EngineState,
    value: &nu_protocol::Value,
    span: nu_protocol::Span,
) -> Result<Value, ShellError> {
    Ok(match value {
        nu_protocol::Value::Bool { val, .. } => Value::Bool(*val),
        nu_protocol::Value::Int { val, .. } => Value::Number((*val).into()),
        nu_protocol::Value::Float { val, .. } => serde_json::Number::from_f64(*val)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        nu_protocol::Value::String { val, .. } => Value::String(val.clone()),
        nu_protocol::Value::Nothing { .. } => Value::Null,
        nu_protocol::Value::List { vals, .. } => {
            let json_vals: Result<Vec<_>, _> = vals
                .iter()
                .map(|v| nu_value_to_json(engine_state, v, span))
                .collect();
            Value::Array(json_vals?)
        }
        nu_protocol::Value::Record { val, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in val.iter() {
                map.insert(k.clone(), nu_value_to_json(engine_state, v, span)?);
            }
            Value::Object(map)
        }
        nu_protocol::Value::Closure { val, .. } => {
            // Convert closure to its string representation instead of serializing internal structure
            let closure_string = val.coerce_into_string(engine_state, value.span())?;
            Value::String(closure_string.to_string())
        }
        nu_protocol::Value::Filesize { val, .. } => Value::Number(val.get().into()),
        nu_protocol::Value::Duration { val, .. } => Value::Number((*val).into()),
        nu_protocol::Value::Date { val, .. } => Value::String(val.to_string()),
        nu_protocol::Value::Glob { val, .. } => Value::String(val.to_string()),
        nu_protocol::Value::CellPath { val, .. } => {
            let parts: Vec<Value> = val
                .members
                .iter()
                .map(|m| match m {
                    nu_protocol::ast::PathMember::String { val, .. } => Value::String(val.clone()),
                    nu_protocol::ast::PathMember::Int { val, .. } => {
                        Value::Number((*val as i64).into())
                    }
                })
                .collect();
            Value::Array(parts)
        }
        nu_protocol::Value::Binary { val, .. } => Value::Array(
            val.iter()
                .map(|b| Value::Number((*b as i64).into()))
                .collect(),
        ),
        nu_protocol::Value::Range { .. } => Value::Null,
        nu_protocol::Value::Error { error, .. } => {
            return Err(*error.clone());
        }
        nu_protocol::Value::Custom { val, .. } => {
            let collected = val.to_base_value(value.span())?;
            nu_value_to_json(engine_state, &collected, span)?
        }
    })
}

/// Convert a serde_json::Value to a nu_protocol::Value
fn json_to_nu_value(
    json: &Value,
    span: nu_protocol::Span,
) -> Result<nu_protocol::Value, Box<dyn Error>> {
    Ok(match json {
        Value::Null => nu_protocol::Value::nothing(span),
        Value::Bool(b) => nu_protocol::Value::bool(*b, span),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                nu_protocol::Value::int(i, span)
            } else if let Some(f) = n.as_f64() {
                nu_protocol::Value::float(f, span)
            } else {
                return Err(format!("Unsupported number: {}", n).into());
            }
        }
        Value::String(s) => nu_protocol::Value::string(s.clone(), span),
        Value::Array(arr) => {
            let values: Result<Vec<_>, _> = arr.iter().map(|v| json_to_nu_value(v, span)).collect();
            nu_protocol::Value::list(values?, span)
        }
        Value::Object(obj) => {
            let mut record = nu_protocol::Record::new();
            for (k, v) in obj {
                record.push(k.clone(), json_to_nu_value(v, span)?);
            }
            nu_protocol::Value::record(record, span)
        }
    })
}

// ==================== CLI Functions (kept from original) ====================

fn is_leaf(value: &Value) -> bool {
    !value.is_object() && !value.is_array()
}

fn render_leaf(value: &Value) -> String {
    serde_json::to_string(value).expect("Failed to serialize value")
}

fn print_json_tree(value: &Value, prefix: &str, is_tail: bool, key: Option<&str>) {
    if let Some(k) = key {
        let connector = if is_tail { "└── " } else { "├── " };
        let leaf_part = if is_leaf(value) {
            format!(" {}", render_leaf(value))
        } else {
            String::new()
        };
        println!("{}{}{}:{}", prefix, connector, k, leaf_part);
    }

    if !is_leaf(value) {
        let branch = if is_tail { "    " } else { "│   " };
        let child_prefix = if key.is_none() {
            prefix.to_string()
        } else {
            format!("{}{}", prefix, branch)
        };

        match value {
            Value::Object(map) => {
                let mut entries: Vec<(&str, &Value)> =
                    map.iter().map(|(kk, vv)| (kk.as_str(), vv)).collect();
                entries.sort_by_key(|(kk, _)| *kk);
                for (idx, &(kk, vv)) in entries.iter().enumerate() {
                    let child_tail = idx == entries.len() - 1;
                    print_json_tree(vv, &child_prefix, child_tail, Some(kk));
                }
            }
            Value::Array(arr) => {
                for (idx, vv) in arr.iter().enumerate() {
                    let child_tail = idx == arr.len() - 1;
                    let idx_str = idx.to_string();
                    print_json_tree(vv, &child_prefix, child_tail, Some(&idx_str));
                }
            }
            _ => {}
        }
    }
}

// ==================== TUI Application ====================

type NodePath = Vec<String>;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    Tree,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EditorMode {
    Normal,
    Editing,
}

/// Information about a node in the tree
#[derive(Debug, Clone)]
struct NodeInfo {
    path: NodePath,
    value_type: ValueType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ValueType {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl ValueType {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => ValueType::Null,
            Value::Bool(_) => ValueType::Bool,
            Value::Number(_) => ValueType::Number,
            Value::String(_) => ValueType::String,
            Value::Array(_) => ValueType::Array,
            Value::Object(_) => ValueType::Object,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ValueType::Null => "null",
            ValueType::Bool => "boolean",
            ValueType::Number => "number",
            ValueType::String => "string",
            ValueType::Array => "array",
            ValueType::Object => "object",
        }
    }

    fn color(&self) -> Color {
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

/// Result from running the app - whether to quit
enum AppResult {
    Continue,
    Quit,
}

struct App {
    tree_state: TreeState<String>,
    json_data: Value,
    tree_items: Vec<TreeItem<'static, String>>,
    node_map: HashMap<String, NodeInfo>,
    focus: Focus,
    editor_mode: EditorMode,
    editor_content: String,
    editor_cursor: usize,
    editor_scroll: usize,
    selected_identifier: String,
    status_message: String,
    modified: bool,
    output_file: Option<String>,
    config_mode: bool,
}

impl App {
    fn new(json_data: Value, output_file: Option<String>, config_mode: bool) -> Self {
        let mut node_map = HashMap::new();
        let tree_items = build_tree_items(&json_data, &mut node_map);

        let status_msg = if config_mode {
            "↑↓ Navigate | ←→ Collapse/Expand | Tab Switch pane | Ctrl+S Apply | q Quit"
        } else {
            "↑↓ Navigate | ←→ Collapse/Expand | Tab Switch pane | Ctrl+S Save | q Quit"
        };

        App {
            tree_state: TreeState::default(),
            json_data,
            tree_items,
            node_map,
            focus: Focus::Tree,
            editor_mode: EditorMode::Normal,
            editor_content: String::new(),
            editor_cursor: 0,
            editor_scroll: 0,
            selected_identifier: String::new(),
            status_message: String::from(status_msg),
            modified: false,
            output_file,
            config_mode,
        }
    }

    fn rebuild_tree(&mut self) {
        // Save current selection path from tree state
        let current_selection = self.tree_state.selected().to_vec();

        let mut node_map = HashMap::new();
        self.tree_items = build_tree_items(&self.json_data, &mut node_map);
        self.node_map = node_map;

        // Try to restore selection if the node still exists
        if let Some(last_id) = current_selection.last() {
            if self.node_map.contains_key(last_id) {
                self.tree_state.select(current_selection);
            }
        }
    }

    fn get_current_node_info(&self) -> Option<&NodeInfo> {
        if self.selected_identifier.is_empty() {
            return None;
        }
        self.node_map.get(&self.selected_identifier)
    }

    fn force_update_editor(&mut self) {
        let selected = self.tree_state.selected();
        if selected.is_empty() {
            self.selected_identifier.clear();
            self.editor_content.clear();
            return;
        }

        // Use last() to get the actual selected node, not first()
        // selected() returns the path through the tree, so last is the actual selection
        self.selected_identifier = selected.last().cloned().unwrap_or_default();

        if let Some(node_info) = self.node_map.get(&self.selected_identifier) {
            if let Some(value) = get_value_at_path(&self.json_data, &node_info.path) {
                self.editor_content = match value {
                    Value::String(s) => s.clone(),
                    Value::Null => String::from("null"),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => serde_json::to_string_pretty(value).unwrap_or_default(),
                };
            } else {
                self.editor_content.clear();
            }
        } else {
            self.editor_content.clear();
        }

        self.editor_cursor = 0;
        self.editor_scroll = 0;
    }

    fn apply_edit(&mut self) {
        if self.selected_identifier.is_empty() {
            self.status_message = String::from("No node selected");
            return;
        }

        let node_info = match self.node_map.get(&self.selected_identifier) {
            Some(info) => info.clone(),
            None => {
                self.status_message = String::from("Node not found");
                return;
            }
        };

        // Determine the new value based on content and original type
        let new_value: Value =
            if let Some(original_value) = get_value_at_path(&self.json_data, &node_info.path) {
                match original_value {
                    // For strings, use content directly (don't parse as JSON)
                    Value::String(_) => Value::String(self.editor_content.clone()),
                    // For other leaf types, try to parse appropriately
                    Value::Null => {
                        if self.editor_content.trim() == "null" {
                            Value::Null
                        } else {
                            // Try to parse as JSON, fall back to string
                            serde_json::from_str(&self.editor_content)
                                .unwrap_or_else(|_| Value::String(self.editor_content.clone()))
                        }
                    }
                    Value::Bool(_) => match self.editor_content.trim() {
                        "true" => Value::Bool(true),
                        "false" => Value::Bool(false),
                        _ => Value::String(self.editor_content.clone()),
                    },
                    Value::Number(_) => {
                        // Try to parse as number
                        if let Ok(n) = self.editor_content.trim().parse::<i64>() {
                            Value::Number(n.into())
                        } else if let Ok(n) = self.editor_content.trim().parse::<f64>() {
                            serde_json::Number::from_f64(n)
                                .map(Value::Number)
                                .unwrap_or_else(|| Value::String(self.editor_content.clone()))
                        } else {
                            Value::String(self.editor_content.clone())
                        }
                    }
                    // For arrays and objects, parse as JSON
                    Value::Array(_) | Value::Object(_) => {
                        match serde_json::from_str(&self.editor_content) {
                            Ok(v) => v,
                            Err(e) => {
                                self.status_message = format!("✗ JSON parse error: {}", e);
                                return;
                            }
                        }
                    }
                }
            } else {
                // Fallback: try to parse as JSON
                serde_json::from_str(&self.editor_content)
                    .unwrap_or_else(|_| Value::String(self.editor_content.clone()))
            };

        // Apply the change to the JSON data
        if set_value_at_path(&mut self.json_data, &node_info.path, new_value) {
            self.rebuild_tree();
            self.modified = true;
            self.status_message = String::from("✓ Value updated successfully");
        } else {
            self.status_message = String::from("✗ Failed to update value");
        }
    }

    fn save_to_file(&mut self) -> io::Result<()> {
        if self.config_mode {
            // In config mode, we mark as "ready to apply" - actual application happens on exit
            self.status_message =
                String::from("✓ Changes staged - will be applied to config on exit");
            return Ok(());
        }

        let filename = self
            .output_file
            .clone()
            .unwrap_or_else(|| String::from("output.json"));
        let json_string = serde_json::to_string_pretty(&self.json_data)?;
        let mut file = File::create(&filename)?;
        file.write_all(json_string.as_bytes())?;
        self.modified = false;
        self.status_message = format!("✓ Saved to {}", filename);
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title bar
                Constraint::Min(1),    // Main content
                Constraint::Length(1), // Status bar
            ])
            .split(frame.area());

        // Title bar
        self.draw_title_bar(frame, chunks[0]);

        // Main content (tree + editor)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(chunks[1]);

        // Left pane: Tree
        self.draw_tree(frame, main_chunks[0]);

        // Right pane: Editor panel
        self.draw_editor_panel(frame, main_chunks[1]);

        // Status bar
        self.draw_status_bar(frame, chunks[2]);
    }

    fn draw_title_bar(&self, frame: &mut Frame, area: Rect) {
        let modified_indicator = if self.modified { " [*]" } else { "" };
        let title = format!(" Nushell Config Explorer{}", modified_indicator);

        let title_bar =
            Paragraph::new(title).style(Style::default().bg(Color::Blue).fg(Color::White).bold());

        frame.render_widget(title_bar, area);
    }

    fn draw_tree(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus == Focus::Tree;
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let tree_block = Block::default()
            .title(if is_focused {
                " Tree [focused] "
            } else {
                " Tree "
            })
            .title_style(Style::default().bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let tree_widget = Tree::new(&self.tree_items)
            .expect("all item identifiers are unique")
            .block(tree_block)
            .experimental_scrollbar(Some(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None),
            ))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ")
            .node_closed_symbol("▸ ")
            .node_open_symbol("▾ ")
            .node_no_children_symbol("  ");

        frame.render_stateful_widget(tree_widget, area, &mut self.tree_state);
    }

    fn draw_editor_panel(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus == Focus::Editor;
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let panel_block = Block::default()
            .title(if is_focused {
                " Editor [focused] "
            } else {
                " Editor "
            })
            .title_style(Style::default().bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner_area = panel_block.inner(area);
        frame.render_widget(panel_block, area);

        // Split the editor panel into sections
        let editor_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Path display
                Constraint::Length(3), // Type info
                Constraint::Min(1),    // Editor area
                Constraint::Length(2), // Help text
            ])
            .split(inner_area);

        // Path display (read-only)
        self.draw_path_widget(frame, editor_chunks[0]);

        // Type info
        self.draw_type_widget(frame, editor_chunks[1]);

        // Editor area
        self.draw_editor_widget(frame, editor_chunks[2]);

        // Help text
        self.draw_editor_help(frame, editor_chunks[3]);
    }

    fn draw_path_widget(&self, frame: &mut Frame, area: Rect) {
        let path_block = Block::default()
            .title(" Path ")
            .title_style(Style::default().fg(Color::Yellow))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let path_display = if let Some(node_info) = self.get_current_node_info() {
            if node_info.path.is_empty() {
                String::from("(root)")
            } else {
                node_info
                    .path
                    .iter()
                    .map(|p| {
                        // Check if it's an array index
                        if p.parse::<usize>().is_ok() {
                            format!("[{}]", p)
                        } else if p.contains(' ') || p.contains('.') {
                            format!("[\"{}\"]", p)
                        } else {
                            format!(".{}", p)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
                    .trim_start_matches('.')
                    .to_string()
            }
        } else {
            String::from("(no selection)")
        };

        let path_text = Paragraph::new(path_display)
            .style(Style::default().fg(Color::White))
            .block(path_block);

        frame.render_widget(path_text, area);
    }

    fn draw_type_widget(&self, frame: &mut Frame, area: Rect) {
        let type_block = Block::default()
            .title(" Type ")
            .title_style(Style::default().fg(Color::Yellow))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let (type_label, type_color, extra_info) =
            if let Some(node_info) = self.get_current_node_info() {
                let extra = match node_info.value_type {
                    ValueType::Array => {
                        if let Some(Value::Array(arr)) =
                            get_value_at_path(&self.json_data, &node_info.path)
                        {
                            format!(" ({} items)", arr.len())
                        } else {
                            String::new()
                        }
                    }
                    ValueType::Object => {
                        if let Some(Value::Object(obj)) =
                            get_value_at_path(&self.json_data, &node_info.path)
                        {
                            format!(" ({} keys)", obj.len())
                        } else {
                            String::new()
                        }
                    }
                    ValueType::String => {
                        if let Some(Value::String(s)) =
                            get_value_at_path(&self.json_data, &node_info.path)
                        {
                            format!(" ({} chars)", s.len())
                        } else {
                            String::new()
                        }
                    }
                    _ => String::new(),
                };
                (
                    node_info.value_type.label(),
                    node_info.value_type.color(),
                    extra,
                )
            } else {
                ("unknown", Color::DarkGray, String::new())
            };

        let type_line = Line::from(vec![
            Span::styled(
                format!(" {} ", type_label),
                Style::default().fg(Color::Black).bg(type_color).bold(),
            ),
            Span::styled(extra_info, Style::default().fg(Color::DarkGray)),
        ]);

        let type_text = Paragraph::new(type_line).block(type_block);

        frame.render_widget(type_text, area);
    }

    fn draw_editor_widget(&self, frame: &mut Frame, area: Rect) {
        let is_editing = self.editor_mode == EditorMode::Editing && self.focus == Focus::Editor;

        let editor_block = Block::default()
            .title(if is_editing {
                " Value [editing] "
            } else {
                " Value "
            })
            .title_style(Style::default().fg(if is_editing {
                Color::Green
            } else {
                Color::Yellow
            }))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if is_editing {
                Color::Green
            } else {
                Color::DarkGray
            }));

        let inner_area = editor_block.inner(area);
        frame.render_widget(editor_block, area);

        // Calculate visible lines
        let visible_height = inner_area.height as usize;
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let total_lines = lines.len().max(1);

        // Calculate cursor position
        let mut cursor_line = 0;
        let mut cursor_col = 0;
        let mut pos = 0;
        for (line_idx, line) in self.editor_content.lines().enumerate() {
            if pos + line.len() >= self.editor_cursor {
                cursor_line = line_idx;
                cursor_col = self.editor_cursor - pos;
                break;
            }
            pos += line.len() + 1; // +1 for newline
            cursor_line = line_idx + 1;
        }

        // Render content with syntax highlighting
        let content_lines: Vec<Line> = self
            .editor_content
            .lines()
            .enumerate()
            .skip(self.editor_scroll)
            .take(visible_height)
            .map(|(idx, line)| {
                let line_style = if is_editing && idx == cursor_line {
                    Style::default().bg(Color::Rgb(40, 40, 40))
                } else {
                    Style::default()
                };
                Line::styled(line.to_string(), line_style)
            })
            .collect();

        let content = if content_lines.is_empty() {
            if self.editor_content.is_empty() {
                Text::from(Line::from(Span::styled(
                    "(empty)",
                    Style::default().fg(Color::DarkGray).italic(),
                )))
            } else {
                Text::from(content_lines)
            }
        } else {
            Text::from(content_lines)
        };

        let paragraph = Paragraph::new(content);
        frame.render_widget(paragraph, inner_area);

        // Show cursor when editing
        if is_editing && inner_area.width > 0 && inner_area.height > 0 {
            let cursor_y = (cursor_line.saturating_sub(self.editor_scroll)) as u16;
            let cursor_x = cursor_col as u16;

            if cursor_y < inner_area.height {
                frame.set_cursor_position((
                    inner_area.x + cursor_x.min(inner_area.width - 1),
                    inner_area.y + cursor_y,
                ));
            }
        }

        // Show scroll indicator if needed
        if total_lines > visible_height {
            let scroll_info = format!(
                " {}-{}/{} ",
                self.editor_scroll + 1,
                (self.editor_scroll + visible_height).min(total_lines),
                total_lines
            );
            let scroll_len = scroll_info.len();
            let scroll_span = Span::styled(scroll_info, Style::default().fg(Color::DarkGray));
            let scroll_paragraph = Paragraph::new(scroll_span);
            let scroll_area = Rect {
                x: area.x + area.width.saturating_sub(scroll_len as u16 + 1),
                y: area.y,
                width: scroll_len as u16,
                height: 1,
            };
            frame.render_widget(scroll_paragraph, scroll_area);
        }
    }

    fn draw_editor_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = if self.focus == Focus::Editor {
            if self.editor_mode == EditorMode::Editing {
                Line::from(vec![
                    Span::styled("Ctrl+Enter", Style::default().fg(Color::Green).bold()),
                    Span::raw(" Apply  "),
                    Span::styled("Esc", Style::default().fg(Color::Red).bold()),
                    Span::raw(" Cancel  "),
                    Span::styled("Ctrl+↑↓", Style::default().fg(Color::Yellow).bold()),
                    Span::raw(" Scroll"),
                ])
            } else {
                Line::from(vec![
                    Span::styled("Enter/e", Style::default().fg(Color::Green).bold()),
                    Span::raw(" Edit  "),
                    Span::styled("Tab", Style::default().fg(Color::Yellow).bold()),
                    Span::raw(" Switch pane  "),
                    Span::styled("↑↓", Style::default().fg(Color::Yellow).bold()),
                    Span::raw(" Scroll"),
                ])
            }
        } else {
            Line::from(vec![
                Span::styled("Tab", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Switch to editor"),
            ])
        };

        let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

        frame.render_widget(help, area);
    }

    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let status_style = Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White);

        let status = Paragraph::new(format!(" {}", self.status_message)).style(status_style);

        frame.render_widget(status, area);
    }

    fn scroll_editor(&mut self, delta: i32) {
        let lines_count = self.editor_content.lines().count();
        if delta < 0 {
            self.editor_scroll = self.editor_scroll.saturating_sub((-delta) as usize);
        } else {
            self.editor_scroll =
                (self.editor_scroll + delta as usize).min(lines_count.saturating_sub(1));
        }
    }
}

fn build_tree_items(
    value: &Value,
    node_map: &mut HashMap<String, NodeInfo>,
) -> Vec<TreeItem<'static, String>> {
    build_tree_items_recursive(value, node_map, Vec::new(), String::new())
}

fn build_tree_items_recursive(
    value: &Value,
    node_map: &mut HashMap<String, NodeInfo>,
    current_path: Vec<String>,
    parent_id: String,
) -> Vec<TreeItem<'static, String>> {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by_key(|(k, _)| k.as_str());

            entries
                .into_iter()
                .map(|(key, val)| {
                    let mut path = current_path.clone();
                    path.push(key.clone());

                    let identifier = if parent_id.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", parent_id, key)
                    };

                    let value_type = ValueType::from_value(val);

                    node_map.insert(
                        identifier.clone(),
                        NodeInfo {
                            path: path.clone(),
                            value_type,
                        },
                    );

                    let display = format_tree_label(key, val);

                    if is_leaf(val) {
                        TreeItem::new_leaf(identifier, display)
                    } else {
                        let children =
                            build_tree_items_recursive(val, node_map, path, identifier.clone());
                        TreeItem::new(identifier, display, children)
                            .expect("all item identifiers are unique")
                    }
                })
                .collect()
        }
        Value::Array(arr) => arr
            .iter()
            .enumerate()
            .map(|(idx, val)| {
                let mut path = current_path.clone();
                path.push(idx.to_string());

                let identifier = if parent_id.is_empty() {
                    format!("[{}]", idx)
                } else {
                    format!("{}[{}]", parent_id, idx)
                };

                let value_type = ValueType::from_value(val);

                node_map.insert(
                    identifier.clone(),
                    NodeInfo {
                        path: path.clone(),
                        value_type,
                    },
                );

                let display = format_array_item_label(idx, val);

                if is_leaf(val) {
                    TreeItem::new_leaf(identifier, display)
                } else {
                    let children =
                        build_tree_items_recursive(val, node_map, path, identifier.clone());
                    TreeItem::new(identifier, display, children)
                        .expect("all item identifiers are unique")
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn format_tree_label(key: &str, value: &Value) -> String {
    match value {
        Value::Null => format!("{}: null", key),
        Value::Bool(b) => format!("{}: {}", key, b),
        Value::Number(n) => format!("{}: {}", key, n),
        Value::String(s) => {
            let preview = if s.len() > 40 {
                format!("{}...", &s[..37])
            } else {
                s.clone()
            };
            format!("{}: \"{}\"", key, preview.replace('\n', "\\n"))
        }
        Value::Array(arr) => format!("{} [{} items]", key, arr.len()),
        Value::Object(obj) => format!("{} {{{} keys}}", key, obj.len()),
    }
}

fn format_array_item_label(idx: usize, value: &Value) -> String {
    match value {
        Value::Null => format!("[{}]: null", idx),
        Value::Bool(b) => format!("[{}]: {}", idx, b),
        Value::Number(n) => format!("[{}]: {}", idx, n),
        Value::String(s) => {
            let preview = if s.len() > 40 {
                format!("{}...", &s[..37])
            } else {
                s.clone()
            };
            format!("[{}]: \"{}\"", idx, preview.replace('\n', "\\n"))
        }
        Value::Array(arr) => format!("[{}] [{} items]", idx, arr.len()),
        Value::Object(obj) => format!("[{}] {{{} keys}}", idx, obj.len()),
    }
}

fn get_value_at_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = value;
    for part in path {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                current = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn set_value_at_path(value: &mut Value, path: &[String], new_value: Value) -> bool {
    if path.is_empty() {
        *value = new_value;
        return true;
    }

    let mut current = value;
    for (i, part) in path.iter().enumerate() {
        if i == path.len() - 1 {
            // Last part, set the value
            match current {
                Value::Object(map) => {
                    map.insert(part.clone(), new_value);
                    return true;
                }
                Value::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        if idx < arr.len() {
                            arr[idx] = new_value;
                            return true;
                        }
                    }
                    return false;
                }
                _ => return false,
            }
        } else {
            // Navigate deeper
            match current {
                Value::Object(map) => {
                    if let Some(next) = map.get_mut(part) {
                        current = next;
                    } else {
                        return false;
                    }
                }
                Value::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        if idx < arr.len() {
                            current = &mut arr[idx];
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }
    false
}

/// Run the TUI and return the modified JSON data if changes were made in config mode
fn run_config_tui(
    json_data: Value,
    output_file: Option<String>,
    config_mode: bool,
) -> Result<Option<Value>, Box<dyn Error>> {
    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the screen initially
    terminal.clear()?;

    let mut app = App::new(json_data, output_file, config_mode);

    // Select the first item
    app.tree_state.select_first();
    app.force_update_editor();

    let res = run_config_app(&mut terminal, &mut app);

    // Restore terminal - this is critical for clean exit
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
        return Ok(None);
    }

    // Return the modified data if in config mode and changes were made
    if config_mode && app.modified {
        Ok(Some(app.json_data))
    } else {
        Ok(None)
    }
}

fn run_config_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global keybindings
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Err(e) = app.save_to_file() {
                        app.status_message = format!("✗ Save failed: {}", e);
                    }
                    continue;
                }
                _ => {}
            }

            // Handle based on focus and mode
            let result = match (app.focus, app.editor_mode) {
                (Focus::Tree, _) => handle_tree_input(app, key.code, key.modifiers),
                (Focus::Editor, EditorMode::Normal) => {
                    handle_editor_normal_input(app, key.code, key.modifiers)
                }
                (Focus::Editor, EditorMode::Editing) => {
                    handle_editor_editing_input(app, key.code, key.modifiers)
                }
            };

            if let AppResult::Quit = result {
                return Ok(());
            }
        }
    }
}

fn handle_tree_input(app: &mut App, key: KeyCode, _modifiers: KeyModifiers) -> AppResult {
    match key {
        KeyCode::Char('q') => {
            if app.modified {
                app.status_message =
                    String::from("Unsaved changes! Press Ctrl+C to force quit or Ctrl+S to save");
                return AppResult::Continue;
            } else {
                return AppResult::Quit;
            }
        }
        KeyCode::Up => {
            app.tree_state.key_up();
            app.force_update_editor();
        }
        KeyCode::Down => {
            app.tree_state.key_down();
            app.force_update_editor();
        }
        KeyCode::Left => {
            app.tree_state.key_left();
            app.force_update_editor();
        }
        KeyCode::Right => {
            app.tree_state.key_right();
            app.force_update_editor();
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.tree_state.toggle_selected();
        }
        KeyCode::Home => {
            app.tree_state.select_first();
            app.force_update_editor();
        }
        KeyCode::End => {
            app.tree_state.select_last();
            app.force_update_editor();
        }
        KeyCode::Tab => {
            app.focus = Focus::Editor;
            app.status_message = String::from("Editor focused - press Enter or 'e' to edit value");
        }
        _ => {}
    }
    AppResult::Continue
}

fn handle_editor_normal_input(app: &mut App, key: KeyCode, _modifiers: KeyModifiers) -> AppResult {
    match key {
        KeyCode::Tab => {
            app.focus = Focus::Tree;
            app.status_message = String::from(
                "↑↓ Navigate | ←→ Collapse/Expand | Tab Switch pane | Ctrl+S Save | q Quit",
            );
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            app.editor_mode = EditorMode::Editing;
            app.editor_cursor = 0;
            app.status_message = String::from("Editing - Ctrl+Enter to apply, Esc to cancel");
        }
        KeyCode::Up => {
            app.scroll_editor(-1);
        }
        KeyCode::Down => {
            app.scroll_editor(1);
        }
        KeyCode::PageUp => {
            app.scroll_editor(-10);
        }
        KeyCode::PageDown => {
            app.scroll_editor(10);
        }
        KeyCode::Char('q') => {
            if app.modified {
                app.status_message =
                    String::from("Unsaved changes! Press Ctrl+C to force quit or Ctrl+S to save");
            } else {
                return AppResult::Quit;
            }
        }
        _ => {}
    }
    AppResult::Continue
}

fn handle_editor_editing_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> AppResult {
    match key {
        KeyCode::Esc => {
            app.editor_mode = EditorMode::Normal;
            app.force_update_editor(); // Restore original value
            app.status_message = String::from("Edit cancelled");
        }
        KeyCode::Enter if modifiers.contains(KeyModifiers::CONTROL) => {
            app.apply_edit();
            app.editor_mode = EditorMode::Normal;
        }
        KeyCode::Enter => {
            // Insert newline
            app.editor_content.insert(app.editor_cursor, '\n');
            app.editor_cursor += 1;
        }
        KeyCode::Backspace => {
            if app.editor_cursor > 0 {
                app.editor_cursor -= 1;
                app.editor_content.remove(app.editor_cursor);
            }
        }
        KeyCode::Delete => {
            if app.editor_cursor < app.editor_content.len() {
                app.editor_content.remove(app.editor_cursor);
            }
        }
        KeyCode::Left => {
            app.editor_cursor = app.editor_cursor.saturating_sub(1);
        }
        KeyCode::Right => {
            app.editor_cursor = (app.editor_cursor + 1).min(app.editor_content.len());
        }
        KeyCode::Up if modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_editor(-1);
        }
        KeyCode::Down if modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_editor(1);
        }
        KeyCode::Up => {
            // Move cursor up one line
            let lines: Vec<&str> = app.editor_content.lines().collect();
            let mut pos = 0;
            let mut cursor_line = 0;
            let mut cursor_col = 0;

            for (line_idx, line) in app.editor_content.lines().enumerate() {
                if pos + line.len() >= app.editor_cursor {
                    cursor_line = line_idx;
                    cursor_col = app.editor_cursor - pos;
                    break;
                }
                pos += line.len() + 1;
                cursor_line = line_idx + 1;
            }

            if cursor_line > 0 {
                let prev_line = lines.get(cursor_line - 1).unwrap_or(&"");
                let new_col = cursor_col.min(prev_line.len());
                let mut new_pos = 0;
                for (i, line) in lines.iter().enumerate() {
                    if i == cursor_line - 1 {
                        app.editor_cursor = new_pos + new_col;
                        break;
                    }
                    new_pos += line.len() + 1;
                }
            }
        }
        KeyCode::Down => {
            // Move cursor down one line
            let lines: Vec<&str> = app.editor_content.lines().collect();
            let mut pos = 0;
            let mut cursor_line = 0;
            let mut cursor_col = 0;

            for (line_idx, line) in app.editor_content.lines().enumerate() {
                if pos + line.len() >= app.editor_cursor {
                    cursor_line = line_idx;
                    cursor_col = app.editor_cursor - pos;
                    break;
                }
                pos += line.len() + 1;
                cursor_line = line_idx + 1;
            }

            if cursor_line < lines.len().saturating_sub(1) {
                let next_line = lines.get(cursor_line + 1).unwrap_or(&"");
                let new_col = cursor_col.min(next_line.len());
                let mut new_pos = 0;
                for (i, line) in lines.iter().enumerate() {
                    if i == cursor_line + 1 {
                        app.editor_cursor = new_pos + new_col;
                        break;
                    }
                    new_pos += line.len() + 1;
                }
            }
        }
        KeyCode::Home => {
            // Move to beginning of line
            let mut pos = 0;
            for line in app.editor_content.lines() {
                if pos + line.len() >= app.editor_cursor {
                    app.editor_cursor = pos;
                    break;
                }
                pos += line.len() + 1;
            }
        }
        KeyCode::End => {
            // Move to end of line
            let mut pos = 0;
            for line in app.editor_content.lines() {
                if pos + line.len() >= app.editor_cursor {
                    app.editor_cursor = pos + line.len();
                    break;
                }
                pos += line.len() + 1;
            }
        }
        KeyCode::Char(c) => {
            app.editor_content.insert(app.editor_cursor, c);
            app.editor_cursor += 1;
        }
        KeyCode::Tab => {
            // Insert 2 spaces for indentation
            app.editor_content.insert_str(app.editor_cursor, "  ");
            app.editor_cursor += 2;
        }
        _ => {}
    }
    AppResult::Continue
}

// fn print_usage() {
//     eprintln!("Usage: `explore config`  [OPTIONS]");
//     eprintln!();
//     eprintln!("Options:");
//     eprintln!("  --cli          Use CLI mode (print tree to stdout)");
//     eprintln!("  --tui          Use TUI mode (interactive, default)");
//     eprintln!("  --example      Use built-in example JSON data instead of stdin");
//     eprintln!("  -o, --output   Output file for saving (default: output.json)");
//     eprintln!("  -h, --help     Show this help message");
//     eprintln!();
//     eprintln!("Reads JSON from stdin and displays it as a tree.");
//     eprintln!();
//     eprintln!("TUI Keybindings:");
//     eprintln!("  Tab           Switch between tree and editor panes");
//     eprintln!("  ↑↓            Navigate tree / scroll editor");
//     eprintln!("  ←→            Collapse/Expand tree nodes");
//     eprintln!("  Enter/Space   Toggle tree node expansion");
//     eprintln!("  Enter/e       Start editing (in editor pane)");
//     eprintln!("  Ctrl+Enter    Apply edit");
//     eprintln!("  Esc           Cancel edit");
//     eprintln!("  Ctrl+S        Save to file");
//     eprintln!("  q             Quit");
//     eprintln!("  Ctrl+C        Force quit");
// }

/// Example JSON data for testing (nushell config)
#[allow(dead_code)]
fn get_example_json() -> Value {
    let json_str = r##"{
  "filesize": {
    "unit": "B",
    "show_unit": false,
    "precision": 2
  },
  "table": {
    "mode": "rounded",
    "index_mode": "always",
    "show_empty": false,
    "padding": {
      "left": 1,
      "right": 1
    },
    "trim": {
      "methodology": "wrapping",
      "wrapping_try_keep_words": true
    },
    "header_on_separator": true,
    "abbreviated_row_count": null,
    "footer_inheritance": true,
    "missing_value_symbol": "❎",
    "batch_duration": 1000000000,
    "stream_page_size": 1000
  },
  "ls": {
    "use_ls_colors": true,
    "clickable_links": true
  },
  "color_config": {
    "shape_internallcall": "cyan_bold",
    "leading_trailing_space_bg": {
      "bg": "dark_gray_dimmed"
    },
    "string": "{|x| if $x =~ '^#[a-fA-F\\d]+' { $x } else { 'default' } }",
    "date": "{||\n    (date now) - $in | if $in < 1hr {\n      'red3b' #\"\\e[38;5;160m\" #'#e61919' # 160\n    } else if $in < 6hr {\n      'orange3' #\"\\e[38;5;172m\" #'#e68019' # 172\n    } else if $in < 1day {\n      'yellow3b' #\"\\e[38;5;184m\" #'#e5e619' # 184\n    } else if $in < 3day {\n      'chartreuse2b' #\"\\e[38;5;112m\" #'#80e619' # 112\n    } else if $in < 1wk {\n      'green3b' #\"\\e[38;5;40m\" #'#19e619' # 40\n    } else if $in < 6wk {\n      'darkturquoise' #\"\\e[38;5;44m\" #'#19e5e6' # 44\n    } else if $in < 52wk {\n      'deepskyblue3b' #\"\\e[38;5;32m\" #'#197fe6' # 32\n    } else { 'dark_gray' }\n  }",
    "hints": "dark_gray",
    "shape_matching_brackets": {
      "fg": "red",
      "bg": "default",
      "attr": "b"
    },
    "nothing": "red",
    "shape_string_interpolation": "cyan_bold",
    "shape_externalarg": "light_purple",
    "shape_external_resolved": "light_yellow_bold",
    "cellpath": "cyan",
    "foreground": "green3b",
    "shape_filepath": "cyan",
    "separator": "yd",
    "shape_garbage": {
      "fg": "red",
      "attr": "u"
    },
    "shape_external": "darkorange",
    "float": "red",
    "shape_block": "#33ff00",
    "shape_bool": "{|| if $in { 'light_cyan' } else { 'light_red' } }",
    "binary": "red",
    "duration": "blue_bold",
    "header": "cb",
    "filesize": "{|e| if $e == 0b { 'black' } else if $e < 1mb { 'ub' } else { 'cyan' } }",
    "range": "purple",
    "search_result": "blue_reverse",
    "bool": "{|| if $in { 'light_cyan' } else { 'light_red' } }",
    "int": "green",
    "row_index": "yb",
    "shape_closure": "#ffb000"
  },
  "footer_mode": "auto",
  "float_precision": 2,
  "recursion_limit": 50,
  "use_ansi_coloring": "true",
  "completions": {
    "sort": "smart",
    "case_sensitive": false,
    "quick": true,
    "partial": true,
    "algorithm": "prefix",
    "external": {
      "enable": true,
      "max_results": 10,
      "completer": null
    },
    "use_ls_colors": true
  },
  "edit_mode": "emacs",
  "history": {
    "max_size": 1000000,
    "sync_on_enter": true,
    "file_format": "sqlite",
    "isolation": true
  },
  "keybindings": [
    {
      "name": "open_command_editor",
      "modifier": "control",
      "keycode": "char_o",
      "event": {
        "send": "openeditor"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "clear_everything",
      "modifier": "control",
      "keycode": "char_l",
      "event": [
        {
          "send": "clearscrollback"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "insert_newline",
      "modifier": "shift",
      "keycode": "enter",
      "event": {
        "edit": "insertnewline"
      },
      "mode": "emacs"
    },
    {
      "name": "completion_menu",
      "modifier": "none",
      "keycode": "tab",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "completion_menu"
          },
          {
            "send": "menunext"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "completion_previous",
      "modifier": "shift",
      "keycode": "backtab",
      "event": {
        "send": "menuprevious"
      },
      "mode": "emacs"
    },
    {
      "name": "insert_last_token",
      "modifier": "alt",
      "keycode": "char_.",
      "event": [
        {
          "edit": "insertstring",
          "value": " !$"
        },
        {
          "send": "enter"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "complete_hint_chunk",
      "modifier": "alt",
      "keycode": "right",
      "event": {
        "until": [
          {
            "send": "historyhintwordcomplete"
          },
          {
            "edit": "movewordright"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "un_complete_hint_chunk",
      "modifier": "alt",
      "keycode": "left",
      "event": [
        {
          "edit": "backspaceword"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "delete-word",
      "modifier": "control",
      "keycode": "backspace",
      "event": {
        "until": [
          {
            "edit": "backspaceword"
          }
        ]
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "trigger-history-menu",
      "modifier": "control",
      "keycode": "char_x",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "history_menu"
          },
          {
            "send": "menupagenext"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "trigger-history-previous",
      "modifier": "control",
      "keycode": "char_z",
      "event": {
        "until": [
          {
            "send": "menupageprevious"
          },
          {
            "edit": "undo"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "change_dir_with_fzf",
      "modifier": "control",
      "keycode": "char_f",
      "event": {
        "send": "executehostcommand",
        "cmd": "cd (ls | where type == dir | each { |it| $it.name} | str join (char nl) | fzf | decode utf-8 | str trim)"
      },
      "mode": "emacs"
    },
    {
      "name": "complete_in_cd",
      "modifier": "none",
      "keycode": "f2",
      "event": [
        {
          "edit": "clear"
        },
        {
          "edit": "insertString",
          "value": "./"
        },
        {
          "send": "Menu",
          "name": "completion_menu"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "reload_config",
      "modifier": "none",
      "keycode": "f5",
      "event": [
        {
          "edit": "clear"
        },
        {
          "send": "executehostcommand",
          "cmd": "source C:\\Users\\username\\AppData\\Roaming\\nushell\\env.nu; source C:\\Users\\username\\AppData\\Roaming\\nushell\\config.nu"
        }
      ],
      "mode": [
        "emacs",
        "vi_insert",
        "vi_normal"
      ]
    },
    {
      "name": "clear",
      "modifier": "none",
      "keycode": "esc",
      "event": {
        "edit": "clear"
      },
      "mode": "emacs"
    },
    {
      "name": "test_fkeys",
      "modifier": "none",
      "keycode": "f3",
      "event": [
        {
          "edit": "clear"
        },
        {
          "edit": "insertstring",
          "value": "C:\\Users\\username\\source\\repos\\forks\\nushell"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "abbr",
      "modifier": "control",
      "keycode": "space",
      "event": [
        {
          "send": "menu",
          "name": "abbr_menu"
        },
        {
          "edit": "insertchar",
          "value": " "
        }
      ],
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fzf_edit",
      "modifier": "control",
      "keycode": "char_d",
      "event": [
        {
          "send": "executehostcommand",
          "cmd": "do { |$file| if (not ($file | is-empty)) { nvim $file } } (fzf | str trim)"
        }
      ],
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "history_menu_by_session",
      "modifier": "alt",
      "keycode": "char_r",
      "event": {
        "send": "menu",
        "name": "history_menu_by_session"
      },
      "mode": "emacs"
    },
    {
      "name": "fuzzy_history",
      "modifier": "control",
      "keycode": "char_r",
      "event": [
        {
          "send": "ExecuteHostCommand",
          "cmd": "do {\n          $env.SHELL = 'c:/progra~1/git/usr/bin/bash.exe'\n          commandline edit -r (\n            history\n            | get command\n            | reverse\n            | uniq\n            | str join (char -i 0)\n            | fzf --scheme=history --read0 --layout=reverse --height=40% --bind 'tab:change-preview-window(right,70%|right)' -q (commandline) --preview='echo -n {} | nu --stdin -c 'nu-highlight''\n            | decode utf-8\n            | str trim\n          )\n        }"
        }
      ],
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fuzzy_dir",
      "modifier": "control",
      "keycode": "char_s",
      "event": {
        "send": "executehostcommand",
        "cmd": "commandline edit -a (ls **/* | where type == dir | get name | to text | fzf -q (commandline) | str trim);commandline set-cursor --end"
      },
      "mode": "emacs"
    },
    {
      "name": "fzf_dir_menu_nu_ui",
      "modifier": "control",
      "keycode": "char_n",
      "event": {
        "send": "menu",
        "name": "fzf_dir_menu_nu_ui"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fzf_history_menu_fzf_ui",
      "modifier": "control",
      "keycode": "char_e",
      "event": {
        "send": "menu",
        "name": "fzf_history_menu_fzf_ui"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fzf_history_menu_nu_ui",
      "modifier": "control",
      "keycode": "char_w",
      "event": {
        "send": "menu",
        "name": "fzf_history_menu_nu_ui"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "commands_menu",
      "modifier": "control",
      "keycode": "char_t",
      "event": {
        "send": "menu",
        "name": "commands_menu"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "vars_menu",
      "modifier": "control",
      "keycode": "char_y",
      "event": {
        "send": "menu",
        "name": "vars_menu"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "commands_with_description",
      "modifier": "control",
      "keycode": "char_u",
      "event": {
        "send": "menu",
        "name": "commands_with_description"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "trigger-help-menu",
      "modifier": "control",
      "keycode": "char_q",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "help_menu"
          },
          {
            "send": "menunext"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "copy_selection",
      "modifier": "control_shift",
      "keycode": "char_c",
      "event": {
        "edit": "copyselection"
      },
      "mode": "emacs"
    },
    {
      "name": "cut_selection",
      "modifier": "control_shift",
      "keycode": "char_x",
      "event": {
        "edit": "cutselection"
      },
      "mode": "emacs"
    },
    {
      "name": "select_all",
      "modifier": "control_shift",
      "keycode": "char_a",
      "event": {
        "edit": "selectall"
      },
      "mode": "emacs"
    },
    {
      "name": "paste",
      "modifier": "control_shift",
      "keycode": "char_v",
      "event": {
        "edit": "pastecutbufferbefore"
      },
      "mode": "emacs"
    },
    {
      "name": "ide_completion_menu",
      "modifier": "control",
      "keycode": "char_n",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "ide_completion_menu"
          },
          {
            "send": "menunext"
          },
          {
            "edit": "complete"
          }
        ]
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "quick_assign",
      "modifier": "alt",
      "keycode": "char_a",
      "event": [
        {
          "edit": "MoveToStart"
        },
        {
          "edit": "InsertString",
          "value": "let foo = "
        },
        {
          "edit": "MoveLeftBefore",
          "value": "o"
        },
        {
          "edit": "MoveLeftUntil",
          "value": "f",
          "select": true
        }
      ],
      "mode": [
        "emacs",
        "vi_insert",
        "vi_normal"
      ]
    }
  ],
  "menus": [
    {
      "name": "ide_completion_menu",
      "marker": " \n❯ 📎 ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": {
          "attr": "r"
        },
        "description_text": "yellow",
        "match_text": {
          "fg": "#33ff00"
        },
        "selected_match_text": {
          "fg": "#33ff00",
          "attr": "r"
        }
      },
      "type": {
        "layout": "ide",
        "min_completion_width": 0,
        "max_completion_width": 50,
        "padding": 0,
        "border": true,
        "cursor_offset": 0,
        "description_mode": "prefer_right",
        "min_description_width": 0,
        "max_description_width": 50,
        "max_description_height": 10,
        "description_offset": 1,
        "correct_cursor_pos": true
      },
      "source": null
    },
    {
      "name": "completion_menu",
      "marker": " \n❯ 📎 ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": {
          "attr": "r"
        },
        "description_text": "yellow",
        "match_text": {
          "fg": "#33ff00"
        },
        "selected_match_text": {
          "fg": "#33ff00",
          "attr": "r"
        }
      },
      "type": {
        "layout": "columnar",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2,
        "tab_traversal": "vertical"
      },
      "source": null
    },
    {
      "name": "history_menu",
      "marker": "🔍 ",
      "only_buffer_difference": false,
      "style": {
        "text": "#ffb000",
        "selected_text": {
          "fg": "#ffb000",
          "attr": "r"
        },
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": null
    },
    {
      "name": "help_menu",
      "marker": "? ",
      "only_buffer_difference": true,
      "style": {
        "text": "#7F00FF",
        "selected_text": {
          "fg": "#ffff00",
          "bg": "#7F00FF",
          "attr": "b"
        },
        "description_text": "#ffff00"
      },
      "type": {
        "layout": "description",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2,
        "selection_rows": 4,
        "description_rows": 10
      },
      "source": null
    },
    {
      "name": "fzf_history_menu_fzf_ui",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "columnar",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2
      },
      "source": "{|buffer, position|\n        open $nu.history-path | get history.command_line | to text | fzf +s --tac | str trim\n        | where $it =~ $buffer\n        | each {|v| {value: ($v | str trim)} }\n      }"
    },
    {
      "name": "fzf_history_menu_nu_ui",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "#66ff66",
        "selected_text": {
          "fg": "#66ff66",
          "attr": "r"
        },
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        open $nu.history-path | get history.command_line | to text\n        | fzf -f $buffer\n        | lines\n        | each {|v| {value: ($v | str trim)} }\n      }"
    },
    {
      "name": "fzf_dir_menu_nu_ui",
      "marker": "# ",
      "only_buffer_difference": true,
      "style": {
        "text": "#66ff66",
        "selected_text": {
          "fg": "#66ff66",
          "attr": "r"
        },
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        ls $env.PWD | where type == dir\n        | sort-by name | get name | to text\n        | fzf -f $buffer\n        | each {|v| {value: ($v | str trim)} }\n      }"
    },
    {
      "name": "commands_menu",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "columnar",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2
      },
      "source": "{|buffer, position|\n        scope commands\n        | where name =~ $buffer\n        | each {|it| {value: $it.name description: $it.usage} }\n      }"
    },
    {
      "name": "vars_menu",
      "marker": "V ",
      "only_buffer_difference": true,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        scope variables\n        | where name =~ $buffer\n        | sort-by name\n        | each {|it| {value: $it.name description: $it.type} }\n      }"
    },
    {
      "name": "commands_with_description",
      "marker": "# ",
      "only_buffer_difference": true,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "description",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2,
        "selection_rows": 4,
        "description_rows": 10
      },
      "source": "{|buffer, position|\n        scope commands\n        | where name =~ $buffer\n        | each {|it| {value: $it.name description: $it.usage} }\n      }"
    },
    {
      "name": "abbr_menu",
      "marker": "👀 ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "columnar",
        "columns": 1,
        "col_width": 20,
        "col_padding": 2
      },
      "source": "{|buffer, position|\n        scope aliases\n        | where name == $buffer\n        | each {|it| {value: $it.expansion} }\n      }"
    },
    {
      "name": "history_menu_by_session",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        history -l\n        | where session_id == (history session)\n        | select command\n        | where command =~ $buffer\n        | each {|it| {value: $it.command} }\n        | reverse\n        | uniq\n      }"
    }
  ],
  "hooks": {
    "pre_prompt": [
      "{|| null }",
      "{||\n  zoxide add -- $env.PWD\n}"
    ],
    "pre_execution": [
      "{|| null }"
    ],
    "env_change": {
      "PWD": [
        "{|before, after|\n          print (lsg)\n          # null\n        }",
        "{|before, _|\n          if $before == null {\n            let file = ($nu.home-path | path join \".local\" \"share\" \"nushell\" \"startup-times.nuon\")\n            if not ($file | path exists) {\n              mkdir ($file | path dirname)\n              touch $file\n            }\n            let ver = (version)\n            open $file | append {\n              date: (date now)\n              time: $nu.startup-time\n              build: ($ver.build_rust_channel)\n              allocator: ($ver.allocator)\n              version: ($ver.version)\n              commit: ($ver.commit_hash)\n              build_time: ($ver.build_time)\n              bytes_loaded: (view files | get size | math sum)\n            } | collect { save --force $file }\n          }\n        }",
        {
          "condition": "{|_, after| not ($after | path join 'toolkit.nu' | path exists) }",
          "code": "hide toolkit"
        },
        {
          "condition": "{|_, after| $after | path join 'toolkit.nu' | path exists }",
          "code": "\n        print $'(ansi default_underline)(ansi default_bold)toolkit(ansi reset) module (ansi green_italic)detected(ansi reset)...'\n        print $'(ansi yellow_italic)activating(ansi reset) (ansi default_underline)(ansi default_bold)toolkit(ansi reset) module with `(ansi default_dimmed)(ansi default_italic)use toolkit.nu(ansi reset)`'\n        use toolkit.nu\n        "
        }
      ]
    },
    "display_output": "{||\n      # if (term size).columns > 100 { table -e } else { table }\n      table\n    }",
    "command_not_found": "{||\n      null # return an error message when a command is not found\n    }"
  },
  "rm": {
    "always_trash": true
  },
  "shell_integration": {
    "osc2": true,
    "osc7": true,
    "osc8": true,
    "osc9_9": true,
    "osc133": true,
    "osc633": true,
    "reset_application_mode": true
  },
  "buffer_editor": "nvim",
  "show_banner": true,
  "bracketed_paste": true,
  "render_right_prompt_on_last_line": false,
  "explore": {
    "try": {
      "reactive": true
    },
    "table": {
      "selected_cell": {
        "bg": "blue"
      },
      "show_cursor": false
    }
  },
  "cursor_shape": {
    "emacs": "underscore",
    "vi_insert": "block",
    "vi_normal": "line"
  },
  "datetime_format": {
    "normal": null,
    "table": null
  },
  "error_style": "fancy",
  "display_errors": {
    "exit_code": true,
    "termination_signal": true
  },
  "use_kitty_protocol": true,
  "highlight_resolved_externals": true,
  "plugins": {},
  "plugin_gc": {
    "default": {
      "enabled": true,
      "stop_after": 0
    },
    "plugins": {
      "gstat": {
        "enabled": true,
        "stop_after": 0
      }
    }
  }
}"##;
    serde_json::from_str(json_str).expect("Failed to parse example JSON")
}

// fn main() -> Result<(), Box<dyn Error>> {
//     let args: Vec<String> = env::args().collect();

//     let mut cli_mode = false;
//     let mut output_file: Option<String> = None;
//     let mut use_example = false;

//     let mut i = 1;
//     while i < args.len() {
//         match args[i].as_str() {
//             "--cli" => cli_mode = true,
//             "--tui" => cli_mode = false,
//             "--example" => use_example = true,
//             "-o" | "--output" => {
//                 i += 1;
//                 if i < args.len() {
//                     output_file = Some(args[i].clone());
//                 }
//             }
//             "-h" | "--help" => {
//                 print_usage();
//                 return Ok(());
//             }
//             _ => {}
//         }
//         i += 1;
//     }

//     // Get JSON data
//     let json_data: Value = if use_example {
//         get_example_json()
//     } else {
//         let mut input = Vec::new();
//         io::stdin().read_to_end(&mut input)?;
//         serde_json::from_slice(&input)?
//     };

//     if cli_mode {
//         // Original CLI behavior
//         print_json_tree(&json_data, "", true, None);
//     } else {
//         // TUI mode
//         run_config_tui(json_data, output_file)?;
//     }

//     Ok(())
// }
