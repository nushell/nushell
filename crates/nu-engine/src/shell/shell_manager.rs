use crate::command_args::EvaluatedWholeStreamCommandArgs;
use crate::maybe_text_codec::StringOrBinary;
use crate::shell::Shell;
use futures::Stream;
use nu_stream::OutputStream;

use crate::shell::shell_args::{CdArgs, CopyArgs, LsArgs, MkdirArgs, MvArgs, RemoveArgs};
use encoding_rs::Encoding;
use nu_errors::ShellError;
use nu_source::{Span, Tag};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ShellManager {
    pub current_shell: Arc<AtomicUsize>,
    pub shells: Arc<Mutex<Vec<Box<dyn Shell + Send>>>>,
}

impl ShellManager {
    pub fn insert_at_current(&self, shell: Box<dyn Shell + Send>) {
        self.shells.lock().push(shell);
        self.current_shell
            .store(self.shells.lock().len() - 1, Ordering::SeqCst);
        self.set_path(self.path());
    }

    pub fn current_shell(&self) -> usize {
        self.current_shell.load(Ordering::SeqCst)
    }

    pub fn remove_at_current(&self) {
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

    pub fn set_path(&self, path: String) {
        self.shells.lock()[self.current_shell()].set_path(path)
    }

    pub fn open(
        &self,
        full_path: &Path,
        name: Span,
        with_encoding: Option<&'static Encoding>,
    ) -> Result<impl Stream<Item = Result<StringOrBinary, ShellError>> + Send + 'static, ShellError>
    {
        self.shells.lock()[self.current_shell()].open(full_path, name, with_encoding)
    }

    pub fn save(
        &self,
        full_path: &Path,
        save_data: &[u8],
        name: Span,
    ) -> Result<OutputStream, ShellError> {
        self.shells.lock()[self.current_shell()].save(full_path, save_data, name)
    }

    pub fn next(&self) {
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

    pub fn prev(&self) {
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
