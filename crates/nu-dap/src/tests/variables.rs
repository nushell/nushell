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

use crate::state::RenderCache;
use crate::variables::{JSON_MAX_ITEMS, RenderCtx, scalar_preview, short_render, to_preview_json};
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
        closure_src: std::collections::HashMap::from([(CLOSURE_BLOCK, "{|x| $x * 2}".to_string())]),
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
fn eval_to_stream(script: &str) -> (nu_protocol::engine::EngineState, nu_protocol::PipelineData) {
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
        let block = nu_parser::parse(&mut working_set, Some("test.nu"), script.as_bytes(), false);
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
