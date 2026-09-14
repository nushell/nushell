//! Mutable interactive state: focus, typing, scrolling, filtering, pages.
use super::app::TuiApp;
use super::keys::{key_event_to_string, normalize_bind};
use super::layout::{SplitterHandle, assign_areas};
use super::theme::{self, Theme};
use super::tree::{self, TreeRow};
use super::widget::{SplitDir, WidgetKind};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use lscolors::LsColors;
use nu_color_config::StyleComputer;
use nu_engine::{ClosureEvalOnce, env_to_string, get_columns};
use nu_protocol::engine::{Closure, EngineState, Stack};
use nu_protocol::{Config, DataSource, IntoPipelineData, PipelineMetadata, Record, Span, Value};
use nu_utils::get_ls_colors;
use ratatui::layout::Rect;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone)]
pub struct Page {
    pub title: String,
    pub body_id: Option<String>,
    pub splitter: Option<usize>,
    pub content: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub app: TuiApp,
    pub page: usize,
    pub focused: Option<String>,
    pub search: String,
    pub search_cursor: usize,
    pub text_values: HashMap<String, String>,
    pub text_cursors: HashMap<String, usize>,
    pub selected: HashMap<String, usize>,
    pub scroll: HashMap<String, usize>,
    /// Splitter position in permille (50–950). Keyboard steps 10 (1%).
    pub splitter_ratio: HashMap<String, u16>,
    pub dragging: Option<String>,
    pub areas: HashMap<String, Rect>,
    pub splitter_handles: Vec<SplitterHandle>,
    pub tab_areas: Vec<Rect>,
    pub outcome: Option<Outcome>,
    pub cwd: PathBuf,
    pub preview_text: HashMap<String, String>,
    pub preview_title: HashMap<String, String>,
    preview_engine: Option<(EngineState, Stack)>,
    pub stream_live: bool,
    pub follow_tail: HashMap<String, bool>,
    pub dialog: Option<DialogFrame>,
    pub theme: Theme,
    pub ls_colors: LsColors,
    pub use_ls_colors: bool,
    /// Expanded node paths per tree widget id.
    pub tree_expanded: HashMap<String, HashSet<String>>,
    /// Directory listings keyed by widget id then node path.
    pub tree_cache: HashMap<String, HashMap<String, Vec<Value>>>,
    pub refresh_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogDrag {
    Move { grab_x: u16, grab_y: u16 },
    Resize,
}

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

impl Session {
    #[cfg(test)]
    pub fn new(app: TuiApp) -> Self {
        Self::with_cwd(app, PathBuf::from("."))
    }

    #[cfg(test)]
    pub fn with_cwd(app: TuiApp, cwd: PathBuf) -> Self {
        Self::with_engine(app, cwd, None)
    }

    pub fn with_engine(
        app: TuiApp,
        cwd: PathBuf,
        preview_engine: Option<(EngineState, Stack)>,
    ) -> Self {
        let mut text_values = HashMap::new();
        let mut text_cursors = HashMap::new();
        let mut splitter_ratio = HashMap::new();
        let mut selected = HashMap::new();

        for w in &app.widgets {
            match &w.kind {
                WidgetKind::TextBox { value, .. } => {
                    text_cursors.insert(w.id.clone(), value.chars().count());
                    text_values.insert(w.id.clone(), value.clone());
                }
                WidgetKind::Splitter { ratio, .. } => {
                    splitter_ratio.insert(w.id.clone(), percent_to_permille(*ratio));
                }
                WidgetKind::Menu { .. }
                | WidgetKind::Table { .. }
                | WidgetKind::List { .. }
                | WidgetKind::Tree { .. }
                | WidgetKind::Keybindings { .. } => {
                    selected.insert(w.id.clone(), 0);
                }
                _ => {}
            }
        }

        let theme = match &preview_engine {
            Some((engine_state, stack)) => Theme::from_config(engine_state, stack),
            None => Theme::default(),
        };
        let (ls_colors, use_ls_colors) = match &preview_engine {
            Some((engine_state, stack)) => {
                let env = stack
                    .get_env_var(engine_state, "LS_COLORS")
                    .and_then(|v| env_to_string("LS_COLORS", v, engine_state, stack).ok());
                (
                    get_ls_colors(env),
                    stack.get_config(engine_state).ls.use_ls_colors,
                )
            }
            None => (get_ls_colors(None), true),
        };

        let mut session = Self {
            app,
            page: 0,
            focused: None,
            search: String::new(),
            search_cursor: 0,
            text_values,
            text_cursors,
            selected,
            scroll: HashMap::new(),
            splitter_ratio,
            dragging: None,
            areas: HashMap::new(),
            splitter_handles: Vec::new(),
            tab_areas: Vec::new(),
            outcome: None,
            cwd,
            preview_text: HashMap::new(),
            preview_title: HashMap::new(),
            preview_engine,
            stream_live: false,
            follow_tail: HashMap::new(),
            dialog: None,
            theme,
            ls_colors,
            use_ls_colors,
            tree_expanded: HashMap::new(),
            tree_cache: HashMap::new(),
            refresh_error: None,
        };
        session.focused = session.default_focus();
        session.refresh_previews();
        session
    }

    pub fn pages(&self) -> Vec<Page> {
        let mut pages: Vec<Page> = Vec::new();
        let mut current: Option<Page> = None;

        for (i, w) in self.app.widgets.iter().enumerate() {
            match &w.kind {
                k if k.is_page_marker() => {
                    let title = match k {
                        WidgetKind::Body { title } | WidgetKind::Tab { title } => title.clone(),
                        _ => "page".into(),
                    };
                    if let Some(page) = current.take() {
                        pages.push(page);
                    }
                    current = Some(Page {
                        title,
                        body_id: Some(w.id.clone()),
                        splitter: None,
                        content: Vec::new(),
                    });
                }
                WidgetKind::Splitter { .. } => {
                    let page = current.get_or_insert_with(implicit_page);
                    page.splitter = Some(i);
                }
                k if k.is_chrome() => {}
                _ => {
                    let page = current.get_or_insert_with(implicit_page);
                    page.content.push(i);
                }
            }
        }
        if let Some(page) = current {
            pages.push(page);
        }
        if pages.is_empty() {
            pages.push(implicit_page());
        }
        pages
    }

    fn default_focus(&self) -> Option<String> {
        let ids = self.focusable_ids();
        let preferred = [
            "table",
            "list",
            "tree",
            "log",
            "keybindings",
            "textbox",
            "search",
            "menu",
            "splitter",
        ];
        for prefix in preferred {
            if let Some(id) = ids.iter().find(|id| id.starts_with(prefix)) {
                return Some(id.clone());
            }
        }
        ids.into_iter().next()
    }

    pub fn focusable_ids(&self) -> Vec<String> {
        let pages = self.pages();
        let page = pages.get(self.page);
        let mut ids = Vec::new();
        for w in &self.app.widgets {
            if matches!(w.kind, WidgetKind::Menu { .. } | WidgetKind::Search { .. })
                && w.kind.is_focusable()
            {
                ids.push(w.id.clone());
            }
        }
        if let Some(page) = page {
            for idx in &page.content {
                if let Some(w) = self.app.widgets.get(*idx)
                    && w.kind.is_focusable()
                {
                    ids.push(w.id.clone());
                }
            }
            if let Some(si) = page.splitter
                && let Some(w) = self.app.widgets.get(si)
            {
                ids.push(w.id.clone());
            }
        }
        ids
    }

    pub fn is_editing(&self) -> bool {
        self.focused
            .as_deref()
            .and_then(|id| self.app.widget_kind(id))
            .is_some_and(|k| k.is_text_input())
    }

    pub fn focused_kind(&self) -> Option<&WidgetKind> {
        self.focused
            .as_deref()
            .and_then(|id| self.app.widget_kind(id))
    }

    pub fn is_focused(&self, id: &str) -> bool {
        self.focused.as_deref() == Some(id)
    }

    pub fn is_resizing(&self) -> bool {
        self.dragging.is_some() || self.dialog.as_ref().is_some_and(|d| d.drag.is_some())
    }

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

    #[cfg(test)]
    pub fn dialog_rect(&self) -> Option<Rect> {
        self.dialog.as_ref().map(|d| d.rect)
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
                self.handle_key(*key);
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
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let chord = key_event_to_string(key);

        if chord == "ctrl+c" {
            self.quit();
            return;
        }

        if !self.is_editing() && self.match_search_bind(&chord) {
            self.focus_search();
            return;
        }

        if self.is_editing() {
            self.handle_text_key(key, &chord);
            return;
        }

        match chord.as_str() {
            "tab" => {
                self.focus_next();
                return;
            }
            "shift+tab" => {
                self.focus_prev();
                return;
            }
            "q" | "esc" => {
                self.quit();
                return;
            }
            "[" | "ctrl+left" | "ctrl+shift+tab" => {
                self.prev_page();
                return;
            }
            "]" | "ctrl+right" | "ctrl+tab" => {
                self.next_page();
                return;
            }
            "enter" => {
                self.submit();
                return;
            }
            _ => {}
        }

        if let Some(n) = digit_page(&chord) {
            let pages = self.pages().len();
            if n < pages {
                self.page = n;
                self.focused = self.default_focus();
                self.refresh_previews();
            }
            return;
        }

        match self.focused_kind() {
            Some(WidgetKind::Tree { .. }) => {
                if self.handle_tree_nav(&chord) {
                    return;
                }
            }
            Some(
                WidgetKind::Table { .. } | WidgetKind::List { .. } | WidgetKind::Keybindings { .. },
            ) => {
                if self.handle_list_nav(&chord) {
                    return;
                }
                if matches!(self.focused_kind(), Some(WidgetKind::Keybindings { .. }))
                    && !is_nav_chord(&chord)
                {
                    self.search = chord;
                    self.search_cursor = self.search.chars().count();
                }
            }
            Some(WidgetKind::Log { .. }) => {
                if let Some(id) = self.focused.clone() {
                    let delta = match chord.as_str() {
                        "up" | "k" => -1,
                        "down" | "j" => 1,
                        "pageup" => -10,
                        "pagedown" => 10,
                        _ => 0,
                    };
                    if delta != 0 {
                        self.scroll_preview(&id, delta);
                        self.follow_tail.insert(id.clone(), self.log_at_bottom(&id));
                    }
                }
            }
            Some(WidgetKind::Menu { .. }) => {
                self.handle_menu_nav(&chord);
            }
            Some(WidgetKind::Splitter { .. }) => {
                self.handle_splitter_nav(&chord);
            }
            _ => {
                if matches!(
                    chord.as_str(),
                    "up" | "k" | "down" | "j" | "pageup" | "pagedown" | "home" | "end"
                ) {
                    // Fall back to first scrollable widget on the page.
                    if let Some(id) = self.first_scrollable_id() {
                        self.focused = Some(id);
                        self.handle_list_nav(&chord);
                    }
                }
            }
        }
    }

    fn match_search_bind(&self, chord: &str) -> bool {
        self.app.widgets.iter().any(|w| {
            if let WidgetKind::Search {
                bind: Some(bind), ..
            } = &w.kind
            {
                normalize_bind(bind) == normalize_bind(chord)
            } else {
                false
            }
        })
    }

    fn focus_search(&mut self) {
        if let Some(w) = self
            .app
            .widgets
            .iter()
            .find(|w| matches!(w.kind, WidgetKind::Search { .. }))
        {
            self.focused = Some(w.id.clone());
        }
    }

    fn handle_text_key(&mut self, key: KeyEvent, chord: &str) {
        match chord {
            "tab" => {
                self.focus_next();
                return;
            }
            "shift+tab" => {
                self.focus_prev();
                return;
            }
            "esc" => {
                if self.search_is_focused() && !self.search.is_empty() {
                    self.search.clear();
                    self.search_cursor = 0;
                } else {
                    self.focused = self
                        .focusable_ids()
                        .into_iter()
                        .find(|id| self.app.widget_kind(id).is_some_and(|k| !k.is_text_input()))
                        .or_else(|| self.focused.clone());
                }
                return;
            }
            "enter" => {
                if self.search_is_focused() {
                    return;
                }
                self.submit();
                return;
            }
            _ => {}
        }

        if self.search_is_focused() {
            let mut text = self.search.clone();
            let mut cursor = self.search_cursor;
            if apply_edit(key, &mut text, &mut cursor) {
                self.search = text;
                self.search_cursor = cursor;
                self.clamp_all_lists();
                self.refresh_previews();
            }
            return;
        }

        if let Some(id) = self.focused.clone()
            && let Some(WidgetKind::TextBox { .. }) = self.app.widget_kind(&id)
        {
            let mut text = self.text_values.get(&id).cloned().unwrap_or_default();
            let mut cursor = self
                .text_cursors
                .get(&id)
                .copied()
                .unwrap_or(text.chars().count());
            if apply_edit(key, &mut text, &mut cursor) {
                self.text_values.insert(id.clone(), text);
                self.text_cursors.insert(id, cursor);
            }
        }
    }

    fn search_is_focused(&self) -> bool {
        matches!(self.focused_kind(), Some(WidgetKind::Search { .. }))
    }

    fn handle_list_nav(&mut self, chord: &str) -> bool {
        let Some(id) = self.focused.clone() else {
            return false;
        };
        let len = self.filtered_len(&id);
        if len == 0 {
            return matches!(
                chord,
                "up" | "k" | "down" | "j" | "pageup" | "pagedown" | "home" | "end"
            );
        }
        let selected = self.selected.get(&id).copied().unwrap_or(0);
        let next = match chord {
            "up" | "k" => selected.saturating_sub(1),
            "down" | "j" => (selected + 1).min(len.saturating_sub(1)),
            "pageup" => selected.saturating_sub(10),
            "pagedown" => (selected + 10).min(len.saturating_sub(1)),
            "home" => 0,
            "end" => len.saturating_sub(1),
            _ => return false,
        };
        self.selected.insert(id.clone(), next);
        self.ensure_visible(&id, next);
        self.refresh_previews();
        true
    }

    fn handle_menu_nav(&mut self, chord: &str) {
        let Some(id) = self.focused.clone() else {
            return;
        };
        let len = match self.app.widget_kind(&id) {
            Some(WidgetKind::Menu { items }) => items.len(),
            _ => 0,
        };
        if len == 0 {
            return;
        }
        let selected = self.selected.get(&id).copied().unwrap_or(0);
        let next = match chord {
            "left" | "h" => selected.saturating_sub(1),
            "right" | "l" => (selected + 1).min(len.saturating_sub(1)),
            "home" => 0,
            "end" => len.saturating_sub(1),
            _ => return,
        };
        self.selected.insert(id, next);
    }

    fn handle_splitter_nav(&mut self, chord: &str) {
        let Some(id) = self.focused.clone() else {
            return;
        };
        let dir = match self.app.widget_kind(&id) {
            Some(WidgetKind::Splitter { direction, .. }) => *direction,
            _ => return,
        };
        let ratio = self.splitter_ratio.get(&id).copied().unwrap_or(500);
        let next = match (dir, chord) {
            (SplitDir::Horizontal, "left" | "h") | (SplitDir::Vertical, "up" | "k") => {
                ratio.saturating_sub(10).max(50)
            }
            (SplitDir::Horizontal, "right" | "l") | (SplitDir::Vertical, "down" | "j") => {
                (ratio + 10).min(950)
            }
            _ => return,
        };
        self.splitter_ratio.insert(id, next);
    }

    fn handle_tree_nav(&mut self, chord: &str) -> bool {
        match chord {
            "right" | "l" => {
                self.tree_set_expanded(true);
                true
            }
            "left" | "h" => {
                self.tree_set_expanded(false);
                true
            }
            _ => self.handle_list_nav(chord),
        }
    }

    fn tree_set_expanded(&mut self, expand: bool) {
        let Some(id) = self.focused.clone() else {
            return;
        };
        let (walk, column) = match self.app.widget_kind(&id) {
            Some(WidgetKind::Tree { walk, column, .. }) => (*walk, column.clone()),
            _ => return,
        };
        let rows = self.tree_rows(&id);
        let sel = self.selected.get(&id).copied().unwrap_or(0);
        let Some(row) = rows.get(sel).cloned() else {
            return;
        };
        if expand {
            if walk && let Some(dir) = tree::dir_path_for_row(&row.value, &column, &self.cwd) {
                let kids = tree::read_dir_listing(&dir, Span::unknown());
                self.tree_cache
                    .entry(id.clone())
                    .or_default()
                    .insert(row.path.clone(), kids);
            }
            self.tree_expanded.entry(id).or_default().insert(row.path);
        } else {
            self.tree_expanded.entry(id).or_default().remove(&row.path);
        }
        self.refresh_previews();
    }

    pub fn tree_rows(&self, id: &str) -> Vec<TreeRow> {
        let Some(WidgetKind::Tree { data, walk, column }) = self.app.widget_kind(id) else {
            return Vec::new();
        };
        let source = data.as_ref().unwrap_or(&self.app.data);
        let expanded = self.tree_expanded.get(id).cloned().unwrap_or_default();
        let cache = self.tree_cache.get(id).cloned().unwrap_or_default();
        let mut rows = tree::flatten(source, &expanded, *walk, column, &self.cwd, &cache);
        let q = self.query_for(id);
        if !q.is_empty() {
            let q = q.to_ascii_lowercase();
            rows.retain(|r| {
                r.label.to_ascii_lowercase().contains(&q) || value_matches(&r.value, &q)
            });
        }
        rows
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.handle_dialog_mouse(mouse) {
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.dragging = None;
                for (i, area) in self.tab_areas.iter().enumerate() {
                    if contains(*area, mouse.column, mouse.row) {
                        self.page = i;
                        self.focused = self.default_focus();
                        return;
                    }
                }
                for handle in &self.splitter_handles {
                    if contains(handle.area, mouse.column, mouse.row) {
                        self.dragging = Some(handle.id.clone());
                        self.focused = Some(handle.id.clone());
                        return;
                    }
                }
                let hit = self.hit_test(mouse.column, mouse.row);
                if let Some((id, area)) = hit {
                    if self.app.widget_kind(&id).is_some_and(|k| k.is_focusable()) {
                        self.focused = Some(id.clone());
                    }
                    if self.app.widget_kind(&id).is_some_and(|k| k.is_scrollable()) {
                        let inner_y = mouse.row.saturating_sub(area.y).saturating_sub(2) as usize;
                        let scroll = self.scroll.get(&id).copied().unwrap_or(0);
                        let idx = scroll + inner_y;
                        let len = self.filtered_len(&id);
                        if len > 0 {
                            self.selected.insert(id.clone(), idx.min(len - 1));
                            self.refresh_previews();
                        }
                    }
                    if let Some(WidgetKind::Menu { items }) = self.app.widget_kind(&id) {
                        if !items.is_empty() && area.width > 0 {
                            let rel = mouse.column.saturating_sub(area.x) as usize;
                            let slot = area.width as usize / items.len().max(1);
                            if slot > 0 {
                                self.selected.insert(id, (rel / slot).min(items.len() - 1));
                            }
                        }
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(id) = self.dragging.clone() {
                    self.drag_splitter(&id, mouse);
                }
            }
            MouseEventKind::Up(_) => {
                self.dragging = None;
            }
            MouseEventKind::ScrollDown => {
                self.scroll_at(mouse.column, mouse.row, 1);
            }
            MouseEventKind::ScrollUp => {
                self.scroll_at(mouse.column, mouse.row, -1);
            }
            _ => {}
        }
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
                match drag {
                    DialogDrag::Move { grab_x, grab_y } => {
                        let next = Rect {
                            x: mouse.column.saturating_sub(grab_x),
                            y: mouse.row.saturating_sub(grab_y),
                            width: rect.width,
                            height: rect.height,
                        };
                        if let Some(dialog) = self.dialog.as_mut() {
                            dialog.rect = clamp_dialog(next, screen);
                        }
                    }
                    DialogDrag::Resize => {
                        let width = mouse
                            .column
                            .saturating_sub(rect.x)
                            .saturating_add(1)
                            .max(20);
                        let height = mouse.row.saturating_sub(rect.y).saturating_add(1).max(8);
                        if let Some(dialog) = self.dialog.as_mut() {
                            dialog.rect = clamp_dialog(
                                Rect {
                                    x: rect.x,
                                    y: rect.y,
                                    width,
                                    height,
                                },
                                screen,
                            );
                        }
                    }
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

    fn drag_splitter(&mut self, id: &str, mouse: MouseEvent) {
        let handle = self.splitter_handles.iter().find(|h| h.id == id).cloned();
        let Some(handle) = handle else {
            return;
        };
        let area = handle.split_area;
        let ratio = match handle.direction {
            SplitDir::Horizontal => {
                if area.width <= 4 {
                    return;
                }
                ((mouse.column.saturating_sub(area.x)) as u32 * 1000 / area.width as u32) as u16
            }
            SplitDir::Vertical => {
                if area.height <= 4 {
                    return;
                }
                ((mouse.row.saturating_sub(area.y)) as u32 * 1000 / area.height as u32) as u16
            }
        };
        self.splitter_ratio
            .insert(id.to_string(), ratio.clamp(50, 950));
    }

    fn scroll_at(&mut self, x: u16, y: u16, delta: i32) {
        let id = self
            .hit_test(x, y)
            .map(|(id, _)| id)
            .or_else(|| self.focused.clone());
        let Some(id) = id else {
            return;
        };
        if matches!(
            self.app.widget_kind(&id),
            Some(WidgetKind::Preview { .. } | WidgetKind::Log { .. })
        ) {
            self.scroll_preview(&id, delta);
            if matches!(self.app.widget_kind(&id), Some(WidgetKind::Log { .. })) {
                self.follow_tail.insert(id.clone(), self.log_at_bottom(&id));
            }
            return;
        }
        if !self.app.widget_kind(&id).is_some_and(|k| k.is_scrollable()) {
            return;
        }
        self.focused = Some(id.clone());
        let len = self.filtered_len(&id);
        if len == 0 {
            return;
        }
        let selected = self.selected.get(&id).copied().unwrap_or(0);
        let next = if delta < 0 {
            selected.saturating_sub(1)
        } else {
            (selected + 1).min(len.saturating_sub(1))
        };
        self.selected.insert(id.clone(), next);
        self.ensure_visible(&id, next);
        self.refresh_previews();
    }

    fn focus_next(&mut self) {
        let ids = self.focusable_ids();
        if ids.is_empty() {
            return;
        }
        let next = match &self.focused {
            Some(cur) => ids
                .iter()
                .position(|id| id == cur)
                .map(|i| (i + 1) % ids.len())
                .unwrap_or(0),
            None => 0,
        };
        self.focused = Some(ids[next].clone());
    }

    fn focus_prev(&mut self) {
        let ids = self.focusable_ids();
        if ids.is_empty() {
            return;
        }
        let prev = match &self.focused {
            Some(cur) => ids
                .iter()
                .position(|id| id == cur)
                .map(|i| if i == 0 { ids.len() - 1 } else { i - 1 })
                .unwrap_or(0),
            None => ids.len() - 1,
        };
        self.focused = Some(ids[prev].clone());
    }

    fn next_page(&mut self) {
        let n = self.pages().len();
        if n == 0 {
            return;
        }
        self.page = (self.page + 1) % n;
        self.focused = self.default_focus();
        self.refresh_previews();
    }

    fn prev_page(&mut self) {
        let n = self.pages().len();
        if n == 0 {
            return;
        }
        self.page = if self.page == 0 { n - 1 } else { self.page - 1 };
        self.focused = self.default_focus();
        self.refresh_previews();
    }

    fn hit_test(&self, x: u16, y: u16) -> Option<(String, Rect)> {
        let mut best: Option<(u32, String, Rect)> = None;
        for (id, area) in &self.areas {
            if !contains(*area, x, y) {
                continue;
            }
            if matches!(self.app.widget_kind(id), Some(WidgetKind::Body { .. })) {
                continue;
            }
            let size = area.width as u32 * area.height as u32;
            let better = match best {
                None => true,
                Some((best_size, _, _)) => size < best_size,
            };
            if better {
                best = Some((size, id.clone(), *area));
            }
        }
        best.map(|(_, id, area)| (id, area))
    }

    fn first_scrollable_id(&self) -> Option<String> {
        let pages = self.pages();
        let page = pages.get(self.page)?;
        page.content.iter().find_map(|i| {
            let w = self.app.widgets.get(*i)?;
            w.kind.is_scrollable().then(|| w.id.clone())
        })
    }

    fn ensure_visible(&mut self, id: &str, selected: usize) {
        let height = self
            .areas
            .get(id)
            .map(|a| a.height.saturating_sub(3) as usize)
            .unwrap_or(10)
            .max(1);
        let scroll = self.scroll.entry(id.to_string()).or_insert(0);
        if selected < *scroll {
            *scroll = selected;
        } else if selected >= *scroll + height {
            *scroll = selected + 1 - height;
        }
    }

    fn clamp_all_lists(&mut self) {
        let ids: Vec<String> = self.app.widgets.iter().map(|w| w.id.clone()).collect();
        for id in ids {
            let len = self.filtered_len(&id);
            if let Some(sel) = self.selected.get_mut(&id) {
                if len == 0 {
                    *sel = 0;
                } else {
                    *sel = (*sel).min(len - 1);
                }
            }
        }
    }

    fn scroll_preview(&mut self, id: &str, delta: i32) {
        if delta == 0 {
            return;
        }
        let lines = if matches!(self.app.widget_kind(id), Some(WidgetKind::Log { .. })) {
            self.filtered_len(id)
        } else {
            self.preview_text
                .get(id)
                .map(|t| t.lines().count())
                .unwrap_or(0)
        };
        let height = self
            .areas
            .get(id)
            .map(|a| a.height.saturating_sub(2) as usize)
            .unwrap_or(10)
            .max(1);
        let max_scroll = lines.saturating_sub(height);
        let step = delta.unsigned_abs() as usize;
        let scroll = self.scroll.entry(id.to_string()).or_insert(0);
        if delta < 0 {
            *scroll = scroll.saturating_sub(step);
        } else {
            *scroll = (*scroll + step).min(max_scroll);
        }
    }

    pub fn refresh_previews(&mut self) {
        let previews: Vec<(String, String, usize, Option<Closure>, Option<String>)> = self
            .app
            .widgets
            .iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Preview {
                    column,
                    max_bytes,
                    transform,
                    from,
                } => Some((
                    w.id.clone(),
                    column.clone(),
                    *max_bytes,
                    transform.clone(),
                    from.clone(),
                )),
                _ => None,
            })
            .collect();

        let engine = self.preview_engine.take();
        for (id, column, max_bytes, transform, from) in previews {
            let table_id = from.or_else(|| self.preview_source_id());
            let row = table_id.as_ref().and_then(|tid| {
                let idx = self.selected.get(tid).copied().unwrap_or(0);
                self.filtered_rows(tid).into_iter().nth(idx)
            });
            let file = match &row {
                Some(row) => preview_for_row(row, &column, max_bytes, &self.cwd),
                None => FilePreview {
                    title: "preview".into(),
                    text: String::new(),
                    path: None,
                    transformable: false,
                },
            };
            let text = if file.transformable {
                if let (Some(closure), Some((engine_state, stack)), Some(row)) =
                    (transform, engine.as_ref(), row.as_ref())
                {
                    apply_preview_transform(
                        engine_state,
                        stack,
                        closure,
                        row,
                        &file.text,
                        file.path.as_deref(),
                    )
                } else {
                    file.text
                }
            } else {
                file.text
            };
            let changed = self.preview_title.get(&id) != Some(&file.title);
            self.preview_title.insert(id.clone(), file.title);
            self.preview_text.insert(id.clone(), text);
            if changed {
                self.scroll.insert(id, 0);
            }
        }
        self.preview_engine = engine;
    }

    fn preview_source_id(&self) -> Option<String> {
        if let Some(id) = &self.focused
            && matches!(
                self.app.widget_kind(id),
                Some(WidgetKind::Table { .. } | WidgetKind::List { .. } | WidgetKind::Tree { .. })
            )
        {
            return Some(id.clone());
        }
        let pages = self.pages();
        let page = pages.get(self.page)?;
        page.content.iter().find_map(|i| {
            let w = self.app.widgets.get(*i)?;
            matches!(
                w.kind,
                WidgetKind::Table { .. } | WidgetKind::List { .. } | WidgetKind::Tree { .. }
            )
            .then(|| w.id.clone())
        })
    }

    pub fn filtered_len(&self, id: &str) -> usize {
        self.filtered_rows(id).len()
    }

    pub fn filtered_rows(&self, id: &str) -> Vec<Value> {
        let Some(kind) = self.app.widget_kind(id) else {
            return Vec::new();
        };
        match kind {
            WidgetKind::Table { data, .. }
            | WidgetKind::List { data }
            | WidgetKind::Keybindings { data } => {
                let source = data.as_ref().unwrap_or(&self.app.data);
                filter_values(source, self.query_for(id))
            }
            WidgetKind::Log { .. } => filter_values(&self.app.data, self.query_for(id)),
            WidgetKind::Tree { .. } => self.tree_rows(id).into_iter().map(|r| r.value).collect(),
            _ => Vec::new(),
        }
    }

    fn query_for(&self, id: &str) -> &str {
        if self.search.is_empty() {
            return "";
        }
        let mut saw_search = false;
        let mut applies = false;
        for w in &self.app.widgets {
            if let WidgetKind::Search { target, .. } = &w.kind {
                saw_search = true;
                match target.as_deref() {
                    None => applies = true,
                    Some(t) if t == id => applies = true,
                    _ => {}
                }
            }
        }
        if !saw_search || applies {
            self.search.as_str()
        } else {
            ""
        }
    }

    pub fn table_columns(&self, id: &str) -> Vec<String> {
        match self.app.widget_kind(id) {
            Some(WidgetKind::Table { columns, data }) => {
                if !columns.is_empty() {
                    columns.clone()
                } else {
                    let source = data.as_ref().unwrap_or(&self.app.data);
                    let rows = as_list(source);
                    get_columns(rows)
                }
            }
            Some(WidgetKind::Keybindings { .. }) => {
                vec!["name".into(), "key".into(), "mode".into(), "event".into()]
            }
            Some(WidgetKind::List { data }) => {
                let source = data.as_ref().unwrap_or(&self.app.data);
                let rows = as_list(source);
                let cols = get_columns(rows);
                if cols.is_empty() {
                    vec!["item".into()]
                } else {
                    cols
                }
            }
            _ => Vec::new(),
        }
    }

    pub fn append_values(&mut self, values: Vec<Value>) {
        if values.is_empty() {
            return;
        }
        let follow: Vec<(String, bool)> = self
            .app
            .widgets
            .iter()
            .filter(|w| {
                matches!(
                    w.kind,
                    WidgetKind::Table { .. }
                        | WidgetKind::List { .. }
                        | WidgetKind::Log { .. }
                        | WidgetKind::Tree { .. }
                )
            })
            .map(|w| {
                let len = self.filtered_len(&w.id);
                let sel = self.selected.get(&w.id).copied().unwrap_or(0);
                let follow = match &w.kind {
                    WidgetKind::Log { .. } => *self.follow_tail.get(&w.id).unwrap_or(&true),
                    _ => len == 0 || sel + 1 >= len,
                };
                (w.id.clone(), follow)
            })
            .collect();

        let mut rows = match &self.app.data {
            Value::List { vals, .. } => vals.to_vec(),
            Value::Nothing { .. } => Vec::new(),
            other => vec![other.clone()],
        };
        rows.extend(values);
        let cap = self.stream_row_cap();
        if rows.len() > cap {
            let drop_n = rows.len() - cap;
            rows.drain(0..drop_n);
        }
        let span = rows.first().map(|v| v.span()).unwrap_or_else(Span::unknown);
        self.app.data = Value::list(rows, span);

        for (id, follow) in follow {
            if !follow {
                continue;
            }
            if matches!(self.app.widget_kind(&id), Some(WidgetKind::Log { .. })) {
                self.follow_tail.insert(id.clone(), true);
                let lines = self.filtered_len(&id);
                let height = self
                    .areas
                    .get(&id)
                    .map(|a| a.height.saturating_sub(2) as usize)
                    .unwrap_or(10)
                    .max(1);
                self.scroll.insert(id, lines.saturating_sub(height));
            } else {
                let len = self.filtered_len(&id);
                if len > 0 {
                    let last = len - 1;
                    self.selected.insert(id.clone(), last);
                    self.ensure_visible(&id, last);
                }
            }
        }
        self.refresh_previews();
    }

    fn stream_row_cap(&self) -> usize {
        const DEFAULT: usize = 10_000;
        let log_cap = self
            .app
            .widgets
            .iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Log { max_lines } => Some(*max_lines),
                _ => None,
            })
            .max()
            .unwrap_or(DEFAULT);
        log_cap.max(DEFAULT)
    }

    pub fn replace_data(&mut self, value: Value) {
        self.app.data = match value {
            Value::List { .. } => value,
            other if other.is_nothing() => Value::list(Vec::new(), other.span()),
            other => {
                let span = other.span();
                Value::list(vec![other], span)
            }
        };
        self.tree_cache.clear();
        self.tree_expanded.clear();
        self.clamp_all_lists();
        self.refresh_previews();
    }

    pub fn apply_refresh(
        &mut self,
        engine_state: &EngineState,
        stack: &Stack,
        closure: Closure,
        span: Span,
    ) {
        let eval = ClosureEvalOnce::new(engine_state, stack, closure);
        match eval.run_with_input(nu_protocol::PipelineData::empty()) {
            Ok(data) => {
                if let Some(meta) = data.metadata_ref()
                    && !meta.path_columns.is_empty()
                {
                    self.app.path_columns = meta.path_columns.clone();
                }
                match data.into_value(span) {
                    Ok(value) => {
                        self.refresh_error = None;
                        self.replace_data(value);
                    }
                    Err(err) => {
                        self.refresh_error = Some(truncate_chars(&err.to_string(), 80));
                    }
                }
            }
            Err(err) => {
                self.refresh_error = Some(truncate_chars(&err.to_string(), 80));
            }
        }
    }

    pub fn is_path_column(&self, column: &str, row: &Value) -> bool {
        if self.app.path_columns.iter().any(|c| c == column) {
            return true;
        }
        column == "name" && row.as_record().ok().and_then(|r| r.get("type")).is_some()
    }

    pub fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Nothing { .. } => String::new(),
            Value::String { val, .. } => val.clone(),
            other => {
                if let Some((engine_state, stack)) = &self.preview_engine {
                    other.to_abbreviated_string(stack.get_config(engine_state).as_ref())
                } else {
                    other.to_abbreviated_string(&Config::default())
                }
            }
        }
    }

    pub fn styled_cell(
        &self,
        row: &Value,
        column: &str,
        computer: Option<&StyleComputer>,
    ) -> (String, ratatui::style::Style) {
        let value = cell_value(row, column);
        let text = self.format_value(&value);
        let surface = self.theme.surface;
        if self.is_path_column(column, row)
            && let Some(path) = value_as_path(&value)
        {
            return (
                text,
                theme::path_cell_style(
                    &path,
                    &self.cwd,
                    &self.ls_colors,
                    surface,
                    self.use_ls_colors,
                ),
            );
        }
        let style = match computer {
            Some(computer) => theme::value_cell_style(&value, computer, surface),
            None => self.theme.text(),
        };
        (text, style)
    }

    pub fn style_computer(&self) -> Option<StyleComputer<'_>> {
        self.preview_engine
            .as_ref()
            .map(|(engine_state, stack)| StyleComputer::from_config(engine_state, stack))
    }

    pub fn tree_node_style(&self, row: &Value) -> ratatui::style::Style {
        if self.use_ls_colors
            && self.is_path_column("name", row)
            && let Some(path) = row_path_name(row)
        {
            return theme::path_cell_style(
                path,
                &self.cwd,
                &self.ls_colors,
                self.theme.surface,
                self.use_ls_colors,
            );
        }
        match self.style_computer() {
            Some(computer) => theme::value_cell_style(row, &computer, self.theme.surface),
            None => self.theme.text(),
        }
    }

    fn log_at_bottom(&self, id: &str) -> bool {
        let lines = self.filtered_len(id);
        let height = self
            .areas
            .get(id)
            .map(|a| a.height.saturating_sub(2) as usize)
            .unwrap_or(10)
            .max(1);
        let scroll = self.scroll.get(id).copied().unwrap_or(0);
        scroll + height >= lines
    }

    pub fn selected_value(&self) -> Value {
        if let Some(id) = &self.focused {
            if let Some(WidgetKind::TextBox { .. }) = self.app.widget_kind(id) {
                return Value::string(
                    self.text_values.get(id).cloned().unwrap_or_default(),
                    Span::unknown(),
                );
            }
            if let Some(WidgetKind::Menu { items }) = self.app.widget_kind(id) {
                let idx = self.selected.get(id).copied().unwrap_or(0);
                if let Some(item) = items.get(idx) {
                    return Value::string(item.clone(), Span::unknown());
                }
            }
            let rows = self.filtered_rows(id);
            let idx = self.selected.get(id).copied().unwrap_or(0);
            if let Some(row) = rows.get(idx) {
                return row.clone();
            }
        }
        // Prefer a selected table/keybindings row even if focus is elsewhere.
        for w in &self.app.widgets {
            if w.kind.is_scrollable() {
                let rows = self.filtered_rows(&w.id);
                let idx = self.selected.get(&w.id).copied().unwrap_or(0);
                if let Some(row) = rows.get(idx) {
                    return row.clone();
                }
            }
        }
        Value::nothing(Span::unknown())
    }

    fn submit(&mut self) {
        self.outcome = Some(Outcome {
            action: Action::Submit,
            selected: self.selected_value(),
        });
    }

    fn quit(&mut self) {
        self.outcome = Some(Outcome {
            action: Action::Quit,
            selected: Value::nothing(Span::unknown()),
        });
    }

    pub fn result_record(&self, span: Span, screen: Option<String>) -> Value {
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
        rec.insert("search", Value::string(self.search.clone(), span));
        rec.insert("page", Value::int(self.page as i64, span));
        let selected = self
            .outcome
            .as_ref()
            .map(|o| o.selected.clone())
            .unwrap_or_else(|| self.selected_value());
        rec.insert("selected", selected);

        let mut values = Record::new();
        for (id, text) in &self.text_values {
            values.insert(id.clone(), Value::string(text.clone(), span));
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
        Value::record(rec, span)
    }

    pub fn status_text(&self) -> String {
        let extra = self.live_status_bits();
        if let Some(text) = self.app.widgets.iter().find_map(|w| match &w.kind {
            WidgetKind::Status { text } => Some(text.clone()),
            _ => None,
        }) {
            if extra.is_empty() {
                text
            } else {
                format!("{text}  {extra}")
            }
        } else {
            extra
        }
    }

    fn live_status_bits(&self) -> String {
        let mut bits = Vec::new();
        if let Some(err) = &self.refresh_error {
            bits.push(format!("refresh error:{err}"));
        }
        if let Some(id) = &self.focused {
            bits.push(format!("focus:{id}"));
        }
        if !self.search.is_empty() {
            bits.push(format!("filter:{}", self.search));
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
}

fn implicit_page() -> Page {
    Page {
        title: "main".into(),
        body_id: None,
        splitter: None,
        content: Vec::new(),
    }
}

fn percent_to_permille(percent: u16) -> u16 {
    (percent.clamp(10, 90) as u32 * 10) as u16
}

fn digit_page(chord: &str) -> Option<usize> {
    if chord.len() != 1 {
        return None;
    }
    let c = chord.chars().next()?;
    if c.is_ascii_digit() && c != '0' {
        Some((c as u8 - b'1') as usize)
    } else {
        None
    }
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

fn is_nav_chord(chord: &str) -> bool {
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
            | "tab"
            | "shift+tab"
            | "enter"
            | "esc"
            | "q"
    )
}

fn apply_edit(key: KeyEvent, text: &mut String, cursor: &mut usize) -> bool {
    let len = text.chars().count();
    *cursor = (*cursor).min(len);
    match (key.code, key.modifiers) {
        (KeyCode::Char(c), m)
            if !m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT) =>
        {
            insert_char(text, cursor, c);
            true
        }
        (KeyCode::Backspace, _) => {
            if *cursor > 0 {
                remove_char(text, *cursor - 1);
                *cursor -= 1;
            }
            true
        }
        (KeyCode::Delete, _) => {
            if *cursor < text.chars().count() {
                remove_char(text, *cursor);
            }
            true
        }
        (KeyCode::Left, _) => {
            *cursor = cursor.saturating_sub(1);
            true
        }
        (KeyCode::Right, _) => {
            *cursor = (*cursor + 1).min(text.chars().count());
            true
        }
        (KeyCode::Home, _) => {
            *cursor = 0;
            true
        }
        (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => {
            *cursor = 0;
            true
        }
        (KeyCode::End, _) => {
            *cursor = text.chars().count();
            true
        }
        (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
            *cursor = text.chars().count();
            true
        }
        (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
            text.clear();
            *cursor = 0;
            true
        }
        _ => false,
    }
}

fn insert_char(text: &mut String, cursor: &mut usize, c: char) {
    let byte = byte_index(text, *cursor);
    text.insert(byte, c);
    *cursor += 1;
}

fn remove_char(text: &mut String, char_idx: usize) {
    let start = byte_index(text, char_idx);
    let end = byte_index(text, char_idx + 1);
    text.replace_range(start..end, "");
}

fn byte_index(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}

pub fn as_list(value: &Value) -> &[Value] {
    value.as_list().ok().unwrap_or(&[])
}

fn cell_value(row: &Value, column: &str) -> Value {
    match row {
        Value::Record { val, .. } => val
            .get(column)
            .cloned()
            .unwrap_or_else(|| Value::nothing(Span::unknown())),
        other if column == "item" || column == "value" => other.clone(),
        other => other.clone(),
    }
}

fn row_path_name(row: &Value) -> Option<&str> {
    match row {
        Value::Record { val, .. } => val.get("name").and_then(|v| v.as_str().ok()),
        Value::String { val, .. } => Some(val.as_str()),
        _ => None,
    }
}

fn filter_values(source: &Value, query: &str) -> Vec<Value> {
    let rows: Vec<Value> = match source {
        Value::List { vals, .. } => vals.to_vec(),
        Value::Nothing { .. } => Vec::new(),
        other => vec![other.clone()],
    };
    if query.is_empty() {
        return rows;
    }
    let q = query.to_ascii_lowercase();
    rows.into_iter().filter(|v| value_matches(v, &q)).collect()
}

fn value_matches(value: &Value, query: &str) -> bool {
    match value {
        Value::Record { val, .. } => {
            let key = formatted_key(val);
            if key.to_ascii_lowercase().contains(query) {
                return true;
            }
            val.iter()
                .any(|(k, v)| k.to_ascii_lowercase().contains(query) || value_matches(v, query))
        }
        Value::List { vals, .. } => vals.iter().any(|v| value_matches(v, query)),
        Value::String { val, .. } => val.to_ascii_lowercase().contains(query),
        other => other
            .to_expanded_string(", ", &nu_protocol::Config::default())
            .to_ascii_lowercase()
            .contains(query),
    }
}

pub fn formatted_key(record: &nu_protocol::Record) -> String {
    let modifier = record
        .get("modifier")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("");
    let keycode = record
        .get("keycode")
        .and_then(|v| v.as_str().ok())
        .unwrap_or("");
    normalize_binding(modifier, keycode)
}

/// Map reedline config keys (`control` + `char_r`) onto `key_event_to_string` (`ctrl+r`).
pub fn normalize_binding(modifier: &str, keycode: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let modifier = modifier.to_ascii_lowercase().replace('-', "_");
    if modifier != "none" && !modifier.is_empty() {
        if modifier.contains("ctrl") || modifier.contains("control") {
            parts.push("ctrl".into());
        }
        if modifier.contains("alt") {
            parts.push("alt".into());
        }
        if modifier.contains("super") || modifier.contains("meta") {
            parts.push("super".into());
        }
        if modifier.contains("shift") {
            parts.push("shift".into());
        }
    }

    let key = keycode.to_ascii_lowercase();
    let key = if let Some(rest) = key.strip_prefix("char_") {
        rest.to_string()
    } else {
        key
    };
    if !key.is_empty() {
        parts.push(key);
    }
    parts.join("+")
}

pub fn format_event_value(value: &Value) -> String {
    match value {
        Value::Record { val, .. } => {
            if let Some(send) = val.get("send").and_then(|v| v.as_str().ok()) {
                send.to_string()
            } else if let Some(cmd) = val.get("cmd").and_then(|v| v.as_str().ok()) {
                format!("cmd:{cmd}")
            } else {
                compact_value(value)
            }
        }
        Value::String { val, .. } => val.clone(),
        Value::List { .. } => compact_value(value),
        other => compact_value(other),
    }
}

fn compact_value(value: &Value) -> String {
    let s = value.to_expanded_string(", ", &nu_protocol::Config::default());
    truncate_chars(&s, 37)
}

struct FilePreview {
    title: String,
    text: String,
    path: Option<PathBuf>,
    transformable: bool,
}

fn preview_for_row(row: &Value, column: &str, max_bytes: usize, cwd: &Path) -> FilePreview {
    let Some(name) = row_path(row, column) else {
        return FilePreview {
            title: "preview".into(),
            text: "(no path on this row)".into(),
            path: None,
            transformable: false,
        };
    };
    let path = {
        let p = Path::new(&name);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    };
    let title = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name.as_str())
        .to_string();

    let kind = row_type(row);
    if kind.as_deref() == Some("dir") || kind.as_deref() == Some("directory") {
        return FilePreview {
            title,
            text: format!("{name}/\n(directory)"),
            path: Some(path),
            transformable: false,
        };
    }

    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_dir() => FilePreview {
            title,
            text: format!("{name}/\n(directory)"),
            path: Some(path),
            transformable: false,
        },
        Ok(meta) if meta.is_file() => {
            let (text, transformable) = read_file_preview(&path, max_bytes, meta.len());
            FilePreview {
                title,
                text,
                path: Some(path),
                transformable,
            }
        }
        Ok(_) => FilePreview {
            title,
            text: format!("{name}\n(not a regular file)"),
            path: Some(path),
            transformable: false,
        },
        Err(err) => FilePreview {
            title,
            text: if kind.as_deref() == Some("file") {
                format!("cannot read {name}: {err}")
            } else {
                format!("{name}\n({err})")
            },
            path: Some(path),
            transformable: false,
        },
    }
}

fn apply_preview_transform(
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    row: &Value,
    text: &str,
    path: Option<&Path>,
) -> String {
    let wants_row = {
        let block = engine_state.get_block(closure.block_id);
        !block.signature.required_positional.is_empty()
            || !block.signature.optional_positional.is_empty()
    };

    let metadata = PipelineMetadata {
        data_source: path
            .map(|p| DataSource::FilePath(p.to_path_buf()))
            .unwrap_or_default(),
        content_type: path.and_then(preview_content_type),
        ..Default::default()
    };
    let input =
        Value::string(text, Span::unknown()).into_pipeline_data_with_metadata(Some(metadata));

    let mut eval = ClosureEvalOnce::new(engine_state, stack, closure);
    if wants_row {
        eval = match eval.add_arg(row.clone()) {
            Ok(eval) => eval,
            Err(err) => return format!("preview error: {err}"),
        };
    }

    match eval.run_with_input(input) {
        Ok(data) => pipeline_to_preview_text(data, engine_state),
        Err(err) => format!("preview error: {err}"),
    }
}

fn pipeline_to_preview_text(data: nu_protocol::PipelineData, engine_state: &EngineState) -> String {
    match data.into_value(Span::unknown()) {
        Ok(Value::String { val, .. }) => val,
        Ok(Value::Binary { val, .. }) => String::from_utf8_lossy(&val).into_owned(),
        Ok(Value::Nothing { .. }) => String::new(),
        Ok(other) => other.to_expanded_string("\n", engine_state.get_config()),
        Err(err) => format!("preview error: {err}"),
    }
}

fn preview_content_type(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "yaml" | "yml" => Some("application/yaml".into()),
        "nu" => Some("application/x-nuscript".into()),
        "json" | "jsonl" | "ndjson" => Some("application/json".into()),
        "nuon" => Some("application/x-nuon".into()),
        other => mime_guess::from_ext(other).first().map(|m| m.to_string()),
    }
}

fn row_path(row: &Value, column: &str) -> Option<String> {
    match row {
        Value::Record { val, .. } => val
            .get(column)
            .and_then(value_as_path)
            .or_else(|| val.get("name").and_then(value_as_path)),
        Value::String { val, .. } => Some(val.clone()),
        _ => None,
    }
}

fn row_type(row: &Value) -> Option<String> {
    row.as_record()
        .ok()
        .and_then(|r| r.get("type"))
        .and_then(|v| v.as_str().ok())
        .map(|s| s.to_ascii_lowercase())
}

fn value_as_path(value: &Value) -> Option<String> {
    match value {
        Value::String { val, .. } => Some(val.clone()),
        Value::Glob { val, .. } => Some(val.clone()),
        other => other.as_str().ok().map(|s| s.to_string()),
    }
}

fn read_file_preview(path: &Path, max_bytes: usize, file_len: u64) -> (String, bool) {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(err) => return (format!("cannot open: {err}"), false),
    };
    let mut buf = vec![0u8; max_bytes];
    let n = match std::io::Read::read(&mut file, &mut buf) {
        Ok(n) => n,
        Err(err) => return (format!("cannot read: {err}"), false),
    };
    buf.truncate(n);
    if buf.contains(&0) {
        return (format!("(binary file, {file_len} bytes)"), false);
    }
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if file_len as usize > n {
        text.push_str(&format!("\n\n… truncated, showing {n} of {file_len} bytes"));
    }
    (text, true)
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
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
    use crate::tui::widget::Widget;
    use crossterm::event::KeyEvent;

    fn table_app() -> TuiApp {
        let rows = vec![
            Value::test_record(record_from(&[("name", "alpha"), ("size", "1")])),
            Value::test_record(record_from(&[("name", "beta"), ("size", "2")])),
            Value::test_record(record_from(&[("name", "gamma"), ("size", "3")])),
        ];
        let mut app = TuiApp::new();
        app.data = Value::test_list(rows);
        app.push(Widget {
            id: "table-0".into(),
            kind: WidgetKind::Table {
                columns: vec!["name".into(), "size".into()],
                data: None,
            },
            place: None,
        });
        app.push(Widget {
            id: "search-0".into(),
            kind: WidgetKind::Search {
                placeholder: "filter".into(),
                bind: Some("ctrl+r".into()),
                target: None,
            },
            place: None,
        });
        app
    }

    fn record_from(pairs: &[(&str, &str)]) -> Record {
        let mut rec = Record::new();
        for (k, v) in pairs {
            rec.insert((*k).to_string(), Value::test_string(*v));
        }
        rec
    }

    fn press(session: &mut Session, code: KeyCode, mods: KeyModifiers) {
        session.handle_event(&Event::Key(KeyEvent::new(code, mods)));
    }

    #[test]
    fn down_moves_table_selection() {
        let mut session = Session::new(table_app());
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(session.selected.get("table-0").copied(), Some(1));
    }

    #[test]
    fn q_quits_when_not_editing() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(matches!(
            session.outcome.as_ref().map(|o| o.action),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn q_types_when_search_focused() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        press(&mut session, KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(session.search, "q");
        assert!(session.outcome.is_none());
    }

    #[test]
    fn search_filters_table() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        press(&mut session, KeyCode::Char('g'), KeyModifiers::NONE);
        press(&mut session, KeyCode::Char('a'), KeyModifiers::NONE);
        let rows = session.filtered_rows("table-0");
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn tab_cycles_focus() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        press(&mut session, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
    }

    #[test]
    fn enter_submits_selected_row() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        press(&mut session, KeyCode::Enter, KeyModifiers::NONE);
        let selected = session.outcome.expect("submitted").selected;
        let name = selected
            .as_record()
            .expect("record")
            .get("name")
            .expect("name")
            .as_str()
            .expect("str");
        assert_eq!(name, "beta");
    }

    #[test]
    fn slash_types_in_search_when_already_focused() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        press(&mut session, KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(session.search, "/");
    }

    fn keybindings_app() -> TuiApp {
        let rows = vec![
            Value::test_record(record_from(&[
                ("name", "history"),
                ("modifier", "control"),
                ("keycode", "char_r"),
                ("mode", "emacs"),
            ])),
            Value::test_record(record_from(&[
                ("name", "clear"),
                ("modifier", "control"),
                ("keycode", "char_l"),
                ("mode", "emacs"),
            ])),
        ];
        let mut app = TuiApp::new();
        app.data = Value::test_list(rows);
        app.push(Widget {
            id: "keybindings-0".into(),
            kind: WidgetKind::Keybindings { data: None },
            place: None,
        });
        app
    }

    #[test]
    fn keybinding_chord_filters_config_rows() {
        let mut session = Session::new(keybindings_app());
        assert_eq!(session.focused.as_deref(), Some("keybindings-0"));
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        let rows = session.filtered_rows("keybindings-0");
        assert_eq!(rows.len(), 1);
        let name = rows[0]
            .as_record()
            .expect("record")
            .get("name")
            .expect("name")
            .as_str()
            .expect("str");
        assert_eq!(name, "history");
    }

    #[test]
    fn normalize_control_char_r() {
        assert_eq!(normalize_binding("control", "char_r"), "ctrl+r");
        assert_eq!(normalize_binding("none", "enter"), "enter");
    }

    #[test]
    fn preview_follows_table_selection_without_stealing_focus() {
        let dir = std::env::temp_dir();
        let file_a = dir.join("nu-tui-preview-a.txt");
        let file_b = dir.join("nu-tui-preview-b.txt");
        std::fs::write(&file_a, "alpha-contents").expect("write a");
        std::fs::write(&file_b, "beta-contents").expect("write b");

        let rows = vec![
            Value::test_record(record_from(&[
                ("name", file_a.to_str().expect("utf8")),
                ("type", "file"),
            ])),
            Value::test_record(record_from(&[
                ("name", file_b.to_str().expect("utf8")),
                ("type", "file"),
            ])),
        ];
        let mut app = TuiApp::new();
        app.data = Value::test_list(rows);
        app.push(Widget {
            id: "table-0".into(),
            kind: WidgetKind::Table {
                columns: vec!["name".into()],
                data: None,
            },
            place: None,
        });
        app.push(Widget {
            id: "preview-0".into(),
            kind: WidgetKind::Preview {
                column: "name".into(),
                max_bytes: 4096,
                transform: None,
                from: None,
            },
            place: None,
        });

        let mut session = Session::with_cwd(app, dir);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        assert!(
            session
                .preview_text
                .get("preview-0")
                .is_some_and(|t| t.contains("alpha-contents"))
        );

        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        assert!(
            session
                .preview_text
                .get("preview-0")
                .is_some_and(|t| t.contains("beta-contents"))
        );
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

    #[test]
    fn dialog_close_button_quits() {
        let mut session = Session::new(table_app());
        session.enable_dialog(Rect::new(0, 0, 80, 24), Some(40), Some(12));
        let rect = session.dialog_rect().expect("dialog");
        click(&mut session, rect.x + rect.width - 2, rect.y);
        assert!(matches!(
            session.outcome.as_ref().map(|o| o.action),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn dialog_title_drag_moves() {
        let mut session = Session::new(table_app());
        session.enable_dialog(Rect::new(0, 0, 80, 24), Some(40), Some(12));
        let orig = session.dialog_rect().expect("dialog");
        click(&mut session, orig.x + 2, orig.y);
        drag(&mut session, orig.x + 7, orig.y + 1);
        let moved = session.dialog_rect().expect("moved");
        assert_eq!(moved.x, orig.x + 5);
        assert_eq!(moved.y, orig.y + 1);
        assert_eq!(moved.width, orig.width);
    }

    #[test]
    fn dialog_corner_drag_resizes() {
        let mut session = Session::new(table_app());
        session.enable_dialog(Rect::new(0, 0, 80, 24), Some(40), Some(12));
        let orig = session.dialog_rect().expect("dialog");
        click(
            &mut session,
            orig.x + orig.width - 1,
            orig.y + orig.height - 1,
        );
        drag(
            &mut session,
            orig.x + orig.width + 4,
            orig.y + orig.height + 2,
        );
        let resized = session.dialog_rect().expect("resized");
        assert!(resized.width > orig.width);
        assert!(resized.height > orig.height);
    }

    #[test]
    fn tree_right_expands_nested_record() {
        let mut rec_b = Record::new();
        rec_b.insert("b", Value::test_int(1));
        let mut rec = Record::new();
        rec.insert("a", Value::test_record(rec_b));
        let mut app = TuiApp::new();
        app.data = Value::test_record(rec);
        app.push(Widget {
            id: "tree-0".into(),
            kind: WidgetKind::Tree {
                data: None,
                walk: false,
                column: "name".into(),
            },
            place: None,
        });
        let mut session = Session::new(app);
        assert_eq!(session.tree_rows("tree-0").len(), 1);
        press(&mut session, KeyCode::Right, KeyModifiers::NONE);
        let rows = session.tree_rows("tree-0");
        assert!(rows.len() > 1);
        assert!(rows.iter().all(|r| r.label != "value"));
        press(&mut session, KeyCode::Down, KeyModifiers::NONE);
        press(&mut session, KeyCode::Right, KeyModifiers::NONE);
        let again = session.tree_rows("tree-0");
        assert_eq!(again.len(), rows.len());
        assert!(again.iter().all(|r| r.label != "value"));
    }

    #[test]
    fn tab_digit_jumps_page() {
        let mut app = TuiApp::new();
        app.push(Widget {
            id: "tab-0".into(),
            kind: WidgetKind::Tab {
                title: "one".into(),
            },
            place: None,
        });
        app.push(Widget {
            id: "label-0".into(),
            kind: WidgetKind::Label {
                text: "PAGE-ONE".into(),
            },
            place: None,
        });
        app.push(Widget {
            id: "tab-1".into(),
            kind: WidgetKind::Tab {
                title: "two".into(),
            },
            place: None,
        });
        app.push(Widget {
            id: "label-1".into(),
            kind: WidgetKind::Label {
                text: "PAGE-TWO".into(),
            },
            place: None,
        });
        let mut session = Session::new(app);
        assert_eq!(session.page, 0);
        press(&mut session, KeyCode::Char('2'), KeyModifiers::NONE);
        assert_eq!(session.page, 1);
    }

    #[test]
    fn splitter_drag_tracks_mouse_in_permille() {
        let mut app = TuiApp::new();
        app.push(Widget {
            id: "split-0".into(),
            kind: WidgetKind::Splitter {
                direction: SplitDir::Horizontal,
                ratio: 50,
            },
            place: None,
        });
        app.push(Widget {
            id: "label-a".into(),
            kind: WidgetKind::Label { text: "A".into() },
            place: None,
        });
        app.push(Widget {
            id: "label-b".into(),
            kind: WidgetKind::Label { text: "B".into() },
            place: None,
        });
        let mut session = Session::new(app);
        session.layout(Rect::new(0, 0, 80, 24));
        assert_eq!(session.splitter_ratio.get("split-0").copied(), Some(500));
        let handle = session
            .splitter_handles
            .iter()
            .find(|h| h.id == "split-0")
            .cloned()
            .expect("handle");
        click(&mut session, handle.area.x, handle.area.y);
        drag(&mut session, 20, handle.area.y);
        let ratio = session.splitter_ratio.get("split-0").copied().unwrap_or(0);
        assert!(
            ratio < 500,
            "expected left drag to shrink first pane, got {ratio}"
        );
        assert!(ratio >= 50);
    }

    #[test]
    fn log_pageup_scrolls_more_than_one_line() {
        let mut app = TuiApp::new();
        app.data = Value::test_list((0..40).map(Value::test_int).collect());
        app.push(Widget {
            id: "log-0".into(),
            kind: WidgetKind::Log { max_lines: 10_000 },
            place: None,
        });
        let mut session = Session::new(app);
        session.layout(Rect::new(0, 0, 40, 12));
        session.scroll.insert("log-0".into(), 20);
        session.follow_tail.insert("log-0".into(), true);
        press(&mut session, KeyCode::PageUp, KeyModifiers::NONE);
        assert_eq!(session.scroll.get("log-0").copied(), Some(10));
        assert_eq!(session.follow_tail.get("log-0").copied(), Some(false));
    }

    #[test]
    fn search_target_does_not_filter_other_widgets() {
        let mut app = TuiApp::new();
        app.data = Value::test_list(vec![
            Value::test_string("alpha"),
            Value::test_string("beta"),
        ]);
        app.push(Widget {
            id: "search-0".into(),
            kind: WidgetKind::Search {
                placeholder: "filter".into(),
                bind: None,
                target: Some("table-0".into()),
            },
            place: None,
        });
        app.push(Widget {
            id: "table-0".into(),
            kind: WidgetKind::Table {
                columns: vec!["item".into()],
                data: None,
            },
            place: None,
        });
        app.push(Widget {
            id: "list-0".into(),
            kind: WidgetKind::List { data: None },
            place: None,
        });
        let mut session = Session::new(app);
        session.search = "al".into();
        assert_eq!(session.filtered_len("table-0"), 1);
        assert_eq!(session.filtered_len("list-0"), 2);
    }
}
