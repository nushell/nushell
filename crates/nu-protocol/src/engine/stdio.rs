use std::{
    fs::File,
    mem,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::IoStream;

use super::Stack;

#[derive(Debug, Clone)]
pub enum EvaluatedRedirection {
    /// A pipe redirection.
    ///
    /// This will only affect the last command of a block.
    /// This is created by pipes and pipe redirections (`|`, `e>|`, `o+e>|`, etc.),
    /// or set by the next command in the pipeline (e.g., `ignore` sets stdout to [`IoStream::Null`]).
    Pipe(IoStream),
    /// A file redirection.
    ///
    /// This will affect all commands in the block.
    /// This is only created by file redirections (`o>`, `e>`, `o+e>`, etc.).
    File(Arc<File>),
}

impl EvaluatedRedirection {
    pub fn file(file: File) -> Self {
        Self::File(Arc::new(file))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StackStdio {
    /// The stream to use for the next command's stdout
    pub pipe_stdout: Option<IoStream>,
    /// The stream to use for the next command's stderr
    pub pipe_stderr: Option<IoStream>,
    /// The stream used for the command stdout if `pipe_stdout` is `None`
    ///
    /// This should only ever be `File` or `Inherit`.
    pub stdout: IoStream,
    /// The stream used for the command stderr if `pipe_stderr` is `None`
    ///
    /// This should only ever be `File` or `Inherit`.
    pub stderr: IoStream,
    /// The previous stdout used before the current `stdout` was set
    ///
    /// This is used only when evaluating arguments to commands,
    /// since the arguments are lazily evaluated inside each command
    /// after redirections have already been applied to the command/stack.
    ///
    /// This should only ever be `File` or `Inherit`.
    pub parent_stdout: Option<IoStream>,
    /// The previous stderr used before the current `stderr` was set
    ///
    /// This is used only when evaluating arguments to commands,
    /// since the arguments are lazily evaluated inside each command
    /// after redirections have already been applied to the command/stack.
    ///
    /// This should only ever be `File` or `Inherit`.
    pub parent_stderr: Option<IoStream>,
}

impl StackStdio {
    pub(crate) fn new() -> Self {
        Self {
            pipe_stdout: None,
            pipe_stderr: None,
            stdout: IoStream::Inherit,
            stderr: IoStream::Inherit,
            parent_stdout: None,
            parent_stderr: None,
        }
    }

    /// Returns the [`IoStream`] to use for current command's stdout.
    ///
    /// This will be the pipe redirection if one is set,
    /// otherwise it will be the current file redirection,
    /// otherwise it will be the process's stdout indicated by [`IoStream::Inherit`].
    pub(crate) fn stdout(&self) -> &IoStream {
        self.pipe_stdout.as_ref().unwrap_or(&self.stdout)
    }

    /// Returns the [`IoStream`] to use for current command's stderr.
    ///
    /// This will be the pipe redirection if one is set,
    /// otherwise it will be the current file redirection,
    /// otherwise it will be the process's stderr indicated by [`IoStream::Inherit`].
    pub(crate) fn stderr(&self) -> &IoStream {
        self.pipe_stderr.as_ref().unwrap_or(&self.stderr)
    }

    fn push_stdout(&mut self, stdout: IoStream) -> Option<IoStream> {
        let stdout = mem::replace(&mut self.stdout, stdout);
        mem::replace(&mut self.parent_stdout, Some(stdout))
    }

    fn push_stderr(&mut self, stderr: IoStream) -> Option<IoStream> {
        let stderr = mem::replace(&mut self.stderr, stderr);
        mem::replace(&mut self.parent_stderr, Some(stderr))
    }
}

pub struct StackIoGuard<'a> {
    stack: &'a mut Stack,
    old_pipe_stdout: Option<IoStream>,
    old_pipe_stderr: Option<IoStream>,
    old_parent_stdout: Option<IoStream>,
    old_parent_stderr: Option<IoStream>,
}

impl<'a> StackIoGuard<'a> {
    pub(crate) fn new(
        stack: &'a mut Stack,
        stdout: Option<EvaluatedRedirection>,
        stderr: Option<EvaluatedRedirection>,
    ) -> Self {
        let stdio = &mut stack.stdio;

        let (old_pipe_stdout, old_parent_stdout) = match stdout {
            Some(EvaluatedRedirection::Pipe(stdout)) => {
                let old = mem::replace(&mut stdio.pipe_stdout, Some(stdout));
                (old, stdio.parent_stdout.take())
            }
            Some(EvaluatedRedirection::File(file)) => {
                let file = IoStream::from(file);
                (
                    mem::replace(&mut stdio.pipe_stdout, Some(file.clone())),
                    stdio.push_stdout(file),
                )
            }
            None => (stdio.pipe_stdout.take(), stdio.parent_stdout.take()),
        };

        let (old_pipe_stderr, old_parent_stderr) = match stderr {
            Some(EvaluatedRedirection::Pipe(stderr)) => {
                let old = mem::replace(&mut stdio.pipe_stderr, Some(stderr));
                (old, stdio.parent_stderr.take())
            }
            Some(EvaluatedRedirection::File(file)) => {
                (stdio.pipe_stderr.take(), stdio.push_stderr(file.into()))
            }
            None => (stdio.pipe_stderr.take(), stdio.parent_stderr.take()),
        };

        StackIoGuard {
            stack,
            old_pipe_stdout,
            old_parent_stdout,
            old_pipe_stderr,
            old_parent_stderr,
        }
    }
}

impl<'a> Deref for StackIoGuard<'a> {
    type Target = Stack;

    fn deref(&self) -> &Self::Target {
        self.stack
    }
}

impl<'a> DerefMut for StackIoGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stack
    }
}

impl Drop for StackIoGuard<'_> {
    fn drop(&mut self) {
        self.stdio.pipe_stdout = self.old_pipe_stdout.take();
        self.stdio.pipe_stderr = self.old_pipe_stderr.take();

        let old_stdout = self.old_parent_stdout.take();
        if let Some(stdout) = mem::replace(&mut self.stdio.parent_stdout, old_stdout) {
            self.stdio.stdout = stdout;
        }

        let old_stderr = self.old_parent_stderr.take();
        if let Some(stderr) = mem::replace(&mut self.stdio.parent_stderr, old_stderr) {
            self.stdio.stderr = stderr;
        }
    }
}

pub struct StackCaptureGuard<'a> {
    stack: &'a mut Stack,
    old_pipe_stdout: Option<IoStream>,
}

impl<'a> StackCaptureGuard<'a> {
    pub(crate) fn new(stack: &'a mut Stack) -> Self {
        let old_pipe_stdout = mem::replace(&mut stack.stdio.pipe_stdout, Some(IoStream::Capture));
        Self {
            stack,
            old_pipe_stdout,
        }
    }
}

impl<'a> Deref for StackCaptureGuard<'a> {
    type Target = Stack;

    fn deref(&self) -> &Self::Target {
        &*self.stack
    }
}

impl<'a> DerefMut for StackCaptureGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stack
    }
}

impl Drop for StackCaptureGuard<'_> {
    fn drop(&mut self) {
        self.stdio.pipe_stdout = self.old_pipe_stdout.take();
    }
}

pub struct StackCallArgGuard<'a> {
    stack: &'a mut Stack,
    old_pipe_stdout: Option<IoStream>,
    old_pipe_stderr: Option<IoStream>,
    old_stdout: Option<IoStream>,
    old_stderr: Option<IoStream>,
}

impl<'a> StackCallArgGuard<'a> {
    pub(crate) fn new(stack: &'a mut Stack) -> Self {
        let old_pipe_stdout = mem::replace(&mut stack.stdio.pipe_stdout, Some(IoStream::Capture));
        let old_pipe_stderr = stack.stdio.pipe_stderr.take();

        let old_stdout = stack
            .stdio
            .parent_stdout
            .take()
            .map(|stdout| mem::replace(&mut stack.stdio.stdout, stdout));

        let old_stderr = stack
            .stdio
            .parent_stderr
            .take()
            .map(|stderr| mem::replace(&mut stack.stdio.stderr, stderr));

        Self {
            stack,
            old_pipe_stdout,
            old_pipe_stderr,
            old_stdout,
            old_stderr,
        }
    }
}

impl<'a> Deref for StackCallArgGuard<'a> {
    type Target = Stack;

    fn deref(&self) -> &Self::Target {
        &*self.stack
    }
}

impl<'a> DerefMut for StackCallArgGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stack
    }
}

impl Drop for StackCallArgGuard<'_> {
    fn drop(&mut self) {
        self.stdio.pipe_stdout = self.old_pipe_stdout.take();
        self.stdio.pipe_stderr = self.old_pipe_stderr.take();
        if let Some(stdout) = self.old_stdout.take() {
            self.stdio.push_stdout(stdout);
        }
        if let Some(stderr) = self.old_stderr.take() {
            self.stdio.push_stderr(stderr);
        }
    }
}
