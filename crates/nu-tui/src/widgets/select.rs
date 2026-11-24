//! `tui select`: a radio (or checkbox, with `--multi`) list of items.
use super::table::{check_key, list_click, list_key, list_scroll, list_selection, list_value};
use crate::filter::value_text;
use crate::hooks::call_closure;
use crate::keys::KeyPress;
use crate::session::{Rows, Session};
use crate::widget::{Caps, Effect, ListState, TuiWidget, WidgetState};
use nu_protocol::engine::Closure;
use nu_protocol::{Record, Span, Value};
use nu_utils::strip_ansi_string_unlikely;
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
    /// What this select offers before filtering: its items, else the
    /// widget's data rows. The session filters them like a table's.
    pub fn items_or_data<'a>(&'a self, id: &str, session: &'a Session) -> &'a [Value] {
        if self.items.is_empty() {
            crate::session::as_list(session.data_for(id))
        } else {
            &self.items
        }
    }

    fn rows<'a>(&self, id: &str, session: &'a Session) -> Rows<'a> {
        session.rows(id)
    }

    /// The text shown for `item`, without ANSI escapes: the row is drawn in
    /// one style, and a hyperlink would otherwise print its URL.
    fn label(&self, item: &Value, session: &Session) -> String {
        let text = match &self.display {
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
        };
        strip_ansi_string_unlikely(text)
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
        let rows = self.rows(id, session);
        let page = super::inner_rows(session.areas.get(id), 2);
        let list = state.as_list_mut()?;
        check_key(list, &key.chord, &rows, self.multi)
            .or_else(|| list_key(id, list, &key.chord, &rows, page))
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
        let rows = self.rows(id, session);
        let page = super::inner_rows(Some(&area), 2);
        let row = y.checked_sub(area.y).and_then(|y| y.checked_sub(1));
        match state.as_list_mut() {
            Some(list) => {
                let (mut effects, hit) = list_click(id, list, row.map(usize::from), &rows, page);
                // Only a click on an item toggles it, not one on the border
                // or the empty space below the items.
                if self.multi && hit {
                    list.toggle(list.highlight);
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
        let rows = self.rows(id, session);
        let page = super::inner_rows(session.areas.get(id), 2);
        match state.as_list_mut() {
            Some(list) => list_scroll(id, list, delta, &rows, page),
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
        let name = super::custom_title(session, id, "select");
        let title = if self.multi {
            format!("{name} ({}, {} checked)", rows.len(), list.checked.len())
        } else {
            format!("{name} ({})", rows.len())
        };
        let block = super::framed(&title, focused, session, id);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let highlight = list.position(&rows);
        let lines: Vec<Line> = rows
            .iter()
            .enumerate()
            .skip(list.scroll_for(&rows, inner.height as usize))
            .take(inner.height as usize)
            .map(|(i, item)| {
                let checked = rows
                    .source(i)
                    .is_some_and(|source| list.checked.contains(&source));
                let highlighted = highlight == Some(i);
                let mark = match (self.multi, checked, highlighted) {
                    (true, true, _) => "[x] ",
                    (true, false, _) => "[ ] ",
                    (false, _, true) => "(•) ",
                    (false, _, false) => "( ) ",
                };
                let style = if highlighted {
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
            Some(list) => list_value(
                list,
                &self.rows(id, session),
                session.source_rows(id),
                self.multi,
            ),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn selection(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        match state.as_list() {
            Some(list) => list_selection(
                list,
                &self.rows(id, session),
                session.source_rows(id),
                self.multi,
                self.index,
            ),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn current_row(&self, id: &str, state: &WidgetState, session: &Session) -> Option<Value> {
        let rows = self.rows(id, session);
        rows.get(state.as_list()?.position(&rows)?).cloned()
    }

    fn debug(&self, id: &str, _state: &WidgetState, session: &Session, rec: &mut Record) {
        rec.insert(
            "rows",
            Value::int(self.rows(id, session).len() as i64, Span::unknown()),
        );
    }
}
