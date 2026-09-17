//! `tui tree`: nested records, lists, or a directory walk.
use super::table::{list_click, list_scroll};
use crate::keys::KeyPress;
use crate::session::Session;
use crate::tree::{self, TreeRow};
use crate::widget::{Caps, Effect, ListState, TreeState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span as TSpan, Text};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeWidget {
    /// Expand directories on disk when a node is opened.
    pub walk: bool,
    /// Path/name column.
    pub column: String,
    pub multi: bool,
}

impl TreeWidget {
    /// Visible rows given an explicit state (used while the state is checked
    /// out of the session during key handling).
    pub fn rows_with(&self, id: &str, state: &TreeState, session: &Session) -> Vec<TreeRow> {
        let mut rows = tree::flatten(
            session.data_for(id),
            &state.expanded,
            self.walk,
            &self.column,
            &session.cwd,
            &state.cache,
        );
        let filter = session.filter_for(id);
        if !filter.is_empty() {
            let mut matcher = filter.matcher();
            rows.retain(|r| matcher.matches_text(&r.label) || matcher.matches(&r.value));
        }
        rows
    }

    pub fn rows(&self, id: &str, session: &Session) -> Vec<TreeRow> {
        let default = TreeState::default();
        let state = session
            .state(id)
            .and_then(WidgetState::as_tree)
            .unwrap_or(&default);
        self.rows_with(id, state, session)
    }

    fn set_expanded(&self, id: &str, state: &mut TreeState, expand: bool, session: &Session) {
        let rows = self.rows_with(id, state, session);
        let Some(row) = rows.get(state.list.selected).cloned() else {
            return;
        };
        if expand {
            if self.walk
                && let Some(dir) = tree::dir_path_for_row(&row.value, &self.column, &session.cwd)
            {
                let kids = tree::read_dir_listing(&dir, Span::unknown());
                state.cache.insert(row.path.clone(), kids);
            }
            state.expanded.insert(row.path);
        } else {
            state.expanded.remove(&row.path);
        }
    }
}

impl TuiWidget for TreeWidget {
    fn type_name(&self) -> &'static str {
        "tree"
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
        WidgetState::Tree(TreeState::default())
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("walk", Value::bool(self.walk, span));
        rec.insert("column", Value::string(self.column.clone(), span));
        rec.insert("multi", Value::bool(self.multi, span));
    }

    fn handle_key(
        &self,
        id: &str,
        state: &mut WidgetState,
        key: &KeyPress,
        session: &Session,
    ) -> Option<Vec<Effect>> {
        let page = super::inner_rows(session.areas.get(id), 2);
        let tree_state = state.as_tree_mut()?;
        match key.chord.as_str() {
            "right" | "l" => {
                self.set_expanded(id, tree_state, true, session);
                Some(vec![Effect::Selected(id.to_string())])
            }
            "left" | "h" => {
                self.set_expanded(id, tree_state, false, session);
                Some(vec![Effect::Selected(id.to_string())])
            }
            chord => {
                let len = self.rows_with(id, tree_state, session).len();
                super::table::list_key(id, &mut tree_state.list, chord, len, page, self.multi)
            }
        }
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
            Some(list) => list_click(id, list, row, len),
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
        let block = super::framed(&format!("tree ({})", rows.len()), focused, theme);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let lines: Vec<Line> = rows
            .iter()
            .enumerate()
            .skip(list.scroll)
            .take(inner.height as usize)
            .map(|(i, row)| {
                let marker = if !row.expandable {
                    "  "
                } else if row.expanded {
                    "▾ "
                } else {
                    "▸ "
                };
                let check = if self.multi {
                    if list.checked.contains(&i) {
                        "[x] "
                    } else {
                        "[ ] "
                    }
                } else {
                    ""
                };
                let indent = "  ".repeat(row.depth);
                let mut style = session.row_style(&row.value);
                if i == list.selected {
                    style = style.patch(theme.selected());
                }
                Line::from(TSpan::styled(
                    format!("{indent}{check}{marker}{}", row.label),
                    style,
                ))
            })
            .collect();
        frame.render_widget(Paragraph::new(Text::from(lines)).style(theme.text()), inner);
    }

    fn value(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        let span = Span::unknown();
        let Some(list) = state.as_list() else {
            return Value::nothing(span);
        };
        let rows = self.rows(id, session);
        let mut rec = Record::new();
        rec.insert("index", Value::int(list.selected as i64, span));
        match rows.get(list.selected) {
            Some(row) => {
                rec.insert("path", Value::string(row.path.clone(), span));
                rec.insert("value", row.value.clone());
            }
            None => {
                rec.insert("path", Value::nothing(span));
                rec.insert("value", Value::nothing(span));
            }
        }
        if self.multi {
            rec.insert(
                "checked",
                Value::list(
                    list.checked
                        .iter()
                        .filter_map(|i| rows.get(*i).map(|r| r.value.clone()))
                        .collect(),
                    span,
                ),
            );
        }
        Value::record(rec, span)
    }

    fn selection(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        let span = Span::unknown();
        let Some(list) = state.as_list() else {
            return Value::nothing(span);
        };
        let rows = self.rows(id, session);
        let values: Vec<Value> = rows.into_iter().map(|r| r.value).collect();
        super::table::list_selection(list, &values, self.multi, false)
    }

    fn current_row(&self, id: &str, state: &WidgetState, session: &Session) -> Option<Value> {
        let list: &ListState = state.as_list()?;
        self.rows(id, session)
            .into_iter()
            .nth(list.selected)
            .map(|r| r.value)
    }

    fn debug(&self, id: &str, _state: &WidgetState, session: &Session, rec: &mut Record) {
        rec.insert(
            "rows",
            Value::int(self.rows(id, session).len() as i64, Span::unknown()),
        );
    }
}
