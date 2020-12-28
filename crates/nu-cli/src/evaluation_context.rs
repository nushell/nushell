use crate::commands::{command::CommandArgs, Command, UnevaluatedCallInfo};
use crate::env::host::Host;
use crate::prelude::*;
use crate::shell::shell_manager::ShellManager;
use nu_protocol::hir;
use nu_source::{Tag, Text};
use nu_stream::{InputStream, OutputStream};
use parking_lot::Mutex;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Clone)]
pub struct EvaluationContext {
    pub scope: Scope,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub user_recently_used_autoenv_untrust: Arc<AtomicBool>,
    pub(crate) shell_manager: ShellManager,

    /// Windows-specific: keep track of previous cwd on each drive
    pub windows_drives_previous_cwd: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl EvaluationContext {
    pub(crate) fn from_raw(raw_args: &CommandArgs) -> EvaluationContext {
        EvaluationContext {
            scope: raw_args.scope.clone(),
            host: raw_args.host.clone(),
            current_errors: raw_args.current_errors.clone(),
            ctrl_c: raw_args.ctrl_c.clone(),
            shell_manager: raw_args.shell_manager.clone(),
            user_recently_used_autoenv_untrust: Arc::new(AtomicBool::new(false)),
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub(crate) fn from_args(args: &CommandArgs) -> EvaluationContext {
        EvaluationContext {
            scope: args.scope.clone(),
            host: args.host.clone(),
            current_errors: args.current_errors.clone(),
            ctrl_c: args.ctrl_c.clone(),
            shell_manager: args.shell_manager.clone(),
            user_recently_used_autoenv_untrust: Arc::new(AtomicBool::new(false)),
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn basic() -> Result<EvaluationContext, Box<dyn Error>> {
        Ok(EvaluationContext {
            scope: Scope::new(),
            host: Arc::new(parking_lot::Mutex::new(Box::new(
                crate::env::host::BasicHost,
            ))),
            current_errors: Arc::new(Mutex::new(vec![])),
            ctrl_c: Arc::new(AtomicBool::new(false)),
            user_recently_used_autoenv_untrust: Arc::new(AtomicBool::new(false)),
            shell_manager: ShellManager::basic()?,
            windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    pub(crate) fn error(&self, error: ShellError) {
        self.with_errors(|errors| errors.push(error))
    }

    pub(crate) fn clear_errors(&self) {
        self.current_errors.lock().clear()
    }

    pub(crate) fn get_errors(&self) -> Vec<ShellError> {
        self.current_errors.lock().clone()
    }

    pub(crate) fn add_error(&self, err: ShellError) {
        self.current_errors.lock().push(err);
    }

    pub(crate) fn maybe_print_errors(&self, source: Text) -> bool {
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

    pub(crate) fn with_host<T>(&self, block: impl FnOnce(&mut dyn Host) -> T) -> T {
        let mut host = self.host.lock();

        block(&mut *host)
    }

    pub(crate) fn with_errors<T>(&self, block: impl FnOnce(&mut Vec<ShellError>) -> T) -> T {
        let mut errors = self.current_errors.lock();

        block(&mut *errors)
    }

    pub fn add_commands(&self, commands: Vec<Command>) {
        for command in commands {
            self.scope.add_command(command.name().to_string(), command);
        }
    }

    #[allow(unused)]
    pub(crate) fn get_command(&self, name: &str) -> Option<Command> {
        self.scope.get_command(name)
    }

    pub(crate) fn is_command_registered(&self, name: &str) -> bool {
        self.scope.has_command(name)
    }

    pub(crate) async fn run_command(
        &self,
        command: Command,
        name_tag: Tag,
        args: hir::Call,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = self.command_args(args, input, name_tag);
        command.run(command_args).await
    }

    fn call_info(&self, args: hir::Call, name_tag: Tag) -> UnevaluatedCallInfo {
        UnevaluatedCallInfo { args, name_tag }
    }

    fn command_args(&self, args: hir::Call, input: InputStream, name_tag: Tag) -> CommandArgs {
        CommandArgs {
            host: self.host.clone(),
            ctrl_c: self.ctrl_c.clone(),
            current_errors: self.current_errors.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: self.call_info(args, name_tag),
            scope: self.scope.clone(),
            input,
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
