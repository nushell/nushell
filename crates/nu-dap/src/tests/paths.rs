//! Unit tests for [`crate::paths`].

use crate::paths::strip_verbatim;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
#[case::verbatim_drive(r"\\?\C:\scripts\demo.nu", r"C:\scripts\demo.nu")]
#[case::verbatim_unc(r"\\?\UNC\server\share\demo.nu", r"\\server\share\demo.nu")]
#[case::plain_drive_is_untouched(r"C:\scripts\demo.nu", r"C:\scripts\demo.nu")]
#[case::posix_is_untouched("/home/user/demo.nu", "/home/user/demo.nu")]
fn strips_windows_verbatim_prefixes(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(strip_verbatim(input), expected);
}
