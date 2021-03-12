use crate::{Command, Host, Scope, ShellManager};
use nu_errors::ShellError;
use nu_source::Tag;
use nu_stream::InputStream;
use parking_lot::Mutex;
use std::sync::{atomic::AtomicBool, Arc};

pub struct RunnableContext {
    pub input: InputStream,
    pub shell_manager: ShellManager,
    pub host: Arc<Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub scope: Scope,
    pub name: Tag,
}

impl RunnableContext {
    pub fn get_command(&self, name: &str) -> Option<Command> {
        self.scope.get_command(name)
    }
}
