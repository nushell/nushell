//! `tui button`: a focusable label that runs a hook or submits its name.
use crate::keys::KeyPress;
use crate::session::Session;
use crate::widget::{Caps, Effect, TuiWidget, WidgetState};
use nu_protocol::engine::Closure;
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span as TSpan};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonWidget {
    pub label: String,
    /// Hook run on activation. Without one, activating submits the label.
    pub action: Option<Closure>,
}

impl ButtonWidget {
    fn activate(&self) -> Vec<Effect> {
        match &self.action {
            Some(closure) => vec![Effect::Hook(closure.clone())],
            None => vec![Effect::Submit(Value::string(
                self.label.clone(),
                Span::unknown(),
            ))],
        }
    }
}

impl TuiWidget for ButtonWidget {
    fn type_name(&self) -> &'static str {
        "button"
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            flows: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Length(1)
    }

    /// ` [ label ]` plus a trailing space.
    fn width_hint(&self) -> Option<u16> {
        Some(self.label.chars().count().min(200) as u16 + 6)
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("label", Value::string(self.label.clone(), span));
        rec.insert("has_action", Value::bool(self.action.is_some(), span));
    }

    fn handle_key(
        &self,
        _id: &str,
        _state: &mut WidgetState,
        key: &KeyPress,
        _session: &Session,
    ) -> Option<Vec<Effect>> {
        match key.chord.as_str() {
            "enter" | "space" => Some(self.activate()),
            // Move along a button row.
            "left" | "h" => Some(vec![Effect::FocusPrev]),
            "right" | "l" => Some(vec![Effect::FocusNext]),
            _ => None,
        }
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
        let mut effects = vec![Effect::Focus(id.to_string())];
        effects.extend(self.activate());
        effects
    }

    fn render(
        &self,
        _id: &str,
        _state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        focused: bool,
    ) {
        let theme = &session.theme;
        let line = Line::from(vec![
            TSpan::styled(" ", theme.text()),
            TSpan::styled(format!("[ {} ]", self.label), theme.button(focused)),
        ]);
        frame.render_widget(Paragraph::new(line).style(theme.text()), area);
    }

    fn value(&self, _id: &str, _state: &WidgetState, _session: &Session) -> Value {
        Value::string(self.label.clone(), Span::unknown())
    }
}
