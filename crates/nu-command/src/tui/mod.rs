//! Composable TUI commands backed by ratatui.
//!
//! Pipeline shape:
//! `ls | tui label --title "files" | tui split [(tui table) (tui preview)] | tui run`

mod app;
mod commands;
mod keys;
mod layout;
mod render;
mod runtime;
mod session;
mod stream;
mod theme;
mod tree;
mod widget;

pub use commands::{
    Tui, TuiDebug, TuiLabel, TuiLog, TuiMenu, TuiPreview, TuiRun, TuiSearch, TuiSplit, TuiTab,
    TuiTable, TuiTextBox, TuiTree,
};
