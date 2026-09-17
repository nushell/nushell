//! `tui select`: a radio (or checkbox, with `--multi`) list of items.
use super::table::{list_click, list_key, list_scroll, list_selection, list_value};
use crate::filter::value_text;
use crate::hooks::call_closure;
use crate::keys::KeyPress;
use crate::session::Session;
use crate::widget::{Caps, Effect, ListState, TuiWidget, WidgetState};
use nu_protocol::engine::Closure;
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span as TSpan, Text};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

/// How an item is shown: a record column, or a closure over the item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Display {
    Column(String),
    Closure(Closure),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectWidget {
    /// Items given to the builder. Empty means the widget's data rows.
    pub items: Vec<Value>,
    pub multi: bool,
    pub index: bool,
    pub display: Option<Display>,
}

impl SelectWidget {
    /// The rows this select offers: its items, else the widget's data,
    /// filtered like a table.
    pub fn rows(&self, id: &str, session: &Session) -> Vec<Value> {
        let items = if self.items.is_empty() {
            crate::session::as_list(session.data_for(id)).to_vec()
        } else {
            self.items.clone()
        };
        session.filter_for(id).apply(items)
    }

    fn label(&self, item: &Value, session: &Session) -> String {
        match &self.display {
            Some(Display::Column(col)) => item
                .as_record()
                .ok()
                .and_then(|r| r.get(col))
                .map(value_text)
                .unwrap_or_else(|| value_text(item)),
            Some(Display::Closure(closure)) => match &session.engine {
                Some((engine_state, stack)) => {
                    call_closure(engine_state, stack, closure.clone(), item.clone())
                        .and_then(|d| d.into_value(Span::unknown()))
                        .map(|v| value_text(&v))
                        .unwrap_or_else(|err| format!("error: {err}"))
                }
                None => value_text(item),
            },
            None => value_text(item),
        }
    }
}

impl TuiWidget for SelectWidget {
    fn type_name(&self) -> &'static str {
        "select"
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            scrollable: true,
            row_source: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Min(5)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::List(ListState::default())
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("items", Value::list(self.items.clone(), span));
        rec.insert("multi", Value::bool(self.multi, span));
        rec.insert("index", Value::bool(self.index, span));
    }

    fn handle_key(
        &self,
        id: &str,
        state: &mut WidgetState,
        key: &KeyPress,
        session: &Session,
    ) -> Option<Vec<Effect>> {
        let len = self.rows(id, session).len();
        let page = super::inner_rows(session.areas.get(id), 2);
        let list = state.as_list_mut()?;
        list_key(id, list, &key.chord, len, page, self.multi)
    }

    fn click(
        &self,
        id: &str,
        state: &mut WidgetState,
        area: Rect,
        _x: u16,
        y: u16,
        session: &Session,
    ) -> Vec<Effect> {
        let len = self.rows(id, session).len();
        let row = y.saturating_sub(area.y).saturating_sub(1) as usize;
        match state.as_list_mut() {
            Some(list) => {
                let mut effects = list_click(id, list, row, len);
                if self.multi && len > 0 {
                    list.toggle(list.selected);
                    effects.push(Effect::Selected(id.to_string()));
                }
                effects
            }
            None => Vec::new(),
        }
    }

    fn scroll(
        &self,
        id: &str,
        state: &mut WidgetState,
        delta: i32,
        session: &Session,
    ) -> Vec<Effect> {
        let len = self.rows(id, session).len();
        let page = super::inner_rows(session.areas.get(id), 2);
        match state.as_list_mut() {
            Some(list) => list_scroll(id, list, delta, len, page),
            None => Vec::new(),
        }
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
        let list = state.as_list().cloned().unwrap_or_default();
        let rows = self.rows(id, session);
        let title = if self.multi {
            format!("select ({}, {} checked)", rows.len(), list.checked.len())
        } else {
            format!("select ({})", rows.len())
        };
        let block = super::framed(&title, focused, theme);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let lines: Vec<Line> = rows
            .iter()
            .enumerate()
            .skip(list.scroll)
            .take(inner.height as usize)
            .map(|(i, item)| {
                let mark = match (self.multi, list.checked.contains(&i), i == list.selected) {
                    (true, true, _) => "[x] ",
                    (true, false, _) => "[ ] ",
                    (false, _, true) => "(•) ",
                    (false, _, false) => "( ) ",
                };
                let style = if i == list.selected {
                    theme.text().patch(theme.selected())
                } else {
                    theme.text()
                };
                Line::from(vec![
                    TSpan::styled(mark, style),
                    TSpan::styled(self.label(item, session), style),
                ])
            })
            .collect();
        frame.render_widget(Paragraph::new(Text::from(lines)).style(theme.text()), inner);
    }

    fn value(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        match state.as_list() {
            Some(list) => list_value(list, &self.rows(id, session), self.multi),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn selection(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        match state.as_list() {
            Some(list) => list_selection(list, &self.rows(id, session), self.multi, self.index),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn current_row(&self, id: &str, state: &WidgetState, session: &Session) -> Option<Value> {
        let list = state.as_list()?;
        self.rows(id, session).into_iter().nth(list.selected)
    }

    fn debug(&self, id: &str, _state: &WidgetState, session: &Session, rec: &mut Record) {
        rec.insert(
            "rows",
            Value::int(self.rows(id, session).len() as i64, Span::unknown()),
        );
    }
}
