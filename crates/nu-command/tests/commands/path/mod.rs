mod basename;
mod dirname;
mod exists;
mod expand;
mod join;
mod parse;
mod split;
mod type_;

use std::path::MAIN_SEPARATOR;

/// Helper function that joins string literals with '/' or '\', based on host OS
fn join_path_sep(pieces: &[&str]) -> String {
    let sep_string = String::from(MAIN_SEPARATOR);
    pieces.join(&sep_string)
}

#[cfg(windows)]
#[test]
fn joins_path_on_windows() {
    let pieces = ["sausage", "bacon", "spam"];
    let actual = join_path_sep(&pieces);

    assert_eq!(&actual, "sausage\\bacon\\spam");
}

#[cfg(not(windows))]
#[test]
fn joins_path_on_other_than_windows() {
    let pieces = ["sausage", "bacon", "spam"];
    let actual = join_path_sep(&pieces);

    assert_eq!(&actual, "sausage/bacon/spam");
}
