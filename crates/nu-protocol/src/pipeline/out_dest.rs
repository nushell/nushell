use std::{fs::File, io, process::Stdio, sync::Arc};

/// Describes where to direct a command's stdout or stderr.
///
/// Used both for external-process stdio wiring and for Nushell's internal
/// pipeline destination (print vs pipe vs collect-to-value, etc.).
#[derive(Debug, Clone)]
pub enum OutDest {
    /// Redirect the stdout and/or stderr of one command as the input for the next command in the pipeline.
    ///
    /// The output pipe will be available as the `stdout` of [`ChildProcess`](crate::process::ChildProcess).
    ///
    /// If stdout and stderr are both set to `Pipe`,
    /// then they will combined into the `stdout` of [`ChildProcess`](crate::process::ChildProcess).
    Pipe,
    /// Redirect the stdout and/or stderr of one command as the input for the next command in the pipeline.
    ///
    /// The output stream(s) will be available in the `stdout` or `stderr` of [`ChildProcess`](crate::process::ChildProcess).
    ///
    /// This is similar to `Pipe` but will never combine stdout and stderr
    /// or place an external command's stderr into `stdout` of [`ChildProcess`](crate::process::ChildProcess).
    PipeSeparate,
    /// Signifies the result of the pipeline will be immediately collected into a value after this command.
    ///
    /// So, it is fine to collect the stream ahead of time in the current command.
    Value,
    /// Ignore output.
    ///
    /// This will forward output to the null device for the platform.
    Null,
    /// Output to nushell's stdout or stderr (only for external commands).
    ///
    /// This causes external commands to inherit nushell's stdout or stderr. This also causes
    /// [`ListStream`](crate::ListStream)s to be drained, but not to be printed.
    Inherit,
    /// Print to nushell's stdout or stderr.
    ///
    /// This is just like `Inherit`, except that [`ListStream`](crate::ListStream)s and
    /// [`Value`](crate::Value)s are also printed.
    ///
    /// This is the only destination treated as "not redirected" by
    /// [`OutDest::is_redirected`] / the `is-redirected` command.
    Print,
    /// Redirect output to a file.
    File(Arc<File>), // Arc<File>, since we sometimes need to clone `OutDest` into iterators, etc.
}

impl OutDest {
    /// Returns `true` when output is *not* sent through the interactive display path.
    ///
    /// Only [`OutDest::Print`] is treated as displayed. Every other variant means the
    /// pipeline result is consumed elsewhere (next command, `let`/subexpression,
    /// file, null device, or inherit-without-printing).
    ///
    /// This is **not** an OS-level TTY check. For whether process stdio is a terminal,
    /// use the `is-terminal` command (`std::io::IsTerminal`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use nu_protocol::OutDest;
    /// assert!(!OutDest::Print.is_redirected());
    /// assert!(OutDest::Pipe.is_redirected());
    /// assert!(OutDest::Value.is_redirected());
    /// ```
    #[must_use]
    pub fn is_redirected(&self) -> bool {
        !matches!(self, Self::Print)
    }
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
            OutDest::Pipe | OutDest::PipeSeparate | OutDest::Value => Ok(Self::piped()),
            OutDest::Null => Ok(Self::null()),
            OutDest::Print | OutDest::Inherit => Ok(Self::inherit()),
            OutDest::File(file) => Ok(file.try_clone()?.into()),
        }
    }
}
