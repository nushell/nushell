use crate::prelude::*;

use std::error::Error;

pub struct Context {
    commands: indexmap::IndexMap<String, Box<dyn crate::Command>>,
    crate host: Box<dyn crate::Host>,
    crate env: Environment,
}

impl Context {
    crate fn basic() -> Result<Context, Box<Error>> {
        Ok(Context {
            commands: indexmap::IndexMap::new(),
            host: Box::new(crate::env::host::BasicHost),
            env: crate::Environment::basic()?,
        })
    }

    pub fn add_commands(&mut self, commands: Vec<(&str, Box<dyn crate::Command>)>) {
        for (name, command) in commands {
            self.commands.insert(name.to_string(), command);
        }
    }

    crate fn has_command(&mut self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    crate fn run_command(
        &self,
        name: &str,
        arg_list: Vec<Value>,
        input: VecDeque<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        let command_args = CommandArgs {
            host: &self.host,
            env: &self.env,
            args: arg_list,
            input,
        };

        match self.commands.get(name) {
            None => Err(ShellError::string(format!(
                "Command {} did not exist",
                name
            ))),
            Some(command) => command.run(command_args),
        }
    }
}
