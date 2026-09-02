//! Stepping decisions and the per-pause snapshot of this frame's locals and
//! environment, read from the real evaluation `Stack`.

use super::DapDebugger;
use crate::state::{RunMode, ShadowVar};
use nu_protocol::engine::{EngineState, Stack};
use std::sync::Arc;

impl DapDebugger {
    /// Whether the run mode wants to pause here (breakpoints are handled in
    /// `enter_instruction`). `is_call` marks pipe-stage boundaries where
    /// step-into also stops, so F11 walks a builtin pipeline stage by stage.
    pub(crate) fn should_pause_mode(
        &self,
        pos: &crate::source_map::SourcePos,
        run_mode: RunMode,
        is_call: bool,
    ) -> Option<&'static str> {
        match run_mode {
            RunMode::Continue => None,
            RunMode::PauseNow => Some("entry"),
            RunMode::StepIn { depth, line } => {
                if pos.line != line || self.depth() != depth || is_call {
                    Some("step")
                } else {
                    None
                }
            }
            RunMode::StepOver { depth, line } => {
                if self.depth() < depth || (self.depth() == depth && pos.line != line) {
                    Some("step")
                } else {
                    None
                }
            }
            RunMode::StepOut { depth } => {
                if self.depth() < depth {
                    Some("step")
                } else {
                    None
                }
            }
        }
    }

    /// Snapshot this frame's locals + env from the real `Stack` (#18708) into
    /// shared state, so the pause snapshot, scratch eval and time-travel tape
    /// read genuine values. `$in` is register-based, so it is injected from
    /// the frame's captured value instead.
    pub(crate) fn sync_locals_from_stack(&self, engine_state: &EngineState, stack: &Stack) {
        let mut vars: std::collections::HashMap<usize, ShadowVar> =
            std::collections::HashMap::new();
        for (var_id, value) in &stack.vars {
            // Reserved specials: $nu/$env live in Globals, $in is injected
            // below.
            if *var_id == nu_protocol::NU_VARIABLE_ID
                || *var_id == nu_protocol::ENV_VARIABLE_ID
                || *var_id == nu_protocol::IN_VARIABLE_ID
            {
                continue;
            }
            vars.insert(
                var_id.get(),
                ShadowVar {
                    name: var_name(engine_state, *var_id),
                    value: value.clone(),
                },
            );
        }
        if let Some(v) = self.frames.last().and_then(|f| f.in_value.clone()) {
            vars.insert(
                nu_protocol::IN_VARIABLE_ID.get(),
                ShadowVar {
                    name: "in".to_string(),
                    value: v,
                },
            );
        }
        // Full runtime env (engine baseline + this stack's overlays/mutations).
        let env = stack.get_env_vars(engine_state);

        let mut session = self.state.session_state.lock();
        session.shadow_vars = vars;
        // Only swap the `Arc` when the environment actually changed, so the
        // tape entries recorded in between all share one copy of it.
        if *session.env_shadow != env {
            session.env_shadow = Arc::new(env);
        }
    }
}

/// Resolve a variable's source name. Nushell stores no name on the `Variable`
/// itself, so two sources are asked in order of authority:
///
/// 1. **The scope.** Overlays map `$name` → `VarId` — nushell's own answer,
///    covering top-level bindings and anything a `use`d module brought in.
/// 2. **The declaration span.** Parameters, closure params and block locals
///    never reach `engine_state.scope` (the parser pops their scope frame
///    before `merge_delta`), but their span points at the identifier in
///    source, so the sigil, type annotation and flag dashes are trimmed off
///    (`--tag: string = "dev"` → `tag`). A heuristic, hence second.
pub(crate) fn var_name(engine_state: &EngineState, var_id: nu_protocol::VarId) -> String {
    if let Some(name) = scope_var_name(engine_state, var_id) {
        return name;
    }

    let var = engine_state.get_var(var_id);
    let bytes = engine_state.get_span_contents(var.declaration_span);
    let s = String::from_utf8_lossy(bytes);

    // `$x` / `x: int` / `--verbose` / `--tag: string` → `x` / `verbose` / `tag`.
    let s = s.trim().trim_start_matches('$');
    let s = s.split([':', ' ', '\t']).next().unwrap_or(s);
    let s = s.trim_start_matches('-');

    if s.is_empty() {
        format!("var{}", var_id.get())
    } else {
        s.to_string()
    }
}

/// Reverse `name` → `VarId` lookup over the active overlays, newest first —
/// the order `ScopeFrame::get_var` itself resolves in, so when two overlays
/// bind the same id we report the one nushell would pick. Scope keys carry
/// the `$` sigil.
fn scope_var_name(engine_state: &EngineState, var_id: nu_protocol::VarId) -> Option<String> {
    let scope = &engine_state.scope;
    scope
        .active_overlays
        .iter()
        .rev()
        .filter_map(|overlay_id| scope.overlays.get(overlay_id.get()))
        .find_map(|(_, overlay)| {
            overlay
                .vars
                .iter()
                .find(|(_, id)| **id == var_id)
                .map(|(name, _)| {
                    String::from_utf8_lossy(name)
                        .trim_start_matches('$')
                        .to_string()
                })
        })
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    //! In-process, unlike the end-to-end suite in `tests/dap.rs`, which
    //! spawns the binary as a child — so a breakpoint set here is really hit
    //! by the test runner.

    use super::var_name;
    use crate::dap::protocol::DapWriter;
    use crate::debugger::DapDebugger;
    use crate::source_map::SourcePos;
    use crate::state::{DebugState, RunMode};
    use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
    use nu_protocol::{Span, Type, Value, VarId};
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use std::sync::Arc;

    /// An `EngineState` holding `source` as a file, plus a variable whose
    /// declaration span covers all of it but is in **no** overlay — the shape
    /// of a parameter or block local. Forces `var_name` down its span-parsing
    /// fallback and pins the exact bytes that path sees.
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

    /// A variable in the active overlay under `name`, the way a top-level
    /// `let` is. Its declaration span points at misleading source text, so a
    /// pass proves the name came from the scope and not from that text.
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

    /// The scope is authoritative: when an overlay maps a name to this id,
    /// that name wins outright and the declaration span is never consulted.
    #[rstest]
    #[case::plain("files")]
    #[case::underscored("my_total")]
    #[case::numeric_suffix("x2")]
    fn var_name_prefers_the_name_the_scope_records(#[case] name: &str) {
        let (engine_state, var_id) = engine_with_scoped_variable(name);
        assert_eq!(var_name(&engine_state, var_id), name);
    }

    /// The sigil is stripped whether or not the caller supplied it: the scope
    /// normalises keys to `$name`, Locals shows the bare identifier.
    #[test]
    fn var_name_strips_the_sigil_from_a_scope_entry() {
        let (engine_state, var_id) = engine_with_scoped_variable("$files");
        assert_eq!(var_name(&engine_state, var_id), "files");
    }

    /// Parameters and block locals are named from the declaration span, which
    /// is source text rather than an identifier: sigil, type annotation, flag
    /// dashes and default value all get trimmed away.
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
    fn var_name_trims_a_declaration_to_its_identifier(
        #[case] source: &str,
        #[case] expected: &str,
    ) {
        let (engine_state, var_id) = engine_with_unscoped_declaration(source);
        assert_eq!(var_name(&engine_state, var_id), expected);
    }

    /// Locals entries need *some* label, so a declaration that trims away to
    /// nothing falls back to the id.
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

    /// A debugger at depth 0 writing DAP output to a sink, plus the shared
    /// state it snapshots into.
    fn debugger() -> (DapDebugger, Arc<DebugState>) {
        let state = Arc::new(DebugState::new(
            false,
            false,
            1,
            crate::file_table::FileTable::default(),
            crate::state::ClientCoords::default(),
        ));
        let writer = DapWriter::new(Box::new(std::io::sink()));
        (DapDebugger::new(Arc::clone(&state), writer), state)
    }

    fn pos(line: u64) -> SourcePos {
        SourcePos {
            file: crate::file_table::FileTable::default().intern("test.nu"),
            line,
            column: 1,
        }
    }

    /// The stepping decision at frame depth 0. `is_call` marks a pipe-stage
    /// boundary: step-into stops there even on the same line, step-over and
    /// step-out ignore it.
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

    /// The pause snapshot mirrors the real `Stack`, named from source. `$nu`
    /// and `$env` are skipped — Globals renders those.
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

        let session = state.session_state.lock();
        let names: Vec<&str> = session
            .shadow_vars
            .values()
            .map(|sv| sv.name.as_str())
            .collect();
        assert_eq!(names, vec!["size"], "only the user's own variable");
        assert_eq!(session.shadow_vars[&size.get()].value, Value::test_int(120));
    }
}
