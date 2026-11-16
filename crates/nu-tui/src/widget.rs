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
use crate::session::{Rows, Session};
use crate::stream::Feed;
use crate::widgets::{
    r#box::BoxWidget, button::ButtonWidget, label::LabelWidget, log::LogWidget, menu::MenuWidget,
    preview::PreviewWidget, progress::ProgressWidget, search::SearchWidget, select::SelectWidget,
    split::SplitWidget, tab::TabWidget, table::TableWidget, textbox::TextBoxWidget,
    tree::TreeWidget,
};
use nu_protocol::engine::Closure;
use nu_protocol::{Record, Span, TableMode, Value};
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
    /// A stream piped into this widget inside a child list, unread until the
    /// TUI runs; its rows are appended to `data`. Root widgets of one child
    /// share it.
    #[serde(skip)]
    pub stream: Option<Feed>,
    /// Another widget whose highlighted row drives this one.
    #[serde(default)]
    pub source: Option<Source>,
    /// Hook run when this widget's selection changes.
    #[serde(default)]
    pub on_select: Option<Closure>,
    /// `--focus`: start with this widget focused.
    #[serde(default)]
    pub focus: bool,
    /// `--title`: replaces the widget's name in its border title (see
    /// `widgets::custom_title`). On `tui preview` it goes in front of the
    /// previewed file's name. Never empty.
    #[serde(default)]
    pub title: Option<String>,
    /// `--border`: the `table --theme` outline this widget's border draws,
    /// instead of `$env.config.tui.border_type`.
    #[serde(default)]
    pub border: Option<TableMode>,
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
            stream: None,
            source: None,
            on_select: None,
            focus: false,
            title: None,
            border: None,
        }
    }

    /// Whether this widget shows something derived from another widget's
    /// highlighted row: it has `--from` or a source closure, or it is a
    /// preview (which always follows the nearest row source).
    pub fn follows_source(&self) -> bool {
        self.source.is_some() || matches!(self.kind, WidgetKind::Preview(_))
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
    /// The builder call, which errors about this source point at.
    pub span: Span,
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
    /// Submit the selection of the focused (or first) row source: its
    /// checked rows under `--multi`, else its highlighted row (or index).
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
    /// The highlighted row, by position in the unfiltered list (see
    /// [`Rows::source`]), so a filter cannot move it to another row. A
    /// tree's rows are its visible nodes, so there it is a position in them.
    pub highlight: usize,
    /// The first shown row on screen, by position in the shown rows.
    pub scroll: usize,
    /// Rows toggled with Space on a `--multi` table or select, by position
    /// in the unfiltered list, like the highlight. Always empty on a tree,
    /// which keeps its checks by node path in [`TreeState::checked`]; the
    /// session still clamps it there, against a length it never uses.
    pub checked: BTreeSet<usize>,
    /// The chord stored by `--capture-keys`; doubles as this widget's filter.
    pub captured: String,
}

impl ListState {
    /// Where the highlight is among the shown `rows` (see
    /// [`Rows::nearest`]). `None` when nothing is shown.
    pub fn position(&self, rows: &Rows) -> Option<usize> {
        rows.nearest(self.highlight)
    }

    /// The highlighted row as `(shown position, position in the unfiltered
    /// list)`. `None` when nothing is shown.
    pub fn highlighted(&self, rows: &Rows) -> Option<(usize, usize)> {
        let pos = self.position(rows)?;
        Some((pos, rows.source(pos)?))
    }

    /// Move the highlight for a navigation chord. Returns `false` when the
    /// chord is not a navigation key.
    pub fn navigate(&mut self, chord: &str, rows: &Rows, page: usize) -> bool {
        let pos = self.position(rows).unwrap_or(0);
        let last = rows.len().saturating_sub(1);
        let next = match chord {
            "up" | "k" => pos.saturating_sub(1),
            "down" | "j" => (pos + 1).min(last),
            "pageup" => pos.saturating_sub(page),
            "pagedown" => (pos + page).min(last),
            "home" => 0,
            "end" => last,
            _ => return false,
        };
        if let Some(source) = rows.source(next) {
            self.highlight = source;
        }
        true
    }

    /// Keep the highlight on screen given `visible` rows of room.
    pub fn ensure_visible(&mut self, rows: &Rows, visible: usize) {
        self.scroll = self.scroll_for(rows, visible);
    }

    /// The scroll offset that shows the highlight given `visible` rows of
    /// room: the stored one when it already does, else the nearest that
    /// does. Renderers use it so a highlight is never drawn off screen.
    pub fn scroll_for(&self, rows: &Rows, visible: usize) -> usize {
        let visible = visible.max(1);
        let pos = self.position(rows).unwrap_or(0);
        if pos < self.scroll {
            pos
        } else if pos >= self.scroll + visible {
            pos + 1 - visible
        } else {
            self.scroll
        }
    }

    /// Put the highlight on a shown row (see [`Rows::nearest`]) and keep
    /// the checks inside the `total` rows of the unfiltered list. The
    /// scroll offset follows the highlight up, so a filter that shrinks the
    /// rows cannot leave the viewport past the end of them; checks on rows
    /// the filter hides are kept. With nothing shown, the highlight stays
    /// where it was, so it comes back when the filter shows the row again.
    pub fn clamp(&mut self, rows: &Rows, total: usize) {
        match self.highlighted(rows) {
            Some((pos, source)) => {
                self.highlight = source;
                self.scroll = self.scroll.min(pos);
            }
            None => {
                self.highlight = self.highlight.min(total.saturating_sub(1));
                self.scroll = 0;
            }
        }
        self.checked.retain(|i| *i < total);
    }

    /// The first `n` rows of the unfiltered list were dropped: move the
    /// highlight and checks so they still name the same rows. Checked rows
    /// that were dropped are forgotten. The scroll offset counts shown
    /// rows, so under a filter it can move further than the rows did;
    /// [`ListState::scroll_for`] keeps the highlight on screen regardless.
    pub fn drop_front(&mut self, n: usize) {
        self.highlight = self.highlight.saturating_sub(n);
        self.scroll = self.scroll.saturating_sub(n);
        self.checked = self
            .checked
            .iter()
            .filter_map(|i| i.checked_sub(n))
            .collect();
    }

    /// Check row `index` of the unfiltered list, or uncheck it.
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
    /// The previewed file's name, or empty when nothing is previewed.
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
    /// Paths of the nodes checked with Space under `--multi`. A path names
    /// its node, so collapsing a parent or filtering cannot move a check.
    pub checked: HashSet<String>,
}

impl TreeState {
    /// Check the node at `path`, or uncheck it.
    pub fn toggle(&mut self, path: &str) {
        if !self.checked.remove(path) {
            self.checked.insert(path.to_string());
        }
    }
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
