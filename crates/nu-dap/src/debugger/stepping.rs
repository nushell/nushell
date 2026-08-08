//! Stepping decisions and the per-pause snapshot of this frame's locals and
//! environment, read from the real evaluation `Stack`.

use super::DapDebugger;
use crate::state::{RunMode, ShadowVar};
use nu_protocol::engine::{EngineState, Stack};

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
    /// shared state, so the pause snapshot, scratch eval, and time-travel tape
    /// read genuine values (params, closure captures, mutations). `$in` is the
    /// exception: register-based, injected from the frame's captured value.
    pub(crate) fn sync_locals_from_stack(&self, engine_state: &EngineState, stack: &Stack) {
        let mut vars: std::collections::HashMap<usize, ShadowVar> =
            std::collections::HashMap::new();
        for (var_id, value) in &stack.vars {
            // Nushell's reserved specials: $nu/$env live in Globals, $in is
            // injected below from register 0.
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

        let mut session = self.state.session_state.lock().expect("session poisoned");
        session.shadow_vars = vars;
        session.env_shadow = env;
    }
}

/// Resolve a variable's source name. Nushell stores no name on the `Variable`
/// itself, so this asks two sources in order of authority:
///
/// 1. **The scope.** Overlays map `$name` → `VarId`, which is nushell's own
///    answer and needs no guessing. It covers the script's top-level bindings
///    plus anything a `use`d module brought in.
/// 2. **The declaration span.** Parameters, closure params, and block locals
///    never reach `engine_state.scope`: the parser declares them inside a
///    scope frame it pops again, and `merge_delta` keeps only the outermost
///    frame. Their span does point at the identifier in source, so trim the
///    sigil, type annotation, and flag dashes off it (`--tag: string = "dev"`
///    → `tag`). This is a heuristic, hence second.
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
/// the same order `ScopeFrame::get_var` resolves a name in, so when two
/// overlays bind the same id we report the one nushell itself would pick.
/// Scope keys carry the `$` sigil (`insert_variable_into_scope` prepends it).
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
