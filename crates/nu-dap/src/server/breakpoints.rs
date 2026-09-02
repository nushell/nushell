//! Breakpoint handlers: setBreakpoints, setExceptionBreakpoints, exceptionInfo.

use super::Session;
use crate::dap::protocol::Request;
use crate::dap::types::{
    Breakpoint, ExceptionInfoResponse, SetBreakpointsArgs, SetBreakpointsResponse,
};

impl Session {
    pub(super) fn on_set_breakpoints(&mut self, seq: i64, cmd: &str, req: Request) {
        let args: SetBreakpointsArgs = match serde_json::from_value(req.arguments) {
            Ok(a) => a,
            Err(e) => {
                self.writer
                    .respond_error(seq, cmd, format!("bad args: {e}"));
                return;
            }
        };

        // Intern before taking the session lock: the table has its own lock
        // and must never be entered while holding this one.
        let file = args.source.path.as_deref().map(|p| self.files.intern(p));

        let mut verified = Vec::new();
        if let (Some(state), Some(file)) = (&self.state, file) {
            let mut session = state.session_state.lock().expect("session poisoned");
            let mut map = std::collections::BTreeMap::new();
            for bp in &args.breakpoints {
                let id = session.next_bp_id;
                session.next_bp_id += 1;
                // Snap to the next line with instructions (optimistic before
                // parsing; the eval thread reconciles + re-announces then).
                let (snapped, ok) = session.snap_line(file, bp.line);
                // At most one breakpoint per steppable line: a second request
                // snapping onto a taken line cannot ever fire, so report it
                // unverified at the requested line rather than silently
                // dropping it or overwriting the winner.
                let line = snapped;
                if map.contains_key(&line) {
                    verified.push(Breakpoint {
                        id: Some(id),
                        verified: false,
                        line: bp.line,
                        source: Some(args.source.clone()),
                        message: Some(format!("another breakpoint already covers line {line}")),
                    });
                    continue;
                }
                map.insert(
                    line,
                    crate::state::Breakpoint {
                        id,
                        verified: ok,
                        condition: bp.condition.clone(),
                        log_message: bp.log_message.clone(),
                    },
                );
                verified.push(Breakpoint {
                    id: Some(id),
                    verified: ok,
                    line,
                    source: Some(args.source.clone()),
                    message: None,
                });
            }
            session.breakpoints.insert(file, map);
        }
        self.writer.respond(
            seq,
            cmd,
            SetBreakpointsResponse {
                breakpoints: verified,
            },
        );
    }

    pub(super) fn on_set_exception_breakpoints(&mut self, seq: i64, cmd: &str, req: Request) {
        let filters: Vec<String> = req
            .arguments
            .get("filters")
            .and_then(|f| serde_json::from_value(f.clone()).ok())
            .unwrap_or_default();
        if let Some(state) = &self.state {
            let mut session = state.session_state.lock().expect("session poisoned");
            session.break_on_error = filters.iter().any(|f| f == "error");
        }
        self.writer.respond(
            seq,
            cmd,
            SetBreakpointsResponse {
                breakpoints: Vec::new(),
            },
        );
    }

    pub(super) fn on_exception_info(&mut self, seq: i64, cmd: &str) {
        let info = self
            .with_state(|session| session.exception_info.clone())
            .flatten();
        match info {
            Some((id, description)) => self.writer.respond(
                seq,
                cmd,
                ExceptionInfoResponse {
                    exception_id: id,
                    description,
                    break_mode: "always",
                },
            ),
            None => self
                .writer
                .respond_error(seq, cmd, "not paused on an exception"),
        }
    }
}
