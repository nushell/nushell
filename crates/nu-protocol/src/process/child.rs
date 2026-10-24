use crate::{
    ShellError, Span,
    byte_stream::convert_file,
    engine::{EngineState, FrozenJob, Job},
    shell_error::{generic::GenericError, io::IoError},
};
use nu_system::{ExitStatus, ForegroundChild, ForegroundWaitStatus};

use os_pipe::PipeReader;
use std::{
    collections::VecDeque,
    fmt::Debug,
    io::{self, Read},
    sync::mpsc::{self, Receiver, RecvError, TryRecvError},
    sync::{
        Arc, Condvar, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

/// Check the exit status of each pipeline element.
///
/// This is used to implement pipefail. The pipes the guards can read ahead are read into memory
/// first, so a process can't block on a pipe that nobody reads yet.
#[cfg(feature = "os")]
pub fn check_exit_status_future(
    exit_status: Vec<Option<ExitStatusGuard>>,
) -> Result<(), ShellError> {
    for guard in exit_status.iter().flatten() {
        guard.read_ahead.iter().for_each(ReadAheadHandle::start);
    }
    for one_status in exit_status.into_iter().rev().flatten() {
        check_exit_status_future_ok(one_status)?
    }
    Ok(())
}

fn check_exit_status_future_ok(exit_status_guard: ExitStatusGuard) -> Result<(), ShellError> {
    let ignore_error = {
        let guard = exit_status_guard
            .ignore_error
            .lock()
            .expect("lock ignore_error should success");
        *guard
    };
    let mut future = exit_status_guard
        .exit_status_future
        .lock()
        .expect("lock exit_status_future should success");
    let span = exit_status_guard.span.unwrap_or_default();
    let exit_status = future.wait(span)?;
    check_ok(exit_status, ignore_error, span)
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

/// A wrapper for both `exit_status_future: Arc<Mutex<ExitStatusFuture>>`
/// and `ignore_error: Arc<Mutex<bool>>`
///
/// It's useful for `pipefail` feature, which tracks exit status code with potentially
/// ignore the error.
#[derive(Debug)]
pub struct ExitStatusGuard {
    pub exit_status_future: Arc<Mutex<ExitStatusFuture>>,
    pub ignore_error: Arc<Mutex<bool>>,
    pub span: Option<Span>,
    /// The process's pipes that [`check_exit_status_future`] reads ahead before it waits.
    pub read_ahead: Vec<ReadAheadHandle>,
}

impl ExitStatusGuard {
    pub fn new(
        exit_status_future: Arc<Mutex<ExitStatusFuture>>,
        ignore_error: Arc<Mutex<bool>>,
    ) -> Self {
        Self {
            exit_status_future,
            ignore_error,
            span: None,
            read_ahead: Vec::new(),
        }
    }

    pub fn with_span(self, span: Span) -> Self {
        Self {
            span: Some(span),
            ..self
        }
    }

    pub fn with_read_ahead(self, read_ahead: Vec<ReadAheadHandle>) -> Self {
        Self { read_ahead, ..self }
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
                    Err(err @ RecvError) => Err(ShellError::Generic(GenericError::new(
                        err.to_string(),
                        "failed to get exit code",
                        span,
                    ))),
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
                    Ok(Err(err)) => Err(ShellError::Generic(GenericError::new(
                        err.to_string(),
                        "failed to get exit code",
                        span,
                    ))),
                    Err(TryRecvError::Disconnected) => Err(ShellError::Generic(GenericError::new(
                        "receiver disconnected",
                        "failed to get exit code",
                        span,
                    ))),
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

#[derive(derive_more::Debug)]
pub enum ChildPipe {
    #[debug("ChildPipe::Pipe")]
    Pipe(PipeReader),

    #[debug("ChildPipe::Tee")]
    Tee(Box<dyn Read + Send + 'static>),

    #[debug("ChildPipe::ReadAhead")]
    ReadAhead(ReadAheadPipe),
}

impl ChildPipe {
    /// Start reading the pipe into memory on another thread (see [`ReadAheadPipe`]).
    pub fn start_read_ahead(&mut self) {
        self.read_ahead_handle().start();
    }

    /// Turn the pipe into a [`ReadAheadPipe`], unless it is one already, and get its handle.
    fn read_ahead_handle(&mut self) -> ReadAheadHandle {
        if let ChildPipe::ReadAhead(pipe) = self {
            return pipe.handle();
        }
        let pipe = ReadAheadPipe::new(std::mem::replace(
            self,
            ChildPipe::Tee(Box::new(io::empty())),
        ));
        let handle = pipe.handle();
        *self = ChildPipe::ReadAhead(pipe);
        handle
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
            ChildPipe::ReadAhead(pipe) => pipe.read(buf),
        }
    }
}

/// A child's pipe that can be read into memory on another thread before anyone asks for it.
///
/// Waiting for a process before reading its output can hang: a process that fills a pipe nobody
/// reads never exits. [`check_exit_status_future`] and [`ChildProcess::wait_keeping_output`]
/// start reading ahead before they wait, so the process can finish, and the output stays
/// readable from memory. Until then, reads go straight to the pipe, and dropping the reader drops
/// the pipe, as with any other [`ChildPipe`].
#[derive(Debug)]
pub struct ReadAheadPipe {
    shared: Arc<ReadAheadShared>,
}

/// A handle to start reading a [`ReadAheadPipe`] ahead.
#[derive(Debug, Clone)]
pub struct ReadAheadHandle(Arc<ReadAheadShared>);

#[derive(Debug)]
struct ReadAheadShared {
    state: Mutex<ReadAheadState>,
    /// Signaled when the read-ahead thread adds bytes or finishes.
    progress: Condvar,
    started: AtomicBool,
}

#[derive(Debug)]
struct ReadAheadState {
    /// The pipe, until the read-ahead thread takes it.
    pipe: Option<ChildPipe>,
    /// What the read-ahead thread has read and the reader hasn't.
    buffer: VecDeque<u8>,
    /// Set when the read-ahead thread is done: `None` at the end of the pipe, or the error it hit.
    finished: Option<Option<io::Error>>,
    /// The reader was dropped, so the read-ahead thread stops and drops the pipe.
    closed: bool,
}

impl ReadAheadShared {
    fn lock(&self) -> MutexGuard<'_, ReadAheadState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Read the pipe to the end into the buffer. Runs on the read-ahead thread.
    fn read_ahead(&self) {
        let Some(mut pipe) = self.lock().pipe.take() else {
            return;
        };
        let mut chunk = [0; 8192];
        let finished = loop {
            match pipe.read(&mut chunk) {
                Ok(0) => break None,
                Ok(n) => {
                    let mut state = self.lock();
                    if state.closed {
                        return;
                    }
                    state.buffer.extend(&chunk[..n]);
                    self.progress.notify_all();
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => break Some(err),
            }
        };
        self.lock().finished = Some(finished);
        self.progress.notify_all();
    }
}

impl ReadAheadPipe {
    fn new(pipe: ChildPipe) -> Self {
        Self {
            shared: Arc::new(ReadAheadShared {
                state: Mutex::new(ReadAheadState {
                    pipe: Some(pipe),
                    buffer: VecDeque::new(),
                    finished: None,
                    closed: false,
                }),
                progress: Condvar::new(),
                started: AtomicBool::new(false),
            }),
        }
    }

    fn handle(&self) -> ReadAheadHandle {
        ReadAheadHandle(self.shared.clone())
    }

    /// Get the OS pipe back, if nothing has been read ahead yet, so it can become another
    /// process's stdin.
    pub fn into_pipe(self) -> Result<PipeReader, Self> {
        let mut state = self.shared.lock();
        match state.pipe.take() {
            Some(ChildPipe::Pipe(pipe)) if state.buffer.is_empty() => return Ok(pipe),
            pipe => state.pipe = pipe,
        }
        drop(state);
        Err(self)
    }
}

impl ReadAheadHandle {
    /// Start reading the pipe into memory on another thread, unless that started already or the
    /// reader is gone.
    pub fn start(&self) {
        if self.0.started.swap(true, Ordering::Relaxed) {
            return;
        }
        // Usually the reader is done by the time pipefail waits, so there is nothing to read.
        // A reader in the middle of a read holds the lock; the thread waits for it instead.
        if let Ok(state) = self.0.state.try_lock()
            && (state.pipe.is_none() || state.closed)
        {
            return;
        }
        let shared = self.0.clone();
        // If the thread can't start, the pipe stays with the reader and waiting can block, as
        // it did before reading ahead existed.
        let _ = thread::Builder::new()
            .name("pipe read-ahead".into())
            .spawn(move || shared.read_ahead());
    }
}

impl Read for ReadAheadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut state = self.shared.lock();
        loop {
            if !state.buffer.is_empty() {
                return state.buffer.read(buf);
            }
            // Not taken by the read-ahead thread (yet): read the pipe directly. The thread waits
            // for the lock, so the bytes stay in order.
            if let Some(pipe) = &mut state.pipe {
                return pipe.read(buf);
            }
            // Otherwise the read-ahead thread has it.
            match &mut state.finished {
                Some(err) => return err.take().map_or(Ok(0), Err),
                None => {
                    state = (self.shared.progress)
                        .wait(state)
                        .unwrap_or_else(PoisonError::into_inner);
                }
            }
        }
    }
}

impl Drop for ReadAheadPipe {
    fn drop(&mut self) {
        let mut state = self.shared.lock();
        state.closed = true;
        state.pipe = None;
        state.buffer = VecDeque::new();
    }
}

#[derive(Debug)]
pub struct ChildProcess {
    pub stdout: Option<ChildPipe>,
    pub stderr: Option<ChildPipe>,
    exit_status: Arc<Mutex<ExitStatusFuture>>,
    ignore_error: Arc<Mutex<bool>>,
    /// Pid of the process, when it was spawned by the shell. Lets a consumer
    /// that stops reading early (an interactive viewer) kill a producer that
    /// would otherwise keep the pipeline waiting on its exit status.
    pid: Option<u32>,
    span: Span,
}

/// A wrapper for a closure that runs once the shell finishes waiting on the process.
#[derive(derive_more::Debug)]
#[debug("<wait_callback>")]
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
    /// The given `description` argument will be used as the description for the newly created job table entry.
    pub fn for_job_control(
        engine_state: &EngineState,
        child_pid: Option<u32>,
        description: Option<String>,
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

                let job_id = jobs.add_job(Job::Frozen(FrozenJob {
                    unfreeze,
                    description,
                }));

                if is_interactive {
                    println!("\nJob {} is frozen", job_id.get());
                }
            }
        })
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
        let (stdout, stderr) = match reader {
            Some(combined) => (Some(combined), None),
            None => {
                let stdout = child.as_mut().stdout.take().map(convert_file);
                let stderr = child.as_mut().stderr.take().map(convert_file);

                if swap {
                    (stderr, stdout)
                } else {
                    (stdout, stderr)
                }
            }
        };
        let pid = child.pid();

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

        let mut this = Self::from_raw(stdout, stderr, Some(exit_status), span);
        this.pid = Some(pid);
        Ok(this)
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
            ignore_error: Arc::new(Mutex::new(false)),
            pid: None,
            span,
        }
    }

    /// The process id, if this child was spawned by the shell.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn ignore_error(&mut self, ignore: bool) -> &mut Self {
        {
            let mut ignore_error = self.ignore_error.lock().expect("lock should success");
            *ignore_error = ignore;
        }
        self
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn into_bytes(mut self) -> Result<Vec<u8>, ShellError> {
        if self.stderr.is_some() {
            debug_assert!(false, "stderr should not exist");
            return Err(ShellError::Generic(GenericError::new(
                "internal error",
                "stderr should not exist",
                self.span,
            )));
        }

        let bytes = (self.stdout.take())
            .map(collect_bytes)
            .transpose()
            .map_err(|err| IoError::new(err, self.span, None))?
            .unwrap_or_default();

        self.check_exit()?;

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
                        Err(err) => ShellError::Generic(GenericError::new(
                            "Unknown error",
                            format!("{err:?}"),
                            self.span,
                        )),
                    })?
                    .map_err(&from_io_error)?;
            }

            res.map_err(&from_io_error)?;
        } else if let Some(stderr) = self.stderr.take() {
            consume_pipe(stderr).map_err(&from_io_error)?;
        }
        self.check_exit()
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, ShellError> {
        let mut exit_status = self
            .exit_status
            .lock()
            .expect("lock exit_status future should success");
        exit_status.try_wait(self.span)
    }

    pub fn wait_with_output(self) -> Result<ProcessOutput, ShellError> {
        let (stdout, stderr) = collect_pipes(self.stdout, self.stderr, self.span)?;

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

    /// Wait for the process without reading its output, and check its exit status like
    /// [`wait`](Self::wait) does.
    ///
    /// The pipes are read ahead into memory meanwhile, so the process can't block on a full
    /// pipe, and they read the same as before afterwards.
    pub fn wait_keeping_output(&mut self) -> Result<(), ShellError> {
        self.read_pipes_ahead();
        self.check_exit()
    }

    fn read_pipes_ahead(&mut self) {
        for pipe in [&mut self.stdout, &mut self.stderr].into_iter().flatten() {
            pipe.start_read_ahead();
        }
    }

    /// Turn the pipes into [`ReadAheadPipe`]s and get their handles, for an [`ExitStatusGuard`].
    pub fn read_ahead_handles(&mut self) -> Vec<ReadAheadHandle> {
        [&mut self.stdout, &mut self.stderr]
            .into_iter()
            .flatten()
            .map(ChildPipe::read_ahead_handle)
            .collect()
    }

    /// Wait for the process, then return an error if it failed, unless its errors are ignored.
    fn check_exit(&self) -> Result<(), ShellError> {
        let ignore_error = *self
            .ignore_error
            .lock()
            .expect("lock ignore error should success");
        let mut exit_status = self
            .exit_status
            .lock()
            .expect("lock exit_status future should success");
        check_ok(exit_status.wait(self.span)?, ignore_error, self.span)
    }

    pub fn clone_exit_status_future(&self) -> Arc<Mutex<ExitStatusFuture>> {
        self.exit_status.clone()
    }

    pub fn clone_ignore_error(&self) -> Arc<Mutex<bool>> {
        self.ignore_error.clone()
    }
}

/// The bytes read from a child's stdout and stderr, `None` for a pipe it doesn't have.
type PipeBytes = (Option<Vec<u8>>, Option<Vec<u8>>);

/// Read both pipes to the end at the same time, so a child that fills one pipe while the
/// other is being read can't block.
fn collect_pipes(
    stdout: Option<ChildPipe>,
    stderr: Option<ChildPipe>,
    span: Span,
) -> Result<PipeBytes, ShellError> {
    let from_io_error = IoError::factory(span, None);

    Ok(match (stdout, stderr) {
        (None, None) => (None, None),
        (None, Some(stderr)) => (None, Some(collect_bytes(stderr).map_err(&from_io_error)?)),
        (Some(stdout), None) => (Some(collect_bytes(stdout).map_err(&from_io_error)?), None),
        (Some(stdout), Some(stderr)) => {
            let stderr = thread::Builder::new()
                .spawn(move || collect_bytes(stderr))
                .map_err(&from_io_error)?;

            let stdout = collect_bytes(stdout).map_err(&from_io_error)?;

            let stderr = stderr
                .join()
                .map_err(|e| match e.downcast::<io::Error>() {
                    Ok(io) => from_io_error(*io).into(),
                    Err(err) => ShellError::Generic(GenericError::new(
                        "Unknown error",
                        format!("{err:?}"),
                        span,
                    )),
                })?
                .map_err(&from_io_error)?;

            (Some(stdout), Some(stderr))
        }
    })
}

fn collect_bytes(pipe: ChildPipe) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    match pipe {
        ChildPipe::Pipe(mut pipe) => pipe.read_to_end(&mut buf),
        ChildPipe::Tee(mut tee) => tee.read_to_end(&mut buf),
        ChildPipe::ReadAhead(mut pipe) => pipe.read_to_end(&mut buf),
    }?;
    Ok(buf)
}

fn consume_pipe(pipe: ChildPipe) -> io::Result<()> {
    match pipe {
        ChildPipe::Pipe(mut pipe) => io::copy(&mut pipe, &mut io::sink()),
        ChildPipe::Tee(mut tee) => io::copy(&mut tee, &mut io::sink()),
        ChildPipe::ReadAhead(mut pipe) => io::copy(&mut pipe, &mut io::sink()),
    }?;
    Ok(())
}

pub struct ProcessOutput {
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub exit_status: ExitStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A pipe holding `bytes`, with the write end closed.
    fn pipe_with(bytes: &[u8]) -> PipeReader {
        let (reader, mut writer) = os_pipe::pipe().expect("create pipe");
        writer.write_all(bytes).expect("write pipe");
        reader
    }

    #[test]
    fn read_ahead_lets_the_writer_finish_and_keeps_the_order() {
        let (reader, mut writer) = os_pipe::pipe().expect("create pipe");
        // Bigger than a pipe's buffer, so the writer only finishes if something reads.
        let data: Vec<u8> = (0..300_000).map(|i| (i % 251) as u8).collect();
        let written = data.clone();
        let writer = thread::spawn(move || writer.write_all(&written));

        let mut pipe = ChildPipe::Pipe(reader);
        let handle = pipe.read_ahead_handle();
        let mut head = [0; 10];
        pipe.read_exact(&mut head)
            .expect("read before reading ahead");
        handle.start();
        writer
            .join()
            .expect("writer thread")
            .expect("write the whole pipe");

        let mut rest = Vec::new();
        pipe.read_to_end(&mut rest).expect("read the rest");
        assert_eq!([&head[..], &rest[..]].concat(), data);
    }

    #[test]
    fn read_ahead_pipe_hands_the_os_pipe_over_before_reading_ahead() {
        let mut pipe = ChildPipe::Pipe(pipe_with(b"out"));
        pipe.read_ahead_handle();
        let ChildPipe::ReadAhead(pipe) = pipe else {
            panic!("not a read-ahead pipe");
        };
        assert!(pipe.into_pipe().is_ok());
    }
}
