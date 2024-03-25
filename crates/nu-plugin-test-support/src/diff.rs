use std::fmt::Write;

use crossterm::style::{Color, Stylize};
use similar::{ChangeTag, TextDiff};

/// Generate a stylized diff of different lines between two strings
pub(crate) fn diff_by_line(old: &str, new: &str) -> String {
    let mut out = String::new();

    let diff = TextDiff::from_lines(old, new);

    for change in diff.iter_all_changes() {
        let color = match change.tag() {
            ChangeTag::Equal => Color::Reset,
            ChangeTag::Delete => Color::Red,
            ChangeTag::Insert => Color::Green,
        };
        let _ = write!(
            out,
            "{}{}",
            change.tag().to_string().with(color),
            change.value_ref().with(color)
        );
    }

    out
}
