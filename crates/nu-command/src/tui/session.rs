//! Mutable interactive state: focus, typing, scrolling, filtering, pages.
use super::app::{TuiApp, widget_to_record};
use super::keys::{key_event_to_string, normalize_bind};
use super::layout::{SplitterHandle, assign_areas, percent_to_permille, subtree_ids};
use super::theme::{self, Theme};
use super::tree::{self, TreeRow};
use super::widget::{MenuItem, Slot, SplitDir, Widget, WidgetKind};
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
use std::fmt::Write;
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

/// One entry in the tab bar: a top-level `Tab` widget.
#[derive(Debug, Clone)]
pub struct Page {
    pub title: String,
    /// Widget id of the tab, or `None` for the implicit page when there are
    /// no top-level tabs.
    pub id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub app: TuiApp,
    /// Child-index path of every widget id (see [`TuiApp::paths`]).
    pub paths: HashMap<String, Vec<usize>>,
    pub page: usize,
    pub focused: Option<String>,
    /// Text of every text box and search box, plus the captured chord of a
    /// `capture_keys` table that has no search box in scope.
    pub text_values: HashMap<String, String>,
    pub text_cursors: HashMap<String, usize>,
    pub selected: HashMap<String, usize>,
    /// Menu whose dropdown is open, and the highlighted row in each dropdown.
    pub menu_open: Option<String>,
    pub menu_item: HashMap<String, usize>,
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
    /// Last failure from a refresh or menu-action closure; shown on the status bar.
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

        for w in app.iter() {
            match &w.kind {
                WidgetKind::TextBox { value, .. } => {
                    text_cursors.insert(w.id.clone(), value.chars().count());
                    text_values.insert(w.id.clone(), value.clone());
                }
                WidgetKind::Search { .. } => {
                    text_cursors.insert(w.id.clone(), 0);
                    text_values.insert(w.id.clone(), String::new());
                }
                WidgetKind::Split { ratio, .. } => {
                    splitter_ratio.insert(w.id.clone(), percent_to_permille(*ratio));
                }
                WidgetKind::Menu { .. } | WidgetKind::Table { .. } | WidgetKind::Tree { .. } => {
                    selected.insert(w.id.clone(), 0);
                }
                _ => {}
            }
        }
        let paths = app.paths();

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
            paths,
            page: 0,
            focused: None,
            text_values,
            text_cursors,
            selected,
            menu_open: None,
            menu_item: HashMap::new(),
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

    /// Tab-bar entries: top-level `Tab` widgets in order, or one implicit
    /// page when there are none.
    pub fn pages(&self) -> Vec<Page> {
        let pages: Vec<Page> = self
            .app
            .widgets
            .iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Tab { title } => Some(Page {
                    title: title.clone(),
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
            .any(|w| matches!(w.kind, WidgetKind::Tab { .. }))
    }

    pub fn is_root(&self, id: &str) -> bool {
        self.paths.get(id).is_some_and(|p| p.len() == 1)
    }

    /// Ids of the widgets on screen right now, preorder: bare content roots
    /// and the active tab's subtree. Chrome and hidden tabs are excluded.
    /// Computed from the tree, not from `areas`, so it is valid before the
    /// first layout pass.
    pub fn visible_ids(&self) -> Vec<String> {
        let active = self.pages().get(self.page).and_then(|p| p.id.clone());
        let mut out = Vec::new();
        for (i, w) in self.app.widgets.iter().enumerate() {
            match &w.kind {
                WidgetKind::Tab { .. } => {
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

    fn default_focus(&self) -> Option<String> {
        let ids = self.focusable_ids();
        let preferred: [fn(&WidgetKind) -> bool; 7] = [
            |k| matches!(k, WidgetKind::Table { .. }),
            |k| matches!(k, WidgetKind::Tree { .. }),
            |k| matches!(k, WidgetKind::Log { .. }),
            |k| matches!(k, WidgetKind::TextBox { .. }),
            |k| matches!(k, WidgetKind::Search { .. }),
            |k| matches!(k, WidgetKind::Menu { .. }),
            |k| matches!(k, WidgetKind::Split { .. }),
        ];
        for wanted in preferred {
            if let Some(id) = ids
                .iter()
                .find(|id| self.app.widget_kind(id).is_some_and(wanted))
            {
                return Some(id.clone());
            }
        }
        ids.into_iter().next()
    }

    /// Tab order: top-level menu/search first, then visible leaves, then
    /// visible split handles last.
    pub fn focusable_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .app
            .widgets
            .iter()
            .filter(|w| matches!(w.kind, WidgetKind::Menu { .. } | WidgetKind::Search { .. }))
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
                .filter(|w| matches!(w.kind, WidgetKind::Split { .. }))
                .map(|w| w.id.clone()),
        );
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

        if !self.is_editing()
            && let Some(id) = self.search_bound_to(&chord)
        {
            self.focused = Some(id);
            return;
        }

        if self.is_editing() {
            self.handle_text_key(key, &chord);
            return;
        }

        // Alt+mnemonic opens a menu-bar item from anywhere.
        if let Some(letter) = chord.strip_prefix("alt+").and_then(single_char)
            && let Some((id, idx)) = self.menu_bar_item_with_mnemonic(letter)
        {
            self.focused = Some(id.clone());
            self.selected.insert(id.clone(), idx);
            self.menu_open_or_activate(&id);
            return;
        }

        if self.menu_open.is_some() {
            self.handle_dropdown_key(&chord);
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
                match self.focused.clone() {
                    Some(id) if matches!(self.focused_kind(), Some(WidgetKind::Menu { .. })) => {
                        self.menu_open_or_activate(&id)
                    }
                    _ => self.submit(),
                }
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
                self.handle_tree_nav(&chord);
            }
            Some(WidgetKind::Table { capture_keys, .. }) => {
                let capture = *capture_keys;
                if self.handle_list_nav(&chord) {
                    return;
                }
                if capture
                    && !is_nav_chord(&chord)
                    && let Some(id) = self.focused.clone()
                {
                    self.capture_chord(&id, chord);
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
            Some(WidgetKind::Split { .. }) => {
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

    /// The search box (visible or top-level) whose `--bind` matches `chord`.
    fn search_bound_to(&self, chord: &str) -> Option<String> {
        let wanted = normalize_bind(chord);
        self.app
            .iter()
            .find(|w| match &w.kind {
                WidgetKind::Search {
                    bind: Some(bind), ..
                } => normalize_bind(bind) == wanted,
                _ => false,
            })
            .map(|w| w.id.clone())
    }

    /// Text of a search box, or the captured chord of a table.
    pub fn query_text(&self, id: &str) -> &str {
        self.text_values.get(id).map(String::as_str).unwrap_or("")
    }

    /// Store a chord pressed on a `capture_keys` table as the filter query:
    /// in the search box that scopes the table if there is one, otherwise on
    /// the table itself.
    fn capture_chord(&mut self, table_id: &str, chord: String) {
        let target = self
            .scoping_search(table_id)
            .unwrap_or_else(|| table_id.to_string());
        self.text_cursors
            .insert(target.clone(), chord.chars().count());
        self.text_values.insert(target, chord);
        self.clamp_all_lists();
        self.refresh_previews();
    }

    fn handle_text_key(&mut self, key: KeyEvent, chord: &str) {
        let Some(id) = self.focused.clone() else {
            return;
        };
        let is_search = self.search_is_focused();
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
                if is_search && !self.query_text(&id).is_empty() {
                    self.text_values.insert(id.clone(), String::new());
                    self.text_cursors.insert(id, 0);
                    self.clamp_all_lists();
                    self.refresh_previews();
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
                self.submit();
                return;
            }
            _ => {}
        }

        let mut text = self.text_values.get(&id).cloned().unwrap_or_default();
        let mut cursor = self
            .text_cursors
            .get(&id)
            .copied()
            .unwrap_or(text.chars().count());
        if apply_edit(key, &mut text, &mut cursor) {
            self.text_values.insert(id.clone(), text);
            self.text_cursors.insert(id, cursor);
            if is_search {
                self.clamp_all_lists();
                self.refresh_previews();
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
        let len = self.menu_items(&id).map(|i| i.len()).unwrap_or(0);
        if len == 0 {
            return;
        }
        let selected = self.selected.get(&id).copied().unwrap_or(0);
        let next = match chord {
            "left" | "h" => selected.saturating_sub(1),
            "right" | "l" => (selected + 1).min(len.saturating_sub(1)),
            "home" => 0,
            "end" => len.saturating_sub(1),
            "down" | "j" => {
                self.menu_open_or_activate(&id);
                return;
            }
            _ => return,
        };
        self.selected.insert(id, next);
    }

    fn menu_items(&self, id: &str) -> Option<&[MenuItem]> {
        match self.app.widget_kind(id) {
            Some(WidgetKind::Menu { items }) => Some(items),
            _ => None,
        }
    }

    /// The bar item (menu id, index) whose mnemonic is `letter`.
    fn menu_bar_item_with_mnemonic(&self, letter: char) -> Option<(String, usize)> {
        let letter = letter.to_ascii_lowercase();
        self.app.iter().find_map(|w| match &w.kind {
            WidgetKind::Menu { items } => items
                .iter()
                .position(|item| item.mnemonic == Some(letter))
                .map(|idx| (w.id.clone(), idx)),
            _ => None,
        })
    }

    /// Enter/Down on a bar item: open its dropdown, or activate it if it
    /// has none.
    fn menu_open_or_activate(&mut self, id: &str) {
        let sel = self.selected.get(id).copied().unwrap_or(0);
        let has_items = self
            .menu_items(id)
            .and_then(|items| items.get(sel))
            .is_some_and(|item| !item.items.is_empty());
        if has_items {
            self.menu_open = Some(id.to_string());
            self.menu_item.insert(id.to_string(), 0);
        } else {
            self.activate_menu_item(id, sel, None);
        }
    }

    /// Keys while a dropdown is open. Letters pick by mnemonic.
    fn handle_dropdown_key(&mut self, chord: &str) {
        let Some(id) = self.menu_open.clone() else {
            return;
        };
        let top = self.selected.get(&id).copied().unwrap_or(0);
        let top_len = self.menu_items(&id).map(|i| i.len()).unwrap_or(0);
        let sub: Vec<Option<char>> = self
            .menu_items(&id)
            .and_then(|items| items.get(top))
            .map(|item| item.items.iter().map(|i| i.mnemonic).collect())
            .unwrap_or_default();
        let row = self.menu_item.get(&id).copied().unwrap_or(0);
        match chord {
            "esc" => self.menu_open = None,
            "tab" => {
                self.menu_open = None;
                self.focus_next();
            }
            "shift+tab" => {
                self.menu_open = None;
                self.focus_prev();
            }
            "up" | "k" => {
                self.menu_item.insert(id, row.saturating_sub(1));
            }
            "down" | "j" => {
                self.menu_item
                    .insert(id, (row + 1).min(sub.len().saturating_sub(1)));
            }
            "left" | "h" | "right" | "l" => {
                let next = if matches!(chord, "left" | "h") {
                    top.saturating_sub(1)
                } else {
                    (top + 1).min(top_len.saturating_sub(1))
                };
                self.selected.insert(id.clone(), next);
                self.menu_open = None;
                self.menu_open_or_activate(&id);
            }
            "enter" => self.activate_menu_item(&id, top, Some(row)),
            _ => {
                if let Some(letter) = single_char(chord)
                    && let Some(idx) = sub
                        .iter()
                        .position(|m| *m == Some(letter.to_ascii_lowercase()))
                {
                    self.activate_menu_item(&id, top, Some(idx));
                }
            }
        }
    }

    /// Run the item's action, or submit it when it has none.
    fn activate_menu_item(&mut self, id: &str, top: usize, sub: Option<usize>) {
        let Some(bar) = self.menu_items(id).and_then(|items| items.get(top)) else {
            return;
        };
        let (item, selected) = match sub {
            Some(i) => {
                let Some(item) = bar.items.get(i) else {
                    return;
                };
                let mut rec = Record::new();
                rec.insert("menu", Value::string(bar.label.clone(), Span::unknown()));
                rec.insert("item", Value::string(item.label.clone(), Span::unknown()));
                // The row highlighted in the first visible table/tree, so a
                // menu entry can act on it.
                rec.insert("row", self.current_row());
                (item.clone(), Value::record(rec, Span::unknown()))
            }
            None => (
                bar.clone(),
                Value::string(bar.label.clone(), Span::unknown()),
            ),
        };
        self.menu_open = None;
        match item.action {
            Some(closure) => self.run_menu_action(closure),
            None => {
                self.outcome = Some(Outcome {
                    action: Action::Submit,
                    selected,
                });
            }
        }
    }

    /// A returned value replaces the data list; `nothing` changes nothing.
    fn run_menu_action(&mut self, closure: Closure) {
        let Some((engine_state, stack)) = self.preview_engine.take() else {
            return;
        };
        let eval = ClosureEvalOnce::new(&engine_state, &stack, closure);
        match eval
            .run_with_input(nu_protocol::PipelineData::empty())
            .and_then(|data| data.into_value(Span::unknown()))
        {
            Ok(value) if value.is_nothing() => self.refresh_error = None,
            Ok(value) => {
                self.refresh_error = None;
                self.preview_engine = Some((engine_state, stack));
                self.replace_data(value);
                return;
            }
            Err(err) => self.refresh_error = Some(truncate_chars(&err.to_string(), 80)),
        }
        self.preview_engine = Some((engine_state, stack));
    }

    /// Horizontal offset and width of each bar item, as drawn.
    pub fn menu_item_ranges(items: &[MenuItem]) -> Vec<(u16, u16)> {
        let mut x = 0u16;
        items
            .iter()
            .map(|item| {
                let width = item.label.chars().count() as u16 + 2;
                let range = (x, width);
                x = x.saturating_add(width);
                range
            })
            .collect()
    }

    /// Where the open dropdown is drawn, below its bar item.
    pub fn menu_dropdown_rect(&self) -> Option<Rect> {
        let id = self.menu_open.as_deref()?;
        let bar = *self.areas.get(id)?;
        let items = self.menu_items(id)?;
        let top = self.selected.get(id).copied().unwrap_or(0);
        let entries = &items.get(top)?.items;
        if entries.is_empty() {
            return None;
        }
        let (offset, _) = Self::menu_item_ranges(items).get(top).copied()?;
        let width = entries
            .iter()
            .map(|i| i.label.chars().count())
            .max()
            .unwrap_or(0) as u16
            + 4;
        let max_x = bar.x.saturating_add(bar.width.saturating_sub(width));
        Some(Rect {
            x: bar.x.saturating_add(offset).min(max_x),
            y: bar.y.saturating_add(bar.height),
            width: width.min(bar.width),
            height: entries.len() as u16 + 2,
        })
    }

    fn handle_splitter_nav(&mut self, chord: &str) {
        let Some(id) = self.focused.clone() else {
            return;
        };
        let dir = match self.app.widget_kind(&id) {
            Some(WidgetKind::Split { direction, .. }) => *direction,
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
            Some(WidgetKind::Tree { walk, column }) => (*walk, column.clone()),
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
        let Some(WidgetKind::Tree { walk, column }) = self.app.widget_kind(id) else {
            return Vec::new();
        };
        let source = &self.app.data;
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
                if let Some(id) = self.menu_open.clone() {
                    let rect = self.menu_dropdown_rect();
                    self.menu_open = None;
                    if let Some(rect) = rect
                        && contains(rect, mouse.column, mouse.row)
                    {
                        let row = mouse.row.saturating_sub(rect.y).saturating_sub(1) as usize;
                        let top = self.selected.get(&id).copied().unwrap_or(0);
                        self.activate_menu_item(&id, top, Some(row));
                        return;
                    }
                }
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
                    if let Some(items) = self.menu_items(&id) {
                        let rel = mouse.column.saturating_sub(area.x);
                        if let Some(idx) = Self::menu_item_ranges(items)
                            .iter()
                            .position(|(x, w)| rel >= *x && rel < x + w)
                        {
                            self.selected.insert(id.clone(), idx);
                            self.menu_open_or_activate(&id);
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
            if self.app.widget_kind(id).is_some_and(|k| k.is_container()) {
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
        self.visible_widgets()
            .find(|w| w.kind.is_scrollable())
            .map(|w| w.id.clone())
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
        let ids: Vec<String> = self.app.iter().map(|w| w.id.clone()).collect();
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
        struct PreviewSpec {
            id: String,
            max_bytes: usize,
            transform: Option<Closure>,
        }
        let previews: Vec<PreviewSpec> = self
            .app
            .iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Preview {
                    max_bytes,
                    transform,
                    ..
                } => Some(PreviewSpec {
                    id: w.id.clone(),
                    max_bytes: *max_bytes,
                    transform: transform.clone(),
                }),
                _ => None,
            })
            .collect();

        let engine = self.preview_engine.take();
        for PreviewSpec {
            id,
            max_bytes,
            transform,
        } in previews
        {
            let row = self.preview_source_id(&id).and_then(|tid| {
                let idx = self.selected.get(&tid).copied().unwrap_or(0);
                self.filtered_rows(&tid).into_iter().nth(idx)
            });
            let closure = transform.zip(engine.as_ref());
            let wants_row = closure
                .as_ref()
                .is_some_and(|(c, (engine_state, _))| closure_wants_row(engine_state, c));
            let file = match (&row, wants_row) {
                // One-parameter closure: it is the source, no file is read.
                (Some(row), true) => FilePreview {
                    title: row_path_name(row).unwrap_or("preview").to_string(),
                    text: String::new(),
                    path: None,
                    transformable: true,
                },
                (Some(row), false) => preview_for_row(row, max_bytes, &self.cwd),
                (None, _) => FilePreview {
                    title: "preview".into(),
                    text: String::new(),
                    path: None,
                    transformable: false,
                },
            };
            let text = match (file.transformable, closure, row.as_ref()) {
                (true, Some((closure, (engine_state, stack))), Some(row)) => {
                    apply_preview_transform(
                        engine_state,
                        stack,
                        closure,
                        wants_row.then_some(row),
                        &file.text,
                        file.path.as_deref(),
                    )
                }
                _ => file.text,
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

    /// Which table/tree a preview follows: `--from` if given, else the
    /// focused row source, else the nearest one in an enclosing container,
    /// else the first visible one.
    pub fn preview_source_id(&self, preview_id: &str) -> Option<String> {
        if let Some(WidgetKind::Preview {
            from: Some(from), ..
        }) = self.app.widget_kind(preview_id)
        {
            return Some(from.clone());
        }
        if let Some(id) = &self.focused
            && self.app.widget_kind(id).is_some_and(|k| k.is_row_source())
        {
            return Some(id.clone());
        }
        let is_source = |id: &String| {
            id != preview_id && self.app.widget_kind(id).is_some_and(|k| k.is_row_source())
        };
        if let Some(path) = self.paths.get(preview_id) {
            for depth in (1..path.len()).rev() {
                if let Some(id) = subtree_ids(&self.app, &path[..depth])
                    .into_iter()
                    .find(is_source)
                {
                    return Some(id);
                }
            }
        }
        self.visible_ids().into_iter().find(is_source)
    }

    pub fn filtered_len(&self, id: &str) -> usize {
        self.filtered_rows(id).len()
    }

    pub fn filtered_rows(&self, id: &str) -> Vec<Value> {
        let Some(kind) = self.app.widget_kind(id) else {
            return Vec::new();
        };
        match kind {
            WidgetKind::Table { .. } | WidgetKind::Log { .. } => {
                filter_values(&self.app.data, self.query_for(id))
            }
            WidgetKind::Tree { .. } => self.tree_rows(id).into_iter().map(|r| r.value).collect(),
            _ => Vec::new(),
        }
    }

    /// The search box that filters widget `id`: the deepest search whose
    /// parent container encloses `id`. A top-level search encloses everything.
    pub fn scoping_search(&self, id: &str) -> Option<String> {
        let path = self.paths.get(id)?;
        let mut best: Option<(usize, String)> = None;
        for w in self.app.iter() {
            if !matches!(w.kind, WidgetKind::Search { .. }) {
                continue;
            }
            let Some(spath) = self.paths.get(&w.id) else {
                continue;
            };
            let parent = &spath[..spath.len().saturating_sub(1)];
            if path.starts_with(parent) && best.as_ref().is_none_or(|(d, _)| parent.len() > *d) {
                best = Some((parent.len(), w.id.clone()));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Active filter text for widget `id`: its own captured chord, else the
    /// text of the search box that scopes it.
    fn query_for(&self, id: &str) -> &str {
        let own = self.query_text(id);
        if !own.is_empty() {
            return own;
        }
        match self.scoping_search(id) {
            Some(search) => self.query_text(&search),
            None => "",
        }
    }

    pub fn table_columns(&self, id: &str) -> Vec<String> {
        match self.app.widget_kind(id) {
            Some(WidgetKind::Table { columns, .. }) => {
                if !columns.is_empty() {
                    return columns.clone();
                }
                let cols = get_columns(as_list(&self.app.data));
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
            .iter()
            .filter(|w| w.kind.is_scrollable())
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
        let value_span = value.span();
        self.app.data = match value {
            Value::List { .. } => value,
            Value::Range { val, .. } if val.is_bounded() => Value::list(
                val.into_range_iter(value_span, nu_protocol::Signals::empty())
                    .collect(),
                value_span,
            ),
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
            if let Some(items) = self.menu_items(id) {
                let idx = self.selected.get(id).copied().unwrap_or(0);
                if let Some(item) = items.get(idx) {
                    return Value::string(item.label.clone(), Span::unknown());
                }
            }
            let rows = self.filtered_rows(id);
            let idx = self.selected.get(id).copied().unwrap_or(0);
            if let Some(row) = rows.get(idx) {
                return row.clone();
            }
        }
        self.current_row()
    }

    /// The highlighted row of the first visible table, log, or tree.
    fn current_row(&self) -> Value {
        for w in self.visible_widgets() {
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

    /// Text of the focused search box, or of the first one.
    fn search_text(&self) -> &str {
        let id = match &self.focused {
            Some(id) if self.search_is_focused() => Some(id.clone()),
            _ => self
                .app
                .iter()
                .find(|w| matches!(w.kind, WidgetKind::Search { .. }))
                .map(|w| w.id.clone()),
        };
        id.map(|id| self.query_text(&id)).unwrap_or("")
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
        Value::record(self.result_fields(span, screen), span)
    }

    /// Fields of the result record: action, focused, selected, search, page,
    /// values, rows, live, and `screen` when rendered headless.
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
        rec.insert("search", Value::string(self.search_text(), span));
        rec.insert("page", Value::int(self.page as i64, span));
        let selected = self
            .outcome
            .as_ref()
            .map(|o| o.selected.clone())
            .unwrap_or_else(|| self.selected_value());
        rec.insert("selected", selected);

        let mut values = Record::new();
        for w in self.app.iter() {
            if let WidgetKind::TextBox { .. } = w.kind {
                values.insert(w.id.clone(), Value::string(self.query_text(&w.id), span));
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

    pub fn status_text(&self) -> String {
        let extra = self.live_status_bits();
        if let Some(text) = self.app.widgets.iter().find_map(|w| match &w.kind {
            WidgetKind::Label {
                text,
                slot: Slot::Status,
            } => Some(text.clone()),
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
            bits.push(format!("error:{err}"));
        }
        if let Some(id) = &self.focused {
            bits.push(format!("focus:{id}"));
        }
        let search = self.search_text();
        if !search.is_empty() {
            bits.push(format!("filter:{search}"));
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

    /// Resolved state for `tui debug`: the widget tree with layout rects,
    /// focusability, resolved columns, search scope and preview source;
    /// the focus order; and the pages. Run `layout` first so rects exist.
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
            match &w.kind {
                WidgetKind::Table { .. } => {
                    let cols = self.table_columns(&w.id);
                    rec.insert(
                        "resolved_columns",
                        Value::list(
                            cols.into_iter().map(|c| Value::string(c, span)).collect(),
                            span,
                        ),
                    );
                    rec.insert("rows", Value::int(self.filtered_len(&w.id) as i64, span));
                }
                WidgetKind::Log { .. } | WidgetKind::Tree { .. } => {
                    rec.insert("rows", Value::int(self.filtered_len(&w.id) as i64, span));
                }
                WidgetKind::Search { .. } => {
                    rec.insert("query", Value::string(self.query_text(&w.id), span));
                }
                WidgetKind::Preview { .. } => {
                    rec.insert(
                        "source",
                        match self.preview_source_id(&w.id) {
                            Some(id) => Value::string(id, span),
                            None => Value::nothing(span),
                        },
                    );
                }
                _ => {}
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

/// The chord's only character, for mnemonic matching.
fn single_char(chord: &str) -> Option<char> {
    let mut chars = chord.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
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

struct FilePreview {
    title: String,
    text: String,
    path: Option<PathBuf>,
    /// Whether a transform closure should run on `text`.
    transformable: bool,
}

/// Read the file named by the row's `name` column (or the row itself when it
/// is a string).
fn preview_for_row(row: &Value, max_bytes: usize, cwd: &Path) -> FilePreview {
    let Some(name) = row_path(row) else {
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

/// A preview closure that declares a parameter receives the selected row and
/// produces the pane text itself; one without parameters transforms file text.
fn closure_wants_row(engine_state: &EngineState, closure: &Closure) -> bool {
    let block = engine_state.get_block(closure.block_id);
    !block.signature.required_positional.is_empty()
        || !block.signature.optional_positional.is_empty()
}

/// Run the preview closure. With `row`, the closure is the source and `$in`
/// is empty; otherwise `$in` is `text` with the file's `content_type`.
fn apply_preview_transform(
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    row: Option<&Value>,
    text: &str,
    path: Option<&Path>,
) -> String {
    let input = match row {
        Some(_) => nu_protocol::PipelineData::empty(),
        None => {
            let metadata = PipelineMetadata {
                data_source: path
                    .map(|p| DataSource::FilePath(p.to_path_buf()))
                    .unwrap_or_default(),
                content_type: path.and_then(preview_content_type),
                ..Default::default()
            };
            Value::string(text, Span::unknown()).into_pipeline_data_with_metadata(Some(metadata))
        }
    };

    let mut eval = ClosureEvalOnce::new(engine_state, stack, closure);
    if let Some(row) = row {
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

fn row_path(row: &Value) -> Option<String> {
    match row {
        Value::Record { val, .. } => val.get("name").and_then(value_as_path),
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
        let _ = write!(text, "\n\n… truncated, showing {n} of {file_len} bytes");
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

    fn label(id: &str, text: &str) -> Widget {
        Widget::leaf(
            id,
            WidgetKind::Label {
                text: text.into(),
                slot: Slot::Content,
            },
        )
    }

    fn table(id: &str, columns: &[&str]) -> Widget {
        Widget::leaf(
            id,
            WidgetKind::Table {
                columns: columns.iter().map(|c| (*c).into()).collect(),
                capture_keys: false,
            },
        )
    }

    fn search(id: &str, bind: Option<&str>) -> Widget {
        Widget::leaf(
            id,
            WidgetKind::Search {
                placeholder: "filter".into(),
                bind: bind.map(Into::into),
            },
        )
    }

    fn split(id: &str, direction: SplitDir, children: Vec<Widget>) -> Widget {
        Widget {
            id: id.into(),
            auto_id: false,
            kind: WidgetKind::Split {
                direction,
                ratio: 50,
            },
            children,
        }
    }

    fn tab(id: &str, title: &str, children: Vec<Widget>) -> Widget {
        Widget {
            id: id.into(),
            auto_id: false,
            kind: WidgetKind::Tab {
                title: title.into(),
            },
            children,
        }
    }

    fn table_app() -> TuiApp {
        let rows = vec![
            Value::test_record(record_from(&[("name", "alpha"), ("size", "1")])),
            Value::test_record(record_from(&[("name", "beta"), ("size", "2")])),
            Value::test_record(record_from(&[("name", "gamma"), ("size", "3")])),
        ];
        let mut app = TuiApp::new();
        app.data = Value::test_list(rows);
        app.push(table("table-0", &["name", "size"]));
        app.push(search("search-0", Some("ctrl+r")));
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
        assert_eq!(session.query_text("search-0"), "q");
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
        assert_eq!(session.query_text("search-0"), "/");
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
        app.push(Widget::leaf(
            "table-0",
            WidgetKind::Table {
                columns: vec![],
                capture_keys: true,
            },
        ));
        app
    }

    #[test]
    fn captured_chord_filters_rows_without_a_search_box() {
        let mut session = Session::new(keybindings_app());
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        let rows = session.filtered_rows("table-0");
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
    fn captured_chord_goes_to_the_scoping_search_box() {
        let mut app = keybindings_app();
        app.widgets.insert(0, search("search-0", None));
        let mut session = Session::new(app);
        press(&mut session, KeyCode::Char('l'), KeyModifiers::CONTROL);
        assert_eq!(session.query_text("search-0"), "ctrl+l");
        assert_eq!(session.filtered_len("table-0"), 1);
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
        app.push(table("table-0", &["name"]));
        app.push(Widget::leaf(
            "preview-0",
            WidgetKind::Preview {
                max_bytes: 4096,
                transform: None,
                from: None,
            },
        ));

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

    #[test]
    fn preview_follows_the_nearest_table_in_its_split() {
        let mut app = TuiApp::new();
        app.data = Value::test_list(vec![Value::test_string("a")]);
        app.push(split(
            "split-0",
            SplitDir::Horizontal,
            vec![table("far", &["item"]), label("label-0", "x")],
        ));
        app.push(split(
            "split-1",
            SplitDir::Horizontal,
            vec![
                table("near", &["item"]),
                Widget::leaf(
                    "preview-0",
                    WidgetKind::Preview {
                        max_bytes: 4096,
                        transform: None,
                        from: None,
                    },
                ),
            ],
        ));
        let mut session = Session::new(app);
        // Focus is on the first table; move it to chrome-less nothing.
        session.focused = None;
        assert_eq!(
            session.preview_source_id("preview-0").as_deref(),
            Some("near")
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
        app.push(Widget::leaf(
            "tree-0",
            WidgetKind::Tree {
                walk: false,
                column: "name".into(),
            },
        ));
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
        app.push(tab("tab-0", "one", vec![label("label-0", "PAGE-ONE")]));
        app.push(tab("tab-1", "two", vec![label("label-1", "PAGE-TWO")]));
        let mut session = Session::new(app);
        assert_eq!(session.page, 0);
        assert_eq!(session.visible_ids(), vec!["tab-0", "label-0"]);
        press(&mut session, KeyCode::Char('2'), KeyModifiers::NONE);
        assert_eq!(session.page, 1);
        assert_eq!(session.visible_ids(), vec!["tab-1", "label-1"]);
    }

    #[test]
    fn nested_tab_is_a_group_box_not_a_page() {
        let mut app = TuiApp::new();
        app.push(split(
            "split-0",
            SplitDir::Horizontal,
            vec![
                tab("tab-0", "left", vec![label("label-0", "A")]),
                label("label-1", "B"),
            ],
        ));
        let mut session = Session::new(app);
        assert!(!session.has_tabs());
        assert_eq!(session.pages().len(), 1);
        session.layout(Rect::new(0, 0, 80, 24));
        let outer = session.areas["tab-0"];
        let inner = session.areas["label-0"];
        assert!(inner.x > outer.x && inner.y > outer.y);
        assert!(session.tab_areas.is_empty());
    }

    #[test]
    fn splitter_drag_tracks_mouse_in_permille() {
        let mut app = TuiApp::new();
        app.push(split(
            "split-0",
            SplitDir::Horizontal,
            vec![label("label-a", "A"), label("label-b", "B")],
        ));
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
    fn split_with_three_children_shares_the_remainder() {
        let mut app = TuiApp::new();
        app.push(split(
            "split-0",
            SplitDir::Horizontal,
            vec![
                label("label-a", "A"),
                label("label-b", "B"),
                label("label-c", "C"),
            ],
        ));
        let mut session = Session::new(app);
        session.layout(Rect::new(0, 0, 80, 24));
        let a = session.areas["label-a"];
        let b = session.areas["label-b"];
        let c = session.areas["label-c"];
        assert!(a.x < b.x && b.x < c.x);
        assert!(b.width > 0 && c.width > 0);
        assert_eq!(session.splitter_handles.len(), 1);
    }

    #[test]
    fn log_pageup_scrolls_more_than_one_line() {
        let mut app = TuiApp::new();
        app.data = Value::test_list((0..40).map(Value::test_int).collect());
        app.push(Widget::leaf("log-0", WidgetKind::Log { max_lines: 10_000 }));
        let mut session = Session::new(app);
        session.layout(Rect::new(0, 0, 40, 12));
        session.scroll.insert("log-0".into(), 20);
        session.follow_tail.insert("log-0".into(), true);
        press(&mut session, KeyCode::PageUp, KeyModifiers::NONE);
        assert_eq!(session.scroll.get("log-0").copied(), Some(10));
        assert_eq!(session.follow_tail.get("log-0").copied(), Some(false));
    }

    fn menu_app() -> TuiApp {
        let mut app = TuiApp::new();
        app.data = Value::test_list(vec![Value::test_string("a")]);
        app.push(Widget::leaf(
            "menu-0",
            WidgetKind::Menu {
                items: vec![
                    MenuItem::new(
                        "&File",
                        vec![
                            MenuItem::new("&Open", vec![], None),
                            MenuItem::new("&Quit", vec![], None),
                        ],
                        None,
                    ),
                    MenuItem::new("&Edit", vec![], None),
                ],
            },
        ));
        app.push(table("table-0", &["item"]));
        app
    }

    #[test]
    fn mnemonic_marker_is_stripped_and_defaults_to_first_letter() {
        let item = MenuItem::new("F&ile", vec![], None);
        assert_eq!(item.label, "File");
        assert_eq!(item.mnemonic, Some('i'));
        assert_eq!(item.mnemonic_index(), Some(1));
        let plain = MenuItem::new("View", vec![], None);
        assert_eq!(plain.mnemonic, Some('v'));
    }

    #[test]
    fn alt_mnemonic_opens_dropdown_and_letter_submits_item() {
        let mut session = Session::new(menu_app());
        assert_eq!(session.focused.as_deref(), Some("table-0"));
        press(&mut session, KeyCode::Char('f'), KeyModifiers::ALT);
        assert_eq!(session.focused.as_deref(), Some("menu-0"));
        assert_eq!(session.menu_open.as_deref(), Some("menu-0"));
        session.layout(Rect::new(0, 0, 40, 12));
        let rect = session.menu_dropdown_rect().expect("dropdown rect");
        assert_eq!(rect.y, 1);
        assert_eq!(rect.height, 4);
        press(&mut session, KeyCode::Char('q'), KeyModifiers::NONE);
        let selected = session.outcome.expect("submitted").selected;
        let rec = selected.as_record().expect("record");
        assert_eq!(rec.get("menu").and_then(|v| v.as_str().ok()), Some("File"));
        assert_eq!(rec.get("item").and_then(|v| v.as_str().ok()), Some("Quit"));
    }

    #[test]
    fn esc_closes_dropdown_without_quitting() {
        let mut session = Session::new(menu_app());
        press(&mut session, KeyCode::Char('f'), KeyModifiers::ALT);
        press(&mut session, KeyCode::Esc, KeyModifiers::NONE);
        assert!(session.menu_open.is_none());
        assert!(session.outcome.is_none());
    }

    #[test]
    fn bar_item_without_dropdown_submits_its_name() {
        let mut session = Session::new(menu_app());
        press(&mut session, KeyCode::Char('e'), KeyModifiers::ALT);
        let selected = session.outcome.expect("submitted").selected;
        assert_eq!(selected.as_str().ok(), Some("Edit"));
    }

    #[test]
    fn enter_in_search_submits_filtered_row() {
        let mut session = Session::new(table_app());
        press(&mut session, KeyCode::Char('r'), KeyModifiers::CONTROL);
        press(&mut session, KeyCode::Char('b'), KeyModifiers::NONE);
        press(&mut session, KeyCode::Enter, KeyModifiers::NONE);
        let selected = session.outcome.expect("submitted").selected;
        let name = selected
            .as_record()
            .expect("record")
            .get("name")
            .and_then(|v| v.as_str().ok());
        assert_eq!(name, Some("beta"));
    }

    #[test]
    fn nested_search_filters_only_its_container() {
        let mut app = TuiApp::new();
        app.data = Value::test_list(vec![
            Value::test_string("alpha"),
            Value::test_string("beta"),
        ]);
        app.push(split(
            "split-0",
            SplitDir::Vertical,
            vec![search("search-0", Some("/")), table("table-0", &["item"])],
        ));
        app.push(table("table-1", &["item"]));
        let mut session = Session::new(app);
        assert_eq!(
            session.scoping_search("table-0").as_deref(),
            Some("search-0")
        );
        assert_eq!(session.scoping_search("table-1"), None);
        press(&mut session, KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(session.focused.as_deref(), Some("search-0"));
        press(&mut session, KeyCode::Char('a'), KeyModifiers::NONE);
        press(&mut session, KeyCode::Char('l'), KeyModifiers::NONE);
        assert_eq!(session.filtered_len("table-0"), 1);
        assert_eq!(session.filtered_len("table-1"), 2);
    }

    #[test]
    fn top_level_search_filters_everything() {
        let mut app = TuiApp::new();
        app.data = Value::test_list(vec![
            Value::test_string("alpha"),
            Value::test_string("beta"),
        ]);
        app.push(search("search-0", None));
        app.push(split(
            "split-0",
            SplitDir::Horizontal,
            vec![table("table-0", &["item"]), table("table-1", &["item"])],
        ));
        let mut session = Session::new(app);
        session.focused = Some("search-0".into());
        press(&mut session, KeyCode::Char('b'), KeyModifiers::NONE);
        assert_eq!(session.filtered_len("table-0"), 1);
        assert_eq!(session.filtered_len("table-1"), 1);
    }
}
