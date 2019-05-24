use crate::prelude::*;

use std::error::Error;
use std::sync::Arc;

pub struct Context {
    commands: indexmap::IndexMap<String, Arc<dyn Command>>,
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

    pub fn add_commands(&mut self, commands: Vec<(&str, Arc<dyn Command>)>) {
        for (name, command) in commands {
            self.commands.insert(name.to_string(), command);
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
