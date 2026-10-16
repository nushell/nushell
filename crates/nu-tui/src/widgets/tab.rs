//! `tui tab`: a page in the tab bar. Only valid at the top level.
use crate::session::Session;
use crate::widget::{Caps, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabWidget {
    pub title: String,
}

impl TuiWidget for TabWidget {
    fn type_name(&self) -> &'static str {
        "tab"
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

    /// A page draws nothing itself; its children fill the content area.
    fn render(
        &self,
        _id: &str,
        _state: &WidgetState,
        _frame: &mut Frame,
        _area: Rect,
        _session: &Session,
        _focused: bool,
    ) {
    }

    fn value(&self, id: &str, _state: &WidgetState, session: &Session) -> Value {
        let active = session
            .pages()
            .get(session.page)
            .is_some_and(|p| p.id.as_deref() == Some(id));
        Value::bool(active, Span::unknown())
    }
}
