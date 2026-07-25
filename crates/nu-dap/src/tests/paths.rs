//! Unit tests for [`crate::paths`].

use crate::paths::strip_verbatim;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
#[case(r"\\?\C:\scripts\demo.nu", r"C:\scripts\demo.nu")]
#[case(r"\\?\UNC\server\share\demo.nu", r"\\server\share\demo.nu")]
#[case(r"C:\scripts\demo.nu", r"C:\scripts\demo.nu")]
#[case("/home/user/demo.nu", "/home/user/demo.nu")]
fn strips_windows_verbatim_prefixes(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(strip_verbatim(input), expected);
}
