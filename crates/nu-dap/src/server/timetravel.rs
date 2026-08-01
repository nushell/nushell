//! Backward navigation over the recorded tape: stepBack, reverseContinue,
//! and the shared `tt_goto` cursor move. None of these resume the eval
//! thread — they move the cursor and re-serve rebuilt history.

use super::{Session, THREAD_ID};
use serde_json::{Value as Json, json};

impl Session {
    pub(super) fn on_step_back(&mut self, seq: i64, cmd: &str) {
        let target = self.with_state(|session| {
            session
                .view_index
                .or_else(|| session.frontier())
                .map(|c| c.saturating_sub(1))
        });
        if let Some(Some(t)) = target {
            self.tt_goto(Some(t), "step");
        }
        self.writer.respond(seq, cmd, Json::Null);
    }

    pub(super) fn on_reverse_continue(&mut self, seq: i64, cmd: &str) {
        let target = self
            .with_state(|session| {
                let cur = session.view_index.or_else(|| session.frontier())?;
                let prev_bp = (0..cur).rev().find(|&j| {
                    session
                        .timeline
                        .get(j)
                        .map(|e| e.is_breakpoint)
                        .unwrap_or(false)
                });
                Some(prev_bp.unwrap_or(0))
            })
            .flatten();
        if let Some(t) = target {
            let reason = self
                .with_state(|i| i.timeline.get(t).map(|e| e.is_breakpoint).unwrap_or(false))
                .unwrap_or(false);
            self.tt_goto(Some(t), if reason { "breakpoint" } else { "step" });
        }
        self.writer
            .respond(seq, cmd, json!({ "allThreadsContinued": true }));
    }

    /// Move the time-travel cursor and emit `stopped`. `target = Some(i)`
    /// views `timeline[i]` (rebuilds `history_snapshot`); `None` returns to
    /// the live frontier. Never resumes the eval thread.
    pub(super) fn tt_goto(&self, target: Option<usize>, reason: &'static str) {
        let Some(state) = &self.state else { return };
        {
            let mut session = state.session_state.lock().expect("session poisoned");

            session.view_index = target;

            if let Some(i) = target {
                let entry = session.timeline.get(i).cloned();
                let baseline = session.baseline_env.clone();
                let nu = session.nu_constant.clone();
                let config = session.config.clone();
                let cache = state.cache.lock().expect("render cache poisoned").clone();

                if let Some(entry) = entry {
                    session.history_snapshot = crate::variables::build_history_snapshot(
                        &entry,
                        baseline.as_ref(),
                        nu.as_ref(),
                        config,
                        cache,
                    );
                }
            }
        }

        self.writer.event(
            "stopped",
            json!({
                "reason": reason,
                "threadId": THREAD_ID,
                "allThreadsStopped": true,
            }),
        );
    }
}
