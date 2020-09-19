use crate::commands::Command;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;
use nu_protocol::Signature;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    registry: Arc<Mutex<IndexMap<String, Command>>>,
}

impl SignatureRegistry for CommandRegistry {
    fn has(&self, name: &str) -> bool {
        let registry = self.registry.lock();
        registry.contains_key(name)
    }
    fn get(&self, name: &str) -> Option<Signature> {
        let registry = self.registry.lock();
        registry.get(name).map(|command| command.signature())
    }
    fn clone_box(&self) -> Box<dyn SignatureRegistry> {
        Box::new(self.clone())
    }
}

impl CommandRegistry {
    pub fn new() -> CommandRegistry {
        CommandRegistry {
            registry: Arc::new(Mutex::new(IndexMap::default())),
        }
    }
}

impl CommandRegistry {
    pub fn get_command(&self, name: &str) -> Option<Command> {
        let registry = self.registry.lock();

        registry.get(name).cloned()
    }

    pub fn expect_command(&self, name: &str) -> Result<Command, ShellError> {
        self.get_command(name).ok_or_else(|| {
            ShellError::untagged_runtime_error(format!("Could not load command: {}", name))
        })
    }

    pub fn has(&self, name: &str) -> bool {
        let registry = self.registry.lock();

        registry.contains_key(name)
    }

    pub fn insert(&mut self, name: impl Into<String>, command: Command) {
        let mut registry = self.registry.lock();
        registry.insert(name.into(), command);
    }

    pub fn names(&self) -> Vec<String> {
        let registry = self.registry.lock();
        registry.keys().cloned().collect()
    }
}
