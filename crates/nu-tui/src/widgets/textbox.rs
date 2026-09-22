//! `tui textbox`: a single-line editable field.
use crate::keys::{KeyPress, apply_edit};
use crate::session::Session;
use crate::widget::{Caps, Effect, TextState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBoxWidget {
    pub placeholder: String,
    pub value: String,
}

impl TuiWidget for TextBoxWidget {
    fn type_name(&self) -> &'static str {
        "textbox"
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            text_input: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Length(3)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::Text(TextState::new(self.value.clone()))
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("placeholder", Value::string(self.placeholder.clone(), span));
        rec.insert("value", Value::string(self.value.clone(), span));
    }

    fn captures_keys(&self, _state: &WidgetState) -> bool {
        true
    }

    fn handle_key(
        &self,
        _id: &str,
        state: &mut WidgetState,
        key: &KeyPress,
        _session: &Session,
    ) -> Option<Vec<Effect>> {
        let text = state.as_text_mut()?;
        match key.chord.as_str() {
            "tab" | "shift+tab" => None,
            "esc" => Some(vec![Effect::LeaveText]),
            "enter" => Some(vec![Effect::Submit(Value::string(
                text.text.clone(),
                Span::unknown(),
            ))]),
            _ => {
                apply_edit(key.event, &mut text.text, &mut text.cursor);
                Some(Vec::new())
            }
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
        vec![Effect::Focus(id.to_string())]
    }

    fn render(
        &self,
        _id: &str,
        state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        focused: bool,
    ) {
        let theme = &session.theme;
        let text = state.as_text().cloned().unwrap_or_default();
        let block = super::framed("input", focused, theme);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let display = if text.text.is_empty() && !focused {
            Paragraph::new(self.placeholder.as_str()).style(theme.muted())
        } else if focused {
            Paragraph::new(super::with_cursor(&text.text, text.cursor, theme.text()))
                .style(theme.text())
        } else {
            Paragraph::new(text.text).style(theme.text())
        };
        frame.render_widget(display, inner);
    }

    fn value(&self, _id: &str, state: &WidgetState, _session: &Session) -> Value {
        Value::string(
            state.as_text().map(|t| t.text.clone()).unwrap_or_default(),
            Span::unknown(),
        )
    }
}
