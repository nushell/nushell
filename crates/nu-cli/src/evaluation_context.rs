use crate::command_registry::CommandRegistry;
use crate::commands::{command::CommandArgs, Command, UnevaluatedCallInfo};
use crate::env::host::Host;
use crate::shell::shell_manager::ShellManager;
use crate::stream::{InputStream, OutputStream};
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{hir, Scope};
use nu_source::{Tag, Text};
use parking_lot::Mutex;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Clone)]
pub struct EvaluationContext {
    pub registry: CommandRegistry,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub raw_input: String,
    pub user_recently_used_autoenv_untrust: bool,
    pub(crate) shell_manager: ShellManager,

    /// Windows-specific: keep track of previous cwd on each drive
    pub windows_drives_previous_cwd: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl EvaluationContext {
    pub(crate) fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    pub(crate) fn from_raw(
        raw_args: &CommandArgs,
        registry: &CommandRegistry,
    ) -> EvaluationContext {
        EvaluationContext {
            registry: registry.clone(),
            host: raw_args.host.clone(),
            current_errors: raw_args.current_errors.clone(),
            ctrl_c: raw_args.ctrl_c.clone(),
            shell_manager: raw_args.shell_manager.clone(),
            user_recently_used_autoenv_untrust: false,
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
            raw_input: String::default(),
        }
    }

    pub(crate) fn from_args(args: &CommandArgs, registry: &CommandRegistry) -> EvaluationContext {
        EvaluationContext {
            registry: registry.clone(),
            host: args.host.clone(),
            current_errors: args.current_errors.clone(),
            ctrl_c: args.ctrl_c.clone(),
            shell_manager: args.shell_manager.clone(),
            user_recently_used_autoenv_untrust: false,
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
            raw_input: String::default(),
        }
    }

    pub fn basic() -> Result<EvaluationContext, Box<dyn Error>> {
        let registry = CommandRegistry::new();

        Ok(EvaluationContext {
            registry,
            host: Arc::new(parking_lot::Mutex::new(Box::new(
                crate::env::host::BasicHost,
            ))),
            current_errors: Arc::new(Mutex::new(vec![])),
            ctrl_c: Arc::new(AtomicBool::new(false)),
            user_recently_used_autoenv_untrust: false,
            shell_manager: ShellManager::basic()?,
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
            raw_input: String::default(),
        })
    }

    pub(crate) fn error(&mut self, error: ShellError) {
        self.with_errors(|errors| errors.push(error))
    }

    pub(crate) fn clear_errors(&mut self) {
        self.current_errors.lock().clear()
    }

    pub(crate) fn get_errors(&self) -> Vec<ShellError> {
        self.current_errors.lock().clone()
    }

    pub(crate) fn add_error(&self, err: ShellError) {
        self.current_errors.lock().push(err);
    }

    pub(crate) fn maybe_print_errors(&mut self, source: Text) -> bool {
        let errors = self.current_errors.clone();
        let mut errors = errors.lock();

        if errors.len() > 0 {
            let error = errors[0].clone();
            *errors = vec![];

            crate::cli::print_err(error, &source);
            true
        } else {
            false
        }
    }

    pub(crate) fn configure<T>(
        &mut self,
        config: &dyn nu_data::config::Conf,
        block: impl FnOnce(&dyn nu_data::config::Conf, &mut Self) -> T,
    ) {
        block(config, &mut *self);
    }

    pub(crate) fn with_host<T>(&mut self, block: impl FnOnce(&mut dyn Host) -> T) -> T {
        let mut host = self.host.lock();

        block(&mut *host)
    }

    pub(crate) fn with_errors<T>(&mut self, block: impl FnOnce(&mut Vec<ShellError>) -> T) -> T {
        let mut errors = self.current_errors.lock();

        block(&mut *errors)
    }

    pub fn add_commands(&mut self, commands: Vec<Command>) {
        for command in commands {
            self.registry.insert(command.name().to_string(), command);
        }
    }

    #[allow(unused)]
    pub(crate) fn get_command(&self, name: &str) -> Option<Command> {
        self.registry.get_command(name)
    }

    pub(crate) fn is_command_registered(&self, name: &str) -> bool {
        self.registry.has(name)
    }

    pub(crate) fn expect_command(&self, name: &str) -> Result<Command, ShellError> {
        self.registry.expect_command(name)
    }

    pub(crate) async fn run_command(
        &mut self,
        command: Command,
        name_tag: Tag,
        args: hir::Call,
        scope: Arc<Scope>,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = self.command_args(args, input, name_tag, scope);
        command.run(command_args, self.registry()).await
    }

    fn call_info(&self, args: hir::Call, name_tag: Tag, scope: Arc<Scope>) -> UnevaluatedCallInfo {
        UnevaluatedCallInfo {
            args,
            name_tag,
            scope,
        }
    }

    fn command_args(
        &self,
        args: hir::Call,
        input: InputStream,
        name_tag: Tag,
        scope: Arc<Scope>,
    ) -> CommandArgs {
        CommandArgs {
            host: self.host.clone(),
            ctrl_c: self.ctrl_c.clone(),
            current_errors: self.current_errors.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: self.call_info(args, name_tag, scope),
            input,
            raw_input: self.raw_input.clone(),
        }
    }

    pub fn get_env(&self) -> IndexMap<String, String> {
        let mut output = IndexMap::new();
        for (var, value) in self.host.lock().vars() {
            output.insert(var, value);
        }
        output
    }
}
