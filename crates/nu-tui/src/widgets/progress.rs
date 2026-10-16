//! `tui progress`: a gauge fed by a value, the data list, or a source row.
use crate::session::Session;
use crate::widget::{Caps, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Gauge;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressWidget {
    /// Fixed value from `--value`; otherwise the widget's data is read.
    pub value: Option<f64>,
    /// Value that fills the bar (default 1.0, or 100 when values exceed 1).
    pub total: Option<f64>,
    pub label: Option<String>,
}

impl ProgressWidget {
    /// The current fraction in `0.0..=1.0` and the number it came from.
    pub fn fraction(&self, id: &str, session: &Session) -> (f64, f64) {
        let raw = match self.value {
            Some(v) => v,
            None => value_number(session.data_for(id)).unwrap_or(0.0),
        };
        let total = self
            .total
            .unwrap_or(if raw > 1.0 { 100.0 } else { 1.0 })
            .max(f64::EPSILON);
        ((raw / total).clamp(0.0, 1.0), raw)
    }
}

/// The number a progress value is read from: a number, a `{value, total}`
/// record's `value`, or the last number of a list (a streamed counter).
fn value_number(value: &Value) -> Option<f64> {
    match value {
        Value::Int { val, .. } => Some(*val as f64),
        Value::Float { val, .. } => Some(*val),
        Value::Record { val, .. } => val.get("value").and_then(value_number),
        Value::List { vals, .. } => vals.iter().rev().find_map(value_number),
        _ => None,
    }
}

impl TuiWidget for ProgressWidget {
    fn type_name(&self) -> &'static str {
        "progress"
    }

    fn caps(&self) -> Caps {
        Caps::default()
    }

    fn constraint(&self) -> Constraint {
        Constraint::Length(1)
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        if let Some(v) = self.value {
            rec.insert("value", Value::float(v, span));
        }
        if let Some(t) = self.total {
            rec.insert("total", Value::float(t, span));
        }
        if let Some(l) = &self.label {
            rec.insert("label", Value::string(l.clone(), span));
        }
    }

    fn render(
        &self,
        id: &str,
        _state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        _focused: bool,
    ) {
        let theme = &session.theme;
        let (fraction, raw) = self.fraction(id, session);
        let label = match &self.label {
            Some(l) => format!("{l} {:.0}%", fraction * 100.0),
            None if self.total.is_some() || raw > 1.0 => {
                format!("{raw:.0}/{:.0}", self.total.unwrap_or(100.0))
            }
            None => format!("{:.0}%", fraction * 100.0),
        };
        let gauge = Gauge::default()
            .ratio(fraction)
            .label(label)
            .gauge_style(theme.progress())
            .style(theme.text());
        frame.render_widget(gauge, area);
    }

    fn value(&self, id: &str, _state: &WidgetState, session: &Session) -> Value {
        Value::float(self.fraction(id, session).1, Span::unknown())
    }
}
