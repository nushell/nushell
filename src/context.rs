use crate::parser::{CommandConfig, CommandRegistry};
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

    pub fn registry(&self) -> CommandMap {
        CommandMap {
            commands: self.clone_commands(),
        }
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
        arg_list: Vec<Value>,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = CommandArgs {
            host: self.host.clone(),
            env: self.env.clone(),
            args: arg_list,
            input,
        };

        command.run(command_args)
    }
}

pub struct CommandMap {
    #[allow(unused)]
    commands: IndexMap<String, Arc<dyn Command>>,
}

impl CommandRegistry for CommandMap {
    fn get(&self, name: &str) -> CommandConfig {
        CommandConfig {
            name: name.to_string(),
            mandatory_positional: vec![],
            optional_positional: vec![],
            rest_positional: true,
            named: IndexMap::new(),
        }
    }
}
