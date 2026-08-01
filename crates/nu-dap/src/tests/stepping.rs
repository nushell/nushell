//! Unit tests for [`crate::debugger::stepping`].
//!
//! These run *in process*, unlike the end-to-end suite in `tests/dap.rs`, which
//! spawns the `nu-dap` binary as a child — so a breakpoint set here is actually
//! hit by the test runner.

use crate::dap::protocol::DapWriter;
use crate::debugger::DapDebugger;
use crate::debugger::stepping::var_name;
use crate::source_map::SourcePos;
use crate::state::{DebugState, RunMode};
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{Span, Type, Value, VarId};
use pretty_assertions::assert_eq;
use rstest::rstest;
use std::sync::Arc;

/// An `EngineState` holding `source` as a file, plus a variable whose
/// declaration span covers the whole of it but which is registered in **no**
/// overlay — the shape of a parameter or block local, whose scope frame the
/// parser pops before the delta is merged. Forces `var_name` down its
/// span-parsing fallback and pins the exact bytes that path sees.
fn engine_with_unscoped_declaration(source: &str) -> (EngineState, VarId) {
    let mut engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let file_id = working_set.add_file("test.nu", source.as_bytes());
    let span = working_set.get_span_for_file(file_id);
    let var_id = working_set.add_variable_without_scope(span, Type::Any, false);
    engine_state
        .merge_delta(working_set.render())
        .expect("merge delta");
    (engine_state, var_id)
}

/// A variable registered in the active overlay under `name`, the way a
/// top-level `let` is. Its declaration span deliberately points at misleading
/// source text, so a passing assertion proves the name came from the scope
/// rather than from parsing that text.
fn engine_with_scoped_variable(name: &str) -> (EngineState, VarId) {
    let mut engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let file_id = working_set.add_file("test.nu", b"wrong: answer");
    let span = working_set.get_span_for_file(file_id);
    let var_id = working_set.add_variable(name.as_bytes().to_vec(), span, Type::Any, false);
    engine_state
        .merge_delta(working_set.render())
        .expect("merge delta");
    (engine_state, var_id)
}

/// The scope is the authoritative source: when an overlay maps a name to this
/// id, that name wins outright and the declaration span is never consulted.
/// Overlay keys carry the `$` sigil, which is not part of the displayed name.
#[rstest]
#[case::plain("files")]
#[case::underscored("my_total")]
#[case::numeric_suffix("x2")]
fn var_name_prefers_the_name_the_scope_records(#[case] name: &str) {
    let (engine_state, var_id) = engine_with_scoped_variable(name);
    assert_eq!(var_name(&engine_state, var_id), name);
}

/// The sigil is stripped whether or not the caller supplied it — the scope
/// normalises keys to `$name`, and Locals shows the bare identifier.
#[test]
fn var_name_strips_the_sigil_from_a_scope_entry() {
    let (engine_state, var_id) = engine_with_scoped_variable("$files");
    assert_eq!(var_name(&engine_state, var_id), "files");
}

/// Parameters and block locals never reach the scope, so their name still
/// comes from the declaration span — which is source text, not an identifier:
/// it can carry the `$` sigil, a type annotation, a flag's dashes, or a
/// default value. All of that is trimmed to the bare name shown in Locals.
#[rstest]
#[case::sigil("$x", "x")]
#[case::bare("x", "x")]
#[case::type_annotation("size: int", "size")]
#[case::annotation_without_space("size:int", "size")]
#[case::long_flag("--verbose", "verbose")]
#[case::flag_with_type_and_default("--tag: string = \"dev\"", "tag")]
#[case::short_flag_alias("--tag (-t)", "tag")]
#[case::surrounding_whitespace("  $count  ", "count")]
#[case::tab_separated("count\tint", "count")]
fn var_name_trims_a_declaration_to_its_identifier(#[case] source: &str, #[case] expected: &str) {
    let (engine_state, var_id) = engine_with_unscoped_declaration(source);
    assert_eq!(var_name(&engine_state, var_id), expected);
}

/// Locals entries need *some* label, so a declaration that trims away to
/// nothing falls back to the id rather than rendering a nameless variable.
#[rstest]
#[case::sigil_only("$")]
#[case::dashes_only("--")]
#[case::annotation_only(": int")]
fn var_name_falls_back_to_the_id_when_nothing_is_left(#[case] source: &str) {
    let (engine_state, var_id) = engine_with_unscoped_declaration(source);
    assert_eq!(
        var_name(&engine_state, var_id),
        format!("var{}", var_id.get())
    );
}

/// An unknown span yields no bytes at all — same fallback, no panic.
#[test]
fn var_name_survives_an_unknown_declaration_span() {
    let mut engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let var_id = working_set.add_variable_without_scope(Span::unknown(), Type::Any, false);
    engine_state
        .merge_delta(working_set.render())
        .expect("merge delta");
    assert_eq!(
        var_name(&engine_state, var_id),
        format!("var{}", var_id.get())
    );
}

/// A debugger with an empty frame stack (depth 0) writing its DAP output to a
/// sink, alongside the shared state it snapshots into.
fn debugger() -> (DapDebugger, Arc<DebugState>) {
    let state = Arc::new(DebugState::new(false, false, 1));
    let writer = DapWriter::new(Box::new(std::io::sink()));
    (DapDebugger::new(Arc::clone(&state), writer), state)
}

fn pos(line: u64) -> SourcePos {
    SourcePos {
        path: "test.nu".to_string(),
        line,
        column: 1,
    }
}

/// The stepping decision, at frame depth 0. `is_call` marks a pipe-stage
/// boundary: step-into stops there even on the same line, so F11 walks a
/// pipeline stage by stage, while step-over and step-out ignore it.
#[rstest]
// Continue never pauses, whatever the position.
#[case::continue_ignores_everything(RunMode::Continue, 7, false, None)]
#[case::continue_ignores_a_call(RunMode::Continue, 7, true, None)]
// An explicit pause request stops at the very next instruction.
#[case::pause_now(RunMode::PauseNow, 7, false, Some("entry"))]
// Step-in: a new line, a depth change, or a call boundary all stop it.
#[case::step_in_same_line(RunMode::StepIn { depth: 0, line: 7 }, 7, false, None)]
#[case::step_in_new_line(RunMode::StepIn { depth: 0, line: 7 }, 8, false, Some("step"))]
#[case::step_in_depth_change(RunMode::StepIn { depth: 1, line: 7 }, 7, false, Some("step"))]
#[case::step_in_call_on_same_line(RunMode::StepIn { depth: 0, line: 7 }, 7, true, Some("step"))]
// Step-over: a new line at the same depth, or returning to a shallower one.
#[case::step_over_same_line(RunMode::StepOver { depth: 0, line: 7 }, 7, false, None)]
#[case::step_over_new_line(RunMode::StepOver { depth: 0, line: 7 }, 8, false, Some("step"))]
#[case::step_over_ignores_a_call(RunMode::StepOver { depth: 0, line: 7 }, 7, true, None)]
#[case::step_over_returned_shallower(RunMode::StepOver { depth: 1, line: 7 }, 7, false, Some("step"))]
#[case::step_over_stays_deeper(RunMode::StepOver { depth: 0, line: 8 }, 8, false, None)]
// Step-out only stops once the frame stack is genuinely shallower.
#[case::step_out_same_depth(RunMode::StepOut { depth: 0 }, 7, false, None)]
#[case::step_out_ignores_a_call(RunMode::StepOut { depth: 0 }, 7, true, None)]
#[case::step_out_returned_shallower(RunMode::StepOut { depth: 1 }, 7, false, Some("step"))]
fn should_pause_mode_decides_by_line_depth_and_call_boundary(
    #[case] run_mode: RunMode,
    #[case] line: u64,
    #[case] is_call: bool,
    #[case] expected: Option<&'static str>,
) {
    let (debugger, _state) = debugger();
    assert_eq!(
        debugger.should_pause_mode(&pos(line), run_mode, is_call),
        expected
    );
}

/// The pause snapshot mirrors the real `Stack`, named from source. `$nu` and
/// `$env` are skipped: they are reserved specials rendered in the Globals
/// scope, so repeating them in Locals would be noise.
#[test]
fn sync_locals_from_stack_names_vars_and_skips_reserved_specials() {
    let mut engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let file_id = working_set.add_file("test.nu", b"size: int");
    let span = working_set.get_span_for_file(file_id);
    let size = working_set.add_variable(b"size".to_vec(), span, Type::Int, false);
    engine_state
        .merge_delta(working_set.render())
        .expect("merge delta");

    let mut stack = Stack::new();
    stack.add_var(size, Value::test_int(120));
    stack.add_var(nu_protocol::NU_VARIABLE_ID, Value::test_nothing());
    stack.add_var(nu_protocol::ENV_VARIABLE_ID, Value::test_nothing());
    stack.add_var(nu_protocol::IN_VARIABLE_ID, Value::test_nothing());

    let (debugger, state) = debugger();
    debugger.sync_locals_from_stack(&engine_state, &stack);

    let session = state.session_state.lock().expect("session poisoned");
    let names: Vec<&str> = session
        .shadow_vars
        .values()
        .map(|sv| sv.name.as_str())
        .collect();
    assert_eq!(names, vec!["size"], "only the user's own variable");
    assert_eq!(session.shadow_vars[&size.get()].value, Value::test_int(120));
}
