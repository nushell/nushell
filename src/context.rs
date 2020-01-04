use crate::commands::{command::CommandArgs, Command, UnevaluatedCallInfo};
use crate::env::host::Host;
use crate::shell::shell_manager::ShellManager;
use crate::stream::{InputStream, OutputStream};
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_parser::{hir, hir::syntax_shape::ExpandContext, hir::syntax_shape::SignatureRegistry};
use nu_protocol::{errln, Signature};
use nu_source::{Tag, Text};
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CommandRegistry {
    registry: Arc<Mutex<IndexMap<String, Arc<Command>>>>,
}

impl SignatureRegistry for CommandRegistry {
    fn has(&self, name: &str) -> Result<bool, ShellError> {
        if let Ok(registry) = self.registry.lock() {
            Ok(registry.contains_key(name))
        } else {
            Err(ShellError::untagged_runtime_error(format!(
                "Could not load from registry: {}",
                name
            )))
        }
    }
    fn get(&self, name: &str) -> Result<Option<Signature>, ShellError> {
        if let Ok(registry) = self.registry.lock() {
            Ok(registry.get(name).map(|command| command.signature()))
        } else {
            Err(ShellError::untagged_runtime_error(format!(
                "Could not get from registry: {}",
                name
            )))
        }
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
    pub(crate) fn empty() -> CommandRegistry {
        CommandRegistry {
            registry: Arc::new(Mutex::new(IndexMap::default())),
        }
    }

    pub(crate) fn get_command(&self, name: &str) -> Result<Option<Arc<Command>>, ShellError> {
        let registry = self.registry.lock().map_err(|_| {
            ShellError::untagged_runtime_error("Internal error: get_command could not get mutex")
        })?;

        Ok(registry.get(name).cloned())
    }

    pub(crate) fn expect_command(&self, name: &str) -> Result<Arc<Command>, ShellError> {
        self.get_command(name)?.ok_or_else(|| {
            ShellError::untagged_runtime_error(format!("Could not load command: {}", name))
        })
    }

    pub(crate) fn has(&self, name: &str) -> Result<bool, ShellError> {
        let registry = self.registry.lock().map_err(|_| {
            ShellError::untagged_runtime_error("Internal error: has could not get mutex")
        })?;

        Ok(registry.contains_key(name))
    }

    pub(crate) fn insert(
        &mut self,
        name: impl Into<String>,
        command: Arc<Command>,
    ) -> Result<(), ShellError> {
        let mut registry = self.registry.lock().map_err(|_| {
            ShellError::untagged_runtime_error("Internal error: insert could not get mutex")
        })?;

        registry.insert(name.into(), command);
        Ok(())
    }

    pub(crate) fn names(&self) -> Result<Vec<String>, ShellError> {
        let registry = self.registry.lock().map_err(|_| {
            ShellError::untagged_runtime_error("Internal error: names could not get mutex")
        })?;
        Ok(registry.keys().cloned().collect())
    }

    pub(crate) fn snapshot(&self) -> Result<IndexMap<String, Arc<Command>>, ShellError> {
        let registry = self.registry.lock().map_err(|_| {
            ShellError::untagged_runtime_error("Internal error: names could not get mutex")
        })?;
        Ok(registry.clone())
    }
}

#[derive(Clone)]
pub struct Context {
    pub registry: CommandRegistry,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub(crate) shell_manager: ShellManager,
}

impl Context {
    pub(crate) fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    pub(crate) fn expand_context<'context>(
        &'context self,
        source: &'context Text,
    ) -> Result<ExpandContext<'context>, ShellError> {
        Ok(ExpandContext::new(
            Box::new(self.registry.clone()),
            source,
            self.shell_manager.homedir()?,
        ))
    }

    pub(crate) fn basic() -> Result<Context, Box<dyn Error>> {
        let registry = CommandRegistry::new();
        Ok(Context {
            registry: registry.clone(),
            host: Arc::new(parking_lot::Mutex::new(Box::new(
                crate::env::host::BasicHost,
            ))),
            current_errors: Arc::new(Mutex::new(vec![])),
            ctrl_c: Arc::new(AtomicBool::new(false)),
            shell_manager: ShellManager::basic(registry)?,
        })
    }

    pub(crate) fn error(&mut self, error: ShellError) -> Result<(), ShellError> {
        self.with_errors(|errors| errors.push(error))
    }

    pub(crate) fn maybe_print_errors(&mut self, source: Text) -> bool {
        let errors = self.current_errors.clone();
        let errors = errors.lock();

        let host = self.host.clone();
        let host = host.lock();

        let result: bool;

        match (errors, host) {
            (Err(err), _) => {
                errln!(
                    "Unexpected error attempting to acquire the lock of the current errors: {:?}",
                    err
                );
                result = false;
            }
            (Ok(mut errors), host) => {
                if errors.len() > 0 {
                    let error = errors[0].clone();
                    *errors = vec![];

                    crate::cli::print_err(error, &*host, &source);
                    result = true;
                } else {
                    result = false;
                }
            }
        };

        result
    }

    pub(crate) fn with_host<T>(
        &mut self,
        block: impl FnOnce(&mut dyn Host) -> T,
    ) -> Result<T, ShellError> {
        let mut host = self.host.lock();
        Ok(block(&mut *host))
    }

    pub(crate) fn with_errors<T>(
        &mut self,
        block: impl FnOnce(&mut Vec<ShellError>) -> T,
    ) -> Result<T, ShellError> {
        if let Ok(mut errors) = self.current_errors.lock() {
            Ok(block(&mut *errors))
        } else {
            Err(ShellError::untagged_runtime_error(
                "Internal error: could not lock host in with_errors",
            ))
        }
    }

    pub fn add_commands(&mut self, commands: Vec<Arc<Command>>) -> Result<(), ShellError> {
        for command in commands {
            self.registry.insert(command.name().to_string(), command)?;
        }

        Ok(())
    }

    pub(crate) fn get_command(&self, name: &str) -> Result<Option<Arc<Command>>, ShellError> {
        self.registry.get_command(name)
    }

    pub(crate) fn expect_command(&self, name: &str) -> Result<Arc<Command>, ShellError> {
        self.registry.expect_command(name)
    }

    pub(crate) fn run_command(
        &mut self,
        command: Arc<Command>,
        name_tag: Tag,
        args: hir::Call,
        source: &Text,
        input: InputStream,
    ) -> OutputStream {
        let command_args = self.command_args(args, input, source, name_tag);
        command.run(command_args, self.registry())
    }

    fn call_info(&self, args: hir::Call, source: &Text, name_tag: Tag) -> UnevaluatedCallInfo {
        UnevaluatedCallInfo {
            args,
            source: source.clone(),
            name_tag,
        }
    }

    fn command_args(
        &self,
        args: hir::Call,
        input: InputStream,
        source: &Text,
        name_tag: Tag,
    ) -> CommandArgs {
        CommandArgs {
            host: self.host.clone(),
            ctrl_c: self.ctrl_c.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: self.call_info(args, source, name_tag),
            input,
        }
    }
}
