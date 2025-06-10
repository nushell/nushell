use thiserror::Error;

/// Represents a specific location in the Rust code.
///
/// This data structure is used to provide detailed information about where in the Rust code
/// an error occurred.
/// While most errors in [`ShellError`](super::ShellError) are related to user-provided Nushell
/// code, some originate from the underlying Rust implementation.
/// With this type, we can pinpoint the exact location of such errors, improving debugging
/// and error reporting.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{file}:{line}:{column}")]
pub struct Location {
    file: &'static str,
    line: u32,
    column: u32,
}

impl Location {
    /// Internal constructor for [`Location`].
    ///
    /// This function is not intended to be called directly.
    /// Instead, use the [`location!`] macro to create instances.
    #[doc(hidden)]
    #[deprecated(
        note = "This function is not meant to be called directly. Use `nu_protocol::location` instead."
    )]
    pub fn new(file: &'static str, line: u32, column: u32) -> Self {
        Location { file, line, column }
    }
}

/// Macro to create a new [`Location`] for the exact position in your code.
///
/// This macro captures the current file, line, and column during compilation,
/// providing an easy way to associate errors with specific locations in the Rust code.
///
/// # Note
/// This macro relies on the [`file!`], [`line!`], and [`column!`] macros to fetch the
/// compilation context.
#[macro_export]
macro_rules! location {
    () => {{
        #[allow(deprecated)]
        $crate::shell_error::location::Location::new(file!(), line!(), column!())
    }};
}

#[test]
fn test_location_macro() {
    let location = crate::location!();
    let line = line!() - 1; // Adjust for the macro call being on the previous line.
    let file = file!();
    assert_eq!(location.line, line);
    assert_eq!(location.file, file);
}
