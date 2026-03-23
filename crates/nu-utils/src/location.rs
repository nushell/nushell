use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{self, Display},
};

/// A serializable source code location.
///
/// This is meant to be a drop in replacement for [`std::panic::Location`]
/// with the added benefit that it implements [`Serialize`] and [`Deserialize`].
///
/// In practice, this is useful when you want to capture caller information
/// and store it, send it across process boundaries, or include it in serialized data.
///
/// For behavior and semantics, see [`std::panic::Location`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Location {
    file: Cow<'static, str>,
    line: u32,
    column: u32,
}

impl Location {
    /// Returns the caller location.
    ///
    /// See [`std::panic::Location::caller`] for details.
    #[track_caller]
    pub const fn caller() -> Self {
        let location = std::panic::Location::caller();
        Self {
            file: Cow::Borrowed(location.file()),
            line: location.line(),
            column: location.column(),
        }
    }

    /// Returns the file name.
    ///
    /// See [`std::panic::Location::file`] for details.
    pub const fn file(&self) -> &str {
        match &self.file {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => s.as_str(),
        }
    }

    /// Returns the column number.
    ///
    /// See [`std::panic::Location::column`] for details.
    pub const fn column(&self) -> u32 {
        self.column
    }

    /// Returns the line number.
    ///
    /// See [`std::panic::Location::line`] for details.
    pub const fn line(&self) -> u32 {
        self.line
    }
}

/// Converts a static [`std::panic::Location`] into a [`Location`].
///
/// `std::panic::Location::caller()` returns a `'static` location, so this
/// conversion can borrow the file path without allocating.
impl From<&'static std::panic::Location<'static>> for Location {
    fn from(location: &'static std::panic::Location<'static>) -> Self {
        Self {
            file: Cow::Borrowed(location.file()),
            line: location.line(),
            column: location.column(),
        }
    }
}

impl Display for Location {
    /// Formats this location as `file:line:column`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}
