use crate::call_info::UnevaluatedCallInfo;
use crate::command_args::CommandArgs;
use crate::env::host::Host;
use crate::evaluate::scope::Scope;
use crate::shell::shell_manager::ShellManager;
use crate::whole_stream_command::Command;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::hir;
use nu_source::Tag;
use nu_stream::{InputStream, OutputStream};
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Clone)]
pub struct EvaluationContext {
    pub scope: Scope,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub user_recently_used_autoenv_untrust: Arc<AtomicBool>,
    pub shell_manager: ShellManager,

    /// Windows-specific: keep track of previous cwd on each drive
    pub windows_drives_previous_cwd: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl EvaluationContext {
    pub fn from_args(args: &CommandArgs) -> EvaluationContext {
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

    pub fn error(&self, error: ShellError) {
        self.with_errors(|errors| errors.push(error))
    }

    pub fn clear_errors(&self) {
        self.current_errors.lock().clear()
    }

    pub fn get_errors(&self) -> Vec<ShellError> {
        self.current_errors.lock().clone()
    }

    pub fn configure<T>(
        &mut self,
        config: &dyn nu_data::config::Conf,
        block: impl FnOnce(&dyn nu_data::config::Conf, &mut Self) -> T,
    ) {
        block(config, &mut *self);
    }

    pub fn with_host<T>(&self, block: impl FnOnce(&mut dyn Host) -> T) -> T {
        let mut host = self.host.lock();

        block(&mut *host)
    }

    pub fn with_errors<T>(&self, block: impl FnOnce(&mut Vec<ShellError>) -> T) -> T {
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

    pub fn is_command_registered(&self, name: &str) -> bool {
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
