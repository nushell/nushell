use crate::parser::Args;
use crate::prelude::*;

use indexmap::IndexMap;
use std::error::Error;
use std::sync::Arc;

pub struct Context {
    commands: IndexMap<String, Arc<dyn Command>>,
    crate host: Arc<Mutex<dyn Host + Send>>,
    crate env: Arc<Mutex<Environment>>,
}

impl Context {
    crate fn basic() -> Result<Context, Box<Error>> {
        Ok(Context {
            commands: indexmap::IndexMap::new(),
            host: Arc::new(Mutex::new(crate::env::host::BasicHost)),
            env: Arc::new(Mutex::new(Environment::basic()?)),
        })
    }

    pub fn add_commands(&mut self, commands: Vec<Arc<dyn Command>>) {
        for command in commands {
            self.commands.insert(command.name().to_string(), command);
        }
    }

    pub fn clone_commands(&self) -> indexmap::IndexMap<String, Arc<dyn Command>> {
        self.commands.clone()
    }

    crate fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    crate fn get_command(&self, name: &str) -> Arc<dyn Command> {
        self.commands.get(name).unwrap().clone()
    }

    crate fn run_command(
        &mut self,
        command: Arc<dyn Command>,
        args: Args,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = CommandArgs {
            host: self.host.clone(),
            env: self.env.clone(),
            positional: args.positional,
            named: args.named,
            input,
        };

        command.run(command_args)
    }
}
