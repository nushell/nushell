//! `tui label`: static text inline, or the title/status bar.
use crate::filter::value_text;
use crate::session::Session;
use crate::widget::{Caps, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

/// Where a label is drawn.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Slot {
    /// One-line title bar at the top; also the dialog title.
    Title,
    /// Inline text inside the page.
    Content,
    /// One-line status bar at the bottom. Live hints are appended.
    Status,
}

impl Slot {
    pub fn as_str(self) -> &'static str {
        match self {
            Slot::Title => "title",
            Slot::Content => "content",
            Slot::Status => "status",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelWidget {
    pub text: String,
    pub slot: Slot,
}

impl LabelWidget {
    /// The text to draw: the source closure's output when the label follows
    /// another widget, else the builder's text.
    pub fn text(&self, id: &str, session: &Session) -> String {
        match session.derived(id) {
            Some(value) => value_text(value),
            None => self.text.clone(),
        }
    }
}

impl TuiWidget for LabelWidget {
    fn type_name(&self) -> &'static str {
        "label"
    }

    fn caps(&self) -> Caps {
        Caps {
            chrome: self.slot != Slot::Content,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Length(1)
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("text", Value::string(self.text.clone(), span));
        rec.insert("slot", Value::string(self.slot.as_str(), span));
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
        let para = match self.slot {
            Slot::Title => {
                Paragraph::new(format!(" {} ", self.text(id, session))).style(theme.title())
            }
            Slot::Content => Paragraph::new(super::ansi_to_text(&self.text(id, session), theme))
                .style(theme.text()),
            Slot::Status => {
                Paragraph::new(format!(" {} ", session.status_text())).style(theme.status())
            }
        };
        frame.render_widget(para, area);
    }

    fn value(&self, id: &str, _state: &WidgetState, session: &Session) -> Value {
        Value::string(self.text(id, session), Span::unknown())
    }
}
