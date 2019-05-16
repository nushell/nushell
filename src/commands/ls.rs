use crate::errors::ShellError;
use crate::object::{dir_entry_dict, Value};
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
pub struct Ls;

impl crate::Command for Ls {
    fn run(&self, args: CommandArgs<'value>) -> Result<VecDeque<ReturnValue>, ShellError> {
        let cwd = args.env.cwd().to_path_buf();

        let entries =
            std::fs::read_dir(&cwd).map_err(|e| ShellError::string(format!("{:?}", e)))?;

        let mut shell_entries = VecDeque::new();

        for entry in entries {
            let value = Value::Object(dir_entry_dict(&entry?)?);
            shell_entries.push_back(ReturnValue::Value(value))
        }

        Ok(shell_entries)
    }
}
