use crate::{
    ShellError, Span,
    byte_stream::convert_file,
    engine::{EngineState, FrozenJob, Job},
    shell_error::io::IoError,
};
use nu_system::{ExitStatus, ForegroundChild, ForegroundWaitStatus};

use os_pipe::PipeReader;
use std::{
    fmt::Debug,
    io::{self, Read},
    sync::mpsc::{self, Receiver, RecvError, TryRecvError},
    sync::{Arc, Mutex},
    thread,
};

/// Check the exit status of each pipeline element.
///
/// This is used to implement pipefail.
#[cfg(feature = "os")]
pub fn check_exit_status_future(
    exit_status: Vec<Option<(Arc<Mutex<ExitStatusFuture>>, Span)>>,
) -> Result<(), ShellError> {
    for (future, span) in exit_status.into_iter().rev().flatten() {
        check_exit_status_future_ok(future, span)?
    }
    Ok(())
}

fn check_exit_status_future_ok(
    exit_status_future: Arc<Mutex<ExitStatusFuture>>,
    span: Span,
) -> Result<(), ShellError> {
    let mut future = exit_status_future
        .lock()
        .expect("lock exit_status_future should success");
    let exit_status = future.wait(span)?;
    check_ok(exit_status, false, span)
}

pub fn check_ok(status: ExitStatus, ignore_error: bool, span: Span) -> Result<(), ShellError> {
    match status {
        ExitStatus::Exited(exit_code) => {
            if ignore_error {
                Ok(())
            } else if let Ok(exit_code) = exit_code.try_into() {
                Err(ShellError::NonZeroExitCode { exit_code, span })
            } else {
                Ok(())
            }
        }
        #[cfg(unix)]
        ExitStatus::Signaled {
            signal,
            core_dumped,
        } => {
            use nix::sys::signal::Signal;

            let sig = Signal::try_from(signal);

            if sig == Ok(Signal::SIGPIPE) || (ignore_error && !core_dumped) {
                // Processes often exit with SIGPIPE, but this is not an error condition.
                Ok(())
            } else {
                let signal_name = sig.map(Signal::as_str).unwrap_or("unknown signal").into();
                Err(if core_dumped {
                    ShellError::CoreDumped {
                        signal_name,
                        signal,
                        span,
                    }
                } else {
                    ShellError::TerminatedBySignal {
                        signal_name,
                        signal,
                        span,
                    }
                })
            }
        }
    }
}

#[derive(Debug)]
pub enum ExitStatusFuture {
    Finished(Result<ExitStatus, Box<ShellError>>),
    Running(Receiver<io::Result<ExitStatus>>),
}

impl ExitStatusFuture {
    pub fn wait(&mut self, span: Span) -> Result<ExitStatus, ShellError> {
        match self {
            ExitStatusFuture::Finished(Ok(status)) => Ok(*status),
            ExitStatusFuture::Finished(Err(err)) => Err(err.as_ref().clone()),
            ExitStatusFuture::Running(receiver) => {
                let code = match receiver.recv() {
                    #[cfg(unix)]
                    Ok(Ok(
                        status @ ExitStatus::Signaled {
                            core_dumped: true, ..
                        },
                    )) => {
                        check_ok(status, false, span)?;
                        Ok(status)
                    }
                    Ok(Ok(status)) => Ok(status),
                    Ok(Err(err)) => Err(ShellError::Io(IoError::new_with_additional_context(
                        err,
                        span,
                        None,
                        "failed to get exit code",
                    ))),
                    Err(err @ RecvError) => Err(ShellError::GenericError {
                        error: err.to_string(),
                        msg: "failed to get exit code".into(),
                        span: span.into(),
                        help: None,
                        inner: vec![],
                    }),
                };

                *self = ExitStatusFuture::Finished(code.clone().map_err(Box::new));

                code
            }
        }
    }

    fn try_wait(&mut self, span: Span) -> Result<Option<ExitStatus>, ShellError> {
        match self {
            ExitStatusFuture::Finished(Ok(code)) => Ok(Some(*code)),
            ExitStatusFuture::Finished(Err(err)) => Err(err.as_ref().clone()),
            ExitStatusFuture::Running(receiver) => {
                let code = match receiver.try_recv() {
                    Ok(Ok(status)) => Ok(Some(status)),
                    Ok(Err(err)) => Err(ShellError::GenericError {
                        error: err.to_string(),
                        msg: "failed to get exit code".to_string(),
                        span: span.into(),
                        help: None,
                        inner: vec![],
                    }),
                    Err(TryRecvError::Disconnected) => Err(ShellError::GenericError {
                        error: "receiver disconnected".to_string(),
                        msg: "failed to get exit code".into(),
                        span: span.into(),
                        help: None,
                        inner: vec![],
                    }),
                    Err(TryRecvError::Empty) => Ok(None),
                };

                if let Some(code) = code.clone().transpose() {
                    *self = ExitStatusFuture::Finished(code.map_err(Box::new));
                }

                code
            }
        }
    }
}

pub enum ChildPipe {
    Pipe(PipeReader),
    Tee(Box<dyn Read + Send + 'static>),
}

impl Debug for ChildPipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildPipe").finish()
    }
}

impl From<PipeReader> for ChildPipe {
    fn from(pipe: PipeReader) -> Self {
        Self::Pipe(pipe)
    }
}

impl Read for ChildPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            ChildPipe::Pipe(pipe) => pipe.read(buf),
            ChildPipe::Tee(tee) => tee.read(buf),
        }
    }
}

#[derive(Debug)]
pub struct ChildProcess {
    pub stdout: Option<ChildPipe>,
    pub stderr: Option<ChildPipe>,
    exit_status: Arc<Mutex<ExitStatusFuture>>,
    ignore_error: bool,
    span: Span,
}

/// A wrapper for a closure that runs once the shell finishes waiting on the process.
pub struct PostWaitCallback(pub Box<dyn FnOnce(ForegroundWaitStatus) + Send>);

impl PostWaitCallback {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(ForegroundWaitStatus) + Send + 'static,
    {
        PostWaitCallback(Box::new(f))
    }

    /// Creates a PostWaitCallback that creates a frozen job in the job table
    /// if the incoming wait status indicates that the job was frozen.
    ///
    /// If `child_pid` is provided, the returned callback will also remove
    /// it from the pid list of the current running job.
    ///
    /// The given `tag` argument will be used as the tag for the newly created job table entry.
    pub fn for_job_control(
        engine_state: &EngineState,
        child_pid: Option<u32>,
        tag: Option<String>,
    ) -> Self {
        let this_job = engine_state.current_thread_job().cloned();
        let jobs = engine_state.jobs.clone();
        let is_interactive = engine_state.is_interactive;

        PostWaitCallback::new(move |status| {
            if let (Some(this_job), Some(child_pid)) = (this_job, child_pid) {
                this_job.remove_pid(child_pid);
            }

            if let ForegroundWaitStatus::Frozen(unfreeze) = status {
                let mut jobs = jobs.lock().expect("jobs lock is poisoned!");

                let job_id = jobs.add_job(Job::Frozen(FrozenJob { unfreeze, tag }));

                if is_interactive {
                    println!("\nJob {} is frozen", job_id.get());
                }
            }
        })
    }
}

impl Debug for PostWaitCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<wait_callback>")
    }
}

impl ChildProcess {
    pub fn new(
        mut child: ForegroundChild,
        reader: Option<PipeReader>,
        swap: bool,
        span: Span,
        callback: Option<PostWaitCallback>,
    ) -> Result<Self, ShellError> {
        let (stdout, stderr) = if let Some(combined) = reader {
            (Some(combined), None)
        } else {
            let stdout = child.as_mut().stdout.take().map(convert_file);
            let stderr = child.as_mut().stderr.take().map(convert_file);

            if swap {
                (stderr, stdout)
            } else {
                (stdout, stderr)
            }
        };

        // Create a thread to wait for the exit status.
        let (exit_status_sender, exit_status) = mpsc::channel();

        thread::Builder::new()
            .name("exit status waiter".into())
            .spawn(move || {
                let matched = match child.wait() {
                    // there are two possible outcomes when we `wait` for a process to finish:
                    // 1. the process finishes as usual
                    // 2. (unix only) the process gets signaled with SIGTSTP
                    //
                    // in the second case, although the process may still be alive in a
                    // cryonic state, we explicitly treat as it has finished with exit code 0
                    // for the sake of the current pipeline
                    Ok(wait_status) => {
                        let next = match &wait_status {
                            ForegroundWaitStatus::Frozen(_) => ExitStatus::Exited(0),
                            ForegroundWaitStatus::Finished(exit_status) => *exit_status,
                        };

                        if let Some(callback) = callback {
                            (callback.0)(wait_status);
                        }

                        Ok(next)
                    }
                    Err(err) => Err(err),
                };

                exit_status_sender.send(matched)
            })
            .map_err(|err| {
                IoError::new_with_additional_context(
                    err,
                    span,
                    None,
                    "Could now spawn exit status waiter",
                )
            })?;

        Ok(Self::from_raw(stdout, stderr, Some(exit_status), span))
    }

    pub fn from_raw(
        stdout: Option<PipeReader>,
        stderr: Option<PipeReader>,
        exit_status: Option<Receiver<io::Result<ExitStatus>>>,
        span: Span,
    ) -> Self {
        Self {
            stdout: stdout.map(Into::into),
            stderr: stderr.map(Into::into),
            exit_status: Arc::new(Mutex::new(
                exit_status
                    .map(ExitStatusFuture::Running)
                    .unwrap_or(ExitStatusFuture::Finished(Ok(ExitStatus::Exited(0)))),
            )),
            ignore_error: false,
            span,
        }
    }

    pub fn ignore_error(&mut self, ignore: bool) -> &mut Self {
        self.ignore_error = ignore;
        self
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, ShellError> {
        if self.stderr.is_some() {
            debug_assert!(false, "stderr should not exist");
            return Err(ShellError::GenericError {
                error: "internal error".into(),
                msg: "stderr should not exist".into(),
                span: self.span.into(),
                help: None,
                inner: vec![],
            });
        }

        let bytes = if let Some(stdout) = self.stdout {
            collect_bytes(stdout).map_err(|err| IoError::new(err, self.span, None))?
        } else {
            Vec::new()
        };

        let mut exit_status = self
            .exit_status
            .lock()
            .expect("lock exit_status future should success");
        check_ok(exit_status.wait(self.span)?, self.ignore_error, self.span)?;

        Ok(bytes)
    }

    pub fn wait(mut self) -> Result<(), ShellError> {
        let from_io_error = IoError::factory(self.span, None);
        if let Some(stdout) = self.stdout.take() {
            let stderr = self
                .stderr
                .take()
                .map(|stderr| {
                    thread::Builder::new()
                        .name("stderr consumer".into())
                        .spawn(move || consume_pipe(stderr))
                })
                .transpose()
                .map_err(&from_io_error)?;

            let res = consume_pipe(stdout);

            if let Some(handle) = stderr {
                handle
                    .join()
                    .map_err(|e| match e.downcast::<io::Error>() {
                        Ok(io) => from_io_error(*io).into(),
                        Err(err) => ShellError::GenericError {
                            error: "Unknown error".into(),
                            msg: format!("{err:?}"),
                            span: Some(self.span),
                            help: None,
                            inner: Vec::new(),
                        },
                    })?
                    .map_err(&from_io_error)?;
            }

            res.map_err(&from_io_error)?;
        } else if let Some(stderr) = self.stderr.take() {
            consume_pipe(stderr).map_err(&from_io_error)?;
        }
        let mut exit_status = self
            .exit_status
            .lock()
            .expect("lock exit_status future should success");
        check_ok(exit_status.wait(self.span)?, self.ignore_error, self.span)
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, ShellError> {
        let mut exit_status = self
            .exit_status
            .lock()
            .expect("lock exit_status future should success");
        exit_status.try_wait(self.span)
    }

    pub fn wait_with_output(self) -> Result<ProcessOutput, ShellError> {
        let from_io_error = IoError::factory(self.span, None);
        let (stdout, stderr) = if let Some(stdout) = self.stdout {
            let stderr = self
                .stderr
                .map(|stderr| thread::Builder::new().spawn(move || collect_bytes(stderr)))
                .transpose()
                .map_err(&from_io_error)?;

            let stdout = collect_bytes(stdout).map_err(&from_io_error)?;

            let stderr = stderr
                .map(|handle| {
                    handle.join().map_err(|e| match e.downcast::<io::Error>() {
                        Ok(io) => from_io_error(*io).into(),
                        Err(err) => ShellError::GenericError {
                            error: "Unknown error".into(),
                            msg: format!("{err:?}"),
                            span: Some(self.span),
                            help: None,
                            inner: Vec::new(),
                        },
                    })
                })
                .transpose()?
                .transpose()
                .map_err(&from_io_error)?;

            (Some(stdout), stderr)
        } else {
            let stderr = self
                .stderr
                .map(collect_bytes)
                .transpose()
                .map_err(&from_io_error)?;

            (None, stderr)
        };

        let mut exit_status = self
            .exit_status
            .lock()
            .expect("lock exit_status future should success");
        let exit_status = exit_status.wait(self.span)?;

        Ok(ProcessOutput {
            stdout,
            stderr,
            exit_status,
        })
    }

    pub fn clone_exit_status_future(&self) -> Arc<Mutex<ExitStatusFuture>> {
        self.exit_status.clone()
    }
}

fn collect_bytes(pipe: ChildPipe) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    match pipe {
        ChildPipe::Pipe(mut pipe) => pipe.read_to_end(&mut buf),
        ChildPipe::Tee(mut tee) => tee.read_to_end(&mut buf),
    }?;
    Ok(buf)
}

fn consume_pipe(pipe: ChildPipe) -> io::Result<()> {
    match pipe {
        ChildPipe::Pipe(mut pipe) => io::copy(&mut pipe, &mut io::sink()),
        ChildPipe::Tee(mut tee) => io::copy(&mut tee, &mut io::sink()),
    }?;
    Ok(())
}

pub struct ProcessOutput {
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub exit_status: ExitStatus,
}
