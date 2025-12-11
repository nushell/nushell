//! Application state and drawing logic for the explore config TUI.

use crate::explore_config::tree::{build_tree_items, get_value_at_path, set_value_at_path};
use crate::explore_config::types::{App, EditorMode, Focus, NodeInfo, NuValueType, ValueType};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, Wrap};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use tui_tree_widget::{Tree, TreeState};

impl App {
    pub fn new(
        json_data: Value,
        output_file: Option<String>,
        config_mode: bool,
        nu_type_map: Option<HashMap<String, NuValueType>>,
        doc_map: Option<HashMap<String, String>>,
    ) -> Self {
        let mut node_map = HashMap::new();
        let tree_items = build_tree_items(&json_data, &mut node_map, &nu_type_map, &doc_map);

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
            doc_map,
        }
    }

    pub fn rebuild_tree(&mut self) {
        // Save current selection path from tree state
        let current_selection = self.tree_state.selected().to_vec();

        let mut node_map = HashMap::new();
        // When rebuilding, we don't have the nu_type_map anymore, so pass None
        // This means after editing, we lose the nushell type info, but that's acceptable
        // since the edited values may have different types anyway
        // We still pass doc_map to preserve documentation status
        self.tree_items = build_tree_items(&self.json_data, &mut node_map, &None, &self.doc_map);
        self.node_map = node_map;

        // Try to restore selection if the node still exists
        if let Some(last_id) = current_selection.last() {
            if self.node_map.contains_key(last_id) {
                self.tree_state.select(current_selection);
            }
        }
    }

    pub fn get_current_node_info(&self) -> Option<&NodeInfo> {
        if self.selected_identifier.is_empty() {
            return None;
        }
        self.node_map.get(&self.selected_identifier)
    }

    pub fn force_update_editor(&mut self) {
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

    pub fn apply_edit(&mut self) {
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

    pub fn save_to_file(&mut self) -> io::Result<()> {
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

    pub fn draw(&mut self, frame: &mut Frame) {
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
                Constraint::Length(3),  // Path display
                Constraint::Length(3),  // Type info
                Constraint::Min(5),     // Editor area
                Constraint::Length(20), // Description (new section)
                Constraint::Length(3),  // Help text (with border)
            ])
            .split(inner_area);

        // Path display (read-only)
        self.draw_path_widget(frame, editor_chunks[0]);

        // Type info
        self.draw_type_widget(frame, editor_chunks[1]);

        // Editor area
        self.draw_editor_widget(frame, editor_chunks[2]);

        // Description (new section)
        self.draw_description_widget(frame, editor_chunks[3]);

        // Help text
        self.draw_editor_help(frame, editor_chunks[4]);
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

                // In config mode, use nushell types if available
                if self.config_mode {
                    if let Some(ref nu_type) = node_info.nu_type {
                        (nu_type.label().to_string(), nu_type.color(), extra)
                    } else {
                        (
                            node_info.value_type.label().to_string(),
                            node_info.value_type.color(),
                            extra,
                        )
                    }
                } else {
                    (
                        node_info.value_type.label().to_string(),
                        node_info.value_type.color(),
                        extra,
                    )
                }
            } else {
                ("unknown".to_string(), Color::DarkGray, String::new())
            };

        let type_line = Line::from(vec![
            Span::styled(
                format!(" {} ", &type_label),
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

    fn draw_description_widget(&self, frame: &mut Frame, area: Rect) {
        let node_info = self.get_current_node_info();

        // Determine if we have documentation for this node
        let (description, has_doc) = if self.config_mode {
            if let Some(ref info) = node_info {
                // Build the config path from the node path (e.g., ["history", "file_format"] -> "history.file_format")
                let config_path = info.path.join(".");

                if let Some(ref doc_map) = self.doc_map {
                    if let Some(doc) = doc_map.get(&config_path) {
                        (doc.clone(), true)
                    } else {
                        // Try parent paths for nested items
                        let mut found_doc = None;
                        let mut path_parts = info.path.clone();
                        while !path_parts.is_empty() && found_doc.is_none() {
                            let parent_path = path_parts.join(".");
                            if let Some(doc) = doc_map.get(&parent_path) {
                                found_doc = Some(doc.clone());
                            }
                            path_parts.pop();
                        }
                        if let Some(doc) = found_doc {
                            (doc, true)
                        } else {
                            (
                                "No documentation available for this setting.".to_string(),
                                false,
                            )
                        }
                    }
                } else {
                    ("Documentation not loaded.".to_string(), false)
                }
            } else {
                ("Select a node to see its description.".to_string(), false)
            }
        } else {
            (
                "Documentation is only available in config mode.".to_string(),
                false,
            )
        };

        // Use different styling based on whether documentation exists
        let (title_style, border_style) = if self.config_mode && !has_doc {
            // Highlight missing documentation with yellow/warning color
            (
                Style::default().fg(Color::Yellow).bold(),
                Style::default().fg(Color::Yellow),
            )
        } else {
            (
                Style::default().fg(Color::Yellow),
                Style::default().fg(Color::DarkGray),
            )
        };

        let title = if self.config_mode && !has_doc {
            " Description [missing] "
        } else {
            " Description "
        };

        let desc_block = Block::default()
            .title(title)
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_style(border_style);

        // Truncate description to fit in the available area
        let inner_height = area.height.saturating_sub(2) as usize; // Account for borders
        let lines: Vec<&str> = description.lines().take(inner_height).collect();
        let display_text = lines.join("\n");

        let desc_text = Paragraph::new(display_text)
            .style(Style::default().fg(if has_doc {
                Color::White
            } else {
                Color::DarkGray
            }))
            .block(desc_block)
            .wrap(Wrap { trim: true });

        frame.render_widget(desc_text, area);
    }

    fn draw_editor_help(&self, frame: &mut Frame, area: Rect) {
        let help_block = Block::default()
            .title(" Help ")
            .title_style(Style::default().fg(Color::Yellow))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

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

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(help_block);

        frame.render_widget(help, area);
    }

    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let status_style = Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White);

        let status = Paragraph::new(format!(" {}", self.status_message)).style(status_style);

        frame.render_widget(status, area);
    }

    pub fn scroll_editor(&mut self, delta: i32) {
        let lines_count = self.editor_content.lines().count();
        if delta < 0 {
            self.editor_scroll = self.editor_scroll.saturating_sub((-delta) as usize);
        } else {
            self.editor_scroll =
                (self.editor_scroll + delta as usize).min(lines_count.saturating_sub(1));
        }
    }
}
