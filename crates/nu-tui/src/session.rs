//! Orchestration of a running TUI: focus, pages, data resolution, hooks,
//! mouse routing, and the result record. Widget-specific behaviour lives in
//! `widgets/*`; the session only applies the [`Effect`]s they return.
use crate::app::{TuiApp, merge_context, widget_to_record};
use crate::filter::Filter;
use crate::hooks::{self, HookAction, HookOutcome, call_closure};
use crate::image::Images;
use crate::keys::{KeyPress, normalize_bind, single_char};
use crate::layout::{SplitterHandle, assign_areas, subtree_ids};
use crate::stream::trim_front;
use crate::theme::{FileFacts, FileKind, FileStyle, Theme};
use crate::tree::TreeRow;
use crate::widget::{Effect, Size, Widget, WidgetKind, WidgetState};
use crate::widgets::label::Slot;
use crate::widgets::split::{SplitDir, SplitWidget};
use crossterm::event::{Event, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use nu_color_config::StyleComputer;
use nu_engine::get_columns;
use nu_protocol::engine::{Closure, EngineState, Stack};
use nu_protocol::{Config, Record, Span, Value};
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
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

/// The rows a list widget shows. Unfiltered rows are borrowed straight
/// from the data, so asking for them costs nothing; filtered rows are a
/// copy shared through the session's cache.
pub enum Rows<'a> {
    Borrowed(&'a [Value]),
    Shared(Rc<Vec<Value>>),
}

impl Deref for Rows<'_> {
    type Target = [Value];

    fn deref(&self) -> &[Value] {
        match self {
            Rows::Borrowed(rows) => rows,
            Rows::Shared(rows) => rows,
        }
    }
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

/// A hook that wants the terminal, from `{action: suspend}`.
#[derive(Debug, Clone)]
pub struct Suspend {
    pub hook: Closure,
    /// Where its output starts; the TUI stays on screen round it.
    pub area: Option<Rect>,
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

/// Whether following `from` would make `id` follow itself: `from` is `id`,
/// or its chain of sources reaches `id`. The chains in `sources` end,
/// because [`Session::sources`] adds no link that closes a loop.
fn leads_to(sources: &HashMap<String, String>, from: &str, id: &str) -> bool {
    std::iter::successors(Some(from), |at| sources.get(*at).map(String::as_str)).any(|at| at == id)
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
    /// The (source id, source row, pane size, view context) each derived
    /// value was computed from, so unchanged selections do not re-run
    /// closures, and a resize or a context write does.
    derived_key: HashMap<String, (String, Value, (u16, u16), Value)>,
    pub page: usize,
    pub focused: Option<String>,
    /// The chords of a bind sequence typed so far (`m` of `m s`). The key
    /// panel lists what can follow; a key that continues no bind drops it.
    pub pending: Vec<String>,
    /// A hook waiting for the terminal (`{action: suspend}`); the runtime
    /// takes it, hands the terminal over, and runs it.
    pub suspend: Option<Suspend>,
    /// What the last layout covered: the screen, or the dialog's content.
    pub laid_out: Rect,
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
    /// A hook replaced the shared data, so streamed rows no longer go there.
    data_from_hook: bool,
    pub dialog: Option<DialogFrame>,
    pub theme: Theme,
    /// Last failure from a hook or source closure; shown on the status bar.
    pub error: Option<String>,
    /// The graphics protocol previews encode images with. `None` until
    /// [`Session::enable_images`], or when `$env.config.tui.image_protocol`
    /// is `none`: previews then describe an image instead of drawing it.
    pub images: Option<Images>,
    /// Filtered rows per widget, kept until data, a filter, or a selection
    /// changes. A frame asks for rows several times; a 100k-row list must
    /// not be filtered and cloned each time.
    rows_cache: RefCell<HashMap<String, Rc<Vec<Value>>>>,
    /// Flattened tree rows per tree widget, kept on the same terms.
    tree_cache: RefCell<HashMap<String, Rc<Vec<TreeRow>>>>,
}

/// The file's mode as `ls -l` shows it. Windows has none.
fn mode_string(meta: &std::fs::Metadata) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Some(umask::Mode::from(meta.permissions().mode()).to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        None
    }
}

impl Session {
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
        let theme = match &engine {
            Some((engine_state, stack)) => Theme::from_config(engine_state, stack, &app.theme),
            None => Theme::default(),
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
            data_from_hook: false,
            dialog: None,
            pending: Vec::new(),
            suspend: None,
            laid_out: Rect::default(),
            theme,
            error: None,
            images: None,
            rows_cache: RefCell::new(HashMap::new()),
            tree_cache: RefCell::new(HashMap::new()),
        };
        // Derived values wait for the first layout pass, which knows the
        // pane sizes their closures see.
        session.focused = session.default_focus();
        session
    }

    // ----- tree queries -------------------------------------------------

    /// The widget with `id`, found by its child-index path. The tree does not
    /// change after [`Session::new`], so `paths` stays valid, and the lookup
    /// walks one branch instead of every widget.
    pub fn widget(&self, id: &str) -> Option<&Widget> {
        self.paths.get(id).and_then(|path| self.app.at_path(path))
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

    /// Rows of a list widget after filtering. Unfiltered rows are borrowed
    /// from the data; filtered rows are computed once and shared until
    /// something changes them (see [`Session::invalidate_rows`]). A tree
    /// lists the values of its visible nodes; a select its items.
    pub fn rows(&self, id: &str) -> Rows<'_> {
        if let Some(rows) = self.rows_cache.borrow().get(id) {
            return Rows::Shared(Rc::clone(rows));
        }
        let rows = match self.kind(id) {
            Some(WidgetKind::Tree(_)) => {
                self.tree_rows(id).iter().map(|r| r.value.clone()).collect()
            }
            Some(WidgetKind::Select(select)) => {
                return self.filtered(id, select.items_or_data(id, self));
            }
            Some(_) => return self.filtered(id, as_list(self.data_for(id))),
            None => Vec::new(),
        };
        self.share_rows(id, rows)
    }

    /// `rows` through the filter on widget `id`: borrowed as they are when
    /// nothing filters them, else filtered once and cached.
    fn filtered<'a>(&'a self, id: &str, rows: &'a [Value]) -> Rows<'a> {
        let filter = self.filter_for(id);
        if filter.is_empty() {
            return Rows::Borrowed(rows);
        }
        self.share_rows(id, filter.apply(rows))
    }

    fn share_rows(&self, id: &str, rows: Vec<Value>) -> Rows<'_> {
        let rows = Rc::new(rows);
        self.rows_cache
            .borrow_mut()
            .insert(id.to_string(), Rc::clone(&rows));
        Rows::Shared(rows)
    }

    /// The visible rows of a tree widget, flattened once per change.
    pub fn tree_rows(&self, id: &str) -> Rc<Vec<TreeRow>> {
        if let Some(rows) = self.tree_cache.borrow().get(id) {
            return Rc::clone(rows);
        }
        let rows = match (self.kind(id), self.state(id).and_then(WidgetState::as_tree)) {
            (Some(WidgetKind::Tree(tree)), Some(state)) => tree.rows_with(id, state, self),
            _ => Vec::new(),
        };
        let rows = Rc::new(rows);
        self.tree_cache
            .borrow_mut()
            .insert(id.to_string(), Rc::clone(&rows));
        rows
    }

    /// Forget cached rows. Called whenever data, a filter, a selection, or
    /// an expansion may have changed.
    fn invalidate_rows(&self) {
        self.rows_cache.borrow_mut().clear();
        self.tree_cache.borrow_mut().clear();
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

    /// Which widget drives `id` (see [`Session::sources`]). `None` for
    /// widgets that follow nothing.
    pub fn source_id(&self, id: &str) -> Option<String> {
        self.sources().remove(id)
    }

    /// The table, tree, or select each follower follows, keyed by follower
    /// id. A follower follows its `--from`, unless that names itself, a
    /// widget that is not a table, tree, or select, or one whose `--from`
    /// chain leads back to it. Every other follower takes the focused one,
    /// else the nearest in an enclosing container, else the first visible,
    /// else the first on a hidden tab page. It skips one that follows it,
    /// directly or along a chain, so followers never follow each other in a
    /// loop. A follower missing from the map has nothing to
    /// follow. Focus and the page change only which one a follower takes,
    /// never whether it finds one.
    pub fn sources(&self) -> HashMap<String, String> {
        let is_row_source = |id: &str| self.kind(id).is_some_and(|k| k.is_row_source());
        let followers: Vec<&Widget> = self.app.iter().filter(|w| w.follows_source()).collect();
        let mut sources = HashMap::new();
        // `--from` first: a follower that picks its own source must see
        // every fixed link, or it could pick one that leads back to it.
        for widget in &followers {
            if let Some(from) = widget.source.as_ref().and_then(|s| s.from.as_deref())
                && is_row_source(from)
                && !leads_to(&sources, from, &widget.id)
            {
                sources.insert(widget.id.clone(), from.to_string());
            }
        }
        let picking: Vec<&Widget> = followers
            .into_iter()
            .filter(|w| w.source.as_ref().is_none_or(|s| s.from.is_none()))
            .collect();
        let nearest = |id: &str| -> Vec<String> {
            let Some(path) = self.paths.get(id) else {
                return Vec::new();
            };
            (1..path.len())
                .rev()
                .flat_map(|depth| subtree_ids(&self.app, &path[..depth]))
                .collect()
        };
        /// The ids one tier offers a follower, best first.
        type Tier<'a> = &'a dyn Fn(&str) -> Vec<String>;
        let tiers: [Tier; 4] = [
            &|_| self.focused.iter().cloned().collect(),
            &nearest,
            &|_| self.visible_ids(),
            &|_| self.app.iter().map(|w| w.id.clone()).collect(),
        ];
        // Each tier is tried for every follower before the next, so a
        // follower takes the focused widget before that widget can take the
        // follower as its own nearest one.
        for candidates in tiers {
            for widget in &picking {
                if sources.contains_key(&widget.id) {
                    continue;
                }
                if let Some(found) = candidates(&widget.id)
                    .into_iter()
                    .find(|other| is_row_source(other) && !leads_to(&sources, other, &widget.id))
                {
                    sources.insert(widget.id.clone(), found);
                }
            }
        }
        sources
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

    /// Draw images with `images` from now on, and give every preview the
    /// Kitty id it transmits under: its `--image-id`, else a fresh one.
    /// Called before the first layout pass, so nothing is encoded twice.
    pub fn enable_images(&mut self, mut images: Images) {
        let ids: Vec<(String, Option<u32>)> = self
            .app
            .iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Preview(p) => Some((w.id.clone(), p.image.kitty_id)),
                _ => None,
            })
            .collect();
        for (id, requested) in ids {
            if let Some(preview) = self
                .states
                .get_mut(&id)
                .and_then(WidgetState::as_preview_mut)
            {
                preview.kitty_id = Some(requested.unwrap_or_else(|| images.alloc_id()));
            }
            self.derived_key.remove(&id);
        }
        self.images = Some(images);
    }

    /// Collect images whose decode-and-encode threads finished (see
    /// [`crate::widget::PreviewState::poll_image`]). `wait` blocks for every running job.
    pub fn poll_images(&mut self, wait: bool) {
        for state in self.states.values_mut() {
            if let Some(preview) = state.as_preview_mut() {
                preview.poll_image(wait);
            }
        }
    }

    /// The Kitty ids this run allocated, to delete from the terminal when
    /// it ends. Ids given with `--image-id` belong to the caller and stay.
    /// Deleting an id that was never transmitted is harmless, so no widget
    /// tracks whether it showed anything.
    pub fn kitty_ids_to_free(&self) -> Vec<u32> {
        if !self.images.as_ref().is_some_and(Images::is_kitty) {
            return Vec::new();
        }
        self.app
            .iter()
            .filter(|w| matches!(&w.kind, WidgetKind::Preview(p) if p.image.kitty_id.is_none()))
            .filter_map(|w| self.state(&w.id)?.as_preview()?.kitty_id)
            .collect()
    }

    /// Recompute every widget that follows a source whose highlighted row
    /// changed, and every `tui image` whose pane changed size. Runs a few
    /// passes so chains (tree → table → preview) settle.
    pub fn refresh_derived(&mut self) {
        let ids: Vec<String> = self
            .app
            .iter()
            .filter(|w| w.follows_source() || w.is_fixed_image())
            .map(|w| w.id.clone())
            .collect();
        let sources = self.sources();
        for _ in 0..3 {
            let mut changed = false;
            for id in &ids {
                changed |= self.refresh_one(id, sources.get(id).map(String::as_str));
            }
            if !changed {
                break;
            }
        }
    }

    /// The size a widget's closures see as `$env.TUI_WIDTH` and
    /// `$env.TUI_HEIGHT`: the area inside its border, or the terminal size
    /// for a widget that is not laid out (on a hidden page).
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

    /// What a preview's `--on-change` hook returns for the state record:
    /// nothing without a hook or an engine, and a hook error is reported
    /// like any other.
    fn on_change(&mut self, hook: Option<&Closure>) -> Value {
        let span = Span::unknown();
        let (Some(closure), Some((engine_state, stack))) = (hook, self.engine.clone()) else {
            return Value::nothing(span);
        };
        let state = self.state_record(span);
        match call_closure(&engine_state, &stack, closure.clone(), state)
            .and_then(|data| data.into_value(span))
        {
            Ok(part) => part,
            Err(err) => {
                self.error = Some(truncate_chars(&err.to_string(), 80));
                Value::nothing(span)
            }
        }
    }

    /// Recompute what `id` shows from the highlighted row of `source`.
    fn refresh_one(&mut self, id: &str, source: Option<&str>) -> bool {
        let row = source.and_then(|src| self.row_of(src));
        let mut key = (
            source.unwrap_or_default().to_string(),
            row.clone()
                .unwrap_or_else(|| Value::nothing(Span::unknown())),
            self.inner_size(id),
            self.view_context(id),
        );
        if self.derived_key.get(id) == Some(&key) {
            return false;
        }
        let Some(widget) = self.widget(id).cloned() else {
            return false;
        };
        let stack = self.closure_stack(id);
        if let WidgetKind::Preview(preview) = &widget.kind {
            let shown = preview.shown_row(row.as_ref()).cloned();
            let changed = self
                .state(id)
                .is_some_and(|state| preview.file_changed(state, shown.as_ref()));
            if changed {
                // A new file: the pane's part of the context starts over
                // from what `--on-change` returns, and the key takes it so
                // the closure runs once for it.
                if let Some(state) = self
                    .states
                    .get_mut(id)
                    .and_then(WidgetState::as_preview_mut)
                {
                    state.row = shown;
                }
                let part = self.on_change(preview.on_change.as_ref());
                self.set_view_context(id, part);
                key.3 = self.view_context(id);
            }
            let Some(mut state) = self.states.remove(id) else {
                return false;
            };
            preview.refresh(id, &mut state, row.as_ref(), self, stack.as_ref(), &key.3);
            self.states.insert(id.to_string(), state);
            self.derived_key.insert(id.to_string(), key);
            return true;
        }
        self.derived_key.insert(id.to_string(), key);
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

    /// Append streamed rows to the data of `owners` (the widgets a child
    /// list's stream feeds), or to the shared data when `owners` is empty.
    /// Tables keep their highlight (a stream that lands while you read
    /// should not move it), even when the oldest rows are dropped to stay
    /// under the cap; logs follow the tail unless scrolled up, resolved when
    /// they draw. Once a hook has replaced the shared data, rows for it are
    /// ignored: the hook's rows win.
    pub fn append_values(&mut self, owners: &[String], values: Vec<Value>) {
        if values.is_empty() || (owners.is_empty() && self.data_from_hook) {
            return;
        }
        let cap = self.app.stream_row_cap();
        match owners.split_last() {
            None => {
                let dropped = append_capped(&mut self.app.data, values, cap);
                self.shift_rows(None, dropped);
            }
            Some((last, rest)) => {
                for owner in rest {
                    self.append_to_widget(owner, values.clone(), cap);
                }
                self.append_to_widget(last, values, cap);
            }
        }
        self.invalidate_rows();
        self.clamp_all_lists();
        self.refresh_derived();
    }

    fn append_to_widget(&mut self, id: &str, values: Vec<Value>, cap: usize) {
        let Some(path) = self.paths.get(id) else {
            return;
        };
        let Some(data) = self.app.at_path_mut(path).and_then(|w| w.data.as_mut()) else {
            return;
        };
        let dropped = append_capped(data, values, cap);
        self.shift_rows(Some(id), dropped);
    }

    /// After `dropped` rows left the front of a list (the shared data, or
    /// widget `owner`'s), move the highlights and check marks of the tables
    /// and selects showing it, so they stay on the same rows.
    fn shift_rows(&mut self, owner: Option<&str>, dropped: usize) {
        if dropped == 0 {
            return;
        }
        let list = match owner {
            None => Some(&self.app.data),
            Some(id) => self.widget(id).and_then(|w| w.data.as_ref()),
        };
        let Some(list) = list else {
            return;
        };
        let showing: Vec<String> = self
            .states
            .keys()
            .filter(|id| self.shows_list(id, list))
            .cloned()
            .collect();
        for id in showing {
            if let Some(list) = self.states.get_mut(&id).and_then(WidgetState::as_list_mut) {
                list.drop_front(dropped);
            }
        }
    }

    /// Whether widget `id` is a table or select whose rows are `list`, so
    /// trimming that list moves its rows.
    fn shows_list(&self, id: &str, list: &Value) -> bool {
        match self.kind(id) {
            Some(WidgetKind::Table(_)) => {}
            Some(WidgetKind::Select(select)) if select.items.is_empty() => {}
            _ => return false,
        }
        std::ptr::eq(self.data_for(id), list)
    }

    /// Replace the shared data list (from a hook or the refresh closure).
    /// From then on the hook owns it: a piped stream still producing no
    /// longer adds to it, or `ls | tui table | tui run --refresh 1sec { ls }`
    /// would show the listing twice.
    pub fn replace_data(&mut self, value: Value) {
        self.app.data = as_rows(value);
        self.data_from_hook = true;
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
        let outcome = self.hook_outcome(closure);
        self.apply_outcome(outcome);
    }

    /// Run a hook closure with the state record. Nothing happens without an
    /// engine (unit tests).
    fn hook_outcome(&mut self, closure: Closure) -> HookOutcome {
        let Some((engine_state, stack)) = self.engine.clone() else {
            return HookOutcome::Nothing;
        };
        let state = self.state_record(Span::unknown());
        hooks::run_hook(&engine_state, &stack, closure, state)
    }

    fn apply_outcome(&mut self, outcome: HookOutcome) {
        match outcome {
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
            HookOutcome::Action(action) => {
                self.error = None;
                self.apply_action(action);
            }
            HookOutcome::Many(items) => {
                for item in items {
                    if self.outcome.is_some() {
                        break;
                    }
                    self.apply_outcome(item);
                }
            }
            HookOutcome::Error(err) => self.error = Some(truncate_chars(&err, 80)),
        }
    }

    /// Change one widget as a hook asked. A wrong id or widget kind is
    /// reported like a hook error.
    fn apply_action(&mut self, action: HookAction) {
        let not_a = |id: &str, kind: &str| Some(format!("`{id}` is not a {kind}"));
        match action {
            HookAction::Focus(id) => {
                if self.kind(&id).is_some_and(WidgetKind::is_focusable) {
                    self.apply(vec![Effect::Focus(id)]);
                } else {
                    self.error = not_a(&id, "focusable widget");
                }
            }
            HookAction::Resize(id, mut sizes) => {
                match self.states.get_mut(&id).and_then(WidgetState::as_split_mut) {
                    Some(split) => {
                        // One size per child, whatever the hook sent.
                        sizes.resize(split.sizes.len(), Size::Fill(1));
                        split.sizes = sizes;
                    }
                    None => self.error = not_a(&id, "split"),
                }
            }
            HookAction::Context(id, value) => {
                if let Some(id) = &id
                    && self.widget(id).is_none()
                {
                    self.error = not_a(id, "widget");
                } else {
                    self.patch_context(id.as_deref(), value);
                    // The view context is in the derived key, so only the
                    // closures that see a change run again.
                    self.refresh_derived();
                }
            }
            HookAction::Scroll(id, line) => {
                let height = crate::widgets::inner_rows(self.areas.get(&id), 2);
                match self
                    .states
                    .get_mut(&id)
                    .and_then(WidgetState::as_preview_mut)
                {
                    Some(preview) => preview.scroll_to(line, height),
                    None => self.error = not_a(&id, "preview"),
                }
            }
            HookAction::Suspend { hook, area } => {
                let area = area.map(|id| (self.areas.get(&id).copied(), id));
                match area {
                    Some((None, id)) => self.error = not_a(&id, "widget on screen"),
                    Some((Some(rect), _)) => {
                        self.suspend = Some(Suspend {
                            hook,
                            area: Some(rect),
                        })
                    }
                    None => self.suspend = Some(Suspend { hook, area: None }),
                }
            }
            HookAction::Select(id, index) => {
                let last = self.rows(&id).len().saturating_sub(1);
                match self.states.get_mut(&id).and_then(WidgetState::as_list_mut) {
                    Some(list) => {
                        list.selected = index.min(last);
                        // Whatever follows the highlighted row moves with it.
                        self.refresh_derived();
                    }
                    None => self.error = not_a(&id, "list widget"),
                }
            }
            HookAction::Columns(id, columns) => {
                let widget = self
                    .paths
                    .get(&id)
                    .and_then(|path| self.app.at_path_mut(path));
                match widget.map(|w| &mut w.kind) {
                    Some(WidgetKind::Table(table)) => table.columns = columns,
                    _ => self.error = not_a(&id, "table"),
                }
            }
        }
    }

    /// Run a suspended hook, the terminal being its to use, and apply what
    /// it returns. The state record gains `area` when the hook has one,
    /// with the whole laid-out `screen` inside it for margins.
    pub fn run_suspended(&mut self, suspend: Suspend) {
        let Some((engine_state, stack)) = self.engine.clone() else {
            return;
        };
        let span = Span::unknown();
        let mut state = self.result_fields(span, None);
        if let Some(area) = suspend.area {
            let int = |n: u16| Value::int(i64::from(n), span);
            let rect = |r: Rect| {
                nu_protocol::record! {
                    "x" => int(r.x),
                    "y" => int(r.y),
                    "width" => int(r.width),
                    "height" => int(r.height),
                }
            };
            let mut area = rect(area);
            area.insert("screen", Value::record(rect(self.laid_out), span));
            state.insert("area", Value::record(area, span));
        }
        let outcome = hooks::run_hook(
            &engine_state,
            &stack,
            suspend.hook,
            Value::record(state, span),
        );
        self.apply_outcome(outcome);
    }

    /// Forget every preview's picture so it is decoded and sent again:
    /// the terminal was lent to something else, which may have cleared them.
    pub fn reset_images(&mut self) {
        let ids: Vec<String> = self
            .app
            .iter()
            .filter(|w| matches!(w.kind, WidgetKind::Preview(_)))
            .map(|w| w.id.clone())
            .collect();
        for id in ids {
            if let Some(preview) = self
                .states
                .get_mut(&id)
                .and_then(WidgetState::as_preview_mut)
            {
                preview.image = None;
            }
            self.derived_key.remove(&id);
        }
        self.refresh_derived();
    }

    /// What a pending bind sequence can still become: the rest of each
    /// reachable chord with its label, for the key panel. A bind scoped
    /// to the focused widget hides a global one on the same keys.
    pub fn pending_keys(&self) -> Vec<(String, String)> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        let prefix = format!("{} ", self.pending.join(" "));
        let mut keys: Vec<(String, String)> = Vec::new();
        let scoped_first = self
            .app
            .binds
            .iter()
            .filter(|b| b.when.is_some() && b.when == self.focused)
            .chain(self.app.binds.iter().filter(|b| b.when.is_none()));
        for bind in scoped_first {
            if let Some(rest) = bind.chord.strip_prefix(&prefix)
                && !keys.iter().any(|(key, _)| key == rest)
            {
                keys.push((rest.to_string(), bind.label.clone().unwrap_or_default()));
            }
        }
        keys.sort();
        keys
    }

    /// Where the status line is, if the app has one.
    pub fn status_area(&self) -> Option<Rect> {
        self.app
            .iter()
            .find(|w| matches!(&w.kind, WidgetKind::Label(l) if l.slot == Slot::Status))
            .and_then(|w| self.areas.get(&w.id).copied())
    }

    // ----- events -------------------------------------------------------

    /// Assign areas, then bring derived values up to date: closures read
    /// their pane size, so they run after layout, and again after a resize
    /// changes it.
    pub fn layout(&mut self, area: Rect) {
        self.laid_out = area;
        assign_areas(self, area);
        self.refresh_derived();
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
                // Pane sizes changed: the next layout pass re-runs the
                // closures that read them.
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
        // The chords typed so far: a pending sequence plus this key.
        let in_sequence = !self.pending.is_empty();
        let typed = if in_sequence {
            format!("{} {}", self.pending.join(" "), key.chord)
        } else {
            key.chord.clone()
        };
        // A bind without a modifier does not fire while typing, unless it
        // continues a sequence already begun. A bind scoped to the focused
        // widget beats a global one on the same chord. A scoped bind whose
        // hook returns nothing passes the key on.
        let (bind, continues) = if in_sequence || !capturing || key.has_modifier() {
            let candidates = || {
                self.app
                    .binds
                    .iter()
                    .filter(|b| b.when.is_none() || b.when == self.focused)
            };
            let exact = candidates()
                .find(|b| b.when.is_some() && b.chord == typed)
                .or_else(|| candidates().find(|b| b.chord == typed))
                .cloned();
            let longer = format!("{typed} ");
            (exact, candidates().any(|b| b.chord.starts_with(&longer)))
        } else {
            (None, false)
        };
        if let Some(bind) = bind {
            self.pending.clear();
            let scoped = bind.when.is_some();
            let outcome = self.hook_outcome(bind.closure);
            let pass_on = scoped && matches!(outcome, HookOutcome::Nothing);
            self.apply_outcome(outcome);
            if !pass_on {
                return;
            }
        } else if continues {
            // A longer sequence starts this way: wait for its next key.
            self.pending.push(key.chord);
            return;
        } else if in_sequence {
            // Nothing continues the sequence: it ends, taking this key with it.
            self.pending.clear();
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

    /// Hand a key to widget `id`. The handler gets a copy of the state to
    /// change while the original stays in the map, so anything it asks the
    /// session (its rows, its filter, its expansion) still sees the state.
    fn dispatch_key(&mut self, id: &str, key: &KeyPress) -> Option<Vec<Effect>> {
        let kind = self.kind(id)?.clone();
        let mut state = self.states.get(id)?.clone();
        let result = kind.handle_key(id, &mut state, key, self);
        self.store_state(id, state);
        result
    }

    /// Put a handler's copy of the state back. Rows computed before it may
    /// be stale (a captured chord, an expansion); recompute on the next ask.
    fn store_state(&mut self, id: &str, state: WidgetState) {
        self.states.insert(id.to_string(), state);
        self.invalidate_rows();
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
            let mut state = self.states.get(&id)?.clone();
            let mut effects = vec![Effect::Focus(id.clone())];
            if let Some(m) = state.as_menu_mut() {
                m.selected = idx;
                effects.extend(menu.open_or_activate(m, self));
            }
            self.store_state(&id, state);
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
            // Filters and data change rows; the row cache must not outlive
            // them. Focus and page changes leave every widget's rows alone.
            if matches!(
                effect,
                Effect::Query | Effect::Capture(..) | Effect::Hook(_) | Effect::Selected(_)
            ) {
                self.invalidate_rows();
            }
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
                    self.focused = self.after_text_input().or_else(|| self.focused.clone());
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

    /// Where Esc from a text input lands: the list the input filters, else
    /// the first content widget, else any non-text widget (the menu bar).
    fn after_text_input(&self) -> Option<String> {
        let candidates: Vec<String> = self
            .focusable_ids()
            .into_iter()
            .filter(|id| self.kind(id).is_some_and(|k| !k.is_text_input()))
            .collect();
        let scoped = |id: &String| self.scoping_search(id) == self.focused;
        let content = |id: &String| self.kind(id).is_some_and(|k| !k.is_chrome());
        candidates
            .iter()
            .find(|id| scoped(id) && content(id))
            .or_else(|| candidates.iter().find(|id| content(id)))
            .or_else(|| candidates.first())
            .cloned()
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
                let at = Position::new(mouse.column, mouse.row);
                if let Some(i) = self.tab_areas.iter().position(|area| area.contains(at)) {
                    self.apply(vec![Effect::Page(i)]);
                    return;
                }
                if let Some(handle) = self.handles.iter().find(|h| h.area.contains(at)) {
                    self.dragging = Some((handle.id.clone(), handle.index));
                    self.focused = Some(handle.id.clone());
                    return;
                }
                if let Some((id, area)) = self.hit_test(at)
                    && let Some(kind) = self.kind(&id).cloned()
                    && let Some(mut state) = self.states.get(&id).cloned()
                {
                    let effects = kind.click(&id, &mut state, area, mouse.column, mouse.row, self);
                    self.store_state(&id, state);
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
        let mut state = self.states.get(&id)?.clone();
        let effects = match state
            .as_menu()
            .and_then(|m| menu.dropdown_rect(m, bar))
            .zip(state.as_menu_mut())
        {
            Some((rect, m)) => {
                let inside = rect.contains(Position::new(x, y));
                let effects = menu.click_dropdown(m, rect, x, y, self);
                inside.then_some(effects)
            }
            None => None,
        };
        self.store_state(&id, state);
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
        let at = Position::new(mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if close.contains(at) {
                    self.quit();
                    return true;
                }
                if resize.contains(at) {
                    if let Some(dialog) = self.dialog.as_mut() {
                        dialog.drag = Some(DialogDrag::Resize);
                    }
                    return true;
                }
                if title.contains(at) {
                    if let Some(dialog) = self.dialog.as_mut() {
                        dialog.drag = Some(DialogDrag::Move {
                            grab_x: mouse.column.saturating_sub(rect.x),
                            grab_y: mouse.row.saturating_sub(rect.y),
                        });
                    }
                    return true;
                }
                !rect.contains(at)
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
        let at = match handle.direction {
            SplitDir::Horizontal => mouse.column,
            SplitDir::Vertical => mouse.row,
        };
        if let Some(split) = self.states.get_mut(id).and_then(WidgetState::as_split_mut) {
            SplitWidget::place_handle(split, handle.direction, index, handle.split_area, at);
        }
    }

    fn scroll_at(&mut self, x: u16, y: u16, delta: i32) {
        let id = self
            .hit_test(Position::new(x, y))
            .map(|(id, _)| id)
            .or_else(|| self.focused.clone());
        let Some(id) = id else {
            return;
        };
        if let Some(kind) = self.kind(&id).cloned()
            && let Some(mut state) = self.states.get(&id).cloned()
        {
            let effects = kind.scroll(&id, &mut state, delta, self);
            self.store_state(&id, state);
            self.apply(effects);
        }
    }

    /// The smallest non-container widget under the point.
    fn hit_test(&self, at: Position) -> Option<(String, Rect)> {
        self.areas
            .iter()
            .filter(|(id, area)| {
                area.contains(at) && !self.kind(id).is_some_and(|k| k.is_container())
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

    /// Whether `column` holds paths: one the data marks, `name` beside a
    /// `type` (as `ls` gives), or the table's own `--color` column.
    pub fn is_path_column(&self, column: &str, row: &Value, path_column: Option<&str>) -> bool {
        if path_column == Some(column) || self.app.path_columns.iter().any(|c| c == column) {
            return true;
        }
        column == "name" && row.as_record().ok().and_then(|r| r.get("type")).is_some()
    }

    /// The file a path cell stands for: `path` under `cwd` when that
    /// exists, else the row's `name` (a `--color` column of bare names
    /// beside the full paths `ls` gives), else `path` as given. Icons and
    /// colors look at the file itself where they can.
    fn disk_path(&self, row: &Value, path: &str) -> PathBuf {
        // `symlink_metadata`, not `exists`: a symlink with no target is on
        // disk too, and is colored as an orphan.
        let on_disk = |path: &PathBuf| path.symlink_metadata().is_ok();
        let full = self.cwd.join(path);
        if on_disk(&full) {
            return full;
        }
        row.as_record()
            .ok()
            .and_then(|r| r.get("name"))
            .and_then(|name| name.as_str().ok())
            .map(|name| self.cwd.join(name))
            .filter(on_disk)
            .unwrap_or(full)
    }

    /// What the `file_styles` rules see of the file a path cell stands
    /// for. The disk decides where the file exists: its kind and, through a
    /// symlink, its target's mode. A row not on disk goes by its `type`
    /// column (`ls` gives `dir` or, with `--mime-type`, the mime). The mime
    /// is that column's where it holds one, else the name's guess.
    fn file_facts<'a>(&self, row: &'a Value, disk: &Path) -> FileFacts<'a> {
        let row_type = row
            .as_record()
            .ok()
            .and_then(|r| r.get("type"))
            .and_then(|t| t.as_str().ok());
        let meta = std::fs::metadata(disk).ok();
        let kind = match &meta {
            None if disk.is_symlink() => FileKind::Orphan,
            _ if disk.is_symlink() => FileKind::Symlink,
            Some(meta) if meta.is_dir() => FileKind::Dir,
            None if row_type == Some("dir") => FileKind::Dir,
            _ => FileKind::File,
        };
        let mime = match row_type {
            _ if kind == FileKind::Dir => None,
            Some(mime) if mime.contains('/') => Some(mime),
            _ => mime_guess::from_path(disk).first_raw(),
        };
        FileFacts {
            kind,
            mime,
            ext: disk
                .extension()
                .map(|ext| ext.to_string_lossy().to_lowercase()),
            mode: meta.as_ref().and_then(mode_string),
        }
    }

    /// The `file_styles` rule for the file a path cell stands for.
    fn file_style(&self, row: &Value, path: &str) -> Option<&FileStyle> {
        let disk = self.disk_path(row, path);
        self.theme.file_style(&self.file_facts(row, &disk))
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

    /// Text and style of one table cell. `path_column` is the table's
    /// `--color` column, shown as paths besides those the data marks.
    pub fn styled_cell(
        &self,
        row: &Value,
        column: &str,
        computer: Option<&StyleComputer>,
        path_column: Option<&str>,
    ) -> (String, Style) {
        let value = match row {
            Value::Record { val, .. } => val
                .get(column)
                .cloned()
                .unwrap_or_else(|| Value::nothing(Span::unknown())),
            other => other.clone(),
        };
        let text = self.format_value(&value);
        if self.is_path_column(column, row, path_column)
            && let Some(path) = value.as_str().ok()
        {
            return (text, self.theme.path_cell(self.file_style(row, path)));
        }
        let style = match computer {
            Some(computer) => self.theme.value_cell(&value, computer),
            None => self.theme.text(),
        };
        (text, style)
    }

    /// The file icon `--icons` puts before a path cell: its rule's `icon`,
    /// else devicons' as `table --icons` and `grid --icons` draw. `None` for
    /// a cell that is not a path.
    pub fn path_icon(
        &self,
        row: &Value,
        column: &str,
        path_column: Option<&str>,
    ) -> Option<ratatui::text::Span<'static>> {
        use devicons::icon_for_file;
        if !self.is_path_column(column, row, path_column) {
            return None;
        }
        let path = row.as_record().ok()?.get(column)?.as_str().ok()?;
        let icon = icon_for_file(&self.disk_path(row, path), &None);
        let span = match self
            .file_style(row, path)
            .filter(|rule| rule.icon().is_some())
        {
            Some(rule) => ratatui::text::Span::styled(
                format!("{} ", rule.icon().unwrap_or_default()),
                self.theme.rule_icon(rule, icon.color),
            ),
            None => {
                ratatui::text::Span::styled(format!("{} ", icon.icon), self.theme.icon(icon.color))
            }
        };
        Some(span)
    }

    /// Style of a tree node or list item: the file palette for paths, else
    /// the value's type color. `computer` comes from
    /// [`Session::style_computer`], built once per frame rather than per row.
    pub fn row_style(&self, row: &Value, computer: Option<&StyleComputer>) -> Style {
        if self.is_path_column("name", row, None)
            && let Some(path) = crate::widgets::preview::row_path(row)
        {
            return self.theme.path_cell(self.file_style(row, &path));
        }
        match computer {
            Some(computer) => self.theme.value_cell(row, computer),
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
        self.status_segments().join("  ")
    }

    /// The status bar's pieces: the `--status` label's text, then the live
    /// bits (error, focus, filter, rows, page, keys).
    pub fn status_segments(&self) -> Vec<String> {
        let label = self.app.widgets.iter().find_map(|w| match &w.kind {
            WidgetKind::Label(l) if l.slot == Slot::Status => Some(l.text(&w.id, self)),
            _ => None,
        });
        let mut segments = self.live_status_bits();
        if let Some(text) = label {
            segments.insert(0, text);
        }
        segments
    }

    fn live_status_bits(&self) -> Vec<String> {
        let mut bits = Vec::new();
        if let Some(err) = &self.error {
            bits.push(format!("error:{err}"));
        }
        if let Some(id) = &self.focused {
            bits.push(format!("focus:{id}"));
            if let Some(hint) = self
                .widget(id)
                .zip(self.state(id))
                .and_then(|(w, state)| w.kind.hint(id, state, self))
            {
                bits.push(hint);
            }
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
        bits
    }

    /// The app context's shared keys: the record without `views`. Nothing
    /// when there is no context.
    pub fn shared_context(&self) -> Value {
        match self.app.context.as_record() {
            Ok(root) => {
                let mut shared = root.clone();
                shared.remove("views");
                Value::record(shared, Span::unknown())
            }
            Err(_) => Value::nothing(Span::unknown()),
        }
    }

    /// The context a widget's closure sees as `$ctx`, and `values.<id>`
    /// reports: the shared keys with its own `views.<id>` laid over.
    pub fn view_context(&self, id: &str) -> Value {
        let own = self
            .app
            .context
            .as_record()
            .ok()
            .and_then(|root| root.get("views")?.as_record().ok()?.get(id).cloned());
        match own {
            Some(own) if !own.is_nothing() => merge_context(&self.shared_context(), own),
            _ => self.shared_context(),
        }
    }

    /// Replace `views.<id>` of the app context, making the records on the
    /// way as needed.
    fn set_view_context(&mut self, id: &str, value: Value) {
        let span = Span::unknown();
        let mut root = self.app.context.as_record().cloned().unwrap_or_default();
        let mut views = root
            .remove("views")
            .and_then(|views| views.as_record().ok().cloned())
            .unwrap_or_default();
        views.insert(id, value);
        root.insert("views", Value::record(views, span));
        self.app.context = Value::record(root, span);
    }

    /// `{action: context}`: merge `patch` into the app context, or with an
    /// id into that widget's `views.<id>`.
    fn patch_context(&mut self, id: Option<&str>, patch: Value) {
        match id {
            Some(id) => {
                let current = self
                    .app
                    .context
                    .as_record()
                    .ok()
                    .and_then(|root| root.get("views")?.as_record().ok()?.get(id).cloned())
                    .unwrap_or_else(|| Value::nothing(Span::unknown()));
                self.set_view_context(id, merge_context(&current, patch));
            }
            None => self.app.context = merge_context(&self.app.context, patch),
        }
    }

    /// The state record every hook receives and `tui run` returns:
    /// `{action, focused, selected, page, values, rows, live, context}`.
    /// `values` holds every widget's state by id; `context` is the app's
    /// record (`tui context`, `{action: context}`).
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
        rec.insert("context", self.app.context.clone());
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
            if w.follows_source() {
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

/// Append `values` to the list in `data`, dropping the oldest rows once it
/// is over `cap` (see [`trim_front`]). Returns how many rows were dropped.
fn append_capped(data: &mut Value, values: Vec<Value>, cap: usize) -> usize {
    // Take the list out rather than clone it: this runs on every poll tick
    // of a live stream.
    let span = data.span();
    let mut rows = match std::mem::replace(data, Value::nothing(span)) {
        Value::List { vals, .. } => vals.into_owned(),
        Value::Nothing { .. } => Vec::new(),
        other => vec![other],
    };
    rows.extend(values);
    let dropped = trim_front(&mut rows, cap);
    let span = rows.first().map(|v| v.span()).unwrap_or(span);
    *data = Value::list(rows, span);
    dropped
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

/// A context patch: records merge key by key, anything else replaces.
fn digit_page(chord: &str) -> Option<usize> {
    let c = single_char(chord)?;
    (c.is_ascii_digit() && c != '0').then(|| (c as u8 - b'1') as usize)
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
                icons: false,
                color: None,
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
                ..PreviewWidget::default()
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

    /// Streamed rows `from..to`, as ints.
    fn ints(from: usize, to: usize) -> Vec<Value> {
        (from..to).map(|i| Value::test_int(i as i64)).collect()
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
                icons: false,
                color: None,
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
    fn followers_never_follow_each_other() {
        // Each table in the split is the other's nearest; the second skips
        // the first, which already follows it, and takes the outer table.
        let follower = |id: &str| {
            let mut widget = table(id, &["name"]);
            widget.source = Some(Source::default());
            widget
        };
        let mut session = Session::new(app(
            vec![
                table("table-a", &["name"]),
                split(
                    "split-0",
                    SplitDir::Horizontal,
                    vec![follower("table-b"), follower("table-c")],
                ),
            ],
            rows(&["x"]),
        ));
        session.focused = None;
        assert_eq!(session.source_id("table-b").as_deref(), Some("table-c"));
        assert_eq!(session.source_id("table-c").as_deref(), Some("table-a"));
        // Focus wins over nearness: the sibling follows the focused table,
        // which then takes the outer one.
        session.focused = Some("table-b".into());
        assert_eq!(session.source_id("table-c").as_deref(), Some("table-b"));
        assert_eq!(session.source_id("table-b").as_deref(), Some("table-a"));
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
            ..Source::default()
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
    fn dialog_text_draws_on_surface_with_terminal_foreground() {
        use ratatui::{Terminal, backend::TestBackend, style::Color};
        let mut session = Session::new(table_app());
        session.theme.surface = Color::Rgb(1, 2, 3);
        session.theme.backdrop = Color::Rgb(4, 5, 6);
        session.enable_dialog(frame(), Some(40), Some(10));
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        terminal
            .draw(|f| crate::render::render(f, &mut session))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let area = buffer.area;
        // `beta` is an unselected row: plain text over the surface.
        let (x, y) = (area.top()..area.bottom())
            .flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
            .find(|&(x, y)| {
                (x..x + 4)
                    .filter_map(|x| buffer.cell((x, y)))
                    .map(|c| c.symbol())
                    .collect::<String>()
                    == "beta"
            })
            .expect("beta drawn");
        let cell = buffer.cell((x, y)).expect("cell");
        assert_eq!(cell.bg, Color::Rgb(1, 2, 3));
        assert_eq!(cell.fg, Color::Reset, "not the backdrop's muted fg");
        assert_eq!(buffer.cell((0, 0)).map(|c| c.bg), Some(Color::Rgb(4, 5, 6)));
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

    /// A session holding one split, `split-0`, of two tables.
    fn two_panes(direction: SplitDir) -> Session {
        Session::new(app(
            vec![split(
                "split-0",
                direction,
                vec![table("table-0", &["name"]), table("table-1", &["name"])],
            )],
            rows(&["x"]),
        ))
    }

    /// First-child lengths after each of `presses` presses of `code` on a
    /// focused two-pane split drawn in `area`.
    fn nudge_lengths(direction: SplitDir, area: Rect, code: KeyCode, presses: usize) -> Vec<u16> {
        let mut session = two_panes(direction);
        session.layout(area);
        session.focused = Some("split-0".into());
        let len = |session: &Session| {
            let child = session.areas["table-0"];
            match direction {
                SplitDir::Horizontal => child.width,
                SplitDir::Vertical => child.height,
            }
        };
        let mut lengths = vec![len(&session)];
        for _ in 0..presses {
            press(&mut session, code, KeyModifiers::NONE);
            session.layout(area);
            lengths.push(len(&session));
        }
        lengths
    }

    #[test]
    fn arrow_keys_move_a_split_handle_one_cell_each_way() {
        // 81 columns and 40 rows are not multiples of 100: a one-percent
        // step used to round back to the same cell and stick.
        let wide = Rect::new(0, 0, 81, 24);
        let tall = Rect::new(0, 0, 80, 40);
        let cases = [
            (SplitDir::Horizontal, wide, KeyCode::Right, 40, 1),
            (SplitDir::Horizontal, wide, KeyCode::Left, 40, -1),
            (SplitDir::Vertical, tall, KeyCode::Down, 20, 1),
            (SplitDir::Vertical, tall, KeyCode::Up, 20, -1),
        ];
        for (direction, area, code, start, step) in cases {
            let lengths = nudge_lengths(direction, area, code, 5);
            let expected: Vec<u16> = (0..=5).map(|n| (start + step * n) as u16).collect();
            assert_eq!(lengths, expected, "{direction:?} {code:?}");
        }
    }

    #[test]
    fn arrow_keys_move_a_wide_split_handle_every_press() {
        // Past 100 cells a percent is more than a cell, so a press may skip
        // one, but it must always move.
        let area = Rect::new(0, 0, 250, 24);
        for code in [KeyCode::Right, KeyCode::Left] {
            let lengths = nudge_lengths(SplitDir::Horizontal, area, code, 5);
            for pair in lengths.windows(2) {
                assert!(pair[0].abs_diff(pair[1]) >= 1, "{code:?}: {lengths:?}");
                assert!(pair[0].abs_diff(pair[1]) <= 3, "{code:?}: {lengths:?}");
            }
        }
    }

    #[test]
    fn splitter_drag_lands_on_the_mouse_column() {
        let area = Rect::new(0, 0, 81, 24);
        let mut session = two_panes(SplitDir::Horizontal);
        session.layout(area);
        let handle = session.handles.first().cloned().expect("handle");
        click(&mut session, handle.area.x, handle.area.y);
        for column in [30, 42, 57] {
            drag(&mut session, column, handle.area.y);
            session.layout(area);
            assert_eq!(session.areas["table-0"].width, column);
        }
    }

    #[test]
    fn dragging_a_later_split_handle_lands_on_the_mouse_column() {
        // Fills before the dragged child shrink as it grows, and a percent
        // before it can end partway into a cell; both used to leave the
        // handle short of the pointer.
        let cases = [
            (vec![Size::Fill(1), Size::Fill(1), Size::Fill(1)], 60),
            (vec![Size::Percent(33), Size::Fill(1), Size::Fill(1)], 34),
        ];
        for (sizes, column) in cases {
            let tables = (0..3)
                .map(|i| table(&format!("table-{i}"), &["name"]))
                .collect();
            let mut w = split("split-0", SplitDir::Horizontal, tables);
            if let WidgetKind::Split(split) = &mut w.kind {
                split.sizes = sizes.clone();
            }
            let mut session = Session::new(app(vec![w], rows(&["x"])));
            session.layout(frame());
            let handle = session.handles[1].clone();
            click(&mut session, handle.area.x, handle.area.y);
            drag(&mut session, column, handle.area.y);
            session.layout(frame());
            assert_eq!(session.handles[1].area.x, column, "{sizes:?}");
        }
    }

    #[test]
    fn a_centred_split_handle_is_fifty_percent() {
        // 49%, 50% and 51% of 40 rows all end on row 20.
        let area = Rect::new(0, 0, 80, 40);
        let mut session = two_panes(SplitDir::Vertical);
        session.layout(area);
        let handle = session.handles.first().cloned().expect("handle");
        click(&mut session, handle.area.x, handle.area.y);
        drag(&mut session, handle.area.x, 20);
        let first = session
            .state("split-0")
            .and_then(WidgetState::as_split)
            .and_then(|split| split.sizes.first().copied());
        assert_eq!(first, Some(Size::Percent(50)));
    }

    #[test]
    fn arrow_keys_step_from_the_sizes_not_the_last_frame() {
        // Keys that arrive together are all handled before the next frame.
        let area = Rect::new(0, 0, 81, 24);
        let mut session = two_panes(SplitDir::Horizontal);
        session.layout(area);
        session.focused = Some("split-0".into());
        for _ in 0..3 {
            press(&mut session, KeyCode::Right, KeyModifiers::NONE);
        }
        session.layout(area);
        assert_eq!(session.areas["table-0"].width, 43);
    }

    #[test]
    fn arrow_keys_never_empty_a_pane() {
        // On short splits 95% plus the one-cell handle used to leave the
        // far pane no rows at all.
        for height in [8, 12, 15, 24] {
            let area = Rect::new(0, 0, 80, height);
            for (code, far) in [(KeyCode::Down, "table-1"), (KeyCode::Up, "table-0")] {
                let mut session = two_panes(SplitDir::Vertical);
                session.layout(area);
                session.focused = Some("split-0".into());
                for _ in 0..40 {
                    press(&mut session, code, KeyModifiers::NONE);
                    session.layout(area);
                }
                assert_eq!(session.areas[far].height, 1, "{height} rows, {code:?}");
            }
        }
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
        session.append_values(&[], vec![Value::test_string("50")]);
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
        session.append_values(&[], as_list(&rows(&["a"])).to_vec());
        session.append_values(&[], as_list(&rows(&["b", "c"])).to_vec());
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
        // Not from the keyboard either.
        session.focused = Some("split-0".into());
        for code in [KeyCode::Down, KeyCode::Up] {
            press(&mut session, code, KeyModifiers::NONE);
            session.layout(frame());
            assert_eq!(session.areas["button-0"].height, 1, "{code:?}");
            assert_eq!(session.areas["button-1"].height, 1, "{code:?}");
        }
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
            icons: false,
            color: None,
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

    fn scroll_wheel(session: &mut Session, x: u16, y: u16, down: bool) {
        session.handle_event(&Event::Mouse(MouseEvent {
            kind: if down {
                MouseEventKind::ScrollDown
            } else {
                MouseEventKind::ScrollUp
            },
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
    }

    #[test]
    fn capturing_table_navigates_the_filtered_rows() {
        let mut session = Session::new(keybindings_app(false));
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(session.rows("table-0").len(), 1);
        // Only one row matches: Down must not move past it.
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(selected_index(&session, "table-0"), 0);
        press(&mut session, KeyCode::Enter, KeyModifiers::NONE);
        let name = session
            .outcome
            .clone()
            .expect("outcome")
            .selected
            .as_record()
            .ok()
            .and_then(|r| r.get("name"))
            .and_then(|v| v.as_str().ok())
            .map(str::to_string);
        assert_eq!(name.as_deref(), Some("history"));
    }

    fn nested_tree_app() -> TuiApp {
        let mut inner = Record::new();
        inner.insert("b", Value::test_int(1));
        inner.insert("c", Value::test_int(2));
        let mut rec = Record::new();
        rec.insert("a", Value::test_record(inner));
        rec.insert(
            "d",
            Value::test_list(vec![Value::test_int(3), Value::test_int(4)]),
        );
        app(
            vec![Widget::new(
                "tree-0",
                WidgetKind::Tree(TreeWidget {
                    walk: false,
                    column: "name".into(),
                    multi: false,
                }),
            )],
            Value::test_record(rec),
        )
    }

    #[test]
    fn tree_mouse_sees_expanded_rows() {
        let mut session = Session::new(nested_tree_app());
        session.layout(frame());
        press(&mut session, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(session.rows("tree-0").len(), 4);
        // Row 2 inside the border is `c` (a, b, c, d).
        click(&mut session, 3, 3);
        assert_eq!(selected_index(&session, "tree-0"), 2);
        for _ in 0..3 {
            scroll_wheel(&mut session, 3, 3, true);
        }
        assert_eq!(selected_index(&session, "tree-0"), 3);
    }

    #[test]
    fn shrinking_filter_pulls_the_viewport_back() {
        let mut session = Session::new(app(
            vec![
                search("search-0", Some("/")),
                Widget::new(
                    "select-0",
                    WidgetKind::Select(SelectWidget {
                        items: (0..30)
                            .map(|i| Value::test_string(format!("item-{i}")))
                            .collect(),
                        multi: false,
                        index: false,
                        display: None,
                    }),
                ),
            ],
            Value::test_nothing(),
        ));
        session.layout(Rect::new(0, 0, 30, 12));
        press(&mut session, KeyCode::End, KeyModifiers::NONE);
        let list = |s: &Session| s.state("select-0").and_then(WidgetState::as_list).cloned();
        assert!(
            list(&session).is_some_and(|l| l.scroll > 0),
            "scrolled down"
        );
        key(&mut session, '/');
        for c in "item-0".chars() {
            key(&mut session, c);
        }
        assert_eq!(session.rows("select-0").len(), 1);
        let after = list(&session).expect("list");
        assert_eq!((after.selected, after.scroll), (0, 0));
    }

    #[test]
    fn trimming_a_live_stream_keeps_the_highlight_on_its_row() {
        let mut session = Session::new(app(
            vec![Widget::new(
                "table-0",
                WidgetKind::Table(TableWidget {
                    columns: vec!["item".into()],
                    capture_keys: false,
                    multi: true,
                    index: false,
                    icons: false,
                    color: None,
                }),
            )],
            Value::test_nothing(),
        ));
        let cap = session.app.stream_row_cap();
        let batch = cap / 8;
        session.append_values(&[], ints(0, cap));
        // Check the first row, then highlight and check the last.
        key(&mut session, ' ');
        press(&mut session, KeyCode::End, KeyModifiers::NONE);
        key(&mut session, ' ');
        let last = Some(Value::test_int(cap as i64 - 1));
        assert_eq!(session.row_of("table-0"), last);
        // Past the cap, rows are dropped a batch at a time, not one by one.
        session.append_values(&[], ints(cap, cap + 3));
        assert_eq!(session.rows("table-0").len(), cap + 3);
        // A batch more drops the oldest batch. The highlight and the check
        // stay on the same values; the check on row 0 went with it.
        session.append_values(&[], ints(cap + 3, cap + 3 + batch));
        assert_eq!(session.rows("table-0").len(), cap + 3);
        assert_eq!(session.row_of("table-0"), last);
        let checked = session
            .state("table-0")
            .and_then(WidgetState::as_list)
            .map(|l| l.checked.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        assert_eq!(checked, vec![cap - 1 - batch]);
    }

    #[test]
    fn a_child_lists_stream_fills_its_own_rows_and_keeps_their_highlight() {
        // `tui split [(1.. | tui table) (1.. | tui box [(tui table)])]`:
        // table-0 owns its list; table-1 shows its container's.
        let mut own = table("table-0", &[]);
        own.data = Some(Value::test_list(Vec::new()));
        let mut container = split("split-0", SplitDir::Vertical, vec![table("table-1", &[])]);
        container.data = Some(Value::test_list(Vec::new()));
        let mut session = Session::new(app(vec![own, container], rows(&["outer"])));
        let cap = session.app.stream_row_cap();
        let owners = ["table-0".to_string(), "split-0".to_string()];
        session.append_values(&owners, ints(0, cap));
        for id in ["table-0", "table-1"] {
            if let Some(list) = session
                .states
                .get_mut(id)
                .and_then(WidgetState::as_list_mut)
            {
                list.selected = cap - 1;
            }
        }
        session.append_values(&owners, ints(cap, cap + cap / 8));
        for id in ["table-0", "table-1"] {
            assert_eq!(session.rows(id).len(), cap, "{id} dropped a batch");
            assert_eq!(
                session.row_of(id),
                Some(Value::test_int(cap as i64 - 1)),
                "{id} keeps its highlight"
            );
        }
        assert_eq!(as_list(&session.app.data).len(), 1, "shared data untouched");
    }

    #[test]
    fn a_hook_that_replaces_the_data_wins_over_the_stream() {
        let mut session = Session::new(app(vec![table("table-0", &["name"])], rows(&["a", "b"])));
        session.replace_data(rows(&["from hook"]));
        session.append_values(&[], as_list(&rows(&["streamed"])).to_vec());
        assert_eq!(session.rows("table-0").len(), 1);
    }

    #[test]
    fn esc_from_search_lands_on_the_list_it_filters_not_the_menu() {
        let mut widgets = menu_app().widgets;
        widgets.insert(1, search("search-0", Some("/")));
        let mut session = Session::new(app(widgets, rows(&["a", "b"])));
        key(&mut session, '/');
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        press(&mut session, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
    }

    #[test]
    fn log_follow_keeps_the_newest_line_visible_when_lines_wrap() {
        let mut session = Session::new(app(
            vec![Widget::new(
                "log-0",
                WidgetKind::Log(crate::widgets::log::LogWidget { max_lines: 100 }),
            )],
            Value::test_list(
                (1..=20)
                    .map(|i| Value::test_string(format!("line-{i:02} {}", "x".repeat(60))))
                    .collect(),
            ),
        ));
        let screen = crate::render::render_to_string(&mut session, 40, 8).expect("render");
        assert!(screen.contains("line-20"), "newest line shown:\n{screen}");
        assert!(
            !screen.contains("line-15"),
            "old lines scrolled off:\n{screen}"
        );
    }

    #[test]
    fn source_closures_run_once_per_layout() {
        // A follower without an engine copies the source row; the derived
        // key records when that happened.
        let mut follower = table("table-1", &["name"]);
        follower.source = Some(Source {
            from: Some("table-0".into()),
            closure: None,
            ..Source::default()
        });
        let mut session = Session::new(app(
            vec![split(
                "split-0",
                SplitDir::Horizontal,
                vec![table("table-0", &["name"]), follower],
            )],
            rows(&["first", "second"]),
        ));
        assert!(
            session.derived("table-1").is_none(),
            "nothing before layout"
        );
        session.layout(frame());
        assert!(session.derived("table-1").is_some());
        let key_before = session.derived_key.get("table-1").cloned();
        session.layout(frame());
        assert_eq!(session.derived_key.get("table-1").cloned(), key_before);
        // A narrower frame is a new pane size, so the closure would re-run.
        session.layout(Rect::new(0, 0, 60, 24));
        assert_ne!(session.derived_key.get("table-1").cloned(), key_before);
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
