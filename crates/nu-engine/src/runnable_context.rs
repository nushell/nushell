use crate::{Command, CommandArgs, EvaluationContext};
use crate::{ConfigHolder, Host, Scope, ShellManager};
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
    pub config_holder: Arc<Mutex<ConfigHolder>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub scope: Scope,
    pub name: Tag,
}

impl RunnableContext {
    pub fn from_command_args(args: CommandArgs) -> Self {
        RunnableContext {
            input: args.input,
            scope: args.scope.clone(),
            shell_manager: args.shell_manager,
            host: args.host,
            ctrl_c: args.ctrl_c,
            config_holder: args.config_holder,
            current_errors: args.current_errors,
            name: args.call_info.name_tag,
        }
    }

    pub fn from_evaluation_context(input: InputStream, ctx: &EvaluationContext) -> Self {
        RunnableContext {
            input,
            shell_manager: ctx.shell_manager.clone(),
            host: ctx.host.clone(),
            ctrl_c: ctx.ctrl_c.clone(),
            config_holder: ctx.config_holder.clone(),
            current_errors: ctx.current_errors.clone(),
            scope: ctx.scope.clone(),
            name: Tag::unknown(),
        }
    }

    pub fn get_command(&self, name: &str) -> Option<Command> {
        self.scope.get_command(name)
    }
}
