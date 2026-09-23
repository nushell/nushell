//! Composable TUI commands backed by ratatui.
//!
//! Pipeline shape:
//! `ls | tui label --title "files" | tui split [(tui table) (tui preview)] | tui run`
//!
//! Builders (`tui table`, `tui split`, …) append widgets to a `tui` custom
//! value; `tui run` owns the terminal and returns one record; `tui debug`
//! paints headlessly for tests and scripts.

mod app;
mod commands;
mod default_context;
mod filter;
mod hooks;
mod keys;
mod layout;
mod render;
mod runtime;
mod session;
mod stream;
mod theme;
mod tree;
mod widget;
mod widgets;

pub use commands::*;
pub use default_context::add_tui_context;
