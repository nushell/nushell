//! Widget kinds that make up a [`TuiApp`](super::app::TuiApp).
//!
//! Widgets form a tree: `tui split` and `tui tab` are containers whose
//! `children` are laid out inside them. Everything else is a leaf.

use nu_protocol::engine::Closure;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    /// `true` when the id was generated (`table-0`) rather than passed with
    /// `--id`. Generated ids are renumbered when a child is adopted by a
    /// container whose tree already uses that id; explicit ids never are.
    #[serde(default)]
    pub auto_id: bool,
    pub kind: WidgetKind,
    /// Nested widgets. Only containers (`Split`, `Tab`) have any.
    #[serde(default)]
    pub children: Vec<Widget>,
}

impl Widget {
    #[cfg(test)]
    pub fn leaf(id: impl Into<String>, kind: WidgetKind) -> Self {
        Self {
            id: id.into(),
            auto_id: false,
            kind,
            children: Vec::new(),
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
}

/// Where a `Label` is drawn.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Slot {
    /// One-line title bar at the top.
    Title,
    /// Inline text inside the page.
    Content,
    /// One-line status bar at the bottom. Live hints are appended.
    Status,
}

impl Slot {
    pub fn as_str(self) -> &'static str {
        match self {
            Slot::Title => "title",
            Slot::Content => "content",
            Slot::Status => "status",
        }
    }
}

/// One entry on a menu bar or in a dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub label: String,
    /// Key that selects this item: `alt+<key>` on the bar, `<key>` inside an
    /// open dropdown. `&` before a letter in the label picks it; otherwise
    /// the first letter.
    pub mnemonic: Option<char>,
    /// Dropdown entries. Empty for a plain item.
    pub items: Vec<MenuItem>,
    /// Runs when the item is activated. A returned value replaces the data
    /// list; `nothing` leaves it alone. Without an action, activating the
    /// item submits it.
    pub action: Option<Closure>,
}

impl MenuItem {
    /// Parse `&` mnemonics: `"&File"` → label `File`, mnemonic `f`.
    pub fn new(raw: &str, items: Vec<MenuItem>, action: Option<Closure>) -> Self {
        let mut label = String::with_capacity(raw.len());
        let mut mnemonic = None;
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '&' && mnemonic.is_none() {
                if let Some(next) = chars.next() {
                    mnemonic = Some(next.to_ascii_lowercase());
                    label.push(next);
                }
            } else {
                label.push(c);
            }
        }
        let mnemonic = mnemonic.or_else(|| {
            label
                .chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
        });
        Self {
            label,
            mnemonic,
            items,
            action,
        }
    }

    /// Byte offset of the mnemonic letter in `label`, for underlining.
    pub fn mnemonic_index(&self) -> Option<usize> {
        let m = self.mnemonic?;
        self.label
            .char_indices()
            .find(|(_, c)| c.to_ascii_lowercase() == m)
            .map(|(i, _)| i)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetKind {
    /// Static text. `slot` picks the title bar, page content, or status bar.
    Label {
        text: String,
        slot: Slot,
    },
    /// Horizontal menu bar. Items with sub-items open a dropdown.
    Menu {
        items: Vec<MenuItem>,
    },
    TextBox {
        placeholder: String,
        value: String,
    },
    /// Navigable rows from the shared data list. Scalars show as one `item`
    /// column. `capture_keys` turns a pressed chord into the filter query.
    Table {
        columns: Vec<String>,
        capture_keys: bool,
    },
    /// Search box. At the top level it filters everything; nested inside a
    /// container it filters only that container's descendants.
    Search {
        placeholder: String,
        bind: Option<String>,
    },
    /// Follows the selected table/tree row and shows text for it.
    /// Not focusable, so arrow keys keep moving the source.
    Preview {
        max_bytes: usize,
        /// Zero parameters: receives file contents as `$in`. One parameter:
        /// receives the selected row and its output is the pane text.
        transform: Option<Closure>,
        /// Table/tree id to follow. Defaults to the nearest one.
        from: Option<String>,
    },
    /// Append-only log. New stream rows appear at the bottom.
    Log {
        max_lines: usize,
    },
    /// Directory / nested-record tree over the shared data list.
    Tree {
        /// Expand directories on disk when a node is opened.
        walk: bool,
        column: String,
    },
    /// Container. At the top level it is a page shown in the tab bar; nested
    /// inside another container it is a titled group box.
    Tab {
        title: String,
    },
    /// Container that divides its area between its children.
    Split {
        direction: SplitDir,
        /// Percent of the area given to the first child, 10-90.
        ratio: u16,
    },
}

impl WidgetKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            WidgetKind::Label { .. } => "label",
            WidgetKind::Menu { .. } => "menu",
            WidgetKind::TextBox { .. } => "textbox",
            WidgetKind::Table { .. } => "table",
            WidgetKind::Search { .. } => "search",
            WidgetKind::Preview { .. } => "preview",
            WidgetKind::Log { .. } => "log",
            WidgetKind::Tree { .. } => "tree",
            WidgetKind::Tab { .. } => "tab",
            WidgetKind::Split { .. } => "split",
        }
    }

    /// Fixed-slot widgets that only make sense at the top level.
    pub fn is_chrome(&self) -> bool {
        matches!(
            self,
            WidgetKind::Label {
                slot: Slot::Title | Slot::Status,
                ..
            } | WidgetKind::Menu { .. }
                | WidgetKind::Search { .. }
        )
    }

    pub fn is_container(&self) -> bool {
        matches!(self, WidgetKind::Tab { .. } | WidgetKind::Split { .. })
    }

    pub fn is_focusable(&self) -> bool {
        matches!(
            self,
            WidgetKind::Menu { .. }
                | WidgetKind::Table { .. }
                | WidgetKind::Log { .. }
                | WidgetKind::Tree { .. }
                | WidgetKind::Search { .. }
                | WidgetKind::TextBox { .. }
                | WidgetKind::Split { .. }
        )
    }

    pub fn is_text_input(&self) -> bool {
        matches!(self, WidgetKind::Search { .. } | WidgetKind::TextBox { .. })
    }

    pub fn is_scrollable(&self) -> bool {
        matches!(
            self,
            WidgetKind::Table { .. } | WidgetKind::Log { .. } | WidgetKind::Tree { .. }
        )
    }

    /// Widgets a preview can follow.
    pub fn is_row_source(&self) -> bool {
        matches!(self, WidgetKind::Table { .. } | WidgetKind::Tree { .. })
    }
}
