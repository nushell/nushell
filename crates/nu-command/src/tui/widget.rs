//! Widget kinds that make up a [`TuiApp`](super::app::TuiApp).

use nu_protocol::Value;
use nu_protocol::engine::Closure;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub kind: WidgetKind,
    pub place: Option<Place>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Rel {
    RightOf,
    LeftOf,
    Above,
    Below,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub rel: Rel,
    pub of: String,
    /// Percent of the split given to the anchor widget (`of`), 10-90.
    pub ratio: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitDir {
    /// Side by side (left | right).
    Horizontal,
    /// Stacked (top / bottom).
    Vertical,
}

impl SplitDir {
    pub fn as_str(self) -> &'static str {
        match self {
            SplitDir::Horizontal => "horizontal",
            SplitDir::Vertical => "vertical",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "horizontal" | "h" | "row" => Some(SplitDir::Horizontal),
            "vertical" | "v" | "column" => Some(SplitDir::Vertical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetKind {
    Title {
        text: String,
    },
    Menu {
        items: Vec<String>,
    },
    Label {
        text: String,
    },
    TextBox {
        placeholder: String,
        editable: bool,
        value: String,
    },
    Table {
        columns: Vec<String>,
        data: Option<Value>,
    },
    /// Starts a page/window. Widgets after this body until the next body
    /// (or status) belong to that page.
    Body {
        title: String,
    },
    Status {
        text: String,
    },
    Keybindings {
        data: Option<Value>,
    },
    Search {
        placeholder: String,
        bind: Option<String>,
        /// If set, only this widget id is filtered.
        target: Option<String>,
    },
    Splitter {
        direction: SplitDir,
        ratio: u16,
    },
    /// Follows the selected table row and shows that path's contents.
    /// Not focusable, so arrow keys keep moving the table.
    Preview {
        column: String,
        max_bytes: usize,
        /// Receives file contents as `$in` (with `content_type` metadata).
        /// If the closure takes an argument, that argument is the selected row.
        transform: Option<Closure>,
        /// Table/list id to follow. Defaults to the focused or first table/list.
        from: Option<String>,
    },
    /// Selectable list. Pipeline values become items.
    List {
        data: Option<Value>,
    },
    /// Append-only log. New stream rows appear at the bottom.
    Log {
        max_lines: usize,
    },
    /// Directory / nested-record tree. Shares `app.data` unless `data` is set.
    Tree {
        data: Option<Value>,
        /// Expand directories on disk when a node is opened.
        walk: bool,
        column: String,
    },
    /// Page marker, like Body, shown in the tab bar.
    Tab {
        title: String,
    },
    /// Explicit tab-bar chrome. Optional; multiple Tab/Body pages already show a bar.
    Tabs,
}

impl WidgetKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            WidgetKind::Title { .. } => "title",
            WidgetKind::Menu { .. } => "menu",
            WidgetKind::Label { .. } => "label",
            WidgetKind::TextBox { .. } => "textbox",
            WidgetKind::Table { .. } => "table",
            WidgetKind::Body { .. } => "body",
            WidgetKind::Status { .. } => "status",
            WidgetKind::Keybindings { .. } => "keybindings",
            WidgetKind::Search { .. } => "search",
            WidgetKind::Splitter { .. } => "splitter",
            WidgetKind::Preview { .. } => "preview",
            WidgetKind::List { .. } => "list",
            WidgetKind::Log { .. } => "log",
            WidgetKind::Tree { .. } => "tree",
            WidgetKind::Tab { .. } => "tab",
            WidgetKind::Tabs => "tabs",
        }
    }

    pub fn is_chrome(&self) -> bool {
        matches!(
            self,
            WidgetKind::Title { .. }
                | WidgetKind::Menu { .. }
                | WidgetKind::Search { .. }
                | WidgetKind::Status { .. }
                | WidgetKind::Tabs
        )
    }

    pub fn is_page_marker(&self) -> bool {
        matches!(self, WidgetKind::Body { .. } | WidgetKind::Tab { .. })
    }

    pub fn is_focusable(&self) -> bool {
        match self {
            WidgetKind::Menu { .. }
            | WidgetKind::Table { .. }
            | WidgetKind::List { .. }
            | WidgetKind::Log { .. }
            | WidgetKind::Tree { .. }
            | WidgetKind::Keybindings { .. }
            | WidgetKind::Search { .. }
            | WidgetKind::Splitter { .. } => true,
            WidgetKind::TextBox { editable, .. } => *editable,
            _ => false,
        }
    }

    pub fn is_text_input(&self) -> bool {
        match self {
            WidgetKind::Search { .. } => true,
            WidgetKind::TextBox { editable, .. } => *editable,
            _ => false,
        }
    }

    pub fn is_scrollable(&self) -> bool {
        matches!(
            self,
            WidgetKind::Table { .. }
                | WidgetKind::List { .. }
                | WidgetKind::Log { .. }
                | WidgetKind::Tree { .. }
                | WidgetKind::Keybindings { .. }
        )
    }
}
