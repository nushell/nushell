use std::{fs::File, io, process::Stdio, sync::Arc};

#[derive(Debug, Clone)]
pub enum Stdoe {
    /// Redirect the `stdout` and/or `stderr` of one command as the input for the next command in the pipeline.
    ///
    /// The output pipe will be available in `PipelineData::ExternalStream::stdout`.
    ///
    /// If both `stdout` and `stderr` are set to `Pipe`,
    /// then they will combined into `ExternalStream::stdout`.
    Pipe,
    /// Capture output to later be collected into a [`Value`](crate::Value), `Vec`, or used in some
    /// other way.
    ///
    /// The output stream(s) will be available in
    /// `PipelineData::ExternalStream::stdout` or `PipelineData::ExternalStream::stderr`.
    ///
    /// This is similar to `Pipe` but will never combine `stdout` and `stderr`
    /// or place an external command's `stderr` into `PipelineData::ExternalStream::stdout`.
    Capture,
    /// Ignore output.
    Null,
    /// Output to nushell's `stdout` or `stderr`.
    ///
    /// This causes external commands to inherit nushell's `stdout` or `stderr`.
    Inherit,
    /// Redirect output to a file.
    File(Arc<File>), // Arc<File>, since we sometimes need to clone `IoStream` into iterators, etc.
}

impl From<File> for Stdoe {
    fn from(file: File) -> Self {
        Arc::new(file).into()
    }
}

impl From<Arc<File>> for Stdoe {
    fn from(file: Arc<File>) -> Self {
        Self::File(file)
    }
}

impl TryFrom<&Stdoe> for Stdio {
    type Error = io::Error;

    fn try_from(stdoe: &Stdoe) -> Result<Self, Self::Error> {
        match stdoe {
            Stdoe::Pipe | Stdoe::Capture => Ok(Self::piped()),
            Stdoe::Null => Ok(Self::null()),
            Stdoe::Inherit => Ok(Self::inherit()),
            Stdoe::File(file) => Ok(file.try_clone()?.into()),
        }
    }
}
