use crate::{engine::Stack, Stdoe};
use std::{
    fs::File,
    mem,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub enum Redirection {
    /// A pipe redirection.
    ///
    /// This will only affect the last command of a block.
    /// This is created by pipes and pipe redirections (`|`, `e>|`, `o+e>|`, etc.),
    /// or set by the next command in the pipeline (e.g., `ignore` sets stdout to [`Stdoe::Null`]).
    Pipe(Stdoe),
    /// A file redirection.
    ///
    /// This will affect all commands in the block.
    /// This is only created by file redirections (`o>`, `e>`, `o+e>`, etc.).
    File(Arc<File>),
}

impl Redirection {
    pub fn file(file: File) -> Self {
        Self::File(Arc::new(file))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StackStdoe {
    /// The stream to use for the next command's stdout.
    pub pipe_stdout: Option<Stdoe>,
    /// The stream to use for the next command's stderr.
    pub pipe_stderr: Option<Stdoe>,
    /// The stream used for the command stdout if `pipe_stdout` is `None`.
    ///
    /// This should only ever be `File` or `Inherit`.
    pub stdout: Stdoe,
    /// The stream used for the command stderr if `pipe_stderr` is `None`.
    ///
    /// This should only ever be `File` or `Inherit`.
    pub stderr: Stdoe,
    /// The previous stdout used before the current `stdout` was set.
    ///
    /// This is used only when evaluating arguments to commands,
    /// since the arguments are lazily evaluated inside each command
    /// after redirections have already been applied to the command/stack.
    ///
    /// This should only ever be `File` or `Inherit`.
    pub parent_stdout: Option<Stdoe>,
    /// The previous stderr used before the current `stderr` was set.
    ///
    /// This is used only when evaluating arguments to commands,
    /// since the arguments are lazily evaluated inside each command
    /// after redirections have already been applied to the command/stack.
    ///
    /// This should only ever be `File` or `Inherit`.
    pub parent_stderr: Option<Stdoe>,
}

impl StackStdoe {
    pub(crate) fn new() -> Self {
        Self {
            pipe_stdout: None,
            pipe_stderr: None,
            stdout: Stdoe::Inherit,
            stderr: Stdoe::Inherit,
            parent_stdout: None,
            parent_stderr: None,
        }
    }

    /// Returns the [`Stdoe`] to use for current command's stdout.
    ///
    /// This will be the pipe redirection if one is set,
    /// otherwise it will be the current file redirection,
    /// otherwise it will be the process's stdout indicated by [`Stdoe::Inherit`].
    pub(crate) fn stdout(&self) -> &Stdoe {
        self.pipe_stdout.as_ref().unwrap_or(&self.stdout)
    }

    /// Returns the [`Stdoe`] to use for current command's stderr.
    ///
    /// This will be the pipe redirection if one is set,
    /// otherwise it will be the current file redirection,
    /// otherwise it will be the process's stderr indicated by [`Stdoe::Inherit`].
    pub(crate) fn stderr(&self) -> &Stdoe {
        self.pipe_stderr.as_ref().unwrap_or(&self.stderr)
    }

    fn push_stdout(&mut self, stdout: Stdoe) -> Option<Stdoe> {
        let stdout = mem::replace(&mut self.stdout, stdout);
        mem::replace(&mut self.parent_stdout, Some(stdout))
    }

    fn push_stderr(&mut self, stderr: Stdoe) -> Option<Stdoe> {
        let stderr = mem::replace(&mut self.stderr, stderr);
        mem::replace(&mut self.parent_stderr, Some(stderr))
    }
}

pub struct StackIoGuard<'a> {
    stack: &'a mut Stack,
    old_pipe_stdout: Option<Stdoe>,
    old_pipe_stderr: Option<Stdoe>,
    old_parent_stdout: Option<Stdoe>,
    old_parent_stderr: Option<Stdoe>,
}

impl<'a> StackIoGuard<'a> {
    pub(crate) fn new(
        stack: &'a mut Stack,
        stdout: Option<Redirection>,
        stderr: Option<Redirection>,
    ) -> Self {
        let stdoe = &mut stack.stdoe;

        let (old_pipe_stdout, old_parent_stdout) = match stdout {
            Some(Redirection::Pipe(stdout)) => {
                let old = mem::replace(&mut stdoe.pipe_stdout, Some(stdout));
                (old, stdoe.parent_stdout.take())
            }
            Some(Redirection::File(file)) => {
                let file = Stdoe::from(file);
                (
                    mem::replace(&mut stdoe.pipe_stdout, Some(file.clone())),
                    stdoe.push_stdout(file),
                )
            }
            None => (stdoe.pipe_stdout.take(), stdoe.parent_stdout.take()),
        };

        let (old_pipe_stderr, old_parent_stderr) = match stderr {
            Some(Redirection::Pipe(stderr)) => {
                let old = mem::replace(&mut stdoe.pipe_stderr, Some(stderr));
                (old, stdoe.parent_stderr.take())
            }
            Some(Redirection::File(file)) => {
                (stdoe.pipe_stderr.take(), stdoe.push_stderr(file.into()))
            }
            None => (stdoe.pipe_stderr.take(), stdoe.parent_stderr.take()),
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
        self.stdoe.pipe_stdout = self.old_pipe_stdout.take();
        self.stdoe.pipe_stderr = self.old_pipe_stderr.take();

        let old_stdout = self.old_parent_stdout.take();
        if let Some(stdout) = mem::replace(&mut self.stdoe.parent_stdout, old_stdout) {
            self.stdoe.stdout = stdout;
        }

        let old_stderr = self.old_parent_stderr.take();
        if let Some(stderr) = mem::replace(&mut self.stdoe.parent_stderr, old_stderr) {
            self.stdoe.stderr = stderr;
        }
    }
}

pub struct StackCaptureGuard<'a> {
    stack: &'a mut Stack,
    old_pipe_stdout: Option<Stdoe>,
    old_pipe_stderr: Option<Stdoe>,
}

impl<'a> StackCaptureGuard<'a> {
    pub(crate) fn new(stack: &'a mut Stack) -> Self {
        let old_pipe_stdout = mem::replace(&mut stack.stdoe.pipe_stdout, Some(Stdoe::Capture));
        let old_pipe_stderr = stack.stdoe.pipe_stderr.take();
        Self {
            stack,
            old_pipe_stdout,
            old_pipe_stderr,
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
        self.stdoe.pipe_stdout = self.old_pipe_stdout.take();
        self.stdoe.pipe_stderr = self.old_pipe_stderr.take();
    }
}

pub struct StackCallArgGuard<'a> {
    stack: &'a mut Stack,
    old_pipe_stdout: Option<Stdoe>,
    old_pipe_stderr: Option<Stdoe>,
    old_stdout: Option<Stdoe>,
    old_stderr: Option<Stdoe>,
}

impl<'a> StackCallArgGuard<'a> {
    pub(crate) fn new(stack: &'a mut Stack) -> Self {
        let old_pipe_stdout = mem::replace(&mut stack.stdoe.pipe_stdout, Some(Stdoe::Capture));
        let old_pipe_stderr = stack.stdoe.pipe_stderr.take();

        let old_stdout = stack
            .stdoe
            .parent_stdout
            .take()
            .map(|stdout| mem::replace(&mut stack.stdoe.stdout, stdout));

        let old_stderr = stack
            .stdoe
            .parent_stderr
            .take()
            .map(|stderr| mem::replace(&mut stack.stdoe.stderr, stderr));

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
        self.stdoe.pipe_stdout = self.old_pipe_stdout.take();
        self.stdoe.pipe_stderr = self.old_pipe_stderr.take();
        if let Some(stdout) = self.old_stdout.take() {
            self.stdoe.push_stdout(stdout);
        }
        if let Some(stderr) = self.old_stderr.take() {
            self.stdoe.push_stderr(stderr);
        }
    }
}
