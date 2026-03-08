#![doc = include_str!("../README.md")]

use std::{borrow::Borrow, fmt::Debug, process::ExitStatus};

pub mod commands;
pub mod fs;
pub mod harness;
pub mod locale_override;
pub mod macros;
pub mod playground;

pub mod tester;
use nu_utils::container::Container;
pub use tester::{Result, ShellErrorExt, TestError as Error, TestResultExt, test};

pub mod prelude {
    #[doc(no_inline)]
    pub use super::{
        Outcome, assert_contains, nu,
        playground::Playground,
        tester::{Result, ShellErrorExt, TestError as Error, TestResultExt, test},
    };

    #[doc(no_inline)]
    pub use nu_protocol::{CompileError, FromValue, IntoValue, ParseError, ShellError, Value};
}

// Expose macros to be used for the test harness.
pub use harness::macros::*;

// Needs to be reexported for `nu!` macro
pub use nu_path;

#[derive(Debug)]
pub struct Outcome {
    pub out: String,
    pub err: String,
    pub status: ExitStatus,
}

#[cfg(windows)]
pub const NATIVE_PATH_ENV_VAR: &str = "Path";
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_VAR: &str = "PATH";

#[cfg(windows)]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ';';
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ':';

impl Outcome {
    pub fn new(out: String, err: String, status: ExitStatus) -> Outcome {
        Outcome { out, err, status }
    }
}

pub fn nu_repl_code(source_lines: &[&str]) -> String {
    let mut out = String::from("nu --testbin=nu_repl ...[ ");

    for line in source_lines.iter() {
        out.push('`');
        out.push_str(line);
        out.push('`');
        out.push(' ');
    }

    out.push(']');

    out
}

pub fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os(NATIVE_PATH_ENV_VAR) {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}

/// Assert that a haystack contains the given needle.
///
/// Uses the [`Container`] abstraction so it works with slices, vectors, sets,
/// maps (by key), strings, and ranges.
/// The error message includes both the container and the item for quick debugging.
///
/// # Panics
///
/// Panics if `haystack.contains(needle)` returns false.
#[track_caller]
pub fn assert_contains<H, N>(needle: N, haystack: H)
where
    H: Container + Debug,
    N: Borrow<H::Item>,
    H::Item: Debug,
{
    let item = needle.borrow();

    assert!(
        haystack.contains(item),
        "{haystack:?} does not contain {item:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::assert_contains;

    #[test]
    #[expect(clippy::needless_borrows_for_generic_args)]
    fn test_something() {
        assert_contains(1, [1, 2, 3]);
        assert_contains(2, &[1, 2, 3]);
        assert_contains("a", "abc");
        assert_contains("b", String::from("abc"));
        assert_contains(String::from("b"), String::from("abc"));
        assert_contains("c", &String::from("abc"));
        assert_contains(2, vec![1, 2, 3]);
        assert_contains(1, &vec![1, 2, 3]);
    }
}
