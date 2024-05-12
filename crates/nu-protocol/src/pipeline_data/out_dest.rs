use parking_lot::Mutex;
use std::{fmt, fs::File, io, process::Stdio, sync::Arc};

/// Describes where to direct the stdout or stderr output stream of external command to.
#[derive(Debug, Clone)]
pub enum OutDest {
    /// Redirect the stdout and/or stderr of one command as the input for the next command in the pipeline.
    ///
    /// The output pipe will be available as the `stdout` of `PipelineData::ExternalStream`.
    ///
    /// If stdout and stderr are both set to `Pipe`,
    /// then they will combined into the `stdout` of `PipelineData::ExternalStream`.
    Pipe,
    /// Capture output to later be collected into a [`Value`](crate::Value), `Vec`, or used in some other way.
    ///
    /// The output stream(s) will be available in the `stdout` or `stderr` of `PipelineData::ExternalStream`.
    ///
    /// This is similar to `Pipe` but will never combine stdout and stderr
    /// or place an external command's stderr into `stdout` of `PipelineData::ExternalStream`.
    Capture,
    /// Ignore output.
    ///
    /// This will forward output to the null device for the platform.
    Null,
    /// Output to nushell's stdout or stderr.
    ///
    /// This causes external commands to inherit nushell's stdout or stderr.
    Inherit,
    /// Redirect output to a file.
    File(Arc<File>), // Arc<File>, since we sometimes need to clone `OutDest` into iterators, etc.
    /// Redirect output to a custom writer.
    ///
    /// This variant isn't used in `nushell` itself but is available for other uses of the `nu` 
    /// language.
    /// It allows capturing stdout and stderr from external commands directly, without executing nu 
    /// code in another process.
    ///
    /// The `Writer` variant is different from the `File` variant, which passes output to a file and 
    /// lets the operating system handle it.
    /// `Stdio` implements for that specific case `From<File>`.
    /// 
    /// Check [`Stack::stdout_writer`] and [`Stack::stderr_writer`] for how to apply a writer to a stack.
    Writer(Arc<Mutex<dyn OutDestWrite + Send + 'static>>),
}

/// Represents a trait object for output destination writer.
///
/// This trait is sealed and is automatically implemented for all types that satisfy [`fmt::Debug`] 
/// and [`io::Write`].
pub trait OutDestWrite: sealed::Sealed + fmt::Debug + io::Write {}

impl<W> OutDestWrite for W where W: fmt::Debug + io::Write {}

pub(crate) mod sealed {
    use std::{fmt, io};

    /// A sealing trait to prevent external implementation of [`OutDestWrite`].
    ///
    /// This trait ensures that `OutDestWrite` cannot be implemented by any type manually.
    pub trait Sealed {}

    impl<T> Sealed for T where T: fmt::Debug + io::Write {}
}

impl From<File> for OutDest {
    fn from(file: File) -> Self {
        Arc::new(file).into()
    }
}

impl From<Arc<File>> for OutDest {
    fn from(file: Arc<File>) -> Self {
        Self::File(file)
    }
}

impl TryFrom<&OutDest> for Stdio {
    type Error = io::Error;

    fn try_from(out_dest: &OutDest) -> Result<Self, Self::Error> {
        match out_dest {
            OutDest::Pipe | OutDest::Capture | OutDest::Writer(_) => Ok(Self::piped()),
            OutDest::Null => Ok(Self::null()),
            OutDest::Inherit => Ok(Self::inherit()),
            OutDest::File(file) => Ok(file.try_clone()?.into()),
        }
    }
}
