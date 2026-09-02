//! Breakpoint handlers: setBreakpoints, setExceptionBreakpoints, exceptionInfo.

use super::Session;
use crate::dap::protocol::Request;
use crate::dap::types::{
    Breakpoint, BreakpointLocation, BreakpointLocationsArgs, BreakpointLocationsResponse,
    ExceptionInfoResponse, SetBreakpointsArgs, SetBreakpointsResponse,
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
            let mut session = state.session_state.lock();
            let mut map = std::collections::BTreeMap::new();
            for bp in &args.breakpoints {
                let id = session.next_bp_id;
                session.next_bp_id += 1;
                // Snap onto a real instruction position: forward to the next
                // line with instructions for a gutter breakpoint, onto one
                // column of the line for an inline one (optimistic before
                // parsing; the eval thread reconciles and re-announces then).
                let (pos, ok) = session.snap(file, bp.line, bp.column);
                // One breakpoint per position: a second one snapping onto a
                // taken position could never fire, so report it unverified at
                // the requested spot instead of dropping it silently.
                if map.contains_key(&pos) {
                    verified.push(Breakpoint {
                        id: Some(id),
                        verified: false,
                        line: bp.line,
                        column: bp.column,
                        source: Some(args.source.clone()),
                        message: Some(format!("another breakpoint already covers {pos}")),
                    });
                    continue;
                }
                map.insert(
                    pos,
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
                    line: pos.line,
                    column: pos.column,
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

    /// Which positions on a line can carry a breakpoint. The client uses this
    /// to snap an inline breakpoint (Shift+F9) onto a real instruction and to
    /// decide whether to offer one at all — a line with a single position gets
    /// no inline marker, which is the right answer for the ordinary case.
    ///
    /// Answers nothing before the parse, when no positions are known yet;
    /// clients re-ask as the user interacts.
    pub(super) fn on_breakpoint_locations(&mut self, seq: i64, cmd: &str, req: Request) {
        let args: BreakpointLocationsArgs = match serde_json::from_value(req.arguments) {
            Ok(a) => a,
            Err(e) => {
                self.writer
                    .respond_error(seq, cmd, format!("bad args: {e}"));
                return;
            }
        };

        // Interned before the session lock, as in `on_set_breakpoints`.
        let file = args.source.path.as_deref().map(|p| self.files.intern(p));
        let end_line = args.end_line.unwrap_or(args.line);

        let breakpoints = file
            .and_then(|file| {
                self.with_state(|session| session.positions_in(file, args.line, end_line))
            })
            .unwrap_or_default()
            .into_iter()
            .map(|(line, column)| BreakpointLocation { line, column })
            .collect();

        self.writer
            .respond(seq, cmd, BreakpointLocationsResponse { breakpoints });
    }

    pub(super) fn on_set_exception_breakpoints(&mut self, seq: i64, cmd: &str, req: Request) {
        let filters: Vec<String> = req
            .arguments
            .get("filters")
            .and_then(|f| serde_json::from_value(f.clone()).ok())
            .unwrap_or_default();
        if let Some(state) = &self.state {
            let mut session = state.session_state.lock();
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
