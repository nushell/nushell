use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::errors::ShellError;
use crate::prelude::*;
use crate::shell::filesystem_shell::FilesystemShell;
use crate::shell::shell::Shell;
use crate::stream::OutputStream;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ShellManager {
    crate shells: Arc<Mutex<Vec<Box<dyn Shell + Send>>>>,
}

impl ShellManager {
    pub fn basic(commands: CommandRegistry) -> Result<ShellManager, Box<dyn Error>> {
        Ok(ShellManager {
            shells: Arc::new(Mutex::new(vec![Box::new(FilesystemShell::basic(
                commands,
            )?)])),
        })
    }

    pub fn push(&mut self, shell: Box<dyn Shell + Send>) {
        self.shells.lock().unwrap().push(shell);
        self.set_path(self.path());
    }

    pub fn pop(&mut self) {
        self.shells.lock().unwrap().pop();
    }

    pub fn is_empty(&self) -> bool {
        self.shells.lock().unwrap().is_empty()
    }

    pub fn path(&self) -> String {
        self.shells.lock().unwrap().last().unwrap().path()
    }

    pub fn set_path(&mut self, path: String) {
        self.shells
            .lock()
            .unwrap()
            .last_mut()
            .unwrap()
            .set_path(path)
    }

    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError> {
        self.shells
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .complete(line, pos, ctx)
    }

    pub fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.shells
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .hint(line, pos, ctx)
    }

    pub fn next(&mut self) {
        {
            let mut x = self.shells.lock().unwrap();
            let shell = x.remove(0);
            x.push(shell);
        }
        self.set_path(self.path());
    }

    pub fn prev(&mut self) {
        {
            let mut x = self.shells.lock().unwrap();
            let shell = x.pop().unwrap();
            x.insert(0, shell);
        }
        self.set_path(self.path());
    }

    pub fn ls(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock().unwrap();

        env.last().unwrap().ls(args)
    }
    pub fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock().unwrap();

        env.last().unwrap().cd(args)
    }
}
