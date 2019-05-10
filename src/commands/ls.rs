use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{DirEntry, ShellObject, Value};
use derive_new::new;
use sysinfo::SystemExt;

#[derive(new)]
pub struct Ls;

impl crate::Command for Ls {
    fn run(
        &mut self,
        _host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Result<Value, ShellError> {
        let entries =
            std::fs::read_dir(env.cwd()).map_err((|e| ShellError::new(format!("{:?}", e))))?;

        let mut shell_entries = vec![];

        for entry in entries {
            let value = Value::object(DirEntry::new(entry?)?);
            shell_entries.push(value)
        }

        Ok(Value::list(shell_entries))
    }
}
