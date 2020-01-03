use crate::commands::command::{EvaluatedWholeStreamCommandArgs, RunnablePerItemContext};
use crate::commands::cp::CopyArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::prelude::*;
use crate::shell::filesystem_shell::FilesystemShell;
use crate::shell::shell::Shell;
use crate::stream::OutputStream;
use nu_errors::ShellError;
use nu_source::Tagged;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ShellManager {
    pub(crate) current_shell: Arc<AtomicUsize>,
    pub(crate) shells: Arc<Mutex<Vec<Box<dyn Shell + Send>>>>,
}

impl ShellManager {
    pub fn basic(commands: CommandRegistry) -> Result<ShellManager, Box<dyn Error>> {
        Ok(ShellManager {
            current_shell: Arc::new(AtomicUsize::new(0)),
            shells: Arc::new(Mutex::new(vec![Box::new(FilesystemShell::basic(
                commands,
            )?)])),
        })
    }

    pub fn insert_at_current(&mut self, shell: Box<dyn Shell + Send>) -> Result<(), ShellError> {
        if let Ok(mut shells) = self.shells.lock() {
            shells.push(shell);
        } else {
            return Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer",
            ));
        }

        let shells_len = if let Ok(shells) = self.shells.lock() {
            shells.len()
        } else {
            return Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer",
            ));
        };

        self.current_shell.store(shells_len - 1, Ordering::SeqCst);
        self.set_path(self.path()?)
    }

    pub fn current_shell(&self) -> usize {
        self.current_shell.load(Ordering::SeqCst)
    }

    pub fn remove_at_current(&mut self) -> Result<(), ShellError> {
        {
            if let Ok(mut shells) = self.shells.lock() {
                if shells.len() > 0 {
                    if self.current_shell() == shells.len() - 1 {
                        shells.pop();
                        let new_len = shells.len();
                        if new_len > 0 {
                            self.current_shell.store(new_len - 1, Ordering::SeqCst);
                        } else {
                            return Ok(());
                        }
                    } else {
                        shells.remove(self.current_shell());
                    }
                }
            } else {
                return Err(ShellError::untagged_runtime_error(
                    "Internal error: could not lock shells ring buffer",
                ));
            }
        }
        self.set_path(self.path()?)
    }

    pub fn is_empty(&self) -> Result<bool, ShellError> {
        if let Ok(shells) = self.shells.lock() {
            Ok(shells.is_empty())
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (is_empty)",
            ))
        }
    }

    pub fn path(&self) -> Result<String, ShellError> {
        if let Ok(shells) = self.shells.lock() {
            Ok(shells[self.current_shell()].path())
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (path)",
            ))
        }
    }

    pub fn pwd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        if let Ok(shells) = self.shells.lock() {
            shells[self.current_shell()].pwd(args)
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (pwd)",
            ))
        }
    }

    pub fn set_path(&mut self, path: String) -> Result<(), ShellError> {
        if let Ok(mut shells) = self.shells.lock() {
            shells[self.current_shell()].set_path(path);
            Ok(())
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (set_path)",
            ))
        }
    }

    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError> {
        if let Ok(shells) = self.shells.lock() {
            shells[self.current_shell()].complete(line, pos, ctx)
        } else {
            Err(rustyline::error::ReadlineError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Internal error: could not lock shells ring buffer (complete)",
            )))
        }
    }

    pub fn hint(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<Option<String>, ShellError> {
        if let Ok(shells) = self.shells.lock() {
            Ok(shells[self.current_shell()].hint(line, pos, ctx))
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (hint)",
            ))
        }
    }

    pub fn next(&mut self) -> Result<(), ShellError> {
        {
            if let Ok(shells) = self.shells.lock() {
                let shell_len = shells.len();
                if self.current_shell() == (shell_len - 1) {
                    self.current_shell.store(0, Ordering::SeqCst);
                } else {
                    self.current_shell
                        .store(self.current_shell() + 1, Ordering::SeqCst);
                }
            } else {
                return Err(ShellError::untagged_runtime_error(
                    "Internal error: could not lock shells ring buffer (next)",
                ));
            }
        }
        self.set_path(self.path()?)
    }

    pub fn prev(&mut self) -> Result<(), ShellError> {
        {
            if let Ok(shells) = self.shells.lock() {
                let shell_len = shells.len();
                if self.current_shell() == 0 {
                    self.current_shell.store(shell_len - 1, Ordering::SeqCst);
                } else {
                    self.current_shell
                        .store(self.current_shell() - 1, Ordering::SeqCst);
                }
            } else {
                return Err(ShellError::untagged_runtime_error(
                    "Internal error: could not lock shells ring buffer (prev)",
                ));
            }
        }
        self.set_path(self.path()?)
    }

    pub fn homedir(&self) -> Result<Option<PathBuf>, ShellError> {
        if let Ok(shells) = self.shells.lock() {
            Ok(shells[self.current_shell()].homedir())
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (homedir)",
            ))
        }
    }

    pub fn ls(
        &self,
        path: Option<Tagged<PathBuf>>,
        context: &RunnableContext,
        full: bool,
    ) -> Result<OutputStream, ShellError> {
        if let Ok(shells) = self.shells.lock() {
            shells[self.current_shell()].ls(path, context, full)
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (ls)",
            ))
        }
    }

    pub fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        if let Ok(shells) = self.shells.lock() {
            shells[self.current_shell()].cd(args)
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock shells ring buffer (cd)",
            ))
        }
    }

    pub fn cp(
        &self,
        args: CopyArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        match shells {
            Ok(x) => {
                let path = x[self.current_shell()].path();
                x[self.current_shell()].cp(args, context.name.clone(), &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                &context.name,
            )),
        }
    }

    pub fn rm(
        &self,
        args: RemoveArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        match shells {
            Ok(x) => {
                let path = x[self.current_shell()].path();
                x[self.current_shell()].rm(args, context.name.clone(), &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                &context.name,
            )),
        }
    }

    pub fn mkdir(
        &self,
        args: MkdirArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        match shells {
            Ok(x) => {
                let path = x[self.current_shell()].path();
                x[self.current_shell()].mkdir(args, context.name.clone(), &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                &context.name,
            )),
        }
    }

    pub fn mv(
        &self,
        args: MoveArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        match shells {
            Ok(x) => {
                let path = x[self.current_shell()].path();
                x[self.current_shell()].mv(args, context.name.clone(), &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                &context.name,
            )),
        }
    }
}
