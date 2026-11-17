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
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeWidget {
    /// Expand directories on disk when a node is opened.
    pub walk: bool,
    /// Path/name column.
    pub column: String,
    pub multi: bool,
}

impl TreeWidget {
    /// Visible rows for an explicit state: the flatten behind
    /// [`Session::tree_rows`], and what a key handler asks after changing
    /// an expansion in its copy of the state.
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

    /// The visible rows, flattened once per change by the session.
    fn rows(&self, id: &str, session: &Session) -> Rc<Vec<TreeRow>> {
        session.tree_rows(id)
    }

    /// The values of the checked nodes in tree order, hidden ones included.
    fn checked_values(&self, id: &str, state: &TreeState, session: &Session) -> Vec<Value> {
        tree::values_at(
            session.data_for(id),
            &state.checked,
            self.walk,
            &self.column,
            &session.cwd,
            &state.cache,
        )
    }

    /// The checked paths that still name a node of this tree's data.
    pub fn existing_checks(
        &self,
        id: &str,
        state: &TreeState,
        session: &Session,
    ) -> HashSet<String> {
        tree::existing_paths(
            session.data_for(id),
            &state.checked,
            self.walk,
            &self.column,
            &session.cwd,
            &state.cache,
        )
    }

    fn set_expanded(&self, id: &str, state: &mut TreeState, expand: bool, session: &Session) {
        let rows = self.rows_with(id, state, session);
        let Some(row) = rows.get(state.list.highlight).cloned() else {
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
            "space" if self.multi => {
                if let Some(row) = self.rows(id, session).get(tree_state.list.highlight) {
                    tree_state.toggle(&row.path);
                }
                Some(Vec::new())
            }
            "right" | "l" => {
                self.set_expanded(id, tree_state, true, session);
                Some(vec![Effect::Selected(id.to_string())])
            }
            "left" | "h" => {
                self.set_expanded(id, tree_state, false, session);
                Some(vec![Effect::Selected(id.to_string())])
            }
            chord => {
                super::table::list_key(id, &mut tree_state.list, chord, &session.rows(id), page)
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
        let rows = session.rows(id);
        let page = super::inner_rows(Some(&area), 2);
        let row = y.checked_sub(area.y).and_then(|y| y.checked_sub(1));
        match state.as_list_mut() {
            Some(list) => list_click(id, list, row.map(usize::from), &rows, page).0,
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
        let rows = session.rows(id);
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
        let checked = state.as_tree().map(|tree| &tree.checked);
        let rows = self.rows(id, session);
        let computer = session.style_computer();
        let name = super::custom_title(session, id, "tree");
        let block = super::framed(&format!("{name} ({})", rows.len()), focused, session, id);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let lines: Vec<Line> = rows
            .iter()
            .enumerate()
            .skip(list.scroll_for(&session.rows(id), inner.height as usize))
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
                    if checked.is_some_and(|checked| checked.contains(&row.path)) {
                        "[x] "
                    } else {
                        "[ ] "
                    }
                } else {
                    ""
                };
                let indent = "  ".repeat(row.depth);
                let mut style = session.row_style(&row.value, computer.as_ref());
                if i == list.highlight {
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
        let Some(tree_state) = state.as_tree() else {
            return Value::nothing(span);
        };
        let list = &tree_state.list;
        let rows = self.rows(id, session);
        let mut rec = Record::new();
        rec.insert("index", Value::int(list.highlight as i64, span));
        match rows.get(list.highlight) {
            Some(row) => {
                rec.insert("path", Value::string(tree::display_path(&row.path), span));
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
                Value::list(self.checked_values(id, tree_state, session), span),
            );
        }
        Value::record(rec, span)
    }

    /// Checked nodes under `--multi` (the highlighted one when no checked
    /// node is left in the data), else the highlighted node.
    fn selection(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        let span = Span::unknown();
        let Some(tree_state) = state.as_tree() else {
            return Value::nothing(span);
        };
        let highlighted = || self.current_row(id, state, session);
        if !self.multi {
            return highlighted().unwrap_or_else(|| Value::nothing(span));
        }
        let mut picked = self.checked_values(id, tree_state, session);
        if picked.is_empty() {
            picked.extend(highlighted());
        }
        Value::list(picked, span)
    }

    fn current_row(&self, id: &str, state: &WidgetState, session: &Session) -> Option<Value> {
        let list: &ListState = state.as_list()?;
        self.rows(id, session)
            .get(list.highlight)
            .map(|r| r.value.clone())
    }

    fn debug(&self, id: &str, _state: &WidgetState, session: &Session, rec: &mut Record) {
        rec.insert(
            "rows",
            Value::int(self.rows(id, session).len() as i64, Span::unknown()),
        );
    }
}
