//! Composable TUI commands backed by ratatui.
//!
//! Pipeline shape:
//! `ls | tui title "files" | tui search --bind / | tui table | tui run`

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
    Tui, TuiBody, TuiKeybindings, TuiLabel, TuiList, TuiLog, TuiMenu, TuiPreview, TuiRun,
    TuiSearch, TuiSplitter, TuiStatus, TuiTab, TuiTable, TuiTabs, TuiTextBox, TuiTitle, TuiTree,
};
