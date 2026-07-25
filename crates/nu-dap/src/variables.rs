//! Converts nu `Value`s into the DAP variable tree stored in a
//! `PauseSnapshot` arena. Records/lists/tables become expandable nodes;
//! everything else renders as a leaf string.

use crate::dap::types::Variable;
use crate::state::{PauseSnapshot, VarNode};
use nu_protocol::Value;

/// Max children materialized per list/record level to keep responses bounded.
const MAX_CHILDREN: usize = 200;
/// Levels materialized eagerly at pause time; deeper levels hydrate on demand
/// (`materialize_children` when the client expands a node), so no depth limit.
const EAGER_DEPTH: usize = 1;

/// A list whose elements are all records is a nu table.
pub(crate) fn is_table(value: &Value) -> bool {
    match value {
        Value::List { vals, .. } => {
            !vals.is_empty() && vals.iter().all(|v| matches!(v, Value::Record { .. }))
        }
        _ => false,
    }
}

pub(crate) fn short_render(value: &Value) -> String {
    match value {
        Value::String { val, .. } => {
            // chars(), not byte slicing: a byte index can land mid-codepoint
            // and panic, which inside a debugger callback hangs the session.
            let n_chars = val.chars().count();
            if n_chars > 120 {
                let head: String = val.chars().take(117).collect();
                format!("\"{head}…\" ({n_chars} chars)")
            } else {
                format!("\"{val}\"")
            }
        }
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Bool { val, .. } => val.to_string(),
        Value::Nothing { .. } => "null".into(),
        Value::Filesize { val, .. } => format!("{val}"),
        Value::Duration { val, .. } => format!("{val}ns"),
        Value::Date { val, .. } => val.to_rfc3339(),
        Value::List { vals, .. } if is_table(value) => {
            format!("[table {} rows]", vals.len())
        }
        Value::List { vals, .. } => {
            // Preview the first few scalar elements: [1, 2, 3, …]
            let mut preview = String::from("[");
            for (i, v) in vals.iter().take(3).enumerate() {
                if i > 0 {
                    preview.push_str(", ");
                }
                preview.push_str(&scalar_preview(v));
            }
            if vals.len() > 3 {
                preview.push_str(", …");
            }
            preview.push(']');
            if preview.len() > 60 {
                format!("[list {} items]", vals.len())
            } else {
                preview
            }
        }
        Value::Record { val, .. } => {
            // Preview the first few fields: {name: a.txt, size: 120, …}
            let mut preview = String::from("{");
            for (i, (k, v)) in val.iter().take(3).enumerate() {
                if i > 0 {
                    preview.push_str(", ");
                }
                preview.push_str(k);
                preview.push_str(": ");
                preview.push_str(&scalar_preview(v));
            }
            if val.len() > 3 {
                preview.push_str(", …");
            }
            preview.push('}');
            if preview.len() > 60 {
                format!("{{record {} fields}}", val.len())
            } else {
                preview
            }
        }
        Value::Closure { .. } => "<closure>".into(),
        Value::Binary { val, .. } => {
            use std::fmt::Write;
            // First bytes as hex, nu-literal style: 0x[de ad be ef …] (N bytes)
            let mut s = String::from("0x[");
            for (i, b) in val.iter().take(8).enumerate() {
                if i > 0 {
                    s.push(' ');
                }
                let _ = write!(s, "{b:02x}");
            }
            if val.len() > 8 {
                s.push_str(" …");
            }
            let _ = write!(s, "] ({} bytes)", val.len());
            s
        }
        Value::Error { error, .. } => format!("<error: {error}>"),
        other => format!("{other:?}").chars().take(120).collect(),
    }
}

/// Describe a stream without consuming it (draining would eat the
/// program's data): kind, origin, and size when known.
pub(crate) fn describe_stream(data: &nu_protocol::PipelineData) -> String {
    use nu_protocol::PipelineData;
    use nu_protocol::byte_stream::ByteStreamSource;
    match data {
        PipelineData::ByteStream(bs, _) => {
            let kind = match bs.type_() {
                nu_protocol::ByteStreamType::Binary => "binary",
                nu_protocol::ByteStreamType::String => "text",
                _ => "unknown type",
            };
            let origin = match bs.source() {
                ByteStreamSource::File(_) => "file",
                ByteStreamSource::Read(_) => "internal",
                _ => "external command",
            };
            match bs.known_size() {
                Some(n) => format!("<byte stream: {kind} from {origin}, {n} bytes>"),
                None => format!("<byte stream: {kind} from {origin}>"),
            }
        }
        PipelineData::ListStream(..) => "<list stream (lazy)>".to_string(),
        _ => "<stream>".to_string(),
    }
}

/// One-token rendering used inside list/record previews.
pub(crate) fn scalar_preview(value: &Value) -> String {
    match value {
        Value::String { val, .. } => {
            let mut s: String = val.chars().take(12).collect();
            if val.chars().count() > 12 {
                s.push('…');
            }
            s
        }
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Bool { val, .. } => val.to_string(),
        Value::Nothing { .. } => "null".into(),
        Value::List { vals, .. } => format!("[{}]", vals.len()),
        Value::Record { val, .. } => format!("{{{}}}", val.len()),
        _ => "…".into(),
    }
}

fn type_name(value: &Value) -> String {
    value.get_type().to_string()
}

/// Adds `value` to the snapshot arena under `name`, returning the arena index.
/// Expandable values get a fresh variablesReference registered in var_refs.
pub(crate) fn add_value(
    snapshot: &mut PauseSnapshot,
    name: String,
    value: &Value,
    depth: usize,
) -> usize {
    // Containers are always expandable — children materialize lazily.
    let expandable = matches!(value, Value::List { .. } | Value::Record { .. });

    let var_ref = if expandable { snapshot.alloc_ref() } else { 0 };

    let node_idx = snapshot.var_arena.len();
    snapshot.var_arena.push(VarNode {
        var: Variable {
            name,
            value: short_render(value),
            type_: Some(type_name(value)),
            variables_reference: var_ref,
        },
        children: Vec::new(),
        // Full value for the visualizer (nuDapVisualize). Clones the subtree
        // once per ancestor — bounded by MAX_DEPTH/MAX_CHILDREN.
        value: value.clone(),
    });

    if expandable && depth < EAGER_DEPTH {
        materialize_at(snapshot, node_idx, depth);
    }

    node_idx
}

/// Materialize the direct children of the node at `node_idx` (one level).
fn materialize_at(snapshot: &mut PauseSnapshot, node_idx: usize, depth: usize) {
    let var_ref = snapshot.var_arena[node_idx].var.variables_reference;
    if var_ref == 0 || snapshot.var_refs.contains_key(&var_ref) {
        return;
    }
    let value = snapshot.var_arena[node_idx].value.clone();
    let mut children = Vec::new();
    match &value {
        Value::Record { val, .. } => {
            for (k, v) in val.iter().take(MAX_CHILDREN) {
                children.push(add_value(snapshot, k.clone(), v, depth + 1));
            }
        }
        Value::List { vals, .. } => {
            for (i, v) in vals.iter().enumerate().take(MAX_CHILDREN) {
                children.push(add_value(snapshot, format!("[{i}]"), v, depth + 1));
            }
        }
        _ => {}
    }
    snapshot.var_refs.insert(var_ref, children.clone());
    snapshot.var_arena[node_idx].children = children;
}

/// On-demand hydration: called by the server thread when the client expands
/// a node whose children were never materialized. No effect on refs that are
/// already populated or unknown.
pub(crate) fn materialize_children(snapshot: &mut PauseSnapshot, var_ref: i64) {
    if var_ref == 0 || snapshot.var_refs.contains_key(&var_ref) {
        return;
    }
    if let Some(idx) = snapshot
        .var_arena
        .iter()
        .position(|n| n.var.variables_reference == var_ref)
    {
        // Children of an on-demand node stay lazy themselves (depth beyond
        // the eager horizon).
        materialize_at(snapshot, idx, EAGER_DEPTH);
    }
}

/// Rebuild a Locals+Globals snapshot for a past timeline entry — pure, no
/// `engine_state` (the server thread must never touch it). Pipeline /
/// Registers / Process are live-only and intentionally omitted.
pub(crate) fn build_history_snapshot(
    entry: &crate::state::TimelineEntry,
    baseline_env: Option<&std::collections::HashMap<String, Value>>,
    nu_constant: Option<&Value>,
) -> crate::state::PauseSnapshot {
    use crate::state::PauseSnapshot;
    let mut snap = PauseSnapshot::new();
    snap.frames = entry.frames.clone();

    // Locals: `return` first, then shadow vars sorted by name (same order as
    // the live build).
    let mut locals = Vec::new();
    if let Some(v) = &entry.last_result {
        locals.push(add_value(&mut snap, "return".to_string(), v, 0));
    }
    let mut vars: Vec<&crate::state::ShadowVar> = entry.shadow_vars.values().collect();
    vars.sort_by(|a, b| a.name.cmp(&b.name));
    for sv in vars {
        locals.push(add_value(&mut snap, sv.name.clone(), &sv.value, 0));
    }
    snap.var_refs.insert(PauseSnapshot::LOCALS_REF, locals);

    // Globals: $nu (cached constant) and $env (baseline overlaid with the
    // entry's env mutations at that moment).
    let mut globals = Vec::new();
    if let Some(nu) = nu_constant {
        let v = nu.clone();
        globals.push(add_value(&mut snap, "$nu".to_string(), &v, 0));
    }
    {
        let mut env_map: std::collections::BTreeMap<String, Value> = baseline_env
            .map(|b| b.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        for (k, v) in &entry.env_shadow {
            env_map.insert(k.clone(), v.clone());
        }
        let mut rec = nu_protocol::Record::new();
        for (k, v) in env_map {
            rec.push(k, v);
        }
        let env_val = Value::record(rec, nu_protocol::Span::unknown());
        globals.push(add_value(&mut snap, "$env".to_string(), &env_val, 0));
    }
    snap.var_refs.insert(PauseSnapshot::GLOBALS_REF, globals);

    // Pipeline: recorded `in → cmd` if this was a pipe-stage boundary.
    let mut pipeline = Vec::new();
    if let Some((cmd, v)) = &entry.pipe_input {
        pipeline.push(add_value(&mut snap, format!("in → {cmd}"), v, 0));
    }
    snap.var_refs.insert(PauseSnapshot::PIPELINE_REF, pipeline);

    // Registers/Process are live-only; empty in the past.
    snap.var_refs
        .insert(PauseSnapshot::REGISTERS_REF, Vec::new());
    snap.var_refs.insert(PauseSnapshot::PROCESS_REF, Vec::new());

    snap
}

/// Bounds for `to_json` (the visualizer payload): generous compared to the
/// Variables tree, but still finite so a huge table can't stall the UI.
pub(crate) const JSON_MAX_ITEMS: usize = 1000;
const JSON_MAX_DEPTH: usize = 8;

/// Converts a nu Value to JSON for the visualizer webview. Sets `truncated`
/// when any bound was hit.
pub(crate) fn to_json(value: &Value, depth: usize, truncated: &mut bool) -> serde_json::Value {
    use serde_json::{Value as J, json};
    if depth >= JSON_MAX_DEPTH {
        *truncated = true;
        return J::String("…".into());
    }
    match value {
        Value::String { val, .. } => json!(val),
        Value::Int { val, .. } => json!(val),
        Value::Float { val, .. } => json!(val),
        Value::Bool { val, .. } => json!(val),
        Value::Nothing { .. } => J::Null,
        Value::Filesize { val, .. } => json!(format!("{val}")),
        Value::Duration { val, .. } => json!(format!("{val}ns")),
        Value::Date { val, .. } => json!(val.to_rfc3339()),
        Value::List { vals, .. } => {
            if vals.len() > JSON_MAX_ITEMS {
                *truncated = true;
            }
            J::Array(
                vals.iter()
                    .take(JSON_MAX_ITEMS)
                    .map(|v| to_json(v, depth + 1, truncated))
                    .collect(),
            )
        }
        Value::Record { val, .. } => {
            if val.len() > JSON_MAX_ITEMS {
                *truncated = true;
            }
            let mut map = serde_json::Map::new();
            for (k, v) in val.iter().take(JSON_MAX_ITEMS) {
                map.insert(k.clone(), to_json(v, depth + 1, truncated));
            }
            J::Object(map)
        }
        Value::Closure { .. } => json!("<closure>"),
        Value::Binary { val, .. } => {
            // Marker object the webview turns into a hex view. Hex keeps the
            // adapter dependency-free; 64 KiB is plenty for a debugger UI.
            const MAX_BYTES: usize = 65536;
            if val.len() > MAX_BYTES {
                *truncated = true;
            }
            let hex: String = val
                .iter()
                .take(MAX_BYTES)
                .map(|b| format!("{b:02x}"))
                .collect();
            json!({ "$nuBinary": hex, "length": val.len() })
        }
        Value::Error { error, .. } => json!(format!("<error: {error}>")),
        other => json!(format!("{other:?}").chars().take(120).collect::<String>()),
    }
}
