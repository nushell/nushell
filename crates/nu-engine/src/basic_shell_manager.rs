use crate::filesystem::filesystem_shell::{FilesystemShell, FilesystemShellMode};
use crate::shell::shell_manager::ShellManager;

use parking_lot::Mutex;
use std::error::Error;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

pub fn basic_shell_manager(mode: FilesystemShellMode) -> Result<ShellManager, Box<dyn Error>> {
    Ok(ShellManager {
        current_shell: Arc::new(AtomicUsize::new(0)),
        shells: Arc::new(Mutex::new(vec![Box::new(FilesystemShell::basic(mode)?)])),
    })
}
