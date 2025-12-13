//! Explore config TUI - an interactive configuration viewer and editor.
//!
//! This module provides the `explore config` command which launches a TUI
//! for viewing and editing nushell configuration interactively.

mod app;
mod command;
mod conversion;
mod example_data;
mod input;
mod tree;
mod tui;
mod types;

pub use command::ExploreConfigCommand;
