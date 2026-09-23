//! `tui box`: a titled, bordered group around its children.
use crate::session::Session;
use crate::widget::{Caps, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxWidget {
    pub title: String,
}

impl TuiWidget for BoxWidget {
    fn type_name(&self) -> &'static str {
        "box"
    }

    fn caps(&self) -> Caps {
        Caps {
            container: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Min(5)
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("title", Value::string(self.title.clone(), span));
    }

    fn render(
        &self,
        _id: &str,
        _state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        _focused: bool,
    ) {
        frame.render_widget(super::framed(&self.title, false, &session.theme), area);
    }
}
