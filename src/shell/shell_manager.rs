use crate::commands::command::{EvaluatedWholeStreamCommandArgs, RunnablePerItemContext};
use crate::commands::cp::CopyArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::errors::ShellError;
use crate::prelude::*;
use crate::shell::filesystem_shell::FilesystemShell;
use crate::shell::shell::Shell;
use crate::stream::OutputStream;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ShellManager {
    crate current_shell: usize,
    crate shells: Arc<Mutex<Vec<Box<dyn Shell + Send>>>>,
}

impl ShellManager {
    pub fn basic(commands: CommandRegistry) -> Result<ShellManager, Box<dyn Error>> {
        Ok(ShellManager {
            current_shell: 0,
            shells: Arc::new(Mutex::new(vec![Box::new(FilesystemShell::basic(
                commands,
            )?)])),
        })
    }

    pub fn insert_at_current(&mut self, shell: Box<dyn Shell + Send>) {
        self.shells.lock().unwrap().push(shell);
        self.current_shell = self.shells.lock().unwrap().len() - 1;
        self.set_path(self.path());
    }

    pub fn remove_at_current(&mut self) {
        {
            let mut shells = self.shells.lock().unwrap();
            if shells.len() > 0 {
                if self.current_shell == shells.len() - 1 {
                    shells.pop();
                    let new_len = shells.len();
                    if new_len > 0 {
                        self.current_shell = new_len - 1;
                    } else {
                        return;
                    }
                } else {
                    shells.remove(self.current_shell);
                }
            }
        }
        self.set_path(self.path());
    }

    pub fn is_empty(&self) -> bool {
        self.shells.lock().unwrap().is_empty()
    }

    pub fn path(&self) -> String {
        self.shells.lock().unwrap()[self.current_shell].path()
    }

    pub fn set_path(&mut self, path: String) {
        self.shells.lock().unwrap()[self.current_shell].set_path(path)
    }

    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError> {
        self.shells.lock().unwrap()[self.current_shell].complete(line, pos, ctx)
    }

    pub fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.shells.lock().unwrap()[self.current_shell].hint(line, pos, ctx)
    }

    pub fn next(&mut self) {
        {
            let shell_len = self.shells.lock().unwrap().len();
            if self.current_shell == (shell_len - 1) {
                self.current_shell = 0;
            } else {
                self.current_shell += 1;
            }
        }
        self.set_path(self.path());
    }

    pub fn prev(&mut self) {
        {
            let shell_len = self.shells.lock().unwrap().len();
            if self.current_shell == 0 {
                self.current_shell = shell_len - 1;
            } else {
                self.current_shell -= 1;
            }
        }
        self.set_path(self.path());
    }

    pub fn ls(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock().unwrap();

        env[self.current_shell].ls(args)
    }
    pub fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock().unwrap();

        env[self.current_shell].cd(args)
    }
    pub fn cp(
        &self,
        args: CopyArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock();

        match env {
            Ok(x) => {
                let path = x[self.current_shell].path();
                x[self.current_shell].cp(args, context.name, &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                context.name,
            )),
        }
    }

    pub fn rm(
        &self,
        args: RemoveArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock();

        match env {
            Ok(x) => {
                let path = x[self.current_shell].path();
                x[self.current_shell].rm(args, context.name, &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                context.name,
            )),
        }
    }

    pub fn mkdir(
        &self,
        args: MkdirArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock();

        match env {
            Ok(x) => {
                let path = x[self.current_shell].path();
                x[self.current_shell].mkdir(args, context.name, &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                context.name,
            )),
        }
    }

    pub fn mv(
        &self,
        args: MoveArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock();

        match env {
            Ok(x) => {
                let path = x[self.current_shell].path();
                x[self.current_shell].mv(args, context.name, &path)
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Internal error: could not lock {}", e),
                "Internal error: could not lock",
                context.name,
            )),
        }
    }
}
