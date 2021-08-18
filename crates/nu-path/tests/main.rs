use std::path::{Path, PathBuf};

// TODO:
// * non-unicode paths
// * canonicalize(_with)
//   * .
//   * ..
//   * ...+
//   * ~
//   * symlink
//   * symlink multiple depths
//   * symlink loop
// * expand_path(_with)
//   * .
//   * ..
//   * ...+
//   * ~
//   * symlink
//   * symlink multiple depths
//   * symlink loop

use nu_path::{canonicalize, canonicalize_with, expand_path, expand_path_with};

#[test]
fn expand_path_with_and_without_relative() {
    let relative_to = Path::new("/foo/bar");
    let path = Path::new("../..");
    let full_path = Path::new("/foo/bar/../..");

    assert_eq!(expand_path(full_path), expand_path_with(path, relative_to),);
}

#[test]
fn expand_path_with_relative() {
    let relative_to = Path::new("/foo/bar");
    let path = Path::new("../..");

    assert_eq!(PathBuf::from("/"), expand_path_with(path, relative_to),);
}

#[test]
fn canonicalize_should_fail() {
    let path = Path::new("/foo/bar/baz/../..");

    assert!(canonicalize(path).is_err());
}

#[test]
fn canonicalize_with_should_fail() {
    let relative_to = Path::new("/foo/bar/baz"); // '/foo' is (hopefully) missing
    let path = Path::new("../..");

    assert!(canonicalize_with(path, relative_to).is_err());
}
