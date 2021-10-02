use nu_path::trim_trailing_slash;
use std::path::MAIN_SEPARATOR;

/// Helper function that joins string literals with '/' or '\', based on the host OS
fn join_path_sep(pieces: &[&str]) -> String {
    let sep_string = String::from(MAIN_SEPARATOR);
    pieces.join(&sep_string)
}

#[test]
fn trims_trailing_slash_without_trailing_slash() {
    let path = join_path_sep(&["some", "path"]);

    let actual = trim_trailing_slash(&path);

    assert_eq!(actual, &path)
}

#[test]
fn trims_trailing_slash() {
    let path = join_path_sep(&["some", "path", ""]);

    let actual = trim_trailing_slash(&path);
    let expected = join_path_sep(&["some", "path"]);

    assert_eq!(actual, &expected)
}

#[test]
fn trims_many_trailing_slashes() {
    let path = join_path_sep(&["some", "path", "", "", "", ""]);

    let actual = trim_trailing_slash(&path);
    let expected = join_path_sep(&["some", "path"]);

    assert_eq!(actual, &expected)
}

#[test]
fn trims_trailing_slash_empty() {
    let path = String::from(MAIN_SEPARATOR);
    let actual = trim_trailing_slash(&path);

    assert_eq!(actual, "")
}
