//! Flatten nested values and directory listings into visible tree rows.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use nu_protocol::{Record, Span, Value};
use nu_utils::strip_ansi_string_unlikely;

#[derive(Debug, Clone)]
pub struct TreeRow {
    pub path: String,
    /// Plain text: ANSI escapes are dropped, so a hyperlink shows its text,
    /// not its URL.
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
    for (path, label, value) in top_nodes(root, column) {
        flatten_node(value, &path, 0, label, &ctx, &mut out);
    }
    out
}

/// The values of the nodes at `paths`, in tree order, whether or not their
/// parents are expanded: what a `--multi` tree submits. A path that names
/// no node any more is skipped. Only nodes on the way to a checked path are
/// opened, and directory listings come from `cache`, so nothing is read
/// from disk.
pub fn values_at(
    root: &Value,
    paths: &HashSet<String>,
    walk: bool,
    column: &str,
    cwd: &Path,
    cache: &HashMap<String, Vec<Value>>,
) -> Vec<Value> {
    let mut out = Vec::new();
    visit_at(root, paths, walk, column, cwd, cache, &mut |_, value| {
        out.push(value.clone())
    });
    out
}

/// The paths in `paths` that still name a node, found the way
/// [`values_at`] finds them. A tree whose data was replaced keeps only
/// these checks.
pub fn existing_paths(
    root: &Value,
    paths: &HashSet<String>,
    walk: bool,
    column: &str,
    cwd: &Path,
    cache: &HashMap<String, Vec<Value>>,
) -> HashSet<String> {
    let mut out = HashSet::new();
    visit_at(root, paths, walk, column, cwd, cache, &mut |path, _| {
        out.insert(path.to_string());
    });
    out
}

/// Call `visit` with the path and value of every node at `paths`, in tree
/// order. The walk behind [`values_at`] and [`existing_paths`].
fn visit_at(
    root: &Value,
    paths: &HashSet<String>,
    walk: bool,
    column: &str,
    cwd: &Path,
    cache: &HashMap<String, Vec<Value>>,
    visit: &mut dyn FnMut(&str, &Value),
) {
    if paths.is_empty() {
        return;
    }
    // `children` does not look at expansion.
    let expanded = HashSet::new();
    let ctx = Ctx {
        expanded: &expanded,
        walk,
        column,
        cwd,
        cache,
    };
    for (path, _, value) in top_nodes(root, column) {
        visit_node(value, &path, paths, &ctx, visit);
    }
}

/// Visit the node at `path` if it is in `paths`, then the children of it
/// that lead to another path in `paths`.
fn visit_node<'a>(
    value: &'a Value,
    path: &str,
    paths: &HashSet<String>,
    ctx: &Ctx<'a>,
    visit: &mut dyn FnMut(&str, &Value),
) {
    if paths.contains(path) {
        visit(path, value);
    }
    let prefix = format!("{path}/");
    if !paths.iter().any(|p| p.starts_with(&prefix)) {
        return;
    }
    for (key, child) in children(value, path, ctx) {
        visit_node(child, &child_path(path, &key), paths, ctx, visit);
    }
}

/// The top-level nodes of a tree as (path, label, value): the items of a
/// list, the fields of a record, or the root itself for an `ls` row or a
/// scalar.
fn top_nodes<'a>(root: &'a Value, column: &str) -> Vec<(String, String, &'a Value)> {
    match root {
        Value::List { vals, .. } => vals
            .iter()
            .enumerate()
            .map(|(i, val)| (i.to_string(), item_label(val, column), val))
            .collect(),
        Value::Record { val, .. } if is_ls_style_row(val) => {
            vec![("root".into(), node_label(root, column), root)]
        }
        Value::Record { val, .. } => val
            .iter()
            .map(|(key, child)| (escape_key(key), field_label(key, child), child))
            .collect(),
        other => vec![("0".into(), node_label(other, column), other)],
    }
}

fn flatten_node<'a>(
    value: &'a Value,
    path: &str,
    depth: usize,
    label: String,
    ctx: &Ctx<'a>,
    out: &mut Vec<TreeRow>,
) {
    let expandable =
        has_children(value, path, ctx) || (ctx.walk && is_dir_value(value, ctx.column, ctx.cwd));
    let is_open = ctx.expanded.contains(path);
    out.push(TreeRow {
        path: path.to_string(),
        label: strip_ansi_string_unlikely(label),
        depth,
        expandable,
        expanded: is_open,
        value: value.clone(),
    });
    if expandable && is_open {
        // Children are only walked for open nodes; a collapsed record
        // costs one row, not a copy of every field.
        for (key, child) in children(value, path, ctx) {
            let child_path = child_path(path, &key);
            let child_label = match value {
                Value::List { .. } => item_label(child, ctx.column),
                _ => field_label(&key, child),
            };
            flatten_node(child, &child_path, depth + 1, child_label, ctx, out);
        }
    }
}

/// Whether a node has children to show (see [`children`]).
fn has_children(value: &Value, path: &str, ctx: &Ctx<'_>) -> bool {
    match value {
        Value::Record { val, .. } => {
            if ctx.walk && is_ls_style_row(val) && is_dir_row(val) {
                return ctx.cache.get(path).is_some_and(|kids| !kids.is_empty());
            }
            !val.is_empty()
        }
        Value::List { vals, .. } => !vals.is_empty(),
        _ => false,
    }
}

/// A node's children, borrowed. A record expands into its fields, except
/// that with `--walk` a directory row expands into its listing instead.
fn children<'a>(value: &'a Value, path: &str, ctx: &Ctx<'a>) -> Vec<(String, &'a Value)> {
    match value {
        Value::Record { val, .. } => {
            if ctx.walk && is_ls_style_row(val) && is_dir_row(val) {
                return ctx
                    .cache
                    .get(path)
                    .into_iter()
                    .flatten()
                    .map(|v| (node_label(v, ctx.column), v))
                    .collect();
            }
            val.iter().map(|(k, v)| (k.clone(), v)).collect()
        }
        Value::List { vals, .. } => vals
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect(),
        _ => Vec::new(),
    }
}

/// The path of child `key` of the node at `parent`. A node path joins its
/// keys with `/`, each escaped by [`escape_key`], so two nodes never share
/// a path: not a key `a/b` and key `b` inside key `a`, nor child `x` of an
/// empty key and a top-level `x`.
fn child_path(parent: &str, key: &str) -> String {
    format!("{parent}/{}", escape_key(key))
}

/// `key` as one segment of a node path: `\` and `/` get a `\` before them.
fn escape_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for c in key.chars() {
        if matches!(c, '\\' | '/') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A node path as `values.<id>.path` shows it: the keys joined with `/`,
/// without the escapes that keep paths apart.
pub fn display_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            c => out.push(c),
        }
    }
    out
}

fn field_label(key: &str, value: &Value) -> String {
    match value {
        Value::Record { .. } | Value::List { .. } => key.to_string(),
        other => format!("{key}: {}", primitive_text(other)),
    }
}

fn item_label(value: &Value, column: &str) -> String {
    match value {
        Value::Record { .. } => node_label(value, column),
        Value::List { vals, .. } => format!("[{}]", vals.len()),
        other => primitive_text(other),
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

    fn ls_rows() -> Value {
        let row = |name: &str, kind: &str| {
            let mut r = Record::new();
            r.insert("name", Value::test_string(name));
            r.insert("type", Value::test_string(kind));
            r.insert("size", Value::test_int(3));
            Value::test_record(r)
        };
        Value::test_list(vec![row("notes.txt", "file"), row("src", "dir")])
    }

    #[test]
    fn ls_rows_expand_into_fields_without_walk() {
        let set: HashSet<String> = ["0".to_string()].into_iter().collect();
        let rows = flatten(
            &ls_rows(),
            &set,
            false,
            "name",
            Path::new("."),
            &HashMap::new(),
        );
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "notes.txt",
                "name: notes.txt",
                "type: file",
                "size: 3",
                "src"
            ]
        );
        assert!(
            rows.iter().filter(|r| r.depth == 0).all(|r| r.expandable),
            "every ls row can open"
        );
    }

    #[test]
    fn walk_expands_dirs_into_listings_and_files_into_fields() {
        let dir = std::env::temp_dir().join("nu-tui-tree-walk-fields");
        let _ = std::fs::create_dir_all(dir.join("src"));
        let _ = std::fs::write(dir.join("src").join("main.nu"), "");
        let _ = std::fs::write(dir.join("notes.txt"), "");
        let mut cache = HashMap::new();
        cache.insert(
            "1".to_string(),
            read_dir_listing(&dir.join("src"), Span::test_data()),
        );
        let set: HashSet<String> = ["0".to_string(), "1".to_string()].into_iter().collect();
        let rows = flatten(&ls_rows(), &set, true, "name", &dir, &cache);
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert!(
            labels.contains(&"type: file"),
            "file rows show fields: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.ends_with("main.nu")),
            "dir rows show their listing: {labels:?}"
        );
        assert!(
            !labels.contains(&"type: dir"),
            "dir rows hide fields: {labels:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escaped_keys_read_back_and_keep_paths_apart() {
        for key in ["plain", "a/b", r"a\b", r"ends\", "", r"\/"] {
            assert_eq!(display_path(&escape_key(key)), key, "{key:?}");
        }
        // Key `a/b` and key `b` inside key `a`.
        assert_ne!(escape_key("a/b"), child_path("a", "b"));
        // Child `x` of an empty key and a top-level `x`.
        assert_ne!(child_path("", "x"), escape_key("x"));
        // Key `b` inside key `a\` and key `a\/b`.
        assert_ne!(child_path(&escape_key(r"a\"), "b"), escape_key(r"a\/b"));
    }

    #[test]
    fn values_at_tells_a_slash_key_from_a_nested_one() {
        let mut inner = Record::new();
        inner.insert("b", Value::test_int(2));
        let mut rec = Record::new();
        rec.insert("a/b", Value::test_int(1));
        rec.insert("a", Value::test_record(inner));
        let root = Value::test_record(rec);
        let values = |paths: &[String]| {
            let set: HashSet<String> = paths.iter().cloned().collect();
            values_at(&root, &set, false, "name", Path::new("."), &HashMap::new())
        };
        assert_eq!(values(&[child_path("a", "b")]), vec![Value::test_int(2)]);
        assert_eq!(values(&[escape_key("a/b")]), vec![Value::test_int(1)]);
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
