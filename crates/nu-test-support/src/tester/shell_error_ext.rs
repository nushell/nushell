use std::panic::Location;

use nu_protocol::{LabeledError, ShellError};

use super::{Result, TestError, TestErrorKind, error::TestLocation};

/// Extensions for interrogating [`ShellError`] values in tests.
pub trait ShellErrorExt {
    /// Tries to convert into an inner value from a [`ShellError`].
    ///
    /// Useful if the error is expected to be a generic error that contains an inner error or a
    /// chained error that chained another error.
    ///
    /// However, this function returns [`TestErrorKind::NoInner`]
    /// - if `inner` of [`ShellError::Generic`] is empty
    /// - if `sources` of [`ShellError::ChainedError`] is empty
    /// - if `sources` of [`ShellError::EvalBlockWithInput`] is empty
    /// - the error is none of the above types
    ///
    /// Also if multiple inner values are found a [`TestErrorKind::MultipleInner`] is returned.
    fn into_inner(self) -> Result<ShellError>;

    /// Extract the [`LabeledError`] from [`ShellError::LabeledError`], if it is one.
    fn into_labeled(self) -> Result<LabeledError>;

    /// Extract the iterator on the sources of the [`ChainedError`] from
    /// [`ShellError::ChainedError`], it it is one.
    fn into_chained_iter(self) -> Result<impl Iterator<Item = ShellError>>;

    /// Extract the error field from [`ShellError::Generic`], if it is one.
    fn generic_error(self) -> Result<String>;

    /// Extract the message field from [`ShellError::Generic`], if it is one.
    fn generic_msg(self) -> Result<String>;
}

impl ShellErrorExt for ShellError {
    #[track_caller]
    fn into_inner(self) -> Result<ShellError> {
        let no_inner = TestError {
            location: TestLocation(Location::caller()),
            kind: TestErrorKind::NoInner,
        };

        let iter: &mut dyn Iterator<Item = ShellError> = match self {
            ShellError::Generic(err) => &mut err.inner.into_iter(),
            ShellError::ChainedError(err) => &mut err.sources_iter(),
            ShellError::EvalBlockWithInput { sources, .. } => &mut sources.into_iter(),
            _ => return Err(no_inner),
        };

        let Some(inner) = iter.next() else {
            return Err(no_inner);
        };

        let rest = iter.count();
        if rest != 0 {
            return Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::MultipleInner { count: rest + 1 },
            });
        }

        Ok(inner)
    }

    #[track_caller]
    fn into_labeled(self) -> Result<LabeledError> {
        match self {
            ShellError::LabeledError(err) => Ok(*err),
            got => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedErrorKind {
                    expected: "Labeled",
                    got,
                },
            }),
        }
    }

    #[track_caller]
    fn into_chained_iter(self) -> Result<impl Iterator<Item = ShellError>> {
        match self {
            ShellError::ChainedError(err) => Ok(err.sources_iter()),
            got => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedErrorKind {
                    expected: "Chained",
                    got,
                },
            }),
        }
    }

    #[track_caller]
    fn generic_error(self) -> Result<String> {
        match self {
            ShellError::Generic(err) => Ok(err.error.into_owned()),
            got => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedErrorKind {
                    expected: "Generic",
                    got,
                },
            }),
        }
    }

    #[track_caller]
    fn generic_msg(self) -> Result<String> {
        match self {
            ShellError::Generic(err) => Ok(err.msg.into_owned()),
            got => Err(TestError {
                location: TestLocation(Location::caller()),
                kind: TestErrorKind::UnexpectedErrorKind {
                    expected: "Generic",
                    got,
                },
            }),
        }
    }
}
