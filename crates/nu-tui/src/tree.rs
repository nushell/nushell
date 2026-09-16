//! Flatten nested values and directory listings into visible tree rows.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use nu_protocol::{Record, Span, Value};

#[derive(Debug, Clone)]
pub struct TreeRow {
    pub path: String,
    pub label: String,
    pub depth: usize,
    pub expandable: bool,
    pub expanded: bool,
    pub value: Value,
}

struct Ctx<'a> {
    expanded: &'a HashSet<String>,
    walk: bool,
    column: &'a str,
    cwd: &'a Path,
    cache: &'a HashMap<String, Vec<Value>>,
}

pub fn flatten(
    root: &Value,
    expanded: &HashSet<String>,
    walk: bool,
    column: &str,
    cwd: &Path,
    cache: &HashMap<String, Vec<Value>>,
) -> Vec<TreeRow> {
    let ctx = Ctx {
        expanded,
        walk,
        column,
        cwd,
        cache,
    };
    let mut out = Vec::new();
    match root {
        Value::List { vals, .. } => {
            for (i, val) in vals.iter().enumerate() {
                flatten_node(
                    val,
                    &i.to_string(),
                    0,
                    item_label(val, column, i),
                    &ctx,
                    &mut out,
                );
            }
        }
        Value::Record { val, .. } if is_ls_style_row(val) => {
            flatten_node(root, "root", 0, node_label(root, column), &ctx, &mut out);
        }
        Value::Record { val, .. } => {
            for (key, child) in val.iter() {
                flatten_node(child, key, 0, field_label(key, child), &ctx, &mut out);
            }
        }
        other => flatten_node(other, "0", 0, node_label(other, column), &ctx, &mut out),
    }
    out
}

fn flatten_node(
    value: &Value,
    path: &str,
    depth: usize,
    label: String,
    ctx: &Ctx<'_>,
    out: &mut Vec<TreeRow>,
) {
    let kids = children(value, path, ctx);
    let expandable = !kids.is_empty() || (ctx.walk && is_dir_value(value, ctx.column, ctx.cwd));
    let is_open = ctx.expanded.contains(path);
    out.push(TreeRow {
        path: path.to_string(),
        label,
        depth,
        expandable,
        expanded: is_open,
        value: value.clone(),
    });
    if expandable && is_open {
        for (key, child) in kids {
            let child_path = child_path(path, &key);
            let child_label = match value {
                Value::List { .. } => {
                    let index = key.parse().unwrap_or(0);
                    item_label(&child, ctx.column, index)
                }
                _ => field_label(&key, &child),
            };
            flatten_node(&child, &child_path, depth + 1, child_label, ctx, out);
        }
    }
}

fn children(value: &Value, path: &str, ctx: &Ctx<'_>) -> Vec<(String, Value)> {
    match value {
        Value::Record { val, .. } => {
            if is_ls_style_row(val) {
                if ctx.walk && is_dir_row(val) {
                    return ctx
                        .cache
                        .get(path)
                        .into_iter()
                        .flatten()
                        .map(|v| (node_label(v, ctx.column), v.clone()))
                        .collect();
                }
                return Vec::new();
            }
            val.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        }
        Value::List { vals, .. } => vals
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn child_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{parent}/{key}")
    }
}

fn field_label(key: &str, value: &Value) -> String {
    match value {
        Value::Record { .. } | Value::List { .. } => key.to_string(),
        other => format!("{key}: {}", primitive_text(other)),
    }
}

fn item_label(value: &Value, column: &str, index: usize) -> String {
    match value {
        Value::Record { .. } => node_label(value, column),
        Value::List { vals, .. } => format!("[{}]", vals.len()),
        other => {
            let _ = index;
            primitive_text(other)
        }
    }
}

fn node_label(value: &Value, column: &str) -> String {
    match value {
        Value::Record { val, .. } => val
            .get(column)
            .and_then(|v| v.as_str().ok())
            .map(|s| s.to_string())
            .or_else(|| val.iter().next().map(|(k, _)| k.clone()))
            .unwrap_or_else(|| "{...}".into()),
        Value::String { val, .. } => val.clone(),
        Value::List { vals, .. } => format!("[{}]", vals.len()),
        other => primitive_text(other),
    }
}

fn primitive_text(value: &Value) -> String {
    match value {
        Value::String { val, .. } => val.clone(),
        Value::Nothing { .. } => String::new(),
        other => other.to_expanded_string(", ", &nu_protocol::Config::default()),
    }
}

fn is_ls_style_row(record: &Record) -> bool {
    record
        .get("type")
        .and_then(|v| v.as_str().ok())
        .is_some_and(|t| {
            matches!(
                t.to_ascii_lowercase().as_str(),
                "dir" | "directory" | "file" | "symlink" | "shortcut"
            )
        })
}

pub fn dir_path_for_row(value: &Value, column: &str, cwd: &Path) -> Option<PathBuf> {
    let name = match value {
        Value::Record { val, .. } => val.get(column).and_then(|v| v.as_str().ok())?,
        Value::String { val, .. } => val.as_str(),
        _ => return None,
    };
    let p = Path::new(name);
    let full = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    full.is_dir().then_some(full)
}

pub fn is_dir_row(record: &Record) -> bool {
    record
        .get("type")
        .and_then(|v| v.as_str().ok())
        .is_some_and(|t| t.eq_ignore_ascii_case("dir") || t.eq_ignore_ascii_case("directory"))
}

fn is_dir_value(value: &Value, column: &str, cwd: &Path) -> bool {
    match value {
        Value::Record { val, .. } => is_dir_row(val),
        Value::String { .. } => dir_path_for_row(value, column, cwd).is_some(),
        _ => false,
    }
}

pub fn read_dir_listing(path: &Path, span: Span) -> Vec<Value> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let mut rec = Record::new();
        rec.insert(
            "name",
            Value::string(path.join(&name).to_string_lossy().into_owned(), span),
        );
        rec.insert(
            "type",
            Value::string(if is_dir { "dir" } else { "file" }, span),
        );
        rows.push(Value::record(rec, span));
    }
    rows.sort_by(|a, b| {
        let na = a
            .as_record()
            .ok()
            .and_then(|r| r.get("name"))
            .and_then(|v| v.as_str().ok())
            .unwrap_or("");
        let nb = b
            .as_record()
            .ok()
            .and_then(|r| r.get("name"))
            .and_then(|v| v.as_str().ok())
            .unwrap_or("");
        na.cmp(nb)
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nested() -> Value {
        let mut inner = Record::new();
        inner.insert("b", Value::test_int(1));
        inner.insert("c", Value::test_int(2));
        let mut rec = Record::new();
        rec.insert("a", Value::test_record(inner));
        rec.insert(
            "d",
            Value::test_list(vec![Value::test_int(3), Value::test_int(4)]),
        );
        Value::test_record(rec)
    }

    fn flatten_with(expanded: &[&str]) -> Vec<TreeRow> {
        let set: HashSet<String> = expanded.iter().map(|s| (*s).to_string()).collect();
        flatten(
            &nested(),
            &set,
            false,
            "name",
            Path::new("."),
            &HashMap::new(),
        )
    }

    #[test]
    fn nested_record_starts_with_top_level_keys() {
        let rows = flatten_with(&[]);
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["a", "d"]);
        assert!(rows.iter().all(|r| r.depth == 0));
        assert!(rows.iter().all(|r| r.expandable));
    }

    #[test]
    fn expanding_does_not_invent_value_nodes() {
        let rows = flatten_with(&["a", "d"]);
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["a", "b: 1", "c: 2", "d", "3", "4"]);
        assert!(rows.iter().all(|r| r.label != "value"));
        assert_eq!(rows.iter().map(|r| r.depth).max(), Some(1));
    }

    #[test]
    fn expanding_leaves_does_not_grow() {
        let rows = flatten_with(&["a", "a/b", "a/c", "d", "d/0", "d/1"]);
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|r| r.label != "value"));
    }

    #[test]
    fn walk_string_nodes_use_session_cwd() {
        let dir = std::env::temp_dir().join("nu-tui-tree-cwd");
        let _ = std::fs::create_dir_all(&dir);
        let nested = dir.join("child");
        let _ = std::fs::create_dir_all(&nested);
        let root = Value::test_list(vec![Value::test_string("child")]);
        let rows = flatten(&root, &HashSet::new(), true, "name", &dir, &HashMap::new());
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].expandable,
            "relative dir should be expandable against cwd"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
