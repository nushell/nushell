use std::{io, panic::Location};

use nu_protocol::{CompileError, ParseError, ShellError, Value};

/// Convenience result type for test helpers.
pub type Result<T = (), E = TestError> = std::result::Result<T, E>;

/// Error returned by [`NuTester`](super::NuTester) helpers.
///
/// This wraps the underlying parse, compile, shell, assertion, or I/O failure
/// with the test call site that produced it, so failures point at the relevant
/// test assertion instead of only the lower-level engine code.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{self:#?}")]
pub struct TestError {
    pub(super) location: TestLocation,
    pub(super) kind: TestErrorKind,
}

/// Source location captured from the test helper call site.
#[derive(Clone, Copy, PartialEq, derive_more::Debug)]
#[debug("{_0}")]
pub(super) struct TestLocation(pub(super) &'static Location<'static>);

/// Errors emitted by `NuTester` when parsing, compiling, or evaluating code.
///
/// This enum is marked as non-exhaustive to allow adding new variants.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum TestErrorKind {
    Parse(ParseError),
    Compile(CompileError),
    Shell(ShellError),
    GotValue {
        got: Value,
    },
    NoInner,
    MultipleInner {
        count: usize,
    },
    UnexpectedErrorKind {
        expected: &'static str,
        got: ShellError,
    },
    UnexpectedValue {
        expected: Value,
        got: Value,
    },
    NoCode {
        expected: String,
    },
    UnexpectedCode {
        expected: String,
        got: String,
    },
    ExampleFailed {
        command: String,
        description: String,
        code: String,
        err: Box<TestErrorKind>,
    },
    Io {
        message: String,
        kind: io::ErrorKind,
    },
}

impl From<ShellError> for TestError {
    #[track_caller]
    fn from(err: ShellError) -> Self {
        Self {
            location: TestLocation(Location::caller()),
            kind: TestErrorKind::Shell(err),
        }
    }
}

impl From<ParseError> for TestError {
    #[track_caller]
    fn from(err: ParseError) -> Self {
        Self {
            location: TestLocation(Location::caller()),
            kind: TestErrorKind::Parse(err),
        }
    }
}

impl From<io::Error> for TestError {
    #[track_caller]
    fn from(value: io::Error) -> Self {
        Self {
            location: TestLocation(Location::caller()),
            kind: TestErrorKind::Io {
                message: value.to_string(),
                kind: value.kind(),
            },
        }
    }
}

impl TestError {
    /// Convert this error into a [`ParseError`], if it is one.
    pub fn parse(self) -> Result<ParseError, TestError> {
        match self.kind {
            TestErrorKind::Parse(err) => Ok(err),
            _ => Err(self),
        }
    }

    /// Convert this error into a [`CompileError`], if it is one.
    pub fn compile(self) -> Result<CompileError, TestError> {
        match self.kind {
            TestErrorKind::Compile(err) => Ok(err),
            _ => Err(self),
        }
    }

    /// Convert this error into a [`ShellError`], if it is one.
    pub fn shell(self) -> Result<ShellError, TestError> {
        match self.kind {
            TestErrorKind::Shell(err) => Ok(err),
            _ => Err(self),
        }
    }

    /// Update it's inner location with the call site of this function.
    #[track_caller]
    pub fn update_location(self) -> Self {
        Self {
            location: TestLocation(Location::caller()),
            ..self
        }
    }
}
