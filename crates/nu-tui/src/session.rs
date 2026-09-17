//! Orchestration of a running TUI: focus, pages, data resolution, hooks,
//! mouse routing, and the result record. Widget-specific behaviour lives in
//! `widgets/*`; the session only applies the [`Effect`]s they return.
use crate::app::{TuiApp, widget_to_record};
use crate::filter::Filter;
use crate::hooks::{self, HookOutcome, call_closure};
use crate::keys::{KeyPress, normalize_bind, single_char};
use crate::layout::{SplitterHandle, assign_areas, subtree_ids};
use crate::theme::Theme;
use crate::widget::{Effect, Widget, WidgetKind, WidgetState};
use crate::widgets::label::Slot;
use crate::widgets::split::{SplitDir, SplitWidget};
use crossterm::event::{Event, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use lscolors::LsColors;
use nu_color_config::StyleComputer;
use nu_engine::get_columns;
use nu_explore::style::create_lscolors;
use nu_protocol::engine::{Closure, EngineState, Stack};
use nu_protocol::{Config, Record, Span, Value};
use nu_utils::get_ls_colors;
use ratatui::layout::Rect;
use ratatui::style::Style;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Submit,
    Quit,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub action: Action,
    pub selected: Value,
}

/// One entry in the tab bar: a top-level `Tab` widget.
#[derive(Debug, Clone)]
pub struct Page {
    pub title: String,
    /// Widget id of the tab, or `None` for the implicit page when there are
    /// no top-level tabs.
    pub id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogDrag {
    Move { grab_x: u16, grab_y: u16 },
    Resize,
}

/// The floating window of `tui run --dialog`.
#[derive(Debug, Clone)]
pub struct DialogFrame {
    pub rect: Rect,
    pub screen: Rect,
    drag: Option<DialogDrag>,
}

impl DialogFrame {
    pub fn close_area(&self) -> Rect {
        Rect {
            x: self
                .rect
                .x
                .saturating_add(self.rect.width.saturating_sub(3)),
            y: self.rect.y,
            width: self.rect.width.min(3),
            height: 1,
        }
    }

    pub fn title_area(&self) -> Rect {
        Rect {
            x: self.rect.x,
            y: self.rect.y,
            width: self.rect.width.saturating_sub(3),
            height: 1,
        }
    }

    pub fn resize_area(&self) -> Rect {
        Rect {
            x: self
                .rect
                .x
                .saturating_add(self.rect.width.saturating_sub(2)),
            y: self
                .rect
                .y
                .saturating_add(self.rect.height.saturating_sub(2)),
            width: self.rect.width.min(2),
            height: self.rect.height.min(2),
        }
    }
}

fn clamp_dialog(rect: Rect, screen: Rect) -> Rect {
    let width = rect.width.clamp(20, screen.width.max(20));
    let height = rect.height.clamp(8, screen.height.max(8));
    let max_x = screen.x + screen.width.saturating_sub(width);
    let max_y = screen.y + screen.height.saturating_sub(height);
    Rect {
        x: rect.x.min(max_x).max(screen.x),
        y: rect.y.min(max_y).max(screen.y),
        width,
        height,
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub app: TuiApp,
    /// Child-index path of every widget id (see [`TuiApp::paths`]).
    pub paths: HashMap<String, Vec<usize>>,
    /// Runtime state of every widget, keyed by id.
    pub states: HashMap<String, WidgetState>,
    /// Data produced for widgets that follow a source (`--from`, a source
    /// closure).
    derived: HashMap<String, Value>,
    /// The (source id, source row, pane size) each derived value was
    /// computed from, so unchanged selections do not re-run closures and a
    /// resize does.
    derived_key: HashMap<String, (String, Value, (u16, u16))>,
    pub page: usize,
    pub focused: Option<String>,
    pub areas: HashMap<String, Rect>,
    pub handles: Vec<SplitterHandle>,
    pub tab_areas: Vec<Rect>,
    /// Split id and handle index being dragged.
    pub dragging: Option<(String, usize)>,
    pub outcome: Option<Outcome>,
    pub cwd: PathBuf,
    /// Engine used to run hooks, source closures, and previews. Absent in
    /// unit tests.
    pub engine: Option<(EngineState, Stack)>,
    pub stream_live: bool,
    pub dialog: Option<DialogFrame>,
    pub theme: Theme,
    pub ls_colors: LsColors,
    pub use_ls_colors: bool,
    /// Last failure from a hook or source closure; shown on the status bar.
    pub error: Option<String>,
    /// Filtered rows per widget, kept until data, a filter, or a selection
    /// changes. A frame asks for rows several times; a 100k-row list must
    /// not be filtered and cloned each time.
    rows_cache: RefCell<HashMap<String, Rc<Vec<Value>>>>,
}

impl Session {
    /// The `--from` ids that name no widget, for an early error.
    pub fn unknown_sources(app: &TuiApp) -> Vec<(String, String)> {
        app.iter()
            .filter_map(|w| {
                let from = w.source.as_ref()?.from.as_deref()?;
                (app.widget(from).is_none()).then(|| (w.id.clone(), from.to_string()))
            })
            .collect()
    }

    #[cfg(test)]
    pub fn new(app: TuiApp) -> Self {
        Self::with_engine(app, PathBuf::from("."), None)
    }

    pub fn with_engine(app: TuiApp, cwd: PathBuf, engine: Option<(EngineState, Stack)>) -> Self {
        let states = app
            .iter()
            .map(|w| {
                let mut state = w.kind.init_state();
                // A split's live sizes cover every child from the start so
                // handles can be dragged before any layout pass.
                if let (WidgetKind::Split(split), Some(sizes)) = (&w.kind, state.as_split_mut()) {
                    sizes.sizes = split.sizes_for(w.children.len());
                }
                (w.id.clone(), state)
            })
            .collect();
        let paths = app.paths();
        let (theme, ls_colors, use_ls_colors) = match &engine {
            Some((engine_state, stack)) => (
                Theme::from_config(engine_state, stack),
                create_lscolors(engine_state, stack),
                stack.get_config(engine_state).ls.use_ls_colors,
            ),
            None => (Theme::default(), get_ls_colors(None), true),
        };
        let mut session = Self {
            app,
            paths,
            states,
            derived: HashMap::new(),
            derived_key: HashMap::new(),
            page: 0,
            focused: None,
            areas: HashMap::new(),
            handles: Vec::new(),
            tab_areas: Vec::new(),
            dragging: None,
            outcome: None,
            cwd,
            engine,
            stream_live: false,
            dialog: None,
            theme,
            ls_colors,
            use_ls_colors,
            error: None,
            rows_cache: RefCell::new(HashMap::new()),
        };
        session.focused = session.default_focus();
        session.refresh_derived();
        session
    }

    // ----- tree queries -------------------------------------------------

    pub fn widget(&self, id: &str) -> Option<&Widget> {
        self.app.widget(id)
    }

    pub fn kind(&self, id: &str) -> Option<&WidgetKind> {
        self.app.widget_kind(id)
    }

    pub fn state(&self, id: &str) -> Option<&WidgetState> {
        self.states.get(id)
    }

    pub fn is_focused(&self, id: &str) -> bool {
        self.focused.as_deref() == Some(id)
    }

    /// Tab-bar entries: top-level `Tab` widgets in order, or one implicit
    /// page when there are none.
    pub fn pages(&self) -> Vec<Page> {
        let pages: Vec<Page> = self
            .app
            .widgets
            .iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Tab(tab) => Some(Page {
                    title: tab.title.clone(),
                    id: Some(w.id.clone()),
                }),
                _ => None,
            })
            .collect();
        if pages.is_empty() {
            vec![Page {
                title: "main".into(),
                id: None,
            }]
        } else {
            pages
        }
    }

    /// The tab bar shows whenever any top-level tab exists.
    pub fn has_tabs(&self) -> bool {
        self.app
            .widgets
            .iter()
            .any(|w| matches!(w.kind, WidgetKind::Tab(_)))
    }

    /// Ids of the widgets on screen right now, preorder: bare content roots
    /// and the active tab's subtree. Chrome and hidden tabs are excluded.
    pub fn visible_ids(&self) -> Vec<String> {
        let active = self.pages().get(self.page).and_then(|p| p.id.clone());
        let mut out = Vec::new();
        for (i, w) in self.app.widgets.iter().enumerate() {
            match &w.kind {
                WidgetKind::Tab(_) => {
                    if Some(&w.id) == active.as_ref() {
                        out.extend(subtree_ids(&self.app, &[i]));
                    }
                }
                k if k.is_chrome() => {}
                _ => out.extend(subtree_ids(&self.app, &[i])),
            }
        }
        out
    }

    fn visible_widgets(&self) -> impl Iterator<Item = &Widget> {
        self.visible_ids()
            .into_iter()
            .filter_map(|id| self.app.widget(&id))
    }

    /// Tab order: top-level menu/search first, then visible leaves, then
    /// visible splits last.
    pub fn focusable_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .app
            .widgets
            .iter()
            .filter(|w| matches!(w.kind, WidgetKind::Menu(_) | WidgetKind::Search(_)))
            .map(|w| w.id.clone())
            .collect();
        let visible: Vec<&Widget> = self.visible_widgets().collect();
        ids.extend(
            visible
                .iter()
                .filter(|w| w.kind.is_focusable() && !w.kind.is_container())
                .map(|w| w.id.clone()),
        );
        ids.extend(
            visible
                .iter()
                .filter(|w| matches!(w.kind, WidgetKind::Split(_)))
                .map(|w| w.id.clone()),
        );
        ids
    }

    /// Focus the widget marked `--focus` if one is reachable, else the most
    /// useful widget on the page: a list before an input, an input before
    /// chrome.
    pub fn default_focus(&self) -> Option<String> {
        let ids = self.focusable_ids();
        if let Some(id) = ids
            .iter()
            .find(|id| self.widget(id).is_some_and(|w| w.focus))
        {
            return Some(id.clone());
        }
        let rank = |k: &WidgetKind| match k {
            WidgetKind::Table(_) => 0,
            WidgetKind::Select(_) => 1,
            WidgetKind::Tree(_) => 2,
            WidgetKind::Log(_) => 3,
            WidgetKind::TextBox(_) => 4,
            WidgetKind::Search(_) => 5,
            WidgetKind::Button(_) => 6,
            WidgetKind::Menu(_) => 7,
            _ => 8,
        };
        ids.into_iter()
            .min_by_key(|id| self.kind(id).map(rank).unwrap_or(9))
    }

    /// Whether the focused widget wants every key: a text input, or a menu
    /// with its dropdown open. Plain-letter binds do not fire then.
    pub fn is_capturing(&self) -> bool {
        self.focused
            .as_deref()
            .and_then(|id| Some((self.kind(id)?, self.state(id)?)))
            .is_some_and(|(kind, state)| kind.captures_keys(state))
    }

    pub fn is_resizing(&self) -> bool {
        self.dragging.is_some() || self.dialog.as_ref().is_some_and(|d| d.drag.is_some())
    }

    // ----- data ---------------------------------------------------------

    /// The value a widget shows: its own `--data`, else what its source
    /// produced, else the nearest ancestor's data, else the outer pipeline's.
    pub fn data_for(&self, id: &str) -> &Value {
        if let Some(w) = self.widget(id)
            && let Some(data) = &w.data
        {
            return data;
        }
        if let Some(value) = self.derived.get(id) {
            return value;
        }
        if let Some(path) = self.paths.get(id) {
            for depth in (1..path.len()).rev() {
                if let Some(ancestor) = self.app.at_path(&path[..depth])
                    && let Some(data) = &ancestor.data
                {
                    return data;
                }
            }
        }
        &self.app.data
    }

    /// What a source closure produced for `id`, if it follows one.
    pub fn derived(&self, id: &str) -> Option<&Value> {
        self.derived.get(id)
    }

    /// Rows of a list widget after filtering, shared until something
    /// changes them (see [`Session::invalidate_rows`]). Trees and selects
    /// compute their own rows; everything else lists its data.
    pub fn rows(&self, id: &str) -> Rc<Vec<Value>> {
        if let Some(rows) = self.rows_cache.borrow().get(id) {
            return Rc::clone(rows);
        }
        let rows = match self.kind(id) {
            Some(WidgetKind::Tree(tree)) => {
                tree.rows(id, self).into_iter().map(|r| r.value).collect()
            }
            Some(WidgetKind::Select(select)) => select.compute_rows(id, self),
            Some(_) => self.filter_for(id).apply(as_list(self.data_for(id))),
            None => Vec::new(),
        };
        let rows = Rc::new(rows);
        self.rows_cache
            .borrow_mut()
            .insert(id.to_string(), Rc::clone(&rows));
        rows
    }

    /// Forget cached rows. Called whenever data, a filter, a selection, or
    /// an expansion may have changed.
    fn invalidate_rows(&self) {
        self.rows_cache.borrow_mut().clear();
    }

    /// Resolved columns of a table: `--columns`, else the data's columns,
    /// else one `item` column for scalars.
    pub fn columns_for(&self, id: &str) -> Vec<String> {
        if let Some(WidgetKind::Table(table)) = self.kind(id)
            && !table.columns.is_empty()
        {
            return table.columns.clone();
        }
        // Read the columns from the first rows only, so a long list is not
        // walked every frame.
        let rows = as_list(self.data_for(id));
        let cols = get_columns(&rows[..rows.len().min(200)]);
        if cols.is_empty() {
            vec!["item".into()]
        } else {
            cols
        }
    }

    /// The filter applied to widget `id`: its own captured chord, else the
    /// search box that scopes it.
    pub fn filter_for(&self, id: &str) -> Filter {
        if let Some(list) = self.state(id).and_then(WidgetState::as_list)
            && !list.captured.is_empty()
        {
            return Filter {
                query: list.captured.clone(),
                ..Filter::default()
            };
        }
        match self.scoping_search(id) {
            Some(search) => match self.kind(&search) {
                Some(WidgetKind::Search(s)) => s.filter(self.state(&search)),
                _ => Filter::default(),
            },
            None => Filter::default(),
        }
    }

    /// The search box that filters widget `id`: the deepest search whose
    /// parent container encloses `id`. A top-level search encloses everything.
    pub fn scoping_search(&self, id: &str) -> Option<String> {
        let path = self.paths.get(id)?;
        let mut best: Option<(usize, String)> = None;
        for w in self.app.iter() {
            if !matches!(w.kind, WidgetKind::Search(_)) {
                continue;
            }
            let Some(spath) = self.paths.get(&w.id) else {
                continue;
            };
            let parent = &spath[..spath.len().saturating_sub(1)];
            if path.starts_with(parent)
                && best.as_ref().is_none_or(|(depth, _)| parent.len() > *depth)
            {
                best = Some((parent.len(), w.id.clone()));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Which widget drives `id`: `--from` if given, else the focused row
    /// source, else the nearest one in an enclosing container, else the first
    /// visible one. `None` for widgets that do not follow anything.
    pub fn source_id(&self, id: &str) -> Option<String> {
        let widget = self.widget(id)?;
        let follows = widget.source.is_some() || matches!(widget.kind, WidgetKind::Preview(_));
        if !follows {
            return None;
        }
        if let Some(from) = widget.source.as_ref().and_then(|s| s.from.clone()) {
            return Some(from);
        }
        let is_source =
            |other: &str| other != id && self.kind(other).is_some_and(|k| k.is_row_source());
        if let Some(focused) = &self.focused
            && is_source(focused)
        {
            return Some(focused.clone());
        }
        if let Some(path) = self.paths.get(id) {
            for depth in (1..path.len()).rev() {
                if let Some(found) = subtree_ids(&self.app, &path[..depth])
                    .into_iter()
                    .find(|other| is_source(other))
                {
                    return Some(found);
                }
            }
        }
        self.visible_ids()
            .into_iter()
            .find(|other| is_source(other))
    }

    /// The highlighted row of widget `id`.
    pub fn row_of(&self, id: &str) -> Option<Value> {
        let widget = self.widget(id)?;
        let state = self.state(id)?;
        widget.kind.current_row(id, state, self)
    }

    /// The row highlighted in the focused row source, else in the first
    /// visible one.
    pub fn current_row(&self) -> Value {
        if let Some(id) = &self.focused
            && self.kind(id).is_some_and(|k| k.is_row_source())
            && let Some(row) = self.row_of(id)
        {
            return row;
        }
        self.visible_widgets()
            .filter(|w| w.kind.is_row_source())
            .find_map(|w| self.row_of(&w.id))
            .unwrap_or_else(|| Value::nothing(Span::unknown()))
    }

    /// What `selected` holds on submit: the focused widget's selection, or
    /// the current row when nothing is focused.
    pub fn selected_value(&self) -> Value {
        if let Some(id) = &self.focused
            && let (Some(widget), Some(state)) = (self.widget(id), self.state(id))
        {
            return widget.kind.selection(id, state, self);
        }
        self.current_row()
    }

    /// Recompute every widget that follows a source whose highlighted row
    /// changed. Runs a few passes so chains (tree → table → preview) settle.
    pub fn refresh_derived(&mut self) {
        let ids: Vec<String> = self
            .app
            .iter()
            .filter(|w| w.source.is_some() || matches!(w.kind, WidgetKind::Preview(_)))
            .map(|w| w.id.clone())
            .collect();
        for _ in 0..3 {
            let mut changed = false;
            for id in &ids {
                changed |= self.refresh_one(id);
            }
            if !changed {
                break;
            }
        }
    }

    /// The size a widget's closures see as `$env.TUI_WIDTH` and
    /// `$env.TUI_HEIGHT`: the area inside its border, or the terminal size
    /// before the first layout.
    pub fn inner_size(&self, id: &str) -> (u16, u16) {
        match self.areas.get(id) {
            Some(area) => (area.width.saturating_sub(2), area.height.saturating_sub(2)),
            None => nu_utils::terminal_size().unwrap_or((80, 24)),
        }
    }

    /// The stack closures run on: the session's, with the widget's pane size
    /// in `TUI_WIDTH` / `TUI_HEIGHT` so `table -w $env.TUI_WIDTH` fits.
    pub fn closure_stack(&self, id: &str) -> Option<Stack> {
        let (_, stack) = self.engine.as_ref()?;
        let (width, height) = self.inner_size(id);
        let mut stack = stack.clone();
        stack.add_env_var(
            "TUI_WIDTH".into(),
            Value::int(width as i64, Span::unknown()),
        );
        stack.add_env_var(
            "TUI_HEIGHT".into(),
            Value::int(height as i64, Span::unknown()),
        );
        Some(stack)
    }

    fn refresh_one(&mut self, id: &str) -> bool {
        let source = self.source_id(id);
        let row = source.as_ref().and_then(|src| self.row_of(src));
        let key = (
            source.unwrap_or_default(),
            row.clone()
                .unwrap_or_else(|| Value::nothing(Span::unknown())),
            self.inner_size(id),
        );
        if self.derived_key.get(id) == Some(&key) {
            return false;
        }
        self.derived_key.insert(id.to_string(), key);
        let Some(widget) = self.widget(id).cloned() else {
            return false;
        };
        let stack = self.closure_stack(id);
        if let WidgetKind::Preview(preview) = &widget.kind {
            let Some(mut state) = self.states.remove(id) else {
                return false;
            };
            preview.refresh(&mut state, row.as_ref(), self, stack.as_ref());
            self.states.insert(id.to_string(), state);
            return true;
        }
        let closure = widget.source.as_ref().and_then(|s| s.closure.clone());
        let value = match (closure, &self.engine, stack, row) {
            (Some(closure), Some((engine_state, _)), Some(stack), Some(row)) => {
                match call_closure(engine_state, &stack, closure, row)
                    .and_then(|d| d.into_value(Span::unknown()))
                {
                    Ok(value) => value,
                    Err(err) => {
                        self.error = Some(truncate_chars(&err.to_string(), 80));
                        Value::nothing(Span::unknown())
                    }
                }
            }
            (_, _, _, Some(row)) => row,
            _ => Value::nothing(Span::unknown()),
        };
        self.derived.insert(id.to_string(), expand_range(value));
        self.invalidate_rows();
        self.clamp_list(id);
        true
    }

    fn clamp_list(&mut self, id: &str) {
        let len = self.rows(id).len();
        if let Some(list) = self.states.get_mut(id).and_then(WidgetState::as_list_mut) {
            list.clamp(len);
        }
    }

    fn clamp_all_lists(&mut self) {
        let ids: Vec<String> = self
            .states
            .iter()
            .filter(|(_, state)| state.as_list().is_some())
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.clamp_list(&id);
        }
    }

    /// Append streamed rows to the shared data. Tables keep their highlight
    /// (a stream that lands while you read should not move it); logs follow
    /// the tail unless scrolled up, resolved when they draw.
    pub fn append_values(&mut self, values: Vec<Value>) {
        if values.is_empty() {
            return;
        }
        let mut rows = as_list(&self.app.data).to_vec();
        rows.extend(values);
        let cap = self.stream_row_cap();
        if rows.len() > cap {
            let drop_n = rows.len() - cap;
            rows.drain(0..drop_n);
        }
        let span = rows.first().map(|v| v.span()).unwrap_or_else(Span::unknown);
        self.app.data = Value::list(rows, span);
        self.invalidate_rows();
        self.clamp_all_lists();
        self.refresh_derived();
    }

    fn stream_row_cap(&self) -> usize {
        const DEFAULT: usize = 10_000;
        self.app
            .iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Log(log) => Some(log.max_lines),
                _ => None,
            })
            .max()
            .unwrap_or(DEFAULT)
            .max(DEFAULT)
    }

    /// Replace the shared data list (from a hook or the refresh closure).
    pub fn replace_data(&mut self, value: Value) {
        self.app.data = as_rows(value);
        for state in self.states.values_mut() {
            if let Some(tree) = state.as_tree_mut() {
                tree.expanded.clear();
                tree.cache.clear();
            }
        }
        self.derived_key.clear();
        self.invalidate_rows();
        self.clamp_all_lists();
        self.refresh_derived();
    }

    /// Run a hook closure with the state record and apply what it returns.
    pub fn run_hook(&mut self, closure: Closure) {
        let Some((engine_state, stack)) = self.engine.clone() else {
            return;
        };
        let state = self.state_record(Span::unknown());
        match hooks::run_hook(&engine_state, &stack, closure, state) {
            HookOutcome::Nothing => self.error = None,
            HookOutcome::Data(value) => {
                self.error = None;
                self.replace_data(value);
            }
            HookOutcome::Submit(selected) => {
                self.outcome = Some(Outcome {
                    action: Action::Submit,
                    selected,
                })
            }
            HookOutcome::Quit => self.quit(),
            HookOutcome::Error(err) => self.error = Some(truncate_chars(&err, 80)),
        }
    }

    // ----- events -------------------------------------------------------

    pub fn layout(&mut self, area: Rect) {
        assign_areas(self, area);
    }

    pub fn enable_dialog(&mut self, screen: Rect, width: Option<u16>, height: Option<u16>) {
        let width = width.unwrap_or_else(|| (screen.width.saturating_mul(3) / 4).max(40));
        let height = height.unwrap_or_else(|| (screen.height.saturating_mul(3) / 4).max(12));
        let rect = clamp_dialog(
            Rect {
                x: screen.x + screen.width.saturating_sub(width) / 2,
                y: screen.y + screen.height.saturating_sub(height) / 2,
                width,
                height,
            },
            screen,
        );
        self.dialog = Some(DialogFrame {
            rect,
            screen,
            drag: None,
        });
    }

    pub fn dialog_content_area(&self, frame: Rect) -> Rect {
        let Some(dialog) = &self.dialog else {
            return frame;
        };
        let area = dialog.rect;
        Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        self.invalidate_rows();
        match event {
            Event::Key(key)
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
            {
                self.handle_key(KeyPress::new(*key));
            }
            Event::Mouse(mouse) => self.handle_mouse(*mouse),
            Event::Resize(w, h) => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.screen = Rect {
                        x: 0,
                        y: 0,
                        width: *w,
                        height: *h,
                    };
                    dialog.rect = clamp_dialog(dialog.rect, dialog.screen);
                }
                // Pane sizes changed: closures that read them run again once
                // the next layout pass has assigned areas.
                self.areas.clear();
                self.refresh_derived();
            }
            _ => {}
        }
    }

    /// Key routing: Ctrl+C, then `tui bind` hooks, then the focused widget,
    /// then the global keys (focus, pages, quit, submit).
    fn handle_key(&mut self, key: KeyPress) {
        if key.chord == "ctrl+c" {
            self.quit();
            return;
        }
        let capturing = self.is_capturing();
        if let Some(bind) = self
            .app
            .binds
            .iter()
            .find(|b| b.chord == key.chord && (!capturing || key.has_modifier()))
        {
            let closure = bind.closure.clone();
            self.apply(vec![Effect::Hook(closure)]);
            return;
        }
        // Page digits win over a `--capture-keys` table, but not over typing.
        if !capturing
            && let Some(n) = digit_page(&key.chord)
            && self.has_tabs()
            && n < self.pages().len()
        {
            self.apply(vec![Effect::Page(n)]);
            return;
        }
        if let Some(id) = self.focused.clone()
            && let Some(effects) = self.dispatch_key(&id, &key)
        {
            self.apply(effects);
            return;
        }
        if let Some(effects) = self.global_key(&key) {
            self.apply(effects);
            return;
        }
        // Arrow keys with nothing useful focused move the first list.
        if matches!(
            key.chord.as_str(),
            "up" | "k" | "down" | "j" | "pageup" | "pagedown" | "home" | "end"
        ) && let Some(id) = self.first_scrollable()
        {
            self.focused = Some(id.clone());
            if let Some(effects) = self.dispatch_key(&id, &key) {
                self.apply(effects);
            }
        }
    }

    fn first_scrollable(&self) -> Option<String> {
        self.visible_widgets()
            .find(|w| w.kind.is_scrollable())
            .map(|w| w.id.clone())
    }

    /// Hand a key to widget `id` with its state checked out of the map.
    fn dispatch_key(&mut self, id: &str, key: &KeyPress) -> Option<Vec<Effect>> {
        let kind = self.kind(id)?.clone();
        let mut state = self.states.remove(id)?;
        let result = kind.handle_key(id, &mut state, key, self);
        self.states.insert(id.to_string(), state);
        // Rows computed while the state was out may have missed it (a
        // captured chord, an expansion); recompute on the next ask.
        self.invalidate_rows();
        result
    }

    fn global_key(&mut self, key: &KeyPress) -> Option<Vec<Effect>> {
        let chord = key.chord.as_str();
        if let Some(id) = self.search_bound_to(chord) {
            return Some(vec![Effect::Focus(id)]);
        }
        // Alt+mnemonic opens a menu-bar item from anywhere.
        if let Some(letter) = chord.strip_prefix("alt+").and_then(single_char)
            && let Some((id, idx)) = self.menu_item_with_mnemonic(letter)
        {
            let kind = self.kind(&id)?.clone();
            let WidgetKind::Menu(menu) = kind else {
                return None;
            };
            let mut state = self.states.remove(&id)?;
            let mut effects = vec![Effect::Focus(id.clone())];
            if let Some(m) = state.as_menu_mut() {
                m.selected = idx;
                effects.extend(menu.open_or_activate(m, self));
            }
            self.states.insert(id, state);
            return Some(effects);
        }
        match chord {
            "tab" => Some(vec![Effect::FocusNext]),
            "shift+tab" => Some(vec![Effect::FocusPrev]),
            "q" | "esc" => Some(vec![Effect::Quit]),
            "[" | "ctrl+left" | "ctrl+shift+tab" => Some(vec![Effect::PagePrev]),
            "]" | "ctrl+right" | "ctrl+tab" => Some(vec![Effect::PageNext]),
            "enter" => Some(vec![Effect::Submit(self.selected_value())]),
            _ => None,
        }
    }

    /// The search box whose `--bind` matches `chord`.
    fn search_bound_to(&self, chord: &str) -> Option<String> {
        let wanted = normalize_bind(chord);
        self.app
            .iter()
            .find(|w| match &w.kind {
                WidgetKind::Search(s) => s
                    .bind
                    .as_deref()
                    .is_some_and(|b| normalize_bind(b) == wanted),
                _ => false,
            })
            .map(|w| w.id.clone())
    }

    fn menu_item_with_mnemonic(&self, letter: char) -> Option<(String, usize)> {
        self.app.iter().find_map(|w| match &w.kind {
            WidgetKind::Menu(menu) => menu
                .item_with_mnemonic(letter)
                .map(|idx| (w.id.clone(), idx)),
            _ => None,
        })
    }

    /// Apply effects in order. Effects may queue more (a hook may submit).
    pub fn apply(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            if self.outcome.is_some() {
                return;
            }
            // Any effect may have changed a filter, a selection, or an
            // expansion behind the row cache.
            self.invalidate_rows();
            match effect {
                Effect::Focus(id) => {
                    if self.kind(&id).is_some_and(|k| k.is_focusable()) {
                        self.focused = Some(id);
                        self.refresh_derived();
                    }
                }
                Effect::FocusNext => self.focus_step(1),
                Effect::FocusPrev => self.focus_step(-1),
                Effect::LeaveText => {
                    self.focused = self
                        .focusable_ids()
                        .into_iter()
                        .find(|id| self.kind(id).is_some_and(|k| !k.is_text_input()))
                        .or_else(|| self.focused.clone());
                    self.refresh_derived();
                }
                Effect::Submit(selected) => {
                    self.outcome = Some(Outcome {
                        action: Action::Submit,
                        selected,
                    })
                }
                Effect::SubmitCurrent => {
                    self.outcome = Some(Outcome {
                        action: Action::Submit,
                        selected: self.current_row(),
                    })
                }
                Effect::Quit => self.quit(),
                Effect::Selected(id) => {
                    self.refresh_derived();
                    if let Some(closure) = self.widget(&id).and_then(|w| w.on_select.clone()) {
                        self.run_hook(closure);
                    }
                }
                Effect::Query => {
                    self.clamp_all_lists();
                    self.refresh_derived();
                }
                Effect::Capture(id, chord) => {
                    match self.scoping_search(&id) {
                        Some(search) => {
                            if let Some(text) = self
                                .states
                                .get_mut(&search)
                                .and_then(WidgetState::as_text_mut)
                            {
                                *text = crate::widget::TextState::new(chord);
                            }
                        }
                        None => {
                            if let Some(list) =
                                self.states.get_mut(&id).and_then(WidgetState::as_list_mut)
                            {
                                list.captured = chord;
                            }
                        }
                    }
                    self.clamp_all_lists();
                    self.refresh_derived();
                }
                Effect::Hook(closure) => self.run_hook(closure),
                Effect::PageNext => self.set_page(self.page + 1),
                Effect::PagePrev => {
                    let n = self.pages().len();
                    self.set_page(self.page.checked_sub(1).unwrap_or(n.saturating_sub(1)))
                }
                Effect::Page(n) => self.set_page(n),
            }
        }
    }

    fn focus_step(&mut self, step: isize) {
        let ids = self.focusable_ids();
        if ids.is_empty() {
            return;
        }
        let n = ids.len() as isize;
        let current = self
            .focused
            .as_ref()
            .and_then(|cur| ids.iter().position(|id| id == cur))
            .map(|i| i as isize);
        let next = match current {
            Some(i) => (i + step).rem_euclid(n),
            None if step > 0 => 0,
            None => n - 1,
        };
        self.focused = Some(ids[next as usize].clone());
        self.refresh_derived();
    }

    fn set_page(&mut self, page: usize) {
        let n = self.pages().len();
        if n == 0 {
            return;
        }
        self.page = page % n;
        self.focused = self.default_focus();
        self.refresh_derived();
    }

    fn quit(&mut self) {
        self.outcome = Some(Outcome {
            action: Action::Quit,
            selected: Value::nothing(Span::unknown()),
        });
    }

    // ----- mouse --------------------------------------------------------

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.handle_dialog_mouse(mouse) {
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.dragging = None;
                if let Some(effects) = self.click_open_dropdown(mouse.column, mouse.row) {
                    self.apply(effects);
                    return;
                }
                for (i, area) in self.tab_areas.clone().iter().enumerate() {
                    if contains(*area, mouse.column, mouse.row) {
                        self.apply(vec![Effect::Page(i)]);
                        return;
                    }
                }
                for handle in self.handles.clone() {
                    if contains(handle.area, mouse.column, mouse.row) {
                        self.dragging = Some((handle.id.clone(), handle.index));
                        self.focused = Some(handle.id);
                        return;
                    }
                }
                if let Some((id, area)) = self.hit_test(mouse.column, mouse.row)
                    && let Some(kind) = self.kind(&id).cloned()
                    && let Some(mut state) = self.states.remove(&id)
                {
                    let effects = kind.click(&id, &mut state, area, mouse.column, mouse.row, self);
                    self.states.insert(id, state);
                    self.invalidate_rows();
                    self.apply(effects);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some((id, index)) = self.dragging.clone() {
                    self.drag_handle(&id, index, mouse);
                }
            }
            MouseEventKind::Up(_) => self.dragging = None,
            MouseEventKind::ScrollDown => self.scroll_at(mouse.column, mouse.row, 1),
            MouseEventKind::ScrollUp => self.scroll_at(mouse.column, mouse.row, -1),
            _ => {}
        }
    }

    /// A click while a dropdown is open goes to the dropdown, or closes it.
    fn click_open_dropdown(&mut self, x: u16, y: u16) -> Option<Vec<Effect>> {
        let (id, menu) = self.app.iter().find_map(|w| match &w.kind {
            WidgetKind::Menu(menu)
                if self
                    .state(&w.id)
                    .and_then(WidgetState::as_menu)
                    .is_some_and(|m| m.open) =>
            {
                Some((w.id.clone(), menu.clone()))
            }
            _ => None,
        })?;
        let bar = *self.areas.get(&id)?;
        let mut state = self.states.remove(&id)?;
        let effects = match state
            .as_menu()
            .and_then(|m| menu.dropdown_rect(m, bar))
            .zip(state.as_menu_mut())
        {
            Some((rect, m)) => {
                let inside = contains(rect, x, y);
                let effects = menu.click_dropdown(m, rect, x, y, self);
                inside.then_some(effects)
            }
            None => None,
        };
        self.states.insert(id, state);
        effects
    }

    fn handle_dialog_mouse(&mut self, mouse: MouseEvent) -> bool {
        let Some(dialog) = &self.dialog else {
            return false;
        };
        let close = dialog.close_area();
        let title = dialog.title_area();
        let resize = dialog.resize_area();
        let rect = dialog.rect;
        let screen = dialog.screen;
        let drag = dialog.drag;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if contains(close, mouse.column, mouse.row) {
                    self.quit();
                    return true;
                }
                if contains(resize, mouse.column, mouse.row) {
                    if let Some(dialog) = self.dialog.as_mut() {
                        dialog.drag = Some(DialogDrag::Resize);
                    }
                    return true;
                }
                if contains(title, mouse.column, mouse.row) {
                    if let Some(dialog) = self.dialog.as_mut() {
                        dialog.drag = Some(DialogDrag::Move {
                            grab_x: mouse.column.saturating_sub(rect.x),
                            grab_y: mouse.row.saturating_sub(rect.y),
                        });
                    }
                    return true;
                }
                !contains(rect, mouse.column, mouse.row)
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let Some(drag) = drag else {
                    return false;
                };
                let next = match drag {
                    DialogDrag::Move { grab_x, grab_y } => Rect {
                        x: mouse.column.saturating_sub(grab_x),
                        y: mouse.row.saturating_sub(grab_y),
                        width: rect.width,
                        height: rect.height,
                    },
                    DialogDrag::Resize => Rect {
                        x: rect.x,
                        y: rect.y,
                        width: mouse
                            .column
                            .saturating_sub(rect.x)
                            .saturating_add(1)
                            .max(20),
                        height: mouse.row.saturating_sub(rect.y).saturating_add(1).max(8),
                    },
                };
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.rect = clamp_dialog(next, screen);
                }
                true
            }
            MouseEventKind::Up(_) => self
                .dialog
                .as_mut()
                .is_some_and(|dialog| dialog.drag.take().is_some()),
            _ => false,
        }
    }

    fn drag_handle(&mut self, id: &str, index: usize, mouse: MouseEvent) {
        let Some(handle) = self
            .handles
            .iter()
            .find(|h| h.id == id && h.index == index)
            .cloned()
        else {
            return;
        };
        let area = handle.split_area;
        let (len, total) = match handle.direction {
            SplitDir::Horizontal => (mouse.column.saturating_sub(area.x), area.width),
            SplitDir::Vertical => (mouse.row.saturating_sub(area.y), area.height),
        };
        // The handle's own edge is relative to the start of the child it
        // sizes, which for later children is not the split's origin.
        let start = handle.child_start;
        if let Some(split) = self.states.get_mut(id).and_then(WidgetState::as_split_mut) {
            SplitWidget::place_handle(split, index, len.saturating_sub(start), total);
        }
    }

    fn scroll_at(&mut self, x: u16, y: u16, delta: i32) {
        let id = self
            .hit_test(x, y)
            .map(|(id, _)| id)
            .or_else(|| self.focused.clone());
        let Some(id) = id else {
            return;
        };
        if let Some(kind) = self.kind(&id).cloned()
            && let Some(mut state) = self.states.remove(&id)
        {
            let effects = kind.scroll(&id, &mut state, delta, self);
            self.states.insert(id, state);
            self.invalidate_rows();
            self.apply(effects);
        }
    }

    /// The smallest non-container widget under the point.
    fn hit_test(&self, x: u16, y: u16) -> Option<(String, Rect)> {
        self.areas
            .iter()
            .filter(|(id, area)| {
                contains(**area, x, y) && !self.kind(id).is_some_and(|k| k.is_container())
            })
            .min_by_key(|(_, area)| area.width as u32 * area.height as u32)
            .map(|(id, area)| (id.clone(), *area))
    }

    // ----- styling ------------------------------------------------------

    pub fn style_computer(&self) -> Option<StyleComputer<'_>> {
        self.engine
            .as_ref()
            .map(|(engine_state, stack)| StyleComputer::from_config(engine_state, stack))
    }

    fn is_path_column(&self, column: &str, row: &Value) -> bool {
        if self.app.path_columns.iter().any(|c| c == column) {
            return true;
        }
        column == "name" && row.as_record().ok().and_then(|r| r.get("type")).is_some()
    }

    pub fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Nothing { .. } => String::new(),
            Value::String { val, .. } => val.clone(),
            other => match &self.engine {
                Some((engine_state, stack)) => {
                    other.to_abbreviated_string(stack.get_config(engine_state).as_ref())
                }
                None => other.to_abbreviated_string(&Config::default()),
            },
        }
    }

    /// Text and style of one table cell.
    pub fn styled_cell(
        &self,
        row: &Value,
        column: &str,
        computer: Option<&StyleComputer>,
    ) -> (String, Style) {
        let value = match row {
            Value::Record { val, .. } => val
                .get(column)
                .cloned()
                .unwrap_or_else(|| Value::nothing(Span::unknown())),
            other => other.clone(),
        };
        let text = self.format_value(&value);
        if self.use_ls_colors
            && self.is_path_column(column, row)
            && let Some(path) = value.as_str().ok()
        {
            return (text, self.theme.path_cell(path, &self.cwd, &self.ls_colors));
        }
        let style = match computer {
            Some(computer) => self.theme.value_cell(&value, computer),
            None => self.theme.text(),
        };
        (text, style)
    }

    /// Style of a tree node or list item: `LS_COLORS` for paths, else the
    /// value's type color.
    pub fn row_style(&self, row: &Value) -> Style {
        if self.use_ls_colors
            && self.is_path_column("name", row)
            && let Some(path) = crate::widgets::preview::row_path(row)
        {
            return self.theme.path_cell(&path, &self.cwd, &self.ls_colors);
        }
        match self.style_computer() {
            Some(computer) => self.theme.value_cell(row, &computer),
            None => self.theme.text(),
        }
    }

    // ----- results ------------------------------------------------------

    /// Text of the first search box with a query.
    fn active_query(&self) -> String {
        self.app
            .iter()
            .filter(|w| matches!(w.kind, WidgetKind::Search(_)))
            .filter_map(|w| self.state(&w.id).and_then(WidgetState::as_text))
            .map(|t| t.text.clone())
            .find(|q| !q.is_empty())
            .unwrap_or_default()
    }

    pub fn status_text(&self) -> String {
        let extra = self.live_status_bits();
        let label = self.app.widgets.iter().find_map(|w| match &w.kind {
            WidgetKind::Label(l) if l.slot == Slot::Status => Some(l.text(&w.id, self)),
            _ => None,
        });
        match label {
            Some(text) if !extra.is_empty() => format!("{text}  {extra}"),
            Some(text) => text,
            None => extra,
        }
    }

    fn live_status_bits(&self) -> String {
        let mut bits = Vec::new();
        if let Some(err) = &self.error {
            bits.push(format!("error:{err}"));
        }
        if let Some(id) = &self.focused {
            bits.push(format!("focus:{id}"));
        }
        let query = self.active_query();
        if !query.is_empty() {
            bits.push(format!("filter:{query}"));
        }
        let n = as_list(&self.app.data).len();
        if n > 0 || self.stream_live {
            let live = if self.stream_live { " live" } else { "" };
            bits.push(format!("rows:{n}{live}"));
        }
        let pages = self.pages();
        if pages.len() > 1
            && let Some(page) = pages.get(self.page)
        {
            bits.push(format!(
                "page:{}/{} {}",
                self.page + 1,
                pages.len(),
                page.title
            ));
        }
        bits.push("tab:focus  [:page  q:quit".into());
        bits.join("  ")
    }

    /// The state record every hook receives and `tui run` returns:
    /// `{action, focused, selected, page, values, rows, live}`. `values`
    /// holds every widget's state by id.
    pub fn state_record(&self, span: Span) -> Value {
        Value::record(self.result_fields(span, None), span)
    }

    pub fn result_fields(&self, span: Span, screen: Option<String>) -> Record {
        let mut rec = Record::new();
        let action = match self.outcome.as_ref().map(|o| o.action) {
            Some(Action::Submit) => "submit",
            Some(Action::Quit) => "quit",
            None => "render",
        };
        rec.insert("action", Value::string(action, span));
        rec.insert(
            "focused",
            Value::string(self.focused.clone().unwrap_or_default(), span),
        );
        let selected = self
            .outcome
            .as_ref()
            .map(|o| o.selected.clone())
            .unwrap_or_else(|| self.selected_value());
        rec.insert("selected", selected);
        rec.insert("page", Value::int(self.page as i64, span));
        let mut values = Record::new();
        for w in self.app.iter() {
            if let Some(state) = self.state(&w.id) {
                values.insert(w.id.clone(), w.kind.value(&w.id, state, self));
            }
        }
        rec.insert("values", Value::record(values, span));
        rec.insert(
            "rows",
            Value::int(as_list(&self.app.data).len() as i64, span),
        );
        rec.insert("live", Value::bool(self.stream_live, span));
        if let Some(screen) = screen {
            rec.insert("screen", Value::string(screen, span));
        }
        rec
    }

    /// Resolved state for `tui debug`: the widget tree with layout rects,
    /// focusability, per-widget debug fields, search scope and source; the
    /// focus order; and the pages. Run `layout` first so rects exist.
    pub fn debug_record(&self, span: Span) -> Record {
        let extend = |w: &Widget, rec: &mut Record| {
            if let Some(area) = self.areas.get(&w.id) {
                let mut r = Record::new();
                r.insert("x", Value::int(area.x as i64, span));
                r.insert("y", Value::int(area.y as i64, span));
                r.insert("width", Value::int(area.width as i64, span));
                r.insert("height", Value::int(area.height as i64, span));
                rec.insert("rect", Value::record(r, span));
            }
            rec.insert("focusable", Value::bool(w.kind.is_focusable(), span));
            if let Some(state) = self.state(&w.id) {
                w.kind.debug(&w.id, state, self, rec);
            }
            if w.kind.is_scrollable() {
                rec.insert(
                    "search_scope",
                    match self.scoping_search(&w.id) {
                        Some(id) => Value::string(id, span),
                        None => Value::nothing(span),
                    },
                );
            }
            if w.source.is_some() {
                rec.insert(
                    "source",
                    match self.source_id(&w.id) {
                        Some(id) => Value::string(id, span),
                        None => Value::nothing(span),
                    },
                );
            }
        };
        let mut rec = Record::new();
        rec.insert(
            "widgets",
            Value::list(
                self.app
                    .widgets
                    .iter()
                    .map(|w| widget_to_record(w, span, &extend))
                    .collect(),
                span,
            ),
        );
        let mut focus = Record::new();
        focus.insert(
            "order",
            Value::list(
                self.focusable_ids()
                    .into_iter()
                    .map(|id| Value::string(id, span))
                    .collect(),
                span,
            ),
        );
        focus.insert(
            "default",
            match self.default_focus() {
                Some(id) => Value::string(id, span),
                None => Value::nothing(span),
            },
        );
        rec.insert("focus", Value::record(focus, span));
        rec.insert(
            "pages",
            Value::list(
                self.pages()
                    .into_iter()
                    .map(|p| {
                        let mut r = Record::new();
                        r.insert("title", Value::string(p.title, span));
                        r.insert(
                            "id",
                            match p.id {
                                Some(id) => Value::string(id, span),
                                None => Value::nothing(span),
                            },
                        );
                        Value::record(r, span)
                    })
                    .collect(),
                span,
            ),
        );
        rec
    }
}

/// A bounded range becomes the list it stands for; anything else is kept.
fn expand_range(value: Value) -> Value {
    let span = value.span();
    match value {
        Value::Range { val, .. } if val.is_bounded() => Value::list(
            val.into_range_iter(span, nu_protocol::Signals::empty())
                .collect(),
            span,
        ),
        other => other,
    }
}

/// A hook's output as a data list: lists as they are, bounded ranges
/// expanded, `null` as no rows, and any other value as one row.
fn as_rows(value: Value) -> Value {
    match expand_range(value) {
        list @ Value::List { .. } => list,
        other if other.is_nothing() => Value::list(Vec::new(), other.span()),
        other => {
            let span = other.span();
            Value::list(vec![other], span)
        }
    }
}

/// The rows of a data value: a list's items, nothing for `null`, or the
/// value itself.
pub fn as_list(value: &Value) -> &[Value] {
    match value {
        Value::List { vals, .. } => vals,
        Value::Nothing { .. } => &[],
        other => std::slice::from_ref(other),
    }
}

fn digit_page(chord: &str) -> Option<usize> {
    let c = single_char(chord)?;
    (c.is_ascii_digit() && c != '0').then(|| (c as u8 - b'1') as usize)
}

pub(crate) fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

pub(crate) fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Size, Source};
    use crate::widgets::r#box::BoxWidget;
    use crate::widgets::label::LabelWidget;
    use crate::widgets::menu::{MenuItem, MenuWidget};
    use crate::widgets::preview::PreviewWidget;
    use crate::widgets::search::SearchWidget;
    use crate::widgets::select::SelectWidget;
    use crate::widgets::split::SplitWidget;
    use crate::widgets::tab::TabWidget;
    use crate::widgets::table::TableWidget;
    use crate::widgets::tree::TreeWidget;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn label(id: &str, text: &str) -> Widget {
        Widget::new(
            id,
            WidgetKind::Label(LabelWidget {
                text: text.into(),
                slot: Slot::Content,
            }),
        )
    }

    fn table(id: &str, columns: &[&str]) -> Widget {
        Widget::new(
            id,
            WidgetKind::Table(TableWidget {
                columns: columns.iter().map(|s| s.to_string()).collect(),
                capture_keys: false,
                multi: false,
                index: false,
            }),
        )
    }

    fn search(id: &str, bind: Option<&str>) -> Widget {
        Widget::new(
            id,
            WidgetKind::Search(SearchWidget {
                placeholder: String::new(),
                bind: bind.map(str::to_string),
                fuzzy: false,
                case_sensitive: false,
                columns: Vec::new(),
            }),
        )
    }

    fn split(id: &str, direction: SplitDir, children: Vec<Widget>) -> Widget {
        let mut w = Widget::new(
            id,
            WidgetKind::Split(SplitWidget {
                direction,
                sizes: Vec::new(),
            }),
        );
        w.children = children;
        w
    }

    fn tab(id: &str, title: &str, children: Vec<Widget>) -> Widget {
        let mut w = Widget::new(
            id,
            WidgetKind::Tab(TabWidget {
                title: title.into(),
            }),
        );
        w.children = children;
        w
    }

    fn preview(id: &str) -> Widget {
        Widget::new(
            id,
            WidgetKind::Preview(PreviewWidget {
                max_bytes: 1024,
                transform: None,
            }),
        )
    }

    fn record_from(pairs: &[(&str, &str)]) -> Record {
        let mut rec = Record::new();
        for (k, v) in pairs {
            rec.insert(*k, Value::test_string(*v));
        }
        rec
    }

    fn rows(names: &[&str]) -> Value {
        Value::test_list(
            names
                .iter()
                .map(|n| Value::test_record(record_from(&[("name", n)])))
                .collect(),
        )
    }

    fn app(widgets: Vec<Widget>, data: Value) -> TuiApp {
        let mut app = TuiApp::new();
        app.widgets = widgets;
        app.data = data;
        app
    }

    fn table_app() -> TuiApp {
        app(
            vec![search("search-0", Some("/")), table("table-0", &["name"])],
            rows(&["alpha", "beta", "gamma"]),
        )
    }

    fn press(session: &mut Session, code: KeyCode, mods: KeyModifiers) {
        session.handle_event(&Event::Key(KeyEvent::new(code, mods)));
    }

    fn key(session: &mut Session, c: char) {
        press(session, KeyCode::Char(c), KeyModifiers::NONE);
    }

    fn click(session: &mut Session, x: u16, y: u16) {
        session.handle_event(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
    }

    fn drag(session: &mut Session, x: u16, y: u16) {
        session.handle_event(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
    }

    fn selected_index(session: &Session, id: &str) -> usize {
        session
            .state(id)
            .and_then(WidgetState::as_list)
            .map(|l| l.selected)
            .unwrap_or(usize::MAX)
    }

    fn frame() -> Rect {
        Rect::new(0, 0, 80, 24)
    }

    #[test]
    fn down_moves_table_selection() {
        let mut session = Session::new(table_app());
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(selected_index(&session, "table-0"), 1);
    }

    #[test]
    fn q_quits_when_not_editing() {
        let mut session = Session::new(table_app());
        key(&mut session, 'q');
        assert!(matches!(
            session.outcome.as_ref().map(|o| o.action),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn q_types_when_search_focused() {
        let mut session = Session::new(table_app());
        key(&mut session, '/');
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        key(&mut session, 'q');
        assert!(session.outcome.is_none());
        assert_eq!(session.filter_for("table-0").query, "q");
    }

    #[test]
    fn search_filters_table_and_esc_clears() {
        let mut session = Session::new(table_app());
        key(&mut session, '/');
        key(&mut session, 'b');
        key(&mut session, 'e');
        assert_eq!(session.rows("table-0").len(), 1);
        press(&mut session, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(session.rows("table-0").len(), 3);
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        press(&mut session, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
    }

    #[test]
    fn tab_cycles_focus() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        press(&mut session, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
    }

    #[test]
    fn enter_submits_selected_row() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        press(&mut session, KeyCode::Enter, KeyModifiers::NONE);
        let outcome = session.outcome.clone().expect("outcome");
        assert_eq!(outcome.action, Action::Submit);
        let name = outcome
            .selected
            .as_record()
            .ok()
            .and_then(|r| r.get("name"))
            .and_then(|v| v.as_str().ok())
            .map(str::to_string);
        assert_eq!(name.as_deref(), Some("beta"));
    }

    #[test]
    fn enter_in_search_submits_filtered_row() {
        let mut session = Session::new(table_app());
        key(&mut session, '/');
        key(&mut session, 'g');
        press(&mut session, KeyCode::Enter, KeyModifiers::NONE);
        let outcome = session.outcome.clone().expect("outcome");
        let name = outcome
            .selected
            .as_record()
            .ok()
            .and_then(|r| r.get("name"))
            .and_then(|v| v.as_str().ok())
            .map(str::to_string);
        assert_eq!(name.as_deref(), Some("gamma"));
    }

    #[test]
    fn values_record_holds_every_widget_by_id() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        let record = session.state_record(Span::test_data());
        let values = record
            .as_record()
            .ok()
            .and_then(|r| r.get("values"))
            .and_then(|v| v.as_record().ok())
            .cloned()
            .expect("values");
        assert_eq!(
            values.get("search-0").and_then(|v| v.as_str().ok()),
            Some("")
        );
        let index = values
            .get("table-0")
            .and_then(|v| v.as_record().ok())
            .and_then(|r| r.get("index"))
            .and_then(|v| v.as_int().ok());
        assert_eq!(index, Some(1));
    }

    fn keybindings_app(with_search: bool) -> TuiApp {
        let rows = Value::test_list(vec![
            Value::test_record(record_from(&[
                ("name", "history"),
                ("modifier", "control"),
                ("keycode", "char_r"),
            ])),
            Value::test_record(record_from(&[
                ("name", "clear"),
                ("modifier", "control"),
                ("keycode", "char_l"),
            ])),
        ]);
        let mut widgets = Vec::new();
        if with_search {
            widgets.push(search("search-0", None));
        }
        widgets.push(Widget::new(
            "table-0",
            WidgetKind::Table(TableWidget {
                columns: vec!["name".into()],
                capture_keys: true,
                multi: false,
                index: false,
            }),
        ));
        app(widgets, rows)
    }

    #[test]
    fn captured_chord_filters_rows_without_a_search_box() {
        let mut session = Session::new(keybindings_app(false));
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(session.rows("table-0").len(), 1);
        assert_eq!(session.filter_for("table-0").query, "ctrl+r");
    }

    #[test]
    fn captured_chord_goes_to_the_scoping_search_box() {
        let mut session = Session::new(keybindings_app(true));
        press(&mut session, KeyCode::Char('l'), KeyModifiers::CONTROL);
        let query = session
            .state("search-0")
            .and_then(WidgetState::as_text)
            .map(|t| t.text.clone());
        assert_eq!(query.as_deref(), Some("ctrl+l"));
        assert_eq!(session.rows("table-0").len(), 1);
    }

    #[test]
    fn preview_follows_table_selection_without_stealing_focus() {
        let mut session = Session::new(app(
            vec![split(
                "split-0",
                SplitDir::Horizontal,
                vec![table("table-0", &["name"]), preview("preview-0")],
            )],
            rows(&["one.txt", "two.txt"]),
        ));
        session.layout(frame());
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        let title = session
            .state("preview-0")
            .and_then(WidgetState::as_preview)
            .map(|p| p.title.clone());
        assert_eq!(title.as_deref(), Some("two.txt"));
        assert_eq!(session.source_id("preview-0").as_deref(), Some("table-0"));
    }

    #[test]
    fn preview_follows_the_nearest_table_in_its_split() {
        let mut session = Session::new(app(
            vec![
                table("table-a", &["name"]),
                split(
                    "split-0",
                    SplitDir::Horizontal,
                    vec![table("table-b", &["name"]), preview("preview-0")],
                ),
            ],
            rows(&["x"]),
        ));
        session.focused = None;
        assert_eq!(session.source_id("preview-0").as_deref(), Some("table-b"));
    }

    #[test]
    fn child_data_wins_over_shared_data() {
        let mut detail = table("table-1", &["name"]);
        detail.data = Some(rows(&["own-a", "own-b", "own-c", "own-d"]));
        let session = Session::new(app(
            vec![split(
                "split-0",
                SplitDir::Horizontal,
                vec![table("table-0", &["name"]), detail],
            )],
            rows(&["shared"]),
        ));
        assert_eq!(session.rows("table-0").len(), 1);
        assert_eq!(session.rows("table-1").len(), 4);
    }

    #[test]
    fn descendants_inherit_a_containers_data() {
        let mut container = split(
            "split-0",
            SplitDir::Vertical,
            vec![table("table-0", &["name"]), table("table-1", &["name"])],
        );
        container.data = Some(rows(&["a", "b"]));
        let session = Session::new(app(vec![container], Value::test_nothing()));
        assert_eq!(session.rows("table-0").len(), 2);
        assert_eq!(session.rows("table-1").len(), 2);
    }

    #[test]
    fn from_without_closure_shows_the_source_row() {
        let mut follower = table("table-1", &["name"]);
        follower.source = Some(Source {
            from: Some("table-0".into()),
            closure: None,
        });
        let mut session = Session::new(app(
            vec![split(
                "split-0",
                SplitDir::Horizontal,
                vec![table("table-0", &["name"]), follower],
            )],
            rows(&["first", "second"]),
        ));
        let name = |s: &Session| {
            s.rows("table-1")
                .first()
                .and_then(|v| v.as_record().ok().and_then(|r| r.get("name")).cloned())
                .and_then(|v| v.as_str().ok().map(str::to_string))
        };
        assert_eq!(name(&session).as_deref(), Some("first"));
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(name(&session).as_deref(), Some("second"));
    }

    #[test]
    fn dialog_close_button_quits() {
        let mut session = Session::new(table_app());
        session.enable_dialog(frame(), Some(40), Some(10));
        let close = session
            .dialog
            .as_ref()
            .map(|d| d.close_area())
            .expect("dialog");
        click(&mut session, close.x + 1, close.y);
        assert!(matches!(
            session.outcome.as_ref().map(|o| o.action),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn dialog_title_drag_moves_and_corner_drag_resizes() {
        let mut session = Session::new(table_app());
        session.enable_dialog(frame(), Some(40), Some(10));
        let before = session.dialog.as_ref().map(|d| d.rect).expect("dialog");
        click(&mut session, before.x + 2, before.y);
        drag(&mut session, before.x + 7, before.y + 3);
        let moved = session.dialog.as_ref().map(|d| d.rect).expect("dialog");
        assert_eq!((moved.x, moved.y), (before.x + 5, before.y + 3));
        session.handle_event(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }));
        let corner = session
            .dialog
            .as_ref()
            .map(|d| d.resize_area())
            .expect("dialog");
        click(&mut session, corner.x + 1, corner.y + 1);
        drag(&mut session, corner.x + 6, corner.y + 4);
        let resized = session.dialog.as_ref().map(|d| d.rect).expect("dialog");
        assert_eq!(resized.width, moved.width + 5);
        assert_eq!(resized.height, moved.height + 3);
    }

    #[test]
    fn tree_right_expands_nested_record() {
        let mut inner = Record::new();
        inner.insert("b", Value::test_int(1));
        let mut rec = Record::new();
        rec.insert("a", Value::test_record(inner));
        let mut session = Session::new(app(
            vec![Widget::new(
                "tree-0",
                WidgetKind::Tree(TreeWidget {
                    walk: false,
                    column: "name".into(),
                    multi: false,
                }),
            )],
            Value::test_record(rec),
        ));
        assert_eq!(session.rows("tree-0").len(), 1);
        press(&mut session, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(session.rows("tree-0").len(), 2);
        press(&mut session, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(session.rows("tree-0").len(), 1);
    }

    #[test]
    fn tab_digit_jumps_page_and_focus_follows() {
        let mut session = Session::new(app(
            vec![
                tab("tab-0", "one", vec![label("label-0", "ONE")]),
                tab("tab-1", "two", vec![table("table-0", &["name"])]),
            ],
            rows(&["x"]),
        ));
        assert_eq!(session.page, 0);
        key(&mut session, '2');
        assert_eq!(session.page, 1);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        key(&mut session, '9');
        assert_eq!(session.page, 1, "out-of-range digits are ignored");
    }

    #[test]
    fn box_is_a_bordered_group_not_a_page() {
        let mut boxed = Widget::new(
            "box-0",
            WidgetKind::Box(BoxWidget {
                title: "grp".into(),
            }),
        );
        boxed.children = vec![table("table-0", &["name"])];
        let mut session = Session::new(app(vec![boxed], rows(&["x"])));
        session.layout(frame());
        assert!(!session.has_tabs());
        let outer = session.areas["box-0"];
        let inner = session.areas["table-0"];
        assert_eq!(inner.x, outer.x + 1);
        assert_eq!(inner.width, outer.width - 2);
    }

    #[test]
    fn splitter_drag_resizes_first_child() {
        let mut session = Session::new(app(
            vec![split(
                "split-0",
                SplitDir::Horizontal,
                vec![table("table-0", &["name"]), table("table-1", &["name"])],
            )],
            rows(&["x"]),
        ));
        session.layout(frame());
        let handle = session.handles.first().cloned().expect("handle");
        click(&mut session, handle.area.x, handle.area.y);
        drag(&mut session, 20, handle.area.y);
        session.layout(frame());
        let left = session.areas["table-0"];
        assert!((18..=22).contains(&left.width), "got {left:?}");
    }

    #[test]
    fn split_sizes_honour_fixed_lengths() {
        let mut w = Widget::new(
            "split-0",
            WidgetKind::Split(SplitWidget {
                direction: SplitDir::Horizontal,
                sizes: vec![Size::Length(20), Size::Fill(1), Size::Fill(1)],
            }),
        );
        w.children = vec![
            table("table-0", &["name"]),
            table("table-1", &["name"]),
            table("table-2", &["name"]),
        ];
        let mut session = Session::new(app(vec![w], rows(&["x"])));
        session.layout(frame());
        assert_eq!(session.areas["table-0"].width, 20);
        assert_eq!(session.handles.len(), 2);
        let b = session.areas["table-1"].width;
        let c = session.areas["table-2"].width;
        assert!(b.abs_diff(c) <= 1, "fills share the rest: {b} vs {c}");
    }

    #[test]
    fn log_follows_the_tail_until_scrolled_up() {
        let mut session = Session::new(app(
            vec![Widget::new(
                "log-0",
                WidgetKind::Log(crate::widgets::log::LogWidget { max_lines: 100 }),
            )],
            Value::test_list((0..50).map(|i| Value::test_string(i.to_string())).collect()),
        ));
        session.layout(frame());
        session.append_values(vec![Value::test_string("50")]);
        let log = |s: &Session| s.state("log-0").and_then(WidgetState::as_log).cloned();
        assert!(log(&session).is_some_and(|l| l.follow), "follows the tail");
        // 51 lines in a 22-row viewport: the tail sits at offset 29.
        press(&mut session, KeyCode::PageUp, KeyModifiers::NONE);
        let paused = log(&session).expect("log");
        assert_eq!(paused.scroll, 19);
        assert!(!paused.follow);
        press(&mut session, KeyCode::End, KeyModifiers::NONE);
        assert!(
            log(&session).is_some_and(|l| l.follow),
            "end resumes following"
        );
    }

    #[test]
    fn streamed_rows_do_not_move_the_table_highlight() {
        let mut session = Session::new(app(
            vec![table("table-0", &["name"])],
            Value::test_nothing(),
        ));
        session.append_values(as_list(&rows(&["a"])).to_vec());
        session.append_values(as_list(&rows(&["b", "c"])).to_vec());
        assert_eq!(selected_index(&session, "table-0"), 0);
        assert_eq!(session.rows("table-0").len(), 3);
    }

    fn button(id: &str, label: &str) -> Widget {
        Widget::new(
            id,
            WidgetKind::Button(crate::widgets::button::ButtonWidget {
                label: label.into(),
                action: None,
            }),
        )
    }

    #[test]
    fn adjacent_buttons_share_a_row() {
        let mut session = Session::new(app(
            vec![
                label("label-0", "Delete?"),
                button("button-0", "Yes"),
                button("button-1", "No"),
                table("table-0", &["name"]),
            ],
            rows(&["x"]),
        ));
        session.layout(frame());
        let yes = session.areas["button-0"];
        let no = session.areas["button-1"];
        assert_eq!(yes.y, 1);
        assert_eq!(no.y, 1, "same row");
        assert_eq!(no.x, yes.x + yes.width, "packed left to right");
        assert_eq!(session.areas["table-0"].y, 2, "the row is one line tall");
        session.focused = Some("button-0".into());
        press(&mut session, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("button-1"));
    }

    #[test]
    fn a_split_of_fixed_leaves_takes_only_their_height() {
        let mut session = Session::new(app(
            vec![
                split(
                    "split-0",
                    SplitDir::Vertical,
                    vec![button("button-0", "Yes"), button("button-1", "No")],
                ),
                table("table-0", &["name"]),
            ],
            rows(&["x"]),
        ));
        session.layout(frame());
        assert_eq!(session.areas["split-0"].height, 2);
        assert_eq!(session.areas["button-0"].y, 0);
        assert_eq!(session.areas["button-1"].y, 1, "stacked, no handle gap");
        assert!(session.handles.is_empty(), "fixed splits are not resizable");
        assert_eq!(session.areas["table-0"].y, 2);
    }

    #[test]
    fn focus_flag_wins_over_the_default_order() {
        let mut boxed = search("search-0", None);
        boxed.focus = true;
        let mut session = Session::new(app(
            vec![boxed, table("table-0", &["name"])],
            rows(&["alpha", "beta"]),
        ));
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        key(&mut session, 'b');
        assert_eq!(session.rows("table-0").len(), 1, "typing filters at once");
    }

    #[test]
    fn page_digits_beat_a_capturing_table() {
        let mut capturing = table("table-0", &["a"]);
        capturing.kind = WidgetKind::Table(TableWidget {
            columns: vec!["a".into()],
            capture_keys: true,
            multi: false,
            index: false,
        });
        let mut session = Session::new(app(
            vec![
                tab("tab-0", "one", vec![capturing]),
                tab("tab-1", "two", vec![label("label-0", "x")]),
            ],
            rows(&["r"]),
        ));
        key(&mut session, '2');
        assert_eq!(session.page, 1);
    }

    #[test]
    fn root_search_takes_a_chrome_slot_only_once() {
        let mut session = Session::new(table_app());
        session.layout(frame());
        assert_eq!(session.areas["search-0"].y, 0);
        assert_eq!(session.areas["search-0"].height, 3);
        assert_eq!(session.areas["table-0"].y, 3);
        let order = session.focusable_ids();
        assert_eq!(
            order.iter().filter(|id| *id == "search-0").count(),
            1,
            "the search box is one tab stop: {order:?}"
        );
    }

    fn menu_app() -> TuiApp {
        let items = vec![
            MenuItem::new(
                "&File",
                vec![
                    MenuItem::new("&Open", Vec::new(), None),
                    MenuItem::new("&Quit", Vec::new(), None),
                ],
                None,
            ),
            MenuItem::new("&Edit", Vec::new(), None),
        ];
        app(
            vec![
                Widget::new("menu-0", WidgetKind::Menu(MenuWidget { items })),
                table("table-0", &["name"]),
            ],
            rows(&["a", "b"]),
        )
    }

    #[test]
    fn alt_mnemonic_opens_dropdown_and_letter_submits_item() {
        let mut session = Session::new(menu_app());
        session.layout(frame());
        press(&mut session, KeyCode::Char('f'), KeyModifiers::ALT);
        assert!(
            session
                .state("menu-0")
                .and_then(WidgetState::as_menu)
                .is_some_and(|m| m.open)
        );
        key(&mut session, 'q');
        let selected = session.outcome.clone().expect("outcome").selected;
        let item = selected
            .as_record()
            .ok()
            .and_then(|r| r.get("item"))
            .and_then(|v| v.as_str().ok())
            .map(str::to_string);
        assert_eq!(item.as_deref(), Some("Quit"));
    }

    #[test]
    fn esc_closes_dropdown_without_quitting_and_plain_q_then_quits() {
        let mut session = Session::new(menu_app());
        press(&mut session, KeyCode::Char('f'), KeyModifiers::ALT);
        press(&mut session, KeyCode::Esc, KeyModifiers::NONE);
        assert!(session.outcome.is_none());
        assert!(
            !session
                .state("menu-0")
                .and_then(WidgetState::as_menu)
                .is_some_and(|m| m.open)
        );
        key(&mut session, 'q');
        assert!(session.outcome.is_some());
    }

    #[test]
    fn bar_item_without_dropdown_submits_its_name() {
        let mut session = Session::new(menu_app());
        press(&mut session, KeyCode::Char('e'), KeyModifiers::ALT);
        let selected = session.outcome.clone().expect("outcome").selected;
        assert_eq!(selected.as_str().ok(), Some("Edit"));
    }

    #[test]
    fn nested_search_filters_only_its_container() {
        let mut session = Session::new(app(
            vec![split(
                "split-0",
                SplitDir::Horizontal,
                vec![
                    split(
                        "split-1",
                        SplitDir::Vertical,
                        vec![search("search-0", Some("/")), table("table-a", &["name"])],
                    ),
                    table("table-b", &["name"]),
                ],
            )],
            rows(&["alpha", "beta"]),
        ));
        key(&mut session, '/');
        key(&mut session, 'z');
        assert_eq!(session.rows("table-a").len(), 0);
        assert_eq!(session.rows("table-b").len(), 2);
        assert_eq!(
            session.scoping_search("table-a").as_deref(),
            Some("search-0")
        );
        assert_eq!(session.scoping_search("table-b"), None);
    }

    #[test]
    fn select_multi_toggles_with_space() {
        let mut session = Session::new(app(
            vec![Widget::new(
                "select-0",
                WidgetKind::Select(SelectWidget {
                    items: vec![
                        Value::test_string("small"),
                        Value::test_string("medium"),
                        Value::test_string("large"),
                    ],
                    multi: true,
                    index: false,
                    display: None,
                }),
            )],
            Value::test_nothing(),
        ));
        key(&mut session, ' ');
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        key(&mut session, ' ');
        press(&mut session, KeyCode::Enter, KeyModifiers::NONE);
        let selected = session.outcome.clone().expect("outcome").selected;
        let names: Vec<String> = selected
            .as_list()
            .expect("list")
            .iter()
            .filter_map(|v| v.as_str().ok().map(str::to_string))
            .collect();
        assert_eq!(names, vec!["small".to_string(), "medium".to_string()]);
    }
}
