use std::panic::Location;

use miette::Diagnostic;
use nu_protocol::{
    CompileError, IntoValue, LabeledError, ParseError, ShellError, Span, Value,
    shell_error::{io::IoError, network::NetworkError},
};

use super::{Result, TestError, TestErrorKind, error::TestLocation};

/// Extensions for asserting error kinds from test helpers.
pub trait TestResultExt: Sized {
    /// Expect the result to be a `Value` equal to the provided input.
    fn expect_value_eq<T: IntoValue>(self, value: T) -> Result;

    /// Expect the result to be an error with a specific [`code`](miette::Diagnostic::code).
    fn expect_error_code_eq(self, code: impl AsRef<str>) -> Result;

    /// Expect the result to be a [`ShellError`].
    fn expect_shell_error(self) -> Result<ShellError>;
    /// Expect the result to be a [`ParseError`].
    fn expect_parse_error(self) -> Result<ParseError>;
    /// Expect the result to be a [`CompileError`].
    fn expect_compile_error(self) -> Result<CompileError>;

    /// Expect the result to be a [`ShellError::Io`].
    fn expect_io_error(self) -> Result<IoError>;
    /// Expect the result to be a [`ShellError::Network`].
    fn expect_network_error(self) -> Result<NetworkError>;
    /// Expect the result to be a [`ShellError::LabeledError`].
    fn expect_labeled_error(self) -> Result<LabeledError>;

    /// Expect the result to be a [`ShellError`].
    #[track_caller]
    fn expect_error(self) -> Result<ShellError> {
        self.expect_shell_error()
    }
}

impl TestResultExt for Result<Value> {
    #[track_caller]
    fn expect_value_eq<T: IntoValue>(self, expected: T) -> Result {
        let expected = expected.into_value(Span::test_data());
        match self {
            Err(err) => Err(err.update_location()),
            Ok(actual) if actual == expected => Ok(()),
            Ok(actual) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedValue {
                    expected,
                    got: actual,
                },
            }),
        }
    }

    #[track_caller]
    fn expect_error_code_eq(self, code: impl AsRef<str>) -> Result {
        let expected = code.as_ref();
        let got = match self {
            Ok(got) => {
                return Err(TestError {
                    location: TestLocation(Location::caller()),
                    kind: TestErrorKind::GotValue { got },
                });
            }
            Err(TestError {
                kind: TestErrorKind::Shell(ref err),
                ..
            }) => err.code(),
            Err(TestError {
                kind: TestErrorKind::Compile(ref err),
                ..
            }) => err.code(),
            Err(TestError {
                kind: TestErrorKind::Parse(ref err),
                ..
            }) => err.code(),
            Err(err) => return Err(err.update_location()),
        };

        let Some(got) = got else {
            return Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::NoCode {
                    expected: expected.to_string(),
                },
            });
        };

        let got = got.to_string();
        match got == expected {
            true => Ok(()),
            false => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedCode {
                    expected: expected.to_string(),
                    got,
                },
            }),
        }
    }

    #[track_caller]
    fn expect_shell_error(self) -> Result<ShellError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(err),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_parse_error(self) -> Result<ParseError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Parse(err),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_compile_error(self) -> Result<CompileError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Compile(err),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_io_error(self) -> Result<IoError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(ShellError::Io(err)),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_network_error(self) -> Result<NetworkError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(ShellError::Network(err)),
                ..
            }) => Ok(err),
            Err(err) => Err(err.update_location()),
        }
    }

    #[track_caller]
    fn expect_labeled_error(self) -> Result<LabeledError> {
        match self {
            Ok(got) => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::GotValue { got },
            }),
            Err(TestError {
                kind: TestErrorKind::Shell(ShellError::LabeledError(err)),
                ..
            }) => Ok(*err),
            Err(err) => Err(err.update_location()),
        }
    }
}
