//! `tui log`: an append-only view that follows the tail of a stream.
use crate::filter::value_text;
use crate::keys::KeyPress;
use crate::session::Session;
use crate::widget::{Caps, Effect, LogState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogWidget {
    pub max_lines: usize,
}

/// The pane a log is drawn in: columns for wrapping and rows of room.
#[derive(Debug, Clone, Copy)]
struct Pane {
    width: usize,
    height: usize,
}

impl Pane {
    fn of(area: Option<&Rect>) -> Self {
        Self {
            width: super::inner_width(area),
            height: super::inner_rows(area, 2),
        }
    }
}

impl LogWidget {
    /// The lines the view holds: the newest `max_lines` of the rows.
    fn lines<'a>(&self, rows: &'a [Value]) -> &'a [Value] {
        &rows[rows.len().saturating_sub(self.max_lines)..]
    }

    /// Index of the first line of the tail window: the newest lines that
    /// fit the pane after wrapping. Long lines take several rows, so this
    /// counts rows, not lines, or the newest line would be pushed below the
    /// pane.
    fn tail_start(&self, lines: &[Value], pane: Pane, session: &Session) -> usize {
        let mut rows = 0;
        for (i, line) in lines.iter().enumerate().rev() {
            rows += super::wrap_line(&self.line(line, session), pane.width).len();
            if rows >= pane.height {
                return i;
            }
        }
        0
    }

    /// Scroll offset in lines: the tail when following, else the stored
    /// offset clamped to the text. Resolved when asked so rows that arrive
    /// before the first layout still land at the bottom.
    fn scroll_offset(&self, state: &LogState, max_scroll: usize) -> usize {
        if state.follow {
            max_scroll
        } else {
            state.scroll.min(max_scroll)
        }
    }

    fn scroll_by(
        &self,
        state: &mut LogState,
        delta: i32,
        rows: &[Value],
        pane: Pane,
        session: &Session,
    ) {
        let max_scroll = self.tail_start(self.lines(rows), pane, session);
        let current = self.scroll_offset(state, max_scroll);
        let step = delta.unsigned_abs() as usize;
        state.scroll = if delta < 0 {
            current.saturating_sub(step)
        } else {
            (current + step).min(max_scroll)
        };
        // Scrolling back up pauses following; reaching the bottom resumes it.
        state.follow = state.scroll >= max_scroll;
    }

    fn line(&self, value: &Value, session: &Session) -> Line<'static> {
        super::ansi_line(&value_text(value), &session.theme)
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
        let rows = session.rows(id);
        let pane = Pane::of(session.areas.get(id));
        let log = state.as_log_mut()?;
        self.scroll_by(log, delta, &rows, pane, session);
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
        let rows = session.rows(id);
        let pane = Pane::of(session.areas.get(id));
        if let Some(log) = state.as_log_mut() {
            self.scroll_by(log, delta, &rows, pane, session);
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
        let lines = self.lines(&rows);
        let pane = Pane::of(Some(&area));
        let max_scroll = self.tail_start(lines, pane, session);
        let start = self.scroll_offset(&log, max_scroll);
        // Only the lines on screen are built and wrapped. When following,
        // the window is aligned to its bottom so the newest row is the
        // last one drawn.
        let mut wrapped: Vec<Line> = Vec::new();
        for line in &lines[start..] {
            wrapped.extend(super::wrap_line(&self.line(line, session), pane.width));
            if wrapped.len() >= pane.height && !log.follow {
                break;
            }
        }
        let visible: Vec<Line> = if log.follow {
            let skip = wrapped.len().saturating_sub(pane.height);
            wrapped.into_iter().skip(skip).collect()
        } else {
            wrapped.into_iter().take(pane.height).collect()
        };
        let title = if session.stream_live {
            "log (live)"
        } else if log.follow {
            "log"
        } else {
            "log (paused)"
        };
        frame.render_widget(
            Paragraph::new(Text::from(visible))
                .style(theme.text())
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
