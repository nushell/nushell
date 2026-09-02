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
    pub config: &'a Config,
    pub cache: &'a RenderCache,
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

/// Describe a stream without consuming it — draining would eat the program's
/// own data, and pulling even one element would run upstream closures from
/// inside a debugger callback, re-entering the evaluator while it holds the
/// `EngineState.debugger` mutex. So everything here is *static*: kind, origin,
/// size, the command that produced it, and whatever the pipeline metadata
/// already carries.
///
/// The element type of a list stream is deliberately absent: nothing short of
/// pulling an element can know it.
///
/// Base wording follows `describe --no-collect`, so the debugger and the shell
/// name the same stream the same way. `describe`'s own code can't be called
/// here: it takes `PipelineData` by value (it drains, or calls
/// `into_debug_value`), while a paused debugger only ever borrows the register
/// it is looking at.
pub(crate) fn describe_stream(
    data: &nu_protocol::PipelineData,
    engine_state: &EngineState,
) -> String {
    use nu_protocol::{ByteStreamSource, PipelineData};
    let (kind, origin, size, span, meta) = match data {
        PipelineData::ByteStream(bs, meta) => {
            // Same call `describe` makes: "binary (stream)" / "string (stream)"
            // / "byte stream".
            let kind = bs.type_().describe().to_string();
            // Origin names match the `origin` field of `describe --detailed`.
            let origin = match bs.source() {
                ByteStreamSource::Read(_) => "unknown",
                ByteStreamSource::File(_) => "file",
                ByteStreamSource::Child(_) => "external",
            };
            // Size is ours: `describe` never reports it, but it costs nothing.
            (kind, Some(origin), bs.known_size(), Some(bs.span()), meta)
        }
        // `describe --detailed` calls this a "list stream".
        PipelineData::ListStream(ls, meta) => {
            ("list stream".to_string(), None, None, Some(ls.span()), meta)
        }
        _ => ("stream".to_string(), None, None, None, &None),
    };

    // The stream's span is the command that produced it (`each`, `where`,
    // `open`) — the difference between "a list stream" and *which* list stream
    // when several are in flight. It subsumes the origin: `open` means a file,
    // `^cmd` means an external, so only fall back to the origin without it.
    let producer = span.and_then(|s| command_word(engine_state, s));
    let mut parts = match (&producer, origin) {
        (Some(p), _) => vec![format!("{kind} from `{p}`")],
        (None, Some(o)) => vec![format!("{kind} from {o}")],
        (None, None) => vec![kind],
    };

    if let Some(meta) = meta {
        // Set by `open`: the file the bytes are coming from.
        if let nu_protocol::DataSource::FilePath(path) = &meta.data_source {
            parts.push(
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string()),
            );
        }
        // Sniffed by `open` from the extension: `text/x-toml`, `application/json`.
        if let Some(ct) = &meta.content_type {
            parts.push(ct.clone());
        }
    }

    if let Some(n) = size {
        parts.push(format!("{n} bytes"));
    }

    format!("<{}>", parts.join(", "))
}

/// The producing command's name from a span, when the span actually points at
/// one.
///
/// A stream's span is not always a command: `"a-b" | split row "-" | get 0`
/// leaves the list stream pointing at the *literal* `"a-b"`, and `from `"a-b"``
/// is worse than saying nothing. So this accepts only what reads as a command
/// — an identifier, optionally `^`-prefixed for an external — and gives up
/// otherwise.
fn command_word(engine_state: &EngineState, span: nu_protocol::Span) -> Option<String> {
    let raw = String::from_utf8_lossy(engine_state.get_span_contents(span));
    let word = raw.split_whitespace().next()?;
    let name = word.strip_prefix('^').unwrap_or(word);
    let looks_like_a_command = name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '\\'));
    looks_like_a_command.then(|| word.to_string())
}

/// One-token rendering used inside list/record previews. Lossier than a full
/// row — several of these share one 60-char line — but never *blank*: a field
/// that renders to nothing reads as a missing value rather than an empty one.
fn scalar_preview(value: &Value) -> String {
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
const JSON_MAX_ITEMS: usize = 1000;
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

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::variables`].
    //!
    //! The three `*_renders_every_variant` tables below are the rendering
    //! contract: one case per `Value` variant, for each of the three functions
    //! that turn a value into something a DAP client displays. They exist because
    //! nushell has several general-purpose renderers and none of them fits a
    //! debugger row — see the rationale on [`crate::variables::short_render`] and
    //! [`crate::variables::to_preview_json`]. Read as a table, they show what each
    //! variant looks like in the pane and where we deliberately differ from the
    //! shell.
    //!
    //! Everything else in this file tests behaviour that is not per-variant:
    //! truncation, container previews, and the truncated-flag plumbing.

    use super::{JSON_MAX_ITEMS, RenderCtx, scalar_preview, short_render, to_preview_json};
    use crate::state::RenderCache;
    use chrono::{DateTime, FixedOffset};
    use nu_protocol::ast::{CellPath, PathMember, RangeInclusion};
    use nu_protocol::casing::Casing;
    use nu_protocol::engine::Closure;
    use nu_protocol::{BlockId, Config, CustomValue, ShellError, Span, Value, record};
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    /// Rendering is config-driven (it defers to `Value::to_abbreviated_string`);
    /// defaults keep these assertions independent of the user's `$env.config`.
    /// No closure labels, so a closure falls back to `<closure>` — the cases that
    /// care supply their own map.
    fn render(value: &Value) -> String {
        let config = Config::default();
        let cache = RenderCache::default();
        short_render(
            value,
            RenderCtx {
                config: &config,
                cache: &cache,
            },
        )
    }

    /// As `render`, with a label registered for [`CLOSURE_BLOCK`].
    fn render_with_labels(value: &Value) -> String {
        let config = Config::default();
        let cache = cache();
        short_render(
            value,
            RenderCtx {
                config: &config,
                cache: &cache,
            },
        )
    }

    fn preview_json(value: &Value) -> serde_json::Value {
        preview_json_flagged(value).0
    }

    /// The payload plus whether any bound was hit.
    fn preview_json_flagged(value: &Value) -> (serde_json::Value, bool) {
        let config = Config::default();
        let cache = cache();
        let mut truncated = false;
        let json = to_preview_json(
            value,
            0,
            &mut truncated,
            RenderCtx {
                config: &config,
                cache: &cache,
            },
        );
        (json, truncated)
    }

    /// Block id used by the closure fixtures below.
    const CLOSURE_BLOCK: usize = 7;

    /// Stands in for what `collect_render_cache` reads out of the engine.
    fn cache() -> RenderCache {
        RenderCache {
            closure_src: std::collections::HashMap::from([(
                CLOSURE_BLOCK,
                "{|x| $x * 2}".to_string(),
            )]),
            var_names: std::collections::HashMap::from([(100, "n".to_string())]),
        }
    }

    // --- one constructor per awkward variant -------------------------------
    // The rest come straight from `Value::test_*`.

    /// Minimal `CustomValue`; nu-protocol ships no public test double
    /// (`Value::test_values` skips the variant for exactly this reason).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct StubCustom(i64);

    #[typetag::serde(name = "nu_dap::tests::StubCustom")]
    impl CustomValue for StubCustom {
        fn clone_value(&self, span: Span) -> Value {
            Value::custom(Box::new(self.clone()), span)
        }
        fn type_name(&self) -> String {
            "StubCustom".into()
        }
        fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
            Ok(Value::int(self.0, span))
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    fn custom() -> Value {
        Value::test_custom_value(Box::new(StubCustom(5)))
    }

    /// Fixed instant, never `now()`: the rendered row is asserted literally.
    fn date() -> Value {
        let d: DateTime<FixedOffset> =
            DateTime::parse_from_rfc3339("2026-08-01T12:34:56+02:00").expect("valid rfc3339");
        Value::test_date(d)
    }

    fn range() -> Value {
        let r = nu_protocol::Range::new(
            Value::test_int(1),
            Value::test_int(2),
            Value::test_int(10),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");
        Value::test_range(r)
    }

    fn closure() -> Value {
        Value::test_closure(Closure {
            block_id: BlockId::new(CLOSURE_BLOCK),
            captures: Vec::new(),
        })
    }

    /// Same block, but capturing two variables from the enclosing scope.
    fn closure_with_captures(n: usize) -> Value {
        Value::test_closure(Closure {
            block_id: BlockId::new(CLOSURE_BLOCK),
            captures: (0..n)
                .map(|i| (nu_protocol::VarId::new(100 + i), Value::test_int(i as i64)))
                .collect(),
        })
    }

    fn error() -> Value {
        Value::error(
            ShellError::DivisionByZero {
                span: Span::test_data(),
            },
            Span::test_data(),
        )
    }

    fn cell_path() -> Value {
        Value::test_cell_path(CellPath {
            members: vec![
                PathMember::test_string("name", false, Casing::Sensitive),
                PathMember::test_int(1, false),
            ],
        })
    }

    fn file_record() -> Value {
        Value::test_record(record! {
            "name" => Value::test_string("a.txt"),
            "size" => Value::test_int(120),
        })
    }

    // --- the contract: every variant, every renderer -----------------------

    /// The Variables-pane row for each `Value` variant.
    ///
    /// Most types read as nushell writes them, via `to_abbreviated_string`. The
    /// six deliberate departures are marked: they are what a debugger needs and a
    /// pipeline does not.
    #[rstest]
    #[case::bool(Value::test_bool(true), "true")]
    #[case::int(Value::test_int(42), "42")]
    #[case::float(Value::test_float(1.0), "1.0")]
    // Quoted, so an empty string is visible as a row and can't be confused with a
    // glob or a bare identifier.
    #[case::string(Value::test_string("hi"), "\"hi\"")]
    // Unquoted, which is how the shell writes a glob — the contrast with `String`
    // above is the only cue distinguishing them in the pane.
    #[case::glob(Value::test_glob("*.nu"), "*.nu")]
    #[case::filesize(Value::test_filesize(1000), "1.0 kB")]
    #[case::duration(Value::test_duration(260_000_000_000), "4min 20sec")]
    // DEPARTURE: rfc3339, not the shell's `human_time_from_now` ("5 hours ago"),
    // which is relative to the wall clock — wrong information in a debugger, and
    // untestable here.
    #[case::date(date(), "2026-08-01T12:34:56+02:00")]
    // Was `Range(1..10)` — `Value`'s `Debug` — before this went through
    // `to_abbreviated_string`.
    #[case::range(range(), "1..10")]
    // DEPARTURE: a preview of the first fields, where `to_abbreviated_string`
    // always collapses to `{record N fields}`. The collapsed row is the one you
    // scan, so it carries data.
    #[case::record(file_record(), "{name: a.txt, size: 120}")]
    #[case::list(Value::test_list(vec![Value::test_int(1), Value::test_int(2)]), "[1, 2]")]
    // A table's rows have no useful one-line preview: keep the shape.
    #[case::table(Value::test_list(vec![file_record(), file_record()]), "[table 2 rows]")]
    // DEPARTURE: the shell writes `closure_7`; the block id means nothing to
    // someone debugging their own script. With no label resolved for the block
    // this is the floor — `closure_source_and_captures` covers the real case.
    #[case::closure(closure(), "<closure>")]
    // DEPARTURE: one line. The shell renders errors as `{error:?}` — the whole
    // multi-line `ShellError` `Debug`.
    #[case::error(error(), "<error: Division by zero.>")]
    // DEPARTURE: a nu literal, not `[222, 173]`.
    #[case::binary(Value::test_binary(vec![0xde, 0xad]), "0x[de ad] (2 bytes)")]
    #[case::cell_path(cell_path(), "$.name.1")]
    // Collapsed through `to_base_value`; `<StubCustom>` would show only on failure.
    #[case::custom(custom(), "5")]
    // DEPARTURE: the shell renders Nothing as `""` — a blank row.
    #[case::nothing(Value::test_nothing(), "null")]
    #[nu_test_support::test]
    #[env(NU_TEST_LOCALE_OVERRIDE = "en_US.utf8")]
    fn short_render_renders_every_variant(#[case] value: Value, #[case] expected: &str) {
        assert_eq!(render(&value), expected);
    }

    /// The one-token form used *inside* a container preview — deliberately
    /// lossier than a full row, and unquoted, because several of these share a
    /// single 60-char line.
    ///
    /// Everything without a cheap scalar form collapses to `…`. That is most
    /// variants; a record of dates previews as `{when: …}`. Pinned as the current
    /// contract, not as an endorsement.
    #[rstest]
    #[case::bool(Value::test_bool(true), "true")]
    #[case::int(Value::test_int(42), "42")]
    #[case::float(Value::test_float(1.0), "1.0")]
    // Unquoted here, unlike a full row — quotes on every element would eat the
    // 60-char line.
    #[case::string(Value::test_string("hi"), "hi")]
    // Except when empty, which would otherwise read `{name: , size: 120}`.
    #[case::empty_string_stays_visible(Value::test_string(""), "\"\"")]
    #[case::nothing(Value::test_nothing(), "null")]
    // Containers show their size only.
    #[case::record(file_record(), "{2}")]
    #[case::list(Value::test_list(vec![Value::test_int(1), Value::test_int(2)]), "[2]")]
    #[case::table(Value::test_list(vec![file_record(), file_record()]), "[2]")]
    // The long tail, all indistinguishable at this size.
    #[case::glob(Value::test_glob("*.nu"), "…")]
    #[case::filesize(Value::test_filesize(1000), "…")]
    #[case::duration(Value::test_duration(260_000_000_000), "…")]
    #[case::date(date(), "…")]
    #[case::range(range(), "…")]
    #[case::closure(closure(), "…")]
    #[case::error(error(), "…")]
    #[case::binary(Value::test_binary(vec![0xde, 0xad]), "…")]
    #[case::cell_path(cell_path(), "…")]
    #[case::custom(custom(), "…")]
    fn scalar_preview_renders_every_variant(#[case] value: Value, #[case] expected: &str) {
        assert_eq!(scalar_preview(&value), expected);
    }

    /// The `nuDapVisualize` payload for each variant — a display projection, not a
    /// serialization, which is why it does not reuse nu-json's `FromValue` (that
    /// path errors on `Value::Error` and expands binary to an integer array).
    ///
    /// Every variant reads the same here as in its Variables row (compare with
    /// `short_render_renders_every_variant`), because the fall-through arm shares
    /// `to_abbreviated_string` with it. The four `matches_the_row` cases below are
    /// the ones that used to emit Rust `Debug` — `Range(1..10)` and friends.
    #[rstest]
    #[case::bool(Value::test_bool(true), json!(true))]
    #[case::int(Value::test_int(42), json!(42))]
    #[case::float(Value::test_float(1.0), json!(1.0))]
    #[case::string(Value::test_string("hi"), json!("hi"))]
    #[case::nothing(Value::test_nothing(), json!(null))]
    #[case::filesize(Value::test_filesize(1000), json!("1.0 kB"))]
    #[case::duration(Value::test_duration(260_000_000_000), json!("260000000000ns"))]
    #[case::date(date(), json!("2026-08-01T12:34:56+02:00"))]
    #[case::record(file_record(), json!({"name": "a.txt", "size": 120}))]
    #[case::list(
        Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
        json!([1, 2])
    )]
    #[case::table(
        Value::test_list(vec![file_record()]),
        json!([{"name": "a.txt", "size": 120}])
    )]
    // `preview_json` supplies labels, so this is the resolved form.
    #[case::closure(closure(), json!("{|x| $x * 2}"))]
    // Rendered inline rather than returned as `Err`: a structure containing an
    // error is usually the thing you are inspecting.
    #[case::error(error(), json!("<error: Division by zero.>"))]
    // Marker object the webview turns into a hex view.
    #[case::binary(Value::test_binary(vec![0xde, 0xad]), json!({"$nuBinary": "dead", "length": 2}))]
    // The fall-through arm. Each of these was the Rust `Debug` form until
    // `to_preview_json` started sharing `to_abbreviated_string` with the row.
    #[case::glob_matches_the_row(Value::test_glob("*.nu"), json!("*.nu"))]
    #[case::range_matches_the_row(range(), json!("1..10"))]
    #[case::cell_path_matches_the_row(cell_path(), json!("$.name.1"))]
    #[case::custom_matches_the_row(custom(), json!("5"))]
    #[nu_test_support::test]
    #[env(NU_TEST_LOCALE_OVERRIDE = "en_US.utf8")]
    fn to_preview_json_renders_every_variant(
        #[case] value: Value,
        #[case] expected: serde_json::Value,
    ) {
        assert_eq!(preview_json(&value), expected);
    }

    /// Tripwire for a variant added upstream: `Value::test_values` yields one of
    /// each, so a new one shows up here without anyone remembering to extend the
    /// tables above. It omits `Custom` and `Glob`, and every value it builds is
    /// empty or zero, so it proves only "renders, doesn't panic" — the tables
    /// above carry the actual expectations.
    #[test]
    fn every_variant_renders_without_panicking() {
        for value in Value::test_values() {
            // A row is what the client displays: never blank, whatever the value.
            // (`scalar_preview` gets no such guarantee — see the empty-string case
            // in `scalar_preview_renders_every_variant`.)
            let row = render(&value);
            assert!(!row.is_empty(), "empty row for {:?}", value.get_type());
            scalar_preview(&value);
            preview_json(&value);
        }
    }

    // --- closures ----------------------------------------------------------

    /// A closure row shows the literal the user wrote plus how many variables it
    /// closed over — `<closure>` alone said nothing about *which* closure.
    ///
    /// The source text can't be derived from the `Value`: it comes from the
    /// block's span, resolved on the eval thread by `collect_closure_labels` and
    /// carried in the snapshot, because the server thread has no `EngineState`.
    #[rstest]
    #[case::no_captures(closure(), "{|x| $x * 2}")]
    #[case::one_capture(closure_with_captures(1), "{|x| $x * 2} +1 capture")]
    #[case::several_captures(closure_with_captures(3), "{|x| $x * 2} +3 captures")]
    fn closure_source_and_captures(#[case] value: Value, #[case] expected: &str) {
        assert_eq!(render_with_labels(&value), expected);
    }

    /// An unresolvable block id degrades to what the row always used to show,
    /// rather than rendering something misleading or empty.
    #[test]
    fn closure_without_a_label_falls_back() {
        let unknown = Value::test_closure(Closure {
            block_id: BlockId::new(CLOSURE_BLOCK + 1),
            captures: Vec::new(),
        });
        assert_eq!(render_with_labels(&unknown), "<closure>");
    }

    /// The capture count still shows when the body doesn't resolve — it comes off
    /// the `Value`, not the engine.
    #[test]
    fn closure_captures_show_without_a_label() {
        let unknown = Value::test_closure(Closure {
            block_id: BlockId::new(CLOSURE_BLOCK + 1),
            captures: vec![(nu_protocol::VarId::new(1), Value::test_int(1))],
        });
        assert_eq!(render_with_labels(&unknown), "<closure> +1 capture");
    }

    /// The count in the row is a summary; the values themselves are the closure's
    /// children, so expanding it shows what it closed over under the source names.
    #[test]
    fn closure_captures_are_children() {
        let mut snap = crate::state::PauseSnapshot::new();
        snap.cache = std::sync::Arc::new(cache());
        let captured = Value::test_closure(Closure {
            block_id: BlockId::new(CLOSURE_BLOCK),
            captures: vec![(nu_protocol::VarId::new(100), Value::test_int(10))],
        });

        let idx = crate::variables::add_value(&mut snap, "scaled".into(), &captured, 0);
        let node = &snap.var_arena[idx];
        assert_ne!(
            node.var.variables_reference, 0,
            "a capturing closure is expandable"
        );

        let children: Vec<(String, String)> = node
            .children
            .iter()
            .map(|&i| {
                (
                    snap.var_arena[i].var.name.clone(),
                    snap.var_arena[i].var.value.clone(),
                )
            })
            .collect();
        assert_eq!(children, vec![("n".to_string(), "10".to_string())]);
    }

    /// Nothing to expand when nothing was captured — no empty twisty in the pane.
    #[test]
    fn closure_without_captures_is_a_leaf() {
        let mut snap = crate::state::PauseSnapshot::new();
        snap.cache = std::sync::Arc::new(cache());
        let idx = crate::variables::add_value(&mut snap, "double".into(), &closure(), 0);
        assert_eq!(snap.var_arena[idx].var.variables_reference, 0);
    }

    /// Closures nest like anything else: inside a record the row still previews.
    #[test]
    fn closure_inside_a_record_previews() {
        let v = Value::test_record(record! {
            "f" => closure(),
            "n" => Value::test_int(1),
        });
        // `scalar_preview` has no cheap form for a closure, so it collapses to `…`
        // the way every other exotic type does.
        assert_eq!(render_with_labels(&v), "{f: …, n: 1}");
    }

    // --- streams -----------------------------------------------------------

    /// A stream is `PipelineData`, not a `Value`, so it has no row in the tables
    /// above — but the Pipeline scope shows one whenever a stage hands one on.
    ///
    /// Everything reported is static. Draining would eat the program's data, and
    /// pulling even one element would re-enter the evaluator from inside a
    /// debugger callback, so the element type is knowingly absent.
    #[rstest]
    // The span names the producing command, which is what tells two live streams
    // apart.
    #[case::command("[1 2 3] | each {|x| $x * 2}", "<list stream from `each`>")]
    #[case::another_command("[1 2 3] | where {|x| $x > 2}", "<list stream from `where`>")]
    // A stream's span is not always a command: here it lands on the *literal*
    // feeding the pipeline, and `from `"a-b"`` would be worse than saying nothing.
    #[case::span_is_a_literal("\"a-b\" | split row \"-\"", "<list stream>")]
    fn describe_stream_names_the_producer(#[case] script: &str, #[case] expected: &str) {
        let (engine_state, data) = eval_to_stream(script);
        assert_eq!(
            crate::variables::describe_stream(&data, &engine_state),
            expected
        );
    }

    /// `open` records the file and sniffs a content type, so a byte stream can say
    /// what it is without a single byte being read.
    #[test]
    fn describe_stream_reports_file_and_content_type() {
        let (engine_state, data) = eval_to_stream("open --raw Cargo.toml");
        assert_eq!(
            crate::variables::describe_stream(&data, &engine_state),
            "<byte stream from `open`, Cargo.toml, text/x-toml>"
        );
    }

    /// Run `script` far enough to get its pipeline data, without draining it.
    fn eval_to_stream(
        script: &str,
    ) -> (nu_protocol::engine::EngineState, nu_protocol::PipelineData) {
        use nu_protocol::debugger::WithoutDebug;
        use nu_protocol::engine::{Stack, StateWorkingSet};

        let mut engine_state = nu_cmd_lang::create_default_context();
        engine_state = nu_command::add_shell_command_context(engine_state);
        engine_state.add_env_var(
            "PWD".to_string(),
            Value::string(
                std::env::current_dir().expect("cwd").to_string_lossy(),
                Span::unknown(),
            ),
        );

        let block = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let block =
                nu_parser::parse(&mut working_set, Some("test.nu"), script.as_bytes(), false);

            assert!(
                working_set.parse_errors.is_empty(),
                "parse: {:?}",
                working_set.parse_errors
            );

            let delta = working_set.render();

            engine_state.merge_delta(delta).expect("merge");

            block
        };

        let mut stack = Stack::new();
        let data = nu_engine::eval_block::<WithoutDebug>(
            &engine_state,
            &mut stack,
            &block,
            nu_protocol::PipelineData::empty(),
        )
        .expect("eval");

        (engine_state, data.body)
    }

    // --- behaviour that is not per-variant ---------------------------------

    #[test]
    fn short_render_list_previews_elements() {
        // More than three elements: preview the first three then an ellipsis.
        let long = Value::test_list((0..50).map(Value::test_int).collect());
        assert_eq!(render(&long), "[0, 1, 2, …]");
    }

    #[test]
    fn containers_too_wide_to_preview_fall_back_to_the_shell_shape() {
        // `to_abbreviated_string` pluralizes, so a single row reads "1 row".
        let row = Value::test_record(record! { "a" => Value::test_int(1) });
        assert_eq!(
            render(&Value::test_list(vec![row.clone()])),
            "[table 1 row]"
        );

        // Lists and records wider than one row collapse to the shape. (Elements
        // are capped at 12 chars by `scalar_preview`, so it takes very long
        // numbers to push a list preview past the limit.)
        let wide = Value::test_list(
            (0..9)
                .map(|n| Value::test_int(100_000_000_000_000_000 + n))
                .collect(),
        );
        assert_eq!(render(&wide), "[list 9 items]");
        let wide_rec = Value::test_record(record! {
            "first-long-field" => Value::test_string("aaaaaaaaaaaaaaaa"),
            "second-long-field" => Value::test_string("bbbbbbbbbbbbbbbb"),
            "third-long-field" => Value::test_string("cccccccccccccccc"),
        });
        assert_eq!(render(&wide_rec), "{record 3 fields}");
        assert_eq!(render(&row), "{a: 1}");
    }

    /// Where `scalar_preview`'s rules actually reach the user: nested in a row.
    /// A float reads `1.0` rather than `1`, and an empty string is visible instead
    /// of leaving `note: ` dangling.
    #[test]
    fn record_preview_shows_floats_and_empty_strings() {
        let v = Value::test_record(record! {
            "ratio" => Value::test_float(1.0),
            "note" => Value::test_string(""),
        });
        assert_eq!(render(&v), "{ratio: 1.0, note: \"\"}");
    }

    #[test]
    fn short_render_binary_shows_the_first_eight_bytes() {
        let many = Value::test_binary((0u8..12).collect::<Vec<_>>());
        assert_eq!(render(&many), "0x[00 01 02 03 04 05 06 07 …] (12 bytes)");
    }

    #[test]
    fn short_render_caps_a_long_row() {
        let long = Value::test_string("x".repeat(500));
        let rendered = render(&long);
        assert!(rendered.ends_with("…\" (500 chars)"), "{rendered}");
        assert!(rendered.chars().count() < 140, "{rendered}");
    }

    #[rstest]
    #[case::short_string_is_whole(Value::test_string("short"), "short")]
    #[case::long_string_is_elided(Value::test_string("abcdefghijklmnop"), "abcdefghijkl…")]
    fn scalar_preview_caps_strings(#[case] value: Value, #[case] expected: &str) {
        assert_eq!(scalar_preview(&value), expected);
    }

    #[test]
    fn to_json_truncates_large_collections() {
        let big = Value::test_list((0..2000).map(Value::test_int).collect());
        let (json, truncated) = preview_json_flagged(&big);
        assert!(truncated, "the truncated flag is set");
        assert_eq!(json.as_array().unwrap().len(), JSON_MAX_ITEMS);
    }

    #[test]
    fn to_json_leaves_small_values_untruncated() {
        let (_, truncated) = preview_json_flagged(&file_record());
        assert!(!truncated, "small values are not truncated");
    }
}
