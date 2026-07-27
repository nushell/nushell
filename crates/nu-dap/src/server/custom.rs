//! Adapter-specific requests used by the VS Code extension:
//! nuDapVisualize (full value as JSON) and nuDapUiReply (answer to a prompt).

use super::Session;
use crate::dap::protocol::Request;
use crate::dap::types::VisualizeArgs;
use serde_json::{Value as Json, json};

impl Session {
    // "Visualize" action: return the full value as JSON (unlike the bounded
    // Variables tree). Expandable nodes are keyed by variablesReference;
    // leaves (strings, binaries) by containerReference + name.
    pub(super) fn on_visualize(&mut self, seq: i64, cmd: &str, req: Request) {
        let args: VisualizeArgs = match serde_json::from_value(req.arguments) {
            Ok(a) => a,
            Err(e) => {
                self.writer
                    .respond_error(seq, cmd, format!("bad args: {e}"));
                return;
            }
        };
        let found = self
            .with_state(|inner| {
                let snap = inner.active_snapshot();
                let node = match (args.variables_reference, args.container_reference) {
                    (Some(r), _) if r > 0 => snap
                        .var_arena
                        .iter()
                        .find(|n| n.var.variables_reference == r),
                    (_, Some(cr)) => snap.var_refs.get(&cr).and_then(|children| {
                        children
                            .iter()
                            .map(|&i| &snap.var_arena[i])
                            .find(|n| Some(&n.var.name) == args.name.as_ref())
                    }),
                    _ => None,
                };
                node.map(|n| {
                    let mut truncated = false;
                    let json = crate::variables::to_preview_json(&n.value, 0, &mut truncated);
                    (json, n.value.get_type().to_string(), truncated)
                })
            })
            .flatten();
        match found {
            Some((value, type_, truncated)) => self.writer.respond(
                seq,
                cmd,
                json!({ "value": value, "type": type_, "truncated": truncated }),
            ),
            None => self.writer.respond_error(
                seq,
                cmd,
                "no value for that reference (stale after resume?)",
            ),
        }
    }

    // Answer to a `nuDapUi` prompt event: hand it to the eval
    // thread blocked inside the input shim.
    pub(super) fn on_ui_reply(&mut self, seq: i64, cmd: &str, req: Request) {
        let id = req.arguments.get("id").and_then(|v| v.as_u64());
        if let (Some(id), Some(state)) = (id, &self.state) {
            let reply = crate::state::UiReply {
                cancelled: req
                    .arguments
                    .get("cancelled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                index: req
                    .arguments
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize),
                indices: req
                    .arguments
                    .get("indices")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_u64())
                            .map(|x| x as usize)
                            .collect()
                    }),
                value: req
                    .arguments
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            };
            state
                .ui
                .replies
                .lock()
                .expect("ui bridge poisoned")
                .insert(id, reply);
            state.ui.cv.notify_all();
        }
        self.writer.respond(seq, cmd, Json::Null);
    }
}
