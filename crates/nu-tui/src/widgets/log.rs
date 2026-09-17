//! `tui log`: an append-only view that follows the tail of a stream.
use crate::filter::value_text;
use crate::keys::KeyPress;
use crate::session::Session;
use crate::widget::{Caps, Effect, LogState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::Text;
use ratatui::widgets::{Paragraph, Wrap};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogWidget {
    pub max_lines: usize,
}

impl LogWidget {
    /// Lines the view holds: the newest `max_lines` of the data.
    fn line_count(&self, rows: usize) -> usize {
        rows.min(self.max_lines)
    }

    /// Scroll offset to draw: the tail when following, else the stored
    /// offset clamped to the text. Resolved at render time so rows that
    /// arrive before the first layout still land at the bottom.
    fn scroll_offset(&self, state: &LogState, rows: usize, height: usize) -> usize {
        let max_scroll = self.line_count(rows).saturating_sub(height.max(1));
        if state.follow {
            max_scroll
        } else {
            state.scroll.min(max_scroll)
        }
    }

    fn scroll_by(&self, state: &mut LogState, delta: i32, rows: usize, height: usize) {
        let max_scroll = self.line_count(rows).saturating_sub(height.max(1));
        let current = self.scroll_offset(state, rows, height);
        let step = delta.unsigned_abs() as usize;
        state.scroll = if delta < 0 {
            current.saturating_sub(step)
        } else {
            (current + step).min(max_scroll)
        };
        // Scrolling back up pauses following; reaching the bottom resumes it.
        state.follow = state.scroll >= max_scroll;
    }
}

impl TuiWidget for LogWidget {
    fn type_name(&self) -> &'static str {
        "log"
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            scrollable: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Min(5)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::Log(LogState::default())
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("max_lines", Value::int(self.max_lines as i64, span));
    }

    fn handle_key(
        &self,
        id: &str,
        state: &mut WidgetState,
        key: &KeyPress,
        session: &Session,
    ) -> Option<Vec<Effect>> {
        let delta = match key.chord.as_str() {
            "up" | "k" => -1,
            "down" | "j" => 1,
            "pageup" => -10,
            "pagedown" => 10,
            "home" => i32::MIN / 2,
            "end" => i32::MAX / 2,
            _ => return None,
        };
        let rows = session.rows(id).len();
        let height = super::inner_rows(session.areas.get(id), 2);
        let log = state.as_log_mut()?;
        self.scroll_by(log, delta, rows, height);
        Some(Vec::new())
    }

    fn click(
        &self,
        id: &str,
        _state: &mut WidgetState,
        _area: Rect,
        _x: u16,
        _y: u16,
        _session: &Session,
    ) -> Vec<Effect> {
        vec![Effect::Focus(id.to_string())]
    }

    fn scroll(
        &self,
        id: &str,
        state: &mut WidgetState,
        delta: i32,
        session: &Session,
    ) -> Vec<Effect> {
        let rows = session.rows(id).len();
        let height = super::inner_rows(session.areas.get(id), 2);
        if let Some(log) = state.as_log_mut() {
            self.scroll_by(log, delta, rows, height);
        }
        Vec::new()
    }

    fn render(
        &self,
        id: &str,
        state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        focused: bool,
    ) {
        let theme = &session.theme;
        let log = state.as_log().cloned().unwrap_or_default();
        let rows = session.rows(id);
        let height = area.height.saturating_sub(2) as usize;
        let scroll = self.scroll_offset(&log, rows.len(), height);
        // Only the lines on screen are built.
        let start = rows.len().saturating_sub(self.max_lines) + scroll;
        let lines: Vec<_> = rows
            .iter()
            .skip(start)
            .take(height)
            .map(|v| super::ansi_line(&value_text(v), theme))
            .collect();
        let title = if session.stream_live {
            "log (live)"
        } else if log.follow {
            "log"
        } else {
            "log (paused)"
        };
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(theme.text())
                .wrap(Wrap { trim: false })
                .block(super::framed(title, focused, theme)),
            area,
        );
    }

    fn value(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        let span = Span::unknown();
        let mut rec = Record::new();
        rec.insert("rows", Value::int(session.rows(id).len() as i64, span));
        rec.insert(
            "follow",
            Value::bool(state.as_log().is_some_and(|l| l.follow), span),
        );
        Value::record(rec, span)
    }

    fn selection(&self, _id: &str, _state: &WidgetState, session: &Session) -> Value {
        session.current_row()
    }

    fn debug(&self, id: &str, _state: &WidgetState, session: &Session, rec: &mut Record) {
        rec.insert(
            "rows",
            Value::int(session.rows(id).len() as i64, Span::unknown()),
        );
    }
}
