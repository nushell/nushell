//! Widget definitions, their runtime state, and the trait every widget
//! implements.
//!
//! A [`Widget`] is what a `tui *` builder appends to the pipeline value: an
//! id, a [`WidgetKind`] holding the builder's flags, optional children for
//! containers, and optional data/source bindings. While the UI runs, each
//! widget owns one [`WidgetState`] in the [`Session`]. Widgets never touch the
//! session directly: key and mouse handlers return [`Effect`]s that the
//! session applies, so focus, submission, hooks, and data flow are decided in
//! one place.

use crate::keys::KeyPress;
use crate::session::Session;
use crate::widgets::{
    r#box::BoxWidget, button::ButtonWidget, label::LabelWidget, log::LogWidget, menu::MenuWidget,
    preview::PreviewWidget, progress::ProgressWidget, search::SearchWidget, select::SelectWidget,
    split::SplitWidget, tab::TabWidget, table::TableWidget, textbox::TextBoxWidget,
    tree::TreeWidget,
};
use nu_protocol::engine::Closure;
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    /// `true` when the id was generated (`table-0`) rather than passed with
    /// `--id`. Generated ids are renumbered when a child is adopted by a
    /// container whose tree already uses that id; explicit ids never are.
    #[serde(default)]
    pub auto_id: bool,
    pub kind: WidgetKind,
    /// Nested widgets. Only containers have any.
    #[serde(default)]
    pub children: Vec<Widget>,
    /// Data bound to this widget alone (`--data`, or a value piped into the
    /// builder inside a container's child list). Descendants inherit it.
    #[serde(default)]
    pub data: Option<Value>,
    /// Another widget whose highlighted row drives this one.
    #[serde(default)]
    pub source: Option<Source>,
    /// Hook run when this widget's selection changes.
    #[serde(default)]
    pub on_select: Option<Closure>,
}

impl Widget {
    #[cfg(test)]
    pub fn new(id: impl Into<String>, kind: WidgetKind) -> Self {
        Self {
            id: id.into(),
            auto_id: false,
            kind,
            children: Vec::new(),
            data: None,
            source: None,
            on_select: None,
        }
    }

    /// Visit this widget and its descendants, preorder.
    pub fn for_each_mut(&mut self, f: &mut impl FnMut(&mut Widget)) {
        f(self);
        for c in &mut self.children {
            c.for_each_mut(f);
        }
    }
}

/// `--from <id>` and the closure that turns the source's highlighted row
/// into this widget's data. Either half may be absent: no `from` means the
/// nearest row source, no closure means the row itself is the data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Source {
    pub from: Option<String>,
    pub closure: Option<Closure>,
}

/// Every widget kind. Each variant wraps the struct that implements
/// [`TuiWidget`]; [`WidgetKind::as_widget`] is the single dispatch point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetKind {
    Label(LabelWidget),
    Menu(MenuWidget),
    TextBox(TextBoxWidget),
    Table(TableWidget),
    Search(SearchWidget),
    Preview(PreviewWidget),
    Log(LogWidget),
    Tree(TreeWidget),
    Tab(TabWidget),
    Box(BoxWidget),
    Split(SplitWidget),
    Select(SelectWidget),
    Button(ButtonWidget),
    Progress(ProgressWidget),
}

impl WidgetKind {
    pub fn as_widget(&self) -> &(dyn TuiWidget + 'static) {
        &**self
    }

    pub fn type_name(&self) -> &'static str {
        self.as_widget().type_name()
    }

    pub fn caps(&self) -> Caps {
        self.as_widget().caps()
    }

    pub fn is_container(&self) -> bool {
        self.caps().container
    }

    pub fn is_chrome(&self) -> bool {
        self.caps().chrome
    }

    pub fn is_focusable(&self) -> bool {
        self.caps().focusable
    }

    pub fn is_scrollable(&self) -> bool {
        self.caps().scrollable
    }

    pub fn is_text_input(&self) -> bool {
        self.caps().text_input
    }

    pub fn is_row_source(&self) -> bool {
        self.caps().row_source
    }
}

impl std::ops::Deref for WidgetKind {
    type Target = dyn TuiWidget + 'static;

    fn deref(&self) -> &Self::Target {
        match self {
            WidgetKind::Label(w) => w,
            WidgetKind::Menu(w) => w,
            WidgetKind::TextBox(w) => w,
            WidgetKind::Table(w) => w,
            WidgetKind::Search(w) => w,
            WidgetKind::Preview(w) => w,
            WidgetKind::Log(w) => w,
            WidgetKind::Tree(w) => w,
            WidgetKind::Tab(w) => w,
            WidgetKind::Box(w) => w,
            WidgetKind::Split(w) => w,
            WidgetKind::Select(w) => w,
            WidgetKind::Button(w) => w,
            WidgetKind::Progress(w) => w,
        }
    }
}

/// What a widget can do. Read by layout, focus order, hit testing, and the
/// builders' nesting rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Caps {
    /// Tab stops here.
    pub focusable: bool,
    /// Shows rows from the data list and moves a highlight through them.
    pub scrollable: bool,
    /// Typing goes here; `q` is a character, not quit.
    pub text_input: bool,
    /// Has a highlighted row that previews and `--from` widgets can follow.
    pub row_source: bool,
    /// Lives in a fixed slot at the top level (title, menu, status).
    pub chrome: bool,
    /// Lays out children.
    pub container: bool,
    /// Packs onto one row with adjacent flowing siblings instead of taking
    /// a row of its own (buttons).
    pub flows: bool,
}

/// Something a handler asks the session to do. Handlers never mutate the
/// session; they return effects and the session applies them in order.
#[derive(Debug, Clone)]
pub enum Effect {
    Focus(String),
    FocusNext,
    FocusPrev,
    /// Leave the focused text input for the first non-text focusable widget.
    LeaveText,
    Submit(Value),
    /// Submit the row highlighted in the focused (or first) row source.
    /// Widgets cannot compute it themselves while their state is checked
    /// out of the session.
    SubmitCurrent,
    Quit,
    /// The highlighted row of widget `id` changed: refresh dependents and run
    /// its `--on-select` hook.
    Selected(String),
    /// A filter changed: clamp every list and refresh dependents.
    Query,
    /// A `--capture-keys` table saw a chord; store it as that table's filter.
    Capture(String, String),
    /// Run a hook closure with the state record.
    Hook(Closure),
    PageNext,
    PagePrev,
    Page(usize),
}

/// Runtime state of one widget. Widgets without state use [`WidgetState::None`].
#[derive(Debug, Clone, Default)]
pub enum WidgetState {
    #[default]
    None,
    Menu(MenuState),
    Text(TextState),
    List(ListState),
    Preview(PreviewState),
    Log(LogState),
    Tree(TreeState),
    Split(SplitState),
}

impl WidgetState {
    pub fn as_list(&self) -> Option<&ListState> {
        match self {
            WidgetState::List(s) => Some(s),
            WidgetState::Tree(t) => Some(&t.list),
            _ => None,
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut ListState> {
        match self {
            WidgetState::List(s) => Some(s),
            WidgetState::Tree(t) => Some(&mut t.list),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&TextState> {
        match self {
            WidgetState::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_text_mut(&mut self) -> Option<&mut TextState> {
        match self {
            WidgetState::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_menu(&self) -> Option<&MenuState> {
        match self {
            WidgetState::Menu(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_menu_mut(&mut self) -> Option<&mut MenuState> {
        match self {
            WidgetState::Menu(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_tree(&self) -> Option<&TreeState> {
        match self {
            WidgetState::Tree(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_tree_mut(&mut self) -> Option<&mut TreeState> {
        match self {
            WidgetState::Tree(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_preview(&self) -> Option<&PreviewState> {
        match self {
            WidgetState::Preview(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_preview_mut(&mut self) -> Option<&mut PreviewState> {
        match self {
            WidgetState::Preview(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_log(&self) -> Option<&LogState> {
        match self {
            WidgetState::Log(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_log_mut(&mut self) -> Option<&mut LogState> {
        match self {
            WidgetState::Log(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_split(&self) -> Option<&SplitState> {
        match self {
            WidgetState::Split(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_split_mut(&mut self) -> Option<&mut SplitState> {
        match self {
            WidgetState::Split(s) => Some(s),
            _ => None,
        }
    }
}

/// Highlight, scroll offset, and check marks of a table, tree, or select.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub selected: usize,
    pub scroll: usize,
    /// Rows toggled with Space when the widget is `--multi`.
    pub checked: BTreeSet<usize>,
    /// The chord stored by `--capture-keys`; doubles as this widget's filter.
    pub captured: String,
}

impl ListState {
    /// Move the highlight for a navigation chord. Returns `false` when the
    /// chord is not a navigation key.
    pub fn navigate(&mut self, chord: &str, len: usize, page: usize) -> bool {
        let last = len.saturating_sub(1);
        let next = match chord {
            "up" | "k" => self.selected.saturating_sub(1),
            "down" | "j" => (self.selected + 1).min(last),
            "pageup" => self.selected.saturating_sub(page),
            "pagedown" => (self.selected + page).min(last),
            "home" => 0,
            "end" => last,
            _ => return false,
        };
        self.selected = if len == 0 { 0 } else { next };
        true
    }

    /// Keep `selected` on screen given `visible` rows of room.
    pub fn ensure_visible(&mut self, visible: usize) {
        let visible = visible.max(1);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + visible {
            self.scroll = self.selected + 1 - visible;
        }
    }

    pub fn clamp(&mut self, len: usize) {
        self.selected = if len == 0 {
            0
        } else {
            self.selected.min(len - 1)
        };
        self.checked.retain(|i| *i < len);
    }

    pub fn toggle(&mut self, index: usize) {
        if !self.checked.remove(&index) {
            self.checked.insert(index);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextState {
    pub text: String,
    /// Cursor position in characters.
    pub cursor: usize,
}

impl TextState {
    pub fn new(text: String) -> Self {
        let cursor = text.chars().count();
        Self { text, cursor }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }
}

#[derive(Debug, Clone, Default)]
pub struct MenuState {
    /// Highlighted bar item.
    pub selected: usize,
    /// Whether the highlighted item's dropdown is open.
    pub open: bool,
    /// Highlighted row inside the open dropdown.
    pub item: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PreviewState {
    pub title: String,
    pub text: String,
    pub scroll: usize,
}

#[derive(Debug, Clone)]
pub struct LogState {
    pub scroll: usize,
    /// Keep the newest line in view as rows arrive.
    pub follow: bool,
}

impl Default for LogState {
    fn default() -> Self {
        Self {
            scroll: 0,
            follow: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TreeState {
    pub list: ListState,
    /// Expanded node paths.
    pub expanded: HashSet<String>,
    /// Directory listings read on expand, keyed by node path.
    pub cache: HashMap<String, Vec<Value>>,
}

#[derive(Debug, Clone, Default)]
pub struct SplitState {
    /// Current size of each child; starts from the builder's `--sizes` and
    /// changes as handles are dragged.
    pub sizes: Vec<Size>,
}

/// One child's share of a split, mirroring ratatui's `Constraint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Size {
    Percent(u16),
    Fill(u16),
    Length(u16),
    Min(u16),
    Max(u16),
}

impl Size {
    pub fn to_constraint(self) -> Constraint {
        match self {
            Size::Percent(n) => Constraint::Percentage(n),
            Size::Fill(n) => Constraint::Fill(n),
            Size::Length(n) => Constraint::Length(n),
            Size::Min(n) => Constraint::Min(n),
            Size::Max(n) => Constraint::Max(n),
        }
    }

    pub fn to_value(self, span: Span) -> Value {
        match self {
            Size::Percent(n) => Value::string(format!("{n}%"), span),
            Size::Fill(n) => Value::string(format!("{n}fr"), span),
            Size::Length(n) => Value::int(n as i64, span),
            Size::Min(n) => Value::string(format!("min:{n}"), span),
            Size::Max(n) => Value::string(format!("max:{n}"), span),
        }
    }
}

/// Behaviour shared by every widget. Layout, focus, rendering, results, and
/// `tui debug` all go through this trait, so adding a widget means one module
/// plus a `WidgetKind` variant.
pub trait TuiWidget {
    /// The `type` field in records and the id prefix (`table-0`).
    fn type_name(&self) -> &'static str;

    fn caps(&self) -> Caps;

    /// Height when stacked vertically among siblings.
    fn constraint(&self) -> Constraint;

    /// Width when packed onto a row with other flowing widgets.
    fn width_hint(&self) -> Option<u16> {
        None
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::None
    }

    /// Builder flags, for `tui debug` and `to_base_value`.
    fn describe(&self, _rec: &mut Record, _span: Span) {}

    /// `true` while the widget wants keys before the global bindings: while
    /// typing, or while a dropdown is open.
    fn captures_keys(&self, _state: &WidgetState) -> bool {
        false
    }

    /// Handle a key while focused. `None` means "not mine"; the session then
    /// applies its global keys.
    fn handle_key(
        &self,
        _id: &str,
        _state: &mut WidgetState,
        _key: &KeyPress,
        _session: &Session,
    ) -> Option<Vec<Effect>> {
        None
    }

    /// Left click at `(x, y)` inside `area`.
    fn click(
        &self,
        _id: &str,
        _state: &mut WidgetState,
        _area: Rect,
        _x: u16,
        _y: u16,
        _session: &Session,
    ) -> Vec<Effect> {
        Vec::new()
    }

    /// Mouse wheel over the widget: `delta` is +1 down, -1 up.
    fn scroll(
        &self,
        _id: &str,
        _state: &mut WidgetState,
        _delta: i32,
        _session: &Session,
    ) -> Vec<Effect> {
        Vec::new()
    }

    fn render(
        &self,
        id: &str,
        state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        focused: bool,
    );

    /// This widget's entry under `values` in the result record.
    fn value(&self, _id: &str, _state: &WidgetState, _session: &Session) -> Value {
        Value::nothing(Span::unknown())
    }

    /// What `selected` holds when this widget is focused on submit. Defaults
    /// to [`TuiWidget::value`].
    fn selection(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        self.value(id, state, session)
    }

    /// The highlighted row, for previews and `--from` dependents.
    fn current_row(&self, _id: &str, _state: &WidgetState, _session: &Session) -> Option<Value> {
        None
    }

    /// Extra fields for `tui debug`.
    fn debug(&self, _id: &str, _state: &WidgetState, _session: &Session, _rec: &mut Record) {}
}

/// The `tui` custom-value type name.
pub const TYPE_NAME: &str = "tui";
