//! Stepping decisions and the per-pause snapshot of this frame's locals and
//! environment, read from the real evaluation `Stack`.

use super::DapDebugger;
use crate::state::{RunMode, ShadowVar};
use nu_protocol::engine::{EngineState, Stack};

/// Resolve a variable's source name. Nushell stores no name on a `VarId`, but
/// the variable's declaration span points at the identifier in source. Trims
/// the leading `$` and any type annotation (`x: int` → `x`).
fn var_name(engine_state: &EngineState, var_id: nu_protocol::VarId) -> String {
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

impl DapDebugger {
    /// Decide whether the current run mode wants to pause at this position
    /// (breakpoints are handled separately in `enter_instruction`).
    /// `is_call` marks call instructions — pipe-stage boundaries — where
    /// step-into also stops, so F11 walks a builtin pipeline stage by stage.
    pub(super) fn should_pause_mode(
        &self,
        pos: Option<&crate::source_map::SourcePos>,
        run_mode: RunMode,
        is_call: bool,
    ) -> Option<&'static str> {
        match run_mode {
            RunMode::Continue => None,
            RunMode::PauseNow => Some("entry"),
            RunMode::StepIn { depth, line } => match pos {
                Some(p) if p.line != line || self.depth() != depth || is_call => Some("step"),
                _ => None,
            },
            RunMode::StepOver { depth, line } => match pos {
                Some(_) if self.depth() < depth => Some("step"),
                Some(p) if self.depth() == depth && p.line != line => Some("step"),
                _ => None,
            },
            RunMode::StepOut { depth } => {
                if self.depth() < depth {
                    Some("step")
                } else {
                    None
                }
            }
        }
    }

    /// Snapshot the current frame's locals and environment from the real
    /// evaluation `Stack` (nushell #18708) into shared state. Called at every
    /// steppable line we might pause on, evaluate a condition/logpoint at, or
    /// record for time-travel — so the pause snapshot, scratch eval, and the
    /// tape all read genuine values (params, closure captures, mutations)
    /// rather than the old IR reconstruction. `$in` is the one exception:
    /// register-based, injected from the frame's captured value.
    pub(super) fn sync_locals_from_stack(&self, engine_state: &EngineState, stack: &Stack) {
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
        let mut inner = self.state.inner.lock().expect("debug state poisoned");
        inner.shadow_vars = vars;
        inner.env_shadow = env;
    }
}
