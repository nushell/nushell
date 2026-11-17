//! `tui table`: navigable rows from the data list.
use crate::keys::KeyPress;
use crate::layout::pin_fills;
use crate::session::{Rows, Session};
use crate::widget::{Caps, Effect, ListState, Size, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::Text;
use ratatui::widgets::{Cell, Row, Table, TableState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableWidget {
    /// Columns to show. Empty means the columns of the data.
    pub columns: Vec<String>,
    /// One width per shown column; missing entries are `1fr`. Empty means
    /// equal shares.
    pub column_sizes: Vec<Size>,
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

/// The `selected` value of a table or select: checked rows for `--multi`,
/// the highlighted row otherwise, or indexes when `index` is set. `rows`
/// are the rows shown, `source` the unfiltered list that checks and
/// indexes count in, so checked rows the filter hides are still picked.
pub(crate) fn list_selection(
    state: &ListState,
    rows: &Rows,
    source: &[Value],
    multi: bool,
    index: bool,
) -> Value {
    let span = Span::unknown();
    let pick = |i: usize| -> Option<Value> {
        if index {
            source.get(i).map(|_| Value::int(i as i64, span))
        } else {
            source.get(i).cloned()
        }
    };
    let highlighted = || state.highlighted(rows).and_then(|(_, source)| pick(source));
    if multi {
        let picked: Vec<Value> = if state.checked.is_empty() {
            highlighted().into_iter().collect()
        } else {
            state.checked.iter().filter_map(|i| pick(*i)).collect()
        };
        Value::list(picked, span)
    } else {
        highlighted().unwrap_or_else(|| Value::nothing(span))
    }
}

/// The `values` entry of a table or select. `index` and `checked` count
/// rows of the unfiltered `source`, like the selection.
pub(crate) fn list_value(state: &ListState, rows: &Rows, source: &[Value], multi: bool) -> Value {
    let span = Span::unknown();
    let highlighted = state.highlighted(rows);
    let mut rec = Record::new();
    rec.insert(
        "index",
        Value::int(
            highlighted.map_or(state.highlight, |(_, source)| source) as i64,
            span,
        ),
    );
    rec.insert(
        "row",
        highlighted
            .and_then(|(pos, _)| rows.get(pos).cloned())
            .unwrap_or_else(|| Value::nothing(span)),
    );
    if multi {
        rec.insert(
            "checked",
            Value::list(
                state
                    .checked
                    .iter()
                    .filter_map(|i| source.get(*i).cloned())
                    .collect(),
                span,
            ),
        );
    }
    Value::record(rec, span)
}

/// Space on a `--multi` table or select: toggle the highlighted row's
/// check, kept under its position in the unfiltered list. `None` for any
/// other chord, or when the widget is not `--multi`.
pub(crate) fn check_key(
    list: &mut ListState,
    chord: &str,
    rows: &Rows,
    multi: bool,
) -> Option<Vec<Effect>> {
    if !multi || chord != "space" {
        return None;
    }
    if let Some((_, source)) = list.highlighted(rows) {
        list.toggle(source);
    }
    Some(Vec::new())
}

/// Shared movement keys for tables, selects, and trees, also used by the
/// mouse wheel.
pub(crate) fn list_key(
    id: &str,
    list: &mut ListState,
    chord: &str,
    rows: &Rows,
    page: usize,
) -> Option<Vec<Effect>> {
    let before = list.highlight;
    if !list.navigate(chord, rows, page) {
        return None;
    }
    list.ensure_visible(rows, page);
    Some(if list.highlight != before {
        vec![Effect::Selected(id.to_string())]
    } else {
        Vec::new()
    })
}

/// A click on visible row `row` (counted from the top of the list area;
/// `None` above it, on a border or header). `page` is the rows of room, so
/// the click maps through the offset the list is actually drawn with. A
/// click above the rows highlights the first one on screen, a click past
/// them the last row. Returns the effects, and whether the click landed on
/// a row.
pub(crate) fn list_click(
    id: &str,
    list: &mut ListState,
    row: Option<usize>,
    rows: &Rows,
    page: usize,
) -> (Vec<Effect>, bool) {
    let mut effects = vec![Effect::Focus(id.to_string())];
    let Some(last) = rows.len().checked_sub(1) else {
        return (effects, false);
    };
    list.scroll = list.scroll_for(rows, page);
    let target = list.scroll + row.unwrap_or(0);
    let hit = row.is_some_and(|row| row < page) && target <= last;
    if let Some(source) = rows.source(target.min(last))
        && source != list.highlight
    {
        list.highlight = source;
        effects.push(Effect::Selected(id.to_string()));
    }
    (effects, hit)
}

pub(crate) fn list_scroll(
    id: &str,
    list: &mut ListState,
    delta: i32,
    rows: &Rows,
    page: usize,
) -> Vec<Effect> {
    let chord = if delta < 0 { "up" } else { "down" };
    let mut effects = vec![Effect::Focus(id.to_string())];
    effects.extend(list_key(id, list, chord, rows, page).unwrap_or_default());
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
        rec.insert(
            "column_sizes",
            Value::list(
                self.column_sizes.iter().map(|s| s.to_value(span)).collect(),
                span,
            ),
        );
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
        let rows = session.rows(id);
        let page = super::inner_rows(session.areas.get(id), 3);
        let list = state.as_list_mut()?;
        if let Some(effects) = check_key(list, &key.chord, &rows, self.multi)
            .or_else(|| list_key(id, list, &key.chord, &rows, page))
        {
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
        let rows = session.rows(id);
        let page = super::inner_rows(Some(&area), 3);
        let row = y.checked_sub(area.y).and_then(|y| y.checked_sub(2));
        let row = row.map(usize::from);
        match state.as_list_mut() {
            Some(list) => list_click(id, list, row, &rows, page).0,
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
        let page = super::inner_rows(session.areas.get(id), 3);
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
        let columns = session.columns_for(id);
        let computer = session.style_computer();
        let rows_data = session.rows(id);
        let count = rows_data.len();
        // Only the rows on screen are built and styled; a list of a hundred
        // thousand rows must not cost a hundred thousand cells a frame.
        let height = area.height.saturating_sub(3).max(1) as usize;
        let highlight = list.position(&rows_data);
        let scroll = list.scroll_for(&rows_data, height);
        let mut header_cells: Vec<Cell> = Vec::new();
        let mut widths: Vec<Constraint> = Vec::new();
        if self.multi {
            header_cells.push(Cell::from(" ").style(theme.header()));
            widths.push(Constraint::Length(3));
        }
        let n = columns.len().max(1);
        for (k, c) in columns.iter().enumerate() {
            header_cells.push(Cell::from(c.as_str()).style(theme.header()));
            widths.push(if self.column_sizes.is_empty() {
                Constraint::Percentage((100 / n) as u16)
            } else {
                self.column_sizes
                    .get(k)
                    .map_or(Constraint::Fill(1), |s| s.to_constraint())
            });
        }
        let rows: Vec<Row> = rows_data
            .iter()
            .enumerate()
            .skip(scroll)
            .take(height)
            .map(|(i, row)| {
                let mut cells: Vec<Cell> = Vec::new();
                if self.multi {
                    let checked = rows_data
                        .source(i)
                        .is_some_and(|source| list.checked.contains(&source));
                    let mark = if checked { "[x]" } else { "[ ]" };
                    cells.push(Cell::from(mark).style(theme.highlight()));
                }
                for col in &columns {
                    let (text, style) = session.styled_cell(row, col, computer.as_ref());
                    cells.push(Cell::from(text).style(style));
                }
                Row::new(cells).height(1)
            })
            .collect();
        let name = super::custom_title(session, id, "table");
        let title = if self.multi && !list.checked.is_empty() {
            format!("{name} ({count}, {} checked)", list.checked.len())
        } else {
            format!("{name} ({count})")
        };
        let block = super::framed(&title, focused, session, id);
        let highlight_symbol = Text::raw("▶ ");
        let selection_width = if highlight.is_some() {
            highlight_symbol.width() as u16
        } else {
            0
        };
        // Pin within the frame, after the row marker, using Table's one-cell column spacing.
        let widths = pin_fills(
            widths,
            1,
            block.inner(area).width.saturating_sub(selection_width),
        );
        let table = Table::new(rows, widths)
            .header(Row::new(header_cells).height(1))
            .style(theme.text())
            .block(block)
            .row_highlight_style(theme.selected())
            .highlight_symbol(highlight_symbol);
        let mut table_state = TableState::default();
        table_state.select(highlight.map(|pos| pos.saturating_sub(scroll)));
        frame.render_stateful_widget(table, area, &mut table_state);
    }

    fn value(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        match state.as_list() {
            Some(list) => list_value(list, &session.rows(id), session.source_rows(id), self.multi),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn selection(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        match state.as_list() {
            Some(list) => list_selection(
                list,
                &session.rows(id),
                session.source_rows(id),
                self.multi,
                self.index,
            ),
            None => Value::nothing(Span::unknown()),
        }
    }

    fn current_row(&self, id: &str, state: &WidgetState, session: &Session) -> Option<Value> {
        let rows = session.rows(id);
        rows.get(state.as_list()?.position(&rows)?).cloned()
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
