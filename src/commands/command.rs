use crate::errors::ShellError;
use crate::object::Value;
use std::path::PathBuf;

pub trait CommandBlueprint {
    fn create(
        &self,
        args: Vec<String>,
        host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Box<dyn Command>;
}

crate enum CommandAction {
    ChangeCwd(PathBuf),
}

pub struct CommandSuccess {
    crate value: Value,
    crate action: Vec<CommandAction>,
}

pub trait Command {
    fn begin(&mut self) -> Result<(), ShellError> {
        Ok(())
    }
    fn run(&mut self) -> Result<CommandSuccess, ShellError>;
    fn end(&mut self) -> Result<(), ShellError> {
        Ok(())
    }
}
