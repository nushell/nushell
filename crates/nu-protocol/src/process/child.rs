use crate::{io::convert_file, process::ExitStatus, ErrSpan, IntoSpanned, ShellError, Span, Value};
use nu_system::ForegroundChild;
use os_pipe::PipeReader;
use std::{
    fmt::Debug,
    io::{self, BufReader, Read},
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Receiver, RecvError, TryRecvError},
        Arc,
    },
    thread,
};

#[derive(Debug)]
enum ExitStatusFuture {
    Finished(Result<ExitStatus, Box<ShellError>>),
    Running(Receiver<io::Result<ExitStatus>>),
}

impl ExitStatusFuture {
    fn wait(&mut self, span: Span) -> Result<ExitStatus, ShellError> {
        match self {
            ExitStatusFuture::Finished(Ok(status)) => Ok(*status),
            ExitStatusFuture::Finished(Err(err)) => Err(err.as_ref().clone()),
            ExitStatusFuture::Running(receiver) => {
                let code = match receiver.recv() {
                    Ok(Ok(status)) => Ok(status),
                    Ok(Err(err)) => Err(ShellError::IOErrorSpanned {
                        msg: format!("failed to get exit code: {err:?}"),
                        span,
                    }),
                    Err(RecvError) => Err(ShellError::IOErrorSpanned {
                        msg: "failed to get exit code".into(),
                        span,
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
                    Ok(Err(err)) => Err(ShellError::IOErrorSpanned {
                        msg: format!("failed to get exit code: {err:?}"),
                        span,
                    }),
                    Err(TryRecvError::Disconnected) => Err(ShellError::IOErrorSpanned {
                        msg: "failed to get exit code".into(),
                        span,
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
    exit_status: ExitStatusFuture,
    span: Span,
    trim_end_newline: bool,
}

impl ChildProcess {
    pub fn new(
        mut child: ForegroundChild,
        reader: Option<PipeReader>,
        swap: bool,
        span: Span,
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
            .spawn(move || exit_status_sender.send(child.wait().map(Into::into)))
            .err_span(span)?;

        Ok(Self::from_raw(stdout, stderr, exit_status, span))
    }

    pub fn from_raw(
        stdout: Option<PipeReader>,
        stderr: Option<PipeReader>,
        exit_status: Receiver<io::Result<ExitStatus>>,
        span: Span,
    ) -> Self {
        Self {
            stdout: stdout.map(Into::into),
            stderr: stderr.map(Into::into),
            exit_status: ExitStatusFuture::Running(exit_status),
            span,
            trim_end_newline: false,
        }
    }

    pub fn trim_end_newline(mut self, trim: bool) -> Self {
        self.trim_end_newline = trim;
        self
    }

    pub fn set_exit_code(&mut self, exit_code: i32) {
        self.exit_status = ExitStatusFuture::Finished(Ok(ExitStatus::Exited(exit_code)));
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn lines(self, ctrlc: Option<Arc<AtomicBool>>) -> Result<Option<Lines>, ShellError> {
        if self.stderr.is_some() {
            debug_assert!(false, "stderr should not exist");
            return Err(ShellError::IOErrorSpanned {
                msg: "internal error".into(),
                span: self.span,
            });
        }
        if let Some(stdout) = self.stdout {
            Ok(Some(Lines(crate::io::Lines::new(
                BufReader::new(stdout),
                self.span,
                ctrlc,
            ))))
        } else {
            Ok(None)
        }
    }

    pub fn values(self, ctrlc: Option<Arc<AtomicBool>>) -> Result<Option<Values>, ShellError> {
        if self.stderr.is_some() {
            debug_assert!(false, "stderr should not exist");
            return Err(ShellError::IOErrorSpanned {
                msg: "internal error".into(),
                span: self.span,
            });
        }
        if let Some(stdout) = self.stdout {
            Ok(Some(Values(crate::io::Values::new(
                BufReader::new(stdout),
                self.span,
                ctrlc,
            ))))
        } else {
            Ok(None)
        }
    }

    pub fn into_bytes(mut self) -> Result<Vec<u8>, ShellError> {
        // todo!() trim end newline?

        if self.stderr.is_some() {
            debug_assert!(false, "stderr should not exist");
            return Err(ShellError::IOErrorSpanned {
                msg: "internal error".into(),
                span: self.span,
            });
        }

        let bytes = if let Some(stdout) = self.stdout {
            collect_bytes(stdout).err_span(self.span)?
        } else {
            Vec::new()
        };

        self.exit_status.wait(self.span)?.check_ok(self.span)?;

        Ok(bytes)
    }

    pub fn into_string(self) -> Result<String, ShellError> {
        // todo!() trim end newline?

        let span = self.span;
        String::from_utf8(self.into_bytes()?).map_err(|_| ShellError::NonUtf8 { span })
    }

    pub fn into_value(self) -> Result<Value, ShellError> {
        // todo!() trim end newline?

        let span = self.span;
        let value = match String::from_utf8(self.into_bytes()?) {
            Ok(str) => Value::string(str, span),
            Err(err) => Value::binary(err.into_bytes(), span),
        };
        Ok(value)
    }

    pub fn wait(mut self) -> Result<ExitStatus, ShellError> {
        if let Some(stdout) = self.stdout.take() {
            let stderr = self
                .stderr
                .take()
                .map(|stderr| thread::Builder::new().spawn(move || consume_pipe(stderr)))
                .transpose()
                .err_span(self.span)?;

            let res = consume_pipe(stdout);

            if let Some(handle) = stderr {
                handle
                    .join()
                    .map_err(|e| match e.downcast::<io::Error>() {
                        Ok(io) => ShellError::from((*io).into_spanned(self.span)),
                        Err(err) => ShellError::GenericError {
                            error: "Unknown error".into(),
                            msg: format!("{err:?}"),
                            span: Some(self.span),
                            help: None,
                            inner: Vec::new(),
                        },
                    })?
                    .err_span(self.span)?;
            }

            res.err_span(self.span)?;
        } else if let Some(stderr) = self.stderr.take() {
            consume_pipe(stderr).err_span(self.span)?;
        }

        self.exit_status.wait(self.span)
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, ShellError> {
        self.exit_status.try_wait(self.span)
    }

    pub fn wait_with_output(mut self) -> Result<ProcessOutput, ShellError> {
        let (stdout, stderr) = if let Some(stdout) = self.stdout {
            let stderr = self
                .stderr
                .map(|stderr| thread::Builder::new().spawn(move || collect_bytes(stderr)))
                .transpose()
                .err_span(self.span)?;

            let stdout = collect_bytes(stdout).err_span(self.span)?;

            let stderr = stderr
                .map(|handle| {
                    handle.join().map_err(|e| match e.downcast::<io::Error>() {
                        Ok(io) => ShellError::from((*io).into_spanned(self.span)),
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
                .err_span(self.span)?;

            (Some(stdout), stderr)
        } else {
            let stderr = self
                .stderr
                .map(collect_bytes)
                .transpose()
                .err_span(self.span)?;

            (None, stderr)
        };

        let exit_status = self.exit_status.wait(self.span)?;

        Ok(ProcessOutput {
            stdout,
            stderr,
            exit_status,
        })
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

pub struct Lines(crate::io::Lines<BufReader<ChildPipe>>);

impl Iterator for Lines {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct Values(crate::io::Values<BufReader<ChildPipe>>);

impl Iterator for Values {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
