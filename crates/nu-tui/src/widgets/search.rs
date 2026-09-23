//! `tui search`: a filter box for the tables, logs, trees, and selects it
//! scopes.
use crate::filter::Filter;
use crate::keys::{KeyPress, apply_edit};
use crate::session::Session;
use crate::widget::{Caps, Effect, TextState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWidget {
    pub placeholder: String,
    /// Chord that focuses this box from anywhere but a text field.
    pub bind: Option<String>,
    pub fuzzy: bool,
    pub case_sensitive: bool,
    /// Record columns to match; empty means every field.
    pub columns: Vec<String>,
}

impl SearchWidget {
    /// The filter this box currently applies.
    pub fn filter(&self, state: Option<&WidgetState>) -> Filter {
        Filter {
            query: state
                .and_then(WidgetState::as_text)
                .map(|t| t.text.clone())
                .unwrap_or_default(),
            fuzzy: self.fuzzy,
            case_sensitive: self.case_sensitive,
            columns: self.columns.clone(),
        }
    }
}

impl TuiWidget for SearchWidget {
    fn type_name(&self) -> &'static str {
        "search"
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            text_input: true,
            // At the top level a search box takes a chrome slot above the
            // content; nested in a container it lays out like any leaf.
            chrome: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Length(3)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::Text(TextState::default())
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("placeholder", Value::string(self.placeholder.clone(), span));
        if let Some(bind) = &self.bind {
            rec.insert("bind", Value::string(bind.clone(), span));
        }
        rec.insert("fuzzy", Value::bool(self.fuzzy, span));
        rec.insert("case_sensitive", Value::bool(self.case_sensitive, span));
        if !self.columns.is_empty() {
            rec.insert("columns", crate::app::string_list(&self.columns, span));
        }
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
            // Esc clears the query; Esc on an empty box leaves it.
            "esc" => {
                if text.text.is_empty() {
                    Some(vec![Effect::LeaveText])
                } else {
                    text.clear();
                    Some(vec![Effect::Query])
                }
            }
            // Enter submits the row highlighted in the list this box filters.
            "enter" => Some(vec![Effect::SubmitCurrent]),
            _ => {
                if apply_edit(key.event, &mut text.text, &mut text.cursor) {
                    Some(vec![Effect::Query])
                } else {
                    Some(Vec::new())
                }
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
        let title = if self.fuzzy {
            "search (fuzzy)"
        } else {
            "search"
        };
        let block = super::framed(title, focused, theme);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let display = if text.text.is_empty() && !focused {
            Paragraph::new(self.placeholder.as_str()).style(theme.muted())
        } else if focused {
            Paragraph::new(super::with_cursor(
                &text.text,
                text.cursor,
                theme.highlight(),
            ))
            .style(theme.highlight())
        } else {
            Paragraph::new(text.text).style(theme.highlight())
        };
        frame.render_widget(display, inner);
    }

    fn value(&self, _id: &str, state: &WidgetState, _session: &Session) -> Value {
        Value::string(
            state.as_text().map(|t| t.text.clone()).unwrap_or_default(),
            Span::unknown(),
        )
    }

    fn selection(&self, _id: &str, _state: &WidgetState, session: &Session) -> Value {
        session.current_row()
    }

    fn debug(&self, _id: &str, state: &WidgetState, _session: &Session, rec: &mut Record) {
        rec.insert(
            "query",
            Value::string(
                state.as_text().map(|t| t.text.clone()).unwrap_or_default(),
                Span::unknown(),
            ),
        );
    }
}
