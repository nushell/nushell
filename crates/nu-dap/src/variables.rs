//! Converts nu `Value`s into the DAP variable tree stored in a
//! `PauseSnapshot` arena. Records/lists/tables become expandable nodes;
//! everything else renders as a leaf string.

use crate::dap::types::Variable;
use crate::state::{PauseSnapshot, RenderCache, VarNode};
use nu_protocol::engine::EngineState;
use nu_protocol::{Config, Type, Value};

/// Max children materialized per list/record level to keep responses bounded.
const MAX_CHILDREN: usize = 200;
/// Levels materialized eagerly at pause time; deeper levels hydrate on demand
/// (`materialize_children` when the client expands a node), so no depth limit.
const EAGER_DEPTH: usize = 1;
/// Cap on any single rendered row, so one hostile value can't flood the pane.
const MAX_ROW_CHARS: usize = 120;
/// Cap on a closure label specifically, leaving room for the capture suffix.
const MAX_CLOSURE_CHARS: usize = 60;

/// Everything a render needs that does not live in the `Value` itself.
///
/// Both fields are resolved on the eval thread and carried in the snapshot:
/// the server thread answers `variables` requests without an `EngineState`
/// (see the concurrency rule in `state.rs`), so anything engine-derived has to
/// arrive pre-computed.
#[derive(Clone, Copy)]
pub(crate) struct RenderCtx<'a> {
    pub(crate) config: &'a Config,
    pub(crate) cache: &'a RenderCache,
}

/// Resolve what rendering will need from the engine: each block's source text
/// (for closure rows) and the name of every variable some block captures (for
/// the rows under an expanded closure).
///
/// Called once after the script is parsed. Blocks are fixed from then on
/// (`source` is a parse-time keyword), and anything missing degrades: an
/// unknown block falls back to `<closure>`, an unknown var to `var{id}`.
pub(crate) fn collect_render_cache(engine_state: &EngineState) -> RenderCache {
    let mut cache = RenderCache::default();
    for id in 0..engine_state.num_blocks() {
        let block = engine_state.get_block(nu_protocol::BlockId::new(id));

        // Names for this block's captures. Taken from the block rather than
        // from every variable in the program, so the map stays small.
        for (var_id, _) in &block.captures {
            cache
                .var_names
                .entry(var_id.get())
                .or_insert_with(|| crate::debugger::stepping::var_name(engine_state, *var_id));
        }

        let Some(span) = block.span else { continue };
        let text = String::from_utf8_lossy(engine_state.get_span_contents(span));
        // Collapse the body onto one line: a row is one line, and a multi-line
        // closure would otherwise break the pane.
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if flat.is_empty() {
            continue;
        }
        cache.closure_src.insert(id, cap(&flat, MAX_CLOSURE_CHARS));
    }
    cache
}

/// Render `value` as the single line shown in the Variables pane.
///
/// Container shapes and every type not special-cased below come from
/// [`Value::to_abbreviated_string`] — the same rendering `table` applies per
/// cell (see `nu-table::common`) — so the debugger names values the way the
/// rest of nushell does. What stays local is what a debugger needs and a
/// pipeline does not:
///
/// - **Bounded.** Variables responses are JSON over a pipe: a multi-megabyte
///   string, or a `Debug`-formatted `ShellError`, must not land in one row.
/// - **Previews, not just shapes.** A container row is expandable, so the
///   collapsed line shows its first entries; `to_abbreviated_string` always
///   collapses to `{record N fields}`.
/// - **Literal, not humanized.** Dates render rfc3339, where the shell shows
///   `human_time_from_now`'s "5 hours ago" — relative to the wall clock, which
///   is both wrong information here and untestable.
/// - **`null` stays visible.** Nothing renders as `""` upstream: a blank row.
/// - **Binary as a nu literal.** `0x[de ad be ef]`, not `[222, 173, 190, 239]`.
///
/// Those last three are not house style: they are what `to nuon` writes, which
/// is the same instinct — a debugger row, like a nuon literal, should say what
/// a value *is* rather than read nicely. `to_nuon` itself is not usable here
/// (it hard-errors on `Value::Error`, `Custom` and `Closure`, and propagates
/// that through containers, so one bad field loses the whole structure — and
/// it is unbounded), but where we depart from `to_abbreviated_string` we
/// mostly land where nuon already is. See [`to_preview_json`] for the same
/// argument applied to the visualizer payload.
pub(crate) fn short_render(value: &Value, ctx: RenderCtx<'_>) -> String {
    let config = ctx.config;
    match value {
        Value::String { val, .. } => {
            // chars(), not byte slicing: a byte index can land mid-codepoint
            // and panic, which inside a debugger callback hangs the session.
            let n_chars = val.chars().count();
            if n_chars > MAX_ROW_CHARS {
                let head: String = val.chars().take(MAX_ROW_CHARS - 3).collect();
                format!("\"{head}…\" ({n_chars} chars)")
            } else {
                format!("\"{val}\"")
            }
        }
        Value::Nothing { .. } => "null".into(),
        Value::Date { val, .. } => val.to_rfc3339(),
        // A table's rows have no useful one-line preview: keep the shape.
        Value::List { .. } if matches!(value.get_type(), Type::Table(_)) => {
            value.to_abbreviated_string(config)
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
                // Too wide to preview: `[list N items]`, from the shell.
                value.to_abbreviated_string(config)
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
                // Ditto: `{record N fields}`.
                value.to_abbreviated_string(config)
            } else {
                preview
            }
        }
        Value::Closure { val, .. } => closure_label(val, ctx.cache),
        Value::Binary { val, .. } => {
            use std::fmt::Write;
            // First bytes as hex, nu-literal style: 0x[de ad be ef …] (N bytes).
            // `to nuon` writes the same `0x[…]` form (unspaced, uppercase, and
            // unbounded); the spacing and byte count are what a row needs.
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
        // Upstream renders errors as `{error:?}` — the whole `ShellError`
        // Debug, multi-line. One line of the message is what fits a row.
        Value::Error { error, .. } => cap_row(&format!("<error: {error}>")),
        // Everything else (int, float, bool, filesize, duration, range, glob,
        // cell-path, custom) renders the way the shell renders it. Capped
        // because a custom value's base form can expand without bound.
        other => cap_row(&other.to_abbreviated_string(config)),
    }
}

/// A closure as `{|x| $x * 2}`, plus the number of variables it captured from
/// the enclosing scope — which is usually what you want to know about a
/// closure you are stopped inside.
///
/// The literal comes from [`collect_closure_labels`]; a block id that isn't in
/// the map (nothing adds blocks after the parse today, but a stale snapshot
/// could) degrades to the bare `<closure>` this used to always show.
fn closure_label(val: &nu_protocol::engine::Closure, cache: &RenderCache) -> String {
    let body = cache
        .closure_src
        .get(&val.block_id.get())
        .cloned()
        .unwrap_or_else(|| "<closure>".to_string());
    match val.captures.len() {
        0 => body,
        1 => format!("{body} +1 capture"),
        n => format!("{body} +{n} captures"),
    }
}

/// Truncate a rendered row to [`MAX_ROW_CHARS`].
fn cap_row(s: &str) -> String {
    cap(s, MAX_ROW_CHARS)
}

/// Truncate to `max` chars on a char boundary — a byte index can land
/// mid-codepoint and panic, and a panic inside a debugger callback hangs the
/// session.
fn cap(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max - 1).collect();
    format!("{head}…")
}

/// Describe a stream without consuming it (draining would eat the
/// program's data): kind, origin, and size when known.
///
/// Wording follows `describe --no-collect` so the debugger and the shell name
/// the same stream the same way. `describe`'s own code can't be called here:
/// it takes `PipelineData` by value (it drains, or calls `into_debug_value`),
/// while a paused debugger only ever borrows the register it is looking at.
pub(crate) fn describe_stream(data: &nu_protocol::PipelineData) -> String {
    use nu_protocol::{ByteStreamSource, PipelineData};
    match data {
        PipelineData::ByteStream(bs, _) => {
            // Same call `describe` makes: "binary (stream)" / "string (stream)"
            // / "byte stream".
            let kind = bs.type_().describe();
            // Origin names match the `origin` field of `describe --detailed`.
            let origin = match bs.source() {
                ByteStreamSource::Read(_) => "unknown",
                ByteStreamSource::File(_) => "file",
                ByteStreamSource::Child(_) => "external",
            };
            // Size is ours: `describe` never reports it, but it costs nothing.
            match bs.known_size() {
                Some(n) => format!("<{kind} from {origin}, {n} bytes>"),
                None => format!("<{kind} from {origin}>"),
            }
        }
        // `describe --detailed` calls this a "list stream".
        PipelineData::ListStream(..) => "<list stream>".to_string(),
        _ => "<stream>".to_string(),
    }
}

/// One-token rendering used inside list/record previews. Lossier than a full
/// row — several of these share one 60-char line — but never *blank*: a field
/// that renders to nothing reads as a missing value rather than an empty one.
pub(crate) fn scalar_preview(value: &Value) -> String {
    match value {
        // Bare, unlike a row: quotes on every element would eat the line. The
        // empty string is the exception, or `{name: , size: 120}`.
        Value::String { val, .. } if val.is_empty() => "\"\"".into(),
        Value::String { val, .. } => {
            let mut s: String = val.chars().take(12).collect();
            if val.chars().count() > 12 {
                s.push('…');
            }
            s
        }
        Value::Int { val, .. } => val.to_string(),
        // `ObviousFloat`, as the shell does it: `1.0`, not `f64`'s bare `1`.
        Value::Float { val, .. } => nu_utils::ObviousFloat(*val).to_string(),
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
    // Containers are always expandable — children materialize lazily. So is a
    // closure that captured something: its captures are its children, which is
    // the only way to see the values it closed over.
    let expandable = match value {
        Value::List { .. } | Value::Record { .. } => true,
        Value::Closure { val, .. } => !val.captures.is_empty(),
        _ => false,
    };

    let var_ref = if expandable { snapshot.alloc_ref() } else { 0 };

    // Cheap `Arc` bumps: rendering needs these while `snapshot` is borrowed
    // mutably below.
    let config = snapshot.config.clone();
    let cache = snapshot.cache.clone();
    let ctx = RenderCtx {
        config: &config,
        cache: &cache,
    };

    let node_idx = snapshot.var_arena.len();
    snapshot.var_arena.push(VarNode {
        var: Variable {
            name,
            value: short_render(value, ctx),
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
    let cache = snapshot.cache.clone();
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
        // A closure's children are the variables it closed over, shown under
        // their source names so the row reads like the enclosing scope did.
        Value::Closure { val, .. } => {
            for (var_id, v) in val.captures.iter().take(MAX_CHILDREN) {
                let name = cache
                    .var_names
                    .get(&var_id.get())
                    .cloned()
                    .unwrap_or_else(|| format!("var{}", var_id.get()));
                children.push(add_value(snapshot, name, v, depth + 1));
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
    config: std::sync::Arc<Config>,
    cache: std::sync::Arc<RenderCache>,
) -> crate::state::PauseSnapshot {
    use crate::state::PauseSnapshot;
    let mut snap = PauseSnapshot::new();
    snap.frames = entry.frames.clone();
    // Both were cached by the eval thread; rendering needs them, and
    // `engine_state` is still off-limits here.
    snap.config = config;
    snap.cache = cache;

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
///
/// This is a *display projection*, not a serialization, which is why it does
/// not reuse nu-json's `FromValue for nu_json::Value` (the conversion behind
/// `to json`). A debugger inspects hostile values — huge, deeply nested, or
/// broken — and must always render something:
///
/// - **Bounded.** The shared path recurses without limit; a 10M-row table or a
///   deeply nested record would stall the webview. We cap depth and item count
///   and report `truncated` so the UI can say so.
/// - **Error-tolerant.** The shared path returns `Err` for `Value::Error`,
///   which would yield *no* payload for a structure containing an error — the
///   very thing you are usually inspecting. We render `<error: …>` inline.
/// - **Binary as hex.** The shared path expands binary to an array of numbers
///   (1 MiB becomes ~4 MB of JSON over stdio). We emit the `$nuBinary` marker
///   the webview turns into a hex view, capped at 64 KiB.
/// - **Closures need no engine.** The shared path either errors on closures or
///   requires an `&EngineState` to coerce them; here `<closure>` is enough.
///
/// Types with no arm of their own fall through to `to_abbreviated_string`, the
/// same source [`short_render`] uses, so a value reads identically whether you
/// glance at its row or open it in the visualizer.
pub(crate) fn to_preview_json(
    value: &Value,
    depth: usize,
    truncated: &mut bool,
    ctx: RenderCtx<'_>,
) -> serde_json::Value {
    let config = ctx.config;
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
        // Config-driven, like the row: `Filesize`'s own `Display` ignores the
        // user's `filesize` settings and reads `1 kB` where the row says
        // `1.0 kB`.
        Value::Filesize { val, .. } => json!(config.filesize.format(*val).to_string()),
        Value::Duration { val, .. } => json!(format!("{val}ns")),
        Value::Date { val, .. } => json!(val.to_rfc3339()),
        Value::List { vals, .. } => {
            if vals.len() > JSON_MAX_ITEMS {
                *truncated = true;
            }
            J::Array(
                vals.iter()
                    .take(JSON_MAX_ITEMS)
                    .map(|v| to_preview_json(v, depth + 1, truncated, ctx))
                    .collect(),
            )
        }
        Value::Record { val, .. } => {
            if val.len() > JSON_MAX_ITEMS {
                *truncated = true;
            }
            let mut map = serde_json::Map::new();
            for (k, v) in val.iter().take(JSON_MAX_ITEMS) {
                map.insert(k.clone(), to_preview_json(v, depth + 1, truncated, ctx));
            }
            J::Object(map)
        }
        Value::Closure { val, .. } => json!(closure_label(val, ctx.cache)),
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
        // Ranges, globs, cell-paths, custom values: the shell's own rendering,
        // matching the Variables row. Was `format!("{other:?}")`, which sent
        // `Value`'s Rust `Debug` (`Range(1..10)`) to the webview.
        other => json!(cap_row(&other.to_abbreviated_string(config))),
    }
}
