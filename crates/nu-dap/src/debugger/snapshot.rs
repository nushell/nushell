//! Pause-snapshot construction: the resolved stack-frame list and the scope
//! tree (Locals / Pipeline / Registers / Process / Globals).

use super::DapDebugger;
use crate::dap::types::{Source, StackFrame};
use crate::state::{PauseSnapshot, ShadowVar};
use crate::variables::add_value;
use nu_protocol::engine::EngineState;
use nu_protocol::ir::Instruction;
use nu_protocol::{PipelineData, Span};

impl DapDebugger {
    /// Stack frames, innermost first (DAP order), resolved to file/line.
    /// Shared by the live snapshot and time-travel recording.
    pub(super) fn build_frames(&self) -> Vec<StackFrame> {
        let mut frames = Vec::new();
        for (i, frame) in self.frames.iter().rev().enumerate() {
            // Innermost frame reports the current instruction position;
            // callers report their call site.
            let span = if i == 0 {
                self.current_span.or(frame.at).or(frame.span)
            } else {
                frame.at.or(frame.span)
            };
            let pos = span.and_then(|s| self.source_map.resolve(s));
            frames.push(StackFrame {
                id: i as i64,
                name: frame.name.clone(),
                source: pos.as_ref().map(|p| Source {
                    name: std::path::Path::new(&p.path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string()),
                    path: Some(p.path.clone()),
                }),
                line: pos.as_ref().map(|p| p.line as i64).unwrap_or(0),
                column: pos.as_ref().map(|p| p.column as i64).unwrap_or(0),
            });
        }
        frames
    }

    /// Build the pause snapshot: stack frames + Locals/Pipeline/Globals/…
    pub(super) fn build_snapshot(
        &self,
        engine_state: &EngineState,
        session: &mut crate::state::SessionState,
        site: &super::Site<'_>,
    ) {
        let mut snap = PauseSnapshot::new();
        snap.frames = self.build_frames();
        // Values render with nushell's own formatting, which is config-driven.
        snap.config = engine_state.get_config().clone();
        snap.cache = self
            .state
            .cache
            .lock()
            .expect("render cache poisoned")
            .clone();
        session.config = snap.config.clone();

        // Cache $nu + baseline env once, so the server can rebuild historical
        // Globals for time-travel without touching engine_state.
        if session.nu_constant.is_none() {
            session.nu_constant = engine_state
                .get_constant(nu_protocol::NU_VARIABLE_ID)
                .cloned();
            session.baseline_env = Some(
                engine_state
                    .render_env_vars()
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
            );
        }

        // Locals scope. `return` (the most recent expression/command result)
        // first, then shadow vars sorted by name for a stable UI.
        let mut locals_children = Vec::new();
        if let Some(v) = &self.last_result {
            locals_children.push(add_value(&mut snap, "return".to_string(), v, 0));
        }
        let mut vars: Vec<&ShadowVar> = session.shadow_vars.values().collect();
        vars.sort_by(|a, b| a.name.cmp(&b.name));
        for sv in vars {
            locals_children.push(add_value(&mut snap, sv.name.clone(), &sv.value, 0));
        }
        snap.var_refs
            .insert(PauseSnapshot::LOCALS_REF, locals_children);

        // Pipeline scope: at a call (pipe-stage boundary), the value flowing
        // INTO that command — nu's `$in` for the next stage. Raw registers below.
        let mut pipeline_children = Vec::new();
        if let Instruction::Call {
            decl_id, src_dst, ..
        } = site.instruction()
            && let Some(reg) = site.registers.get(src_dst.get() as usize)
        {
            let name = format!("in → {}", engine_state.get_decl(*decl_id).name());
            match &reg.body {
                PipelineData::Value(v, _) if !matches!(v, nu_protocol::Value::Nothing { .. }) => {
                    pipeline_children.push(add_value(&mut snap, name, v, 0));
                }
                other @ (PipelineData::ListStream(..) | PipelineData::ByteStream(..)) => {
                    // Streams can't be inspected without draining them,
                    // but kind/origin/size are known without reading.
                    let v = nu_protocol::Value::string(
                        crate::variables::describe_stream(other),
                        Span::unknown(),
                    );
                    pipeline_children.push(add_value(&mut snap, name, &v, 0));
                }
                _ => {}
            }
        }
        snap.var_refs
            .insert(PauseSnapshot::PIPELINE_REF, pipeline_children);

        // Registers scope: the evaluator's raw working slots, for reading
        // alongside the IR panel.
        let mut register_children = Vec::new();
        for (i, reg) in site.registers.iter().enumerate() {
            if let PipelineData::Value(v, _) = &reg.body
                && !matches!(v, nu_protocol::Value::Nothing { .. })
            {
                register_children.push(add_value(&mut snap, format!("%{i}"), v, 0));
            }
        }
        snap.var_refs
            .insert(PauseSnapshot::REGISTERS_REF, register_children);

        // Process scope: rolling tails of what externals/drains wrote to the
        // captured process stdout/stderr.
        let mut process_children = Vec::new();
        for (label, category) in [("last output", "stdout"), ("last error output", "stderr")] {
            let tail = crate::stdio::recent_output(category);
            if !tail.is_empty() {
                let v = nu_protocol::Value::string(tail, Span::unknown());
                process_children.push(add_value(&mut snap, label.to_string(), &v, 0));
            }
        }
        snap.var_refs
            .insert(PauseSnapshot::PROCESS_REF, process_children);

        // Globals scope: `$nu` (config/paths/pid/os-info) and `$env` (the full
        // runtime env snapshotted from the Stack as `env_shadow`).
        let mut globals_children = Vec::new();
        if let Some(nu) = engine_state.get_constant(nu_protocol::NU_VARIABLE_ID) {
            let v = nu.clone();
            globals_children.push(add_value(&mut snap, "$nu".to_string(), &v, 0));
        }
        {
            let env_map: std::collections::BTreeMap<&String, &nu_protocol::Value> =
                session.env_shadow.iter().collect();
            let mut rec = nu_protocol::Record::new();
            for (k, v) in env_map {
                rec.push(k.clone(), v.clone());
            }
            let env_val = nu_protocol::Value::record(rec, Span::unknown());
            globals_children.push(add_value(&mut snap, "$env".to_string(), &env_val, 0));
        }
        snap.var_refs
            .insert(PauseSnapshot::GLOBALS_REF, globals_children);

        session.snapshot = snap;
    }
}
