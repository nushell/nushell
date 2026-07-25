//! Forward execution control: continue, next/stepIn, stepOut, pause.
//! When scrubbing recorded history these navigate the tape; only at the live
//! frontier do they resume the eval thread.

use super::Session;
use crate::state::RunMode;
use serde_json::{Value as Json, json};

impl Session {
    pub(super) fn on_continue(&mut self, seq: i64, cmd: &str) {
        // In the past: forward to the next recorded breakpoint, else to the
        // frontier. Only at the frontier do we resume real execution.
        if let Some(Some(cur)) = self.with_state(|i| i.view_index) {
            let target = self
                .with_state(|inner| {
                    let front = inner.frontier().unwrap_or(cur);
                    (cur + 1..front).find(|&j| {
                        inner
                            .timeline
                            .get(j)
                            .map(|e| e.is_breakpoint)
                            .unwrap_or(false)
                    })
                })
                .flatten();
            match target {
                Some(t) => self.tt_goto(Some(t), "breakpoint"),
                None => self.tt_goto(None, "step"), // back at the live frontier
            }
        } else {
            self.resume(RunMode::Continue);
        }
        self.writer
            .respond(seq, cmd, json!({ "allThreadsContinued": true }));
    }

    pub(super) fn on_next_or_step_in(&mut self, seq: i64, cmd: &str) {
        if let Some(Some(cur)) = self.with_state(|i| i.view_index) {
            // Forward one line through recorded history.
            let front = self.with_state(|i| i.frontier()).flatten().unwrap_or(cur);
            if cur + 1 >= front {
                self.tt_goto(None, "step"); // reached the live frontier
            } else {
                self.tt_goto(Some(cur + 1), "step");
            }
        } else {
            let mode = if cmd == "next" {
                self.with_state(|inner| RunMode::StepOver {
                    depth: inner.paused_depth,
                    line: inner.paused_line,
                })
            } else {
                self.with_state(|inner| RunMode::StepIn {
                    depth: inner.paused_depth,
                    line: inner.paused_line,
                })
            }
            .unwrap_or(RunMode::Continue);
            self.resume(mode);
        }
        self.writer.respond(seq, cmd, Json::Null);
    }

    pub(super) fn on_step_out(&mut self, seq: i64, cmd: &str) {
        if let Some(Some(cur)) = self.with_state(|i| i.view_index) {
            let target = self.with_state(|inner| {
                let front = inner.frontier().unwrap_or(cur);
                let cur_depth = inner.timeline.get(cur).map(|e| e.depth).unwrap_or(0);
                let j = (cur + 1..=front).find(|&j| {
                    inner
                        .timeline
                        .get(j)
                        .map(|e| e.depth < cur_depth)
                        .unwrap_or(false)
                });
                (j, front)
            });
            match target {
                Some((Some(j), front)) if j < front => self.tt_goto(Some(j), "step"),
                _ => self.tt_goto(None, "step"),
            }
        } else {
            let mode = self
                .with_state(|inner| RunMode::StepOut {
                    depth: inner.paused_depth,
                })
                .unwrap_or(RunMode::Continue);
            self.resume(mode);
        }
        self.writer.respond(seq, cmd, Json::Null);
    }

    pub(super) fn on_pause(&mut self, seq: i64, cmd: &str) {
        if let Some(state) = &self.state {
            let mut inner = state.inner.lock().expect("state poisoned");
            inner.run_mode = RunMode::PauseNow;
        }
        self.writer.respond(seq, cmd, Json::Null);
    }
}
