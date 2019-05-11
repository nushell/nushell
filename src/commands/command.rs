use crate::errors::ShellError;
use crate::object::Value;

pub trait Command {
    fn run(
        &mut self,
        args: Vec<String>,
        host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Result<Value, ShellError>;
}
