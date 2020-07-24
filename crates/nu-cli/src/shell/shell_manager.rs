use crate::commands::cd::CdArgs;
use crate::commands::classified::maybe_text_codec::StringOrBinary;
use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::ls::LsArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::move_::mv::Arguments as MvArgs;
use crate::commands::rm::RemoveArgs;
use crate::prelude::*;
use crate::shell::filesystem_shell::FilesystemShell;
use crate::shell::shell::Shell;
use crate::stream::OutputStream;

use encoding_rs::Encoding;
use nu_errors::ShellError;
use parking_lot::Mutex;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ShellManager {
    pub(crate) current_shell: Arc<AtomicUsize>,
    pub(crate) shells: Arc<Mutex<Vec<Box<dyn Shell + Send>>>>,
}

impl ShellManager {
    pub fn basic() -> Result<ShellManager, Box<dyn Error>> {
        Ok(ShellManager {
            current_shell: Arc::new(AtomicUsize::new(0)),
            shells: Arc::new(Mutex::new(vec![Box::new(FilesystemShell::basic()?)])),
        })
    }

    pub fn insert_at_current(&mut self, shell: Box<dyn Shell + Send>) {
        self.shells.lock().push(shell);
        self.current_shell
            .store(self.shells.lock().len() - 1, Ordering::SeqCst);
        self.set_path(self.path());
    }

    pub fn current_shell(&self) -> usize {
        self.current_shell.load(Ordering::SeqCst)
    }

    pub fn remove_at_current(&mut self) {
        {
            let mut shells = self.shells.lock();
            if shells.len() > 0 {
                if self.current_shell() == shells.len() - 1 {
                    shells.pop();
                    let new_len = shells.len();
                    if new_len > 0 {
                        self.current_shell.store(new_len - 1, Ordering::SeqCst);
                    } else {
                        return;
                    }
                } else {
                    shells.remove(self.current_shell());
                }
            }
        }
        self.set_path(self.path())
    }

    pub fn is_empty(&self) -> bool {
        self.shells.lock().is_empty()
    }

    pub fn path(&self) -> String {
        self.shells.lock()[self.current_shell()].path()
    }

    pub fn pwd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock();

        env[self.current_shell()].pwd(args)
    }

    pub fn set_path(&mut self, path: String) {
        self.shells.lock()[self.current_shell()].set_path(path)
    }

    pub fn open(
        &self,
        full_path: &PathBuf,
        name: Span,
        with_encoding: Option<&'static Encoding>,
    ) -> Result<impl Stream<Item = Result<StringOrBinary, ShellError>> + Send + 'static, ShellError>
    {
        self.shells.lock()[self.current_shell()].open(full_path, name, with_encoding)
    }

    pub fn save(
        &mut self,
        full_path: &PathBuf,
        save_data: &[u8],
        name: Span,
    ) -> Result<OutputStream, ShellError> {
        self.shells.lock()[self.current_shell()].save(full_path, save_data, name)
    }

    pub fn next(&mut self) {
        {
            let shell_len = self.shells.lock().len();
            if self.current_shell() == (shell_len - 1) {
                self.current_shell.store(0, Ordering::SeqCst);
            } else {
                self.current_shell
                    .store(self.current_shell() + 1, Ordering::SeqCst);
            }
        }
        self.set_path(self.path())
    }

    pub fn prev(&mut self) {
        {
            let shell_len = self.shells.lock().len();
            if self.current_shell() == 0 {
                self.current_shell.store(shell_len - 1, Ordering::SeqCst);
            } else {
                self.current_shell
                    .store(self.current_shell() - 1, Ordering::SeqCst);
            }
        }
        self.set_path(self.path())
    }

    pub fn homedir(&self) -> Option<PathBuf> {
        let env = self.shells.lock();

        env[self.current_shell()].homedir()
    }

    pub fn ls(
        &self,
        args: LsArgs,
        name: Tag,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock();

        env[self.current_shell()].ls(args, name, ctrl_c)
    }

    pub fn cd(&self, args: CdArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let env = self.shells.lock();

        env[self.current_shell()].cd(args, name)
    }

    pub fn cp(&self, args: CopyArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        let path = shells[self.current_shell()].path();
        shells[self.current_shell()].cp(args, name, &path)
    }

    pub fn rm(&self, args: RemoveArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        let path = shells[self.current_shell()].path();
        shells[self.current_shell()].rm(args, name, &path)
    }

    pub fn mkdir(&self, args: MkdirArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        let path = shells[self.current_shell()].path();
        shells[self.current_shell()].mkdir(args, name, &path)
    }

    pub fn mv(&self, args: MvArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let shells = self.shells.lock();

        let path = shells[self.current_shell()].path();
        shells[self.current_shell()].mv(args, name, &path)
    }
}
