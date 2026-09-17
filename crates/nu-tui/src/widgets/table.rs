//! `tui table`: navigable rows from the data list.
use crate::keys::KeyPress;
use crate::session::Session;
use crate::widget::{Caps, Effect, ListState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Cell, Row, Table, TableState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableWidget {
    /// Columns to show. Empty means the columns of the data.
    pub columns: Vec<String>,
    /// A chord pressed on the focused table becomes its filter query.
    pub capture_keys: bool,
    /// Space toggles rows; the selection is the checked rows.
    pub multi: bool,
    /// Return row indexes instead of rows.
    pub index: bool,
}

/// Chords `--capture-keys` leaves alone: list navigation, and the global
/// keys (focus, submit, quit, pages) that must keep working on a capturing
/// table.
pub(crate) fn is_reserved_chord(chord: &str) -> bool {
    matches!(
        chord,
        "up" | "down"
            | "left"
            | "right"
            | "k"
            | "j"
            | "h"
            | "l"
            | "pageup"
            | "pagedown"
            | "home"
            | "end"
            | "space"
            | "tab"
            | "shift+tab"
            | "enter"
            | "esc"
            | "q"
            | "["
            | "]"
    )
}

/// The `selected` value of a list widget: checked rows for `--multi`, the
/// highlighted row otherwise, or indexes when `index` is set.
pub(crate) fn list_selection(state: &ListState, rows: &[Value], multi: bool, index: bool) -> Value {
    let span = Span::unknown();
    let pick = |i: usize| -> Option<Value> {
        if index {
            rows.get(i).map(|_| Value::int(i as i64, span))
        } else {
            rows.get(i).cloned()
        }
    };
    if multi {
        let picked: Vec<Value> = if state.checked.is_empty() {
            pick(state.selected).into_iter().collect()
        } else {
            state.checked.iter().filter_map(|i| pick(*i)).collect()
        };
        Value::list(picked, span)
    } else {
        pick(state.selected).unwrap_or_else(|| Value::nothing(span))
    }
}

/// The `values` entry of a list widget.
pub(crate) fn list_value(state: &ListState, rows: &[Value], multi: bool) -> Value {
    let span = Span::unknown();
    let mut rec = Record::new();
    rec.insert("index", Value::int(state.selected as i64, span));
    rec.insert(
        "row",
        rows.get(state.selected)
            .cloned()
            .unwrap_or_else(|| Value::nothing(span)),
    );
    if multi {
        rec.insert(
            "checked",
            Value::list(
                state
                    .checked
                    .iter()
                    .filter_map(|i| rows.get(*i).cloned())
                    .collect(),
                span,
            ),
        );
    }
    Value::record(rec, span)
}

/// Shared key handling for tables and selects: movement, Space to check,
/// and mouse wheel.
pub(crate) fn list_key(
    id: &str,
    list: &mut ListState,
    chord: &str,
    len: usize,
    page: usize,
    multi: bool,
) -> Option<Vec<Effect>> {
    if multi && chord == "space" {
        if len > 0 {
            list.toggle(list.selected);
        }
        return Some(Vec::new());
    }
    let before = list.selected;
    if !list.navigate(chord, len, page) {
        return None;
    }
    list.ensure_visible(page);
    Some(if list.selected != before {
        vec![Effect::Selected(id.to_string())]
    } else {
        Vec::new()
    })
}

pub(crate) fn list_click(id: &str, list: &mut ListState, row: usize, len: usize) -> Vec<Effect> {
    let mut effects = vec![Effect::Focus(id.to_string())];
    if len > 0 {
        let next = (list.scroll + row).min(len - 1);
        if next != list.selected {
            list.selected = next;
            effects.push(Effect::Selected(id.to_string()));
        }
    }
    effects
}

pub(crate) fn list_scroll(
    id: &str,
    list: &mut ListState,
    delta: i32,
    len: usize,
    page: usize,
) -> Vec<Effect> {
    let chord = if delta < 0 { "up" } else { "down" };
    let mut effects = vec![Effect::Focus(id.to_string())];
    effects.extend(list_key(id, list, chord, len, page, false).unwrap_or_default());
    effects
}

impl TuiWidget for TableWidget {
    fn type_name(&self) -> &'static str {
        "table"
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
        rec.insert("columns", crate::app::string_list(&self.columns, span));
        rec.insert("capture_keys", Value::bool(self.capture_keys, span));
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
        let len = session.rows(id).len();
        let page = super::inner_rows(session.areas.get(id), 3);
        let list = state.as_list_mut()?;
        if let Some(effects) = list_key(id, list, &key.chord, len, page, self.multi) {
            return Some(effects);
        }
        if self.capture_keys && !is_reserved_chord(&key.chord) {
            return Some(vec![Effect::Capture(id.to_string(), key.chord.clone())]);
        }
        None
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
        let len = session.rows(id).len();
        let row = y.saturating_sub(area.y).saturating_sub(2) as usize;
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
        let len = session.rows(id).len();
        let page = super::inner_rows(session.areas.get(id), 3);
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
        let columns = session.columns_for(id);
        let computer = session.style_computer();
        let rows_data = session.rows(id);
        let count = rows_data.len();
        // Only the rows on screen are built and styled; a list of a hundred
        // thousand rows must not cost a hundred thousand cells a frame.
        let height = area.height.saturating_sub(3).max(1) as usize;
        let selected = list.selected.min(count.saturating_sub(1));
        let scroll = if selected < list.scroll {
            selected
        } else if selected >= list.scroll + height {
            selected + 1 - height
        } else {
            list.scroll
        };
        let mut header_cells: Vec<Cell> = Vec::new();
        let mut widths: Vec<Constraint> = Vec::new();
        if self.multi {
            header_cells.push(Cell::from(" ").style(theme.header()));
            widths.push(Constraint::Length(3));
        }
        let n = columns.len().max(1);
        for c in &columns {
            header_cells.push(Cell::from(c.as_str()).style(theme.header()));
            widths.push(Constraint::Percentage((100 / n) as u16));
        }
        let rows: Vec<Row> = rows_data
            .iter()
            .enumerate()
            .skip(scroll)
            .take(height)
            .map(|(i, row)| {
                let mut cells: Vec<Cell> = Vec::new();
                if self.multi {
                    let mark = if list.checked.contains(&i) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    cells.push(Cell::from(mark).style(theme.highlight()));
                }
                for col in &columns {
                    let (text, style) = session.styled_cell(row, col, computer.as_ref());
                    cells.push(Cell::from(text).style(style));
                }
                Row::new(cells).height(1)
            })
            .collect();
        let title = if self.multi && !list.checked.is_empty() {
            format!("table ({count}, {} checked)", list.checked.len())
        } else {
            format!("table ({count})")
        };
        let table = Table::new(rows, widths)
            .header(Row::new(header_cells).height(1))
            .style(theme.text())
            .block(super::framed(&title, focused, theme))
            .row_highlight_style(theme.selected())
            .highlight_symbol("▶ ");
        let mut table_state = TableState::default();
        if count > 0 {
            table_state.select(Some(selected - scroll));
        }
        frame.render_stateful_widget(table, area, &mut table_state);
    }

    fn value(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        match state.as_list() {
            Some(list) => list_value(list, &session.rows(id), self.multi),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn selection(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        match state.as_list() {
            Some(list) => list_selection(list, &session.rows(id), self.multi, self.index),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn current_row(&self, id: &str, state: &WidgetState, session: &Session) -> Option<Value> {
        let list = state.as_list()?;
        session.rows(id).get(list.selected).cloned()
    }

    fn debug(&self, id: &str, _state: &WidgetState, session: &Session, rec: &mut Record) {
        let span = Span::unknown();
        rec.insert(
            "resolved_columns",
            crate::app::string_list(&session.columns_for(id), span),
        );
        rec.insert("rows", Value::int(session.rows(id).len() as i64, span));
    }
}
