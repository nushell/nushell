use std::fmt::Write;

use nu_ansi_term::{Color, Style};
use similar::{ChangeTag, TextDiff};

/// Generate a stylized diff of different lines between two strings
pub(crate) fn diff_by_line(old: &str, new: &str) -> String {
    let mut out = String::new();

    let diff = TextDiff::from_lines(old, new);

    for change in diff.iter_all_changes() {
        let style = match change.tag() {
            ChangeTag::Equal => Style::new(),
            ChangeTag::Delete => Color::Red.into(),
            ChangeTag::Insert => Color::Green.into(),
        };
        let _ = write!(
            out,
            "{}{}",
            style.paint(change.tag().to_string()),
            style.paint(change.value()),
        );
    }

    out
}
