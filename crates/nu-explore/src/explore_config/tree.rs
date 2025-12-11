//! Tree building, path operations, and CLI tree printing utilities.

use crate::explore_config::types::{NodeInfo, NuValueType, ValueType};
use serde_json::Value;
use std::collections::HashMap;
use tui_tree_widget::TreeItem;

/// Check if a JSON value is a leaf node (not an object or array)
pub fn is_leaf(value: &Value) -> bool {
    !value.is_object() && !value.is_array()
}

/// Render a leaf value as a string
pub fn render_leaf(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unserializable>".to_string())
}

/// Print a JSON tree structure to stdout (for CLI mode)
pub fn print_json_tree(value: &Value, prefix: &str, is_tail: bool, key: Option<&str>) {
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

/// Build tree items for the TUI tree widget
pub fn build_tree_items(
    json_data: &Value,
    node_map: &mut HashMap<String, NodeInfo>,
    nu_type_map: &Option<HashMap<String, NuValueType>>,
    doc_map: &Option<HashMap<String, String>>,
) -> Vec<TreeItem<'static, String>> {
    build_tree_items_recursive(
        json_data,
        node_map,
        nu_type_map,
        doc_map,
        Vec::new(),
        String::new(),
    )
}

fn build_tree_items_recursive(
    value: &Value,
    node_map: &mut HashMap<String, NodeInfo>,
    nu_type_map: &Option<HashMap<String, NuValueType>>,
    doc_map: &Option<HashMap<String, String>>,
    current_path: Vec<String>,
    parent_id: String,
) -> Vec<TreeItem<'static, String>> {
    match value {
        Value::Object(map) => {
            // Sort keys alphabetically
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by_key(|(k, _)| *k);

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
                    let nu_type = nu_type_map
                        .as_ref()
                        .and_then(|m| m.get(&identifier).cloned());

                    // Check if documentation exists for this path
                    let config_path = path.join(".");
                    let has_doc = doc_map
                        .as_ref()
                        .is_some_and(|m| m.contains_key(&config_path));

                    node_map.insert(
                        identifier.clone(),
                        NodeInfo {
                            path: path.clone(),
                            value_type,
                            nu_type,
                        },
                    );

                    let display = format_tree_label(key, val, has_doc, doc_map.is_some());

                    if is_leaf(val) {
                        TreeItem::new_leaf(identifier, display)
                    } else {
                        let children = build_tree_items_recursive(
                            val,
                            node_map,
                            nu_type_map,
                            doc_map,
                            path,
                            identifier.clone(),
                        );
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
                let nu_type = nu_type_map
                    .as_ref()
                    .and_then(|m| m.get(&identifier).cloned());

                // Check if documentation exists for this path
                let config_path = path.join(".");
                let has_doc = doc_map
                    .as_ref()
                    .is_some_and(|m| m.contains_key(&config_path));

                node_map.insert(
                    identifier.clone(),
                    NodeInfo {
                        path: path.clone(),
                        value_type,
                        nu_type,
                    },
                );

                let display = format_array_item_label(idx, val, has_doc, doc_map.is_some());

                if is_leaf(val) {
                    TreeItem::new_leaf(identifier, display)
                } else {
                    let children = build_tree_items_recursive(
                        val,
                        node_map,
                        nu_type_map,
                        doc_map,
                        path,
                        identifier.clone(),
                    );
                    TreeItem::new(identifier, display, children)
                        .expect("all item identifiers are unique")
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn format_tree_label(key: &str, value: &Value, has_doc: bool, is_config_mode: bool) -> String {
    let doc_marker = if is_config_mode && !has_doc {
        "⚠ "
    } else {
        ""
    };
    match value {
        Value::Null => format!("{}{}: null", doc_marker, key),
        Value::Bool(b) => format!("{}{}: {}", doc_marker, key, b),
        Value::Number(n) => format!("{}{}: {}", doc_marker, key, n),
        Value::String(s) => {
            let preview = if s.len() > 40 {
                format!("{}...", &s[..37])
            } else {
                s.clone()
            };
            format!(
                "{}{}: \"{}\"",
                doc_marker,
                key,
                preview.replace('\n', "\\n")
            )
        }
        Value::Array(arr) => format!("{}{} [{} items]", doc_marker, key, arr.len()),
        Value::Object(obj) => format!("{}{} {{{} keys}}", doc_marker, key, obj.len()),
    }
}

fn format_array_item_label(
    idx: usize,
    value: &Value,
    has_doc: bool,
    is_config_mode: bool,
) -> String {
    let doc_marker = if is_config_mode && !has_doc {
        "⚠ "
    } else {
        ""
    };
    match value {
        Value::Null => format!("{}[{}]: null", doc_marker, idx),
        Value::Bool(b) => format!("{}[{}]: {}", doc_marker, idx, b),
        Value::Number(n) => format!("{}[{}]: {}", doc_marker, idx, n),
        Value::String(s) => {
            let preview = if s.len() > 40 {
                format!("{}...", &s[..37])
            } else {
                s.clone()
            };
            format!(
                "{}[{}]: \"{}\"",
                doc_marker,
                idx,
                preview.replace('\n', "\\n")
            )
        }
        Value::Array(arr) => format!("{}[{}] [{} items]", doc_marker, idx, arr.len()),
        Value::Object(obj) => format!("{}[{}] {{{} keys}}", doc_marker, idx, obj.len()),
    }
}

/// Get a value at a specific path in the JSON tree
pub fn get_value_at_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
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

/// Set a value at a specific path in the JSON tree
pub fn set_value_at_path(value: &mut Value, path: &[String], new_value: Value) -> bool {
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
                    if let Ok(idx) = part.parse::<usize>()
                        && idx < arr.len()
                    {
                        arr[idx] = new_value;
                        return true;
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
