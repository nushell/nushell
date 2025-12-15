//! Explore regex TUI - an interactive regular expression explorer.
//!
//! This module provides the `explore regex` command which launches a TUI
//! for creating and testing regular expressions interactively.

mod app;
mod colors;
mod command;
mod ui;

pub use command::ExploreRegex;
