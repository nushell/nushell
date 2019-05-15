use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{dir_entry_dict, Value};
use crate::prelude::*;
use crate::Args;
use crate::Command;
use derive_new::new;
use std::path::PathBuf;
use sysinfo::SystemExt;

#[derive(new)]
pub struct LsBlueprint;

impl crate::CommandBlueprint for LsBlueprint {
    fn create(
        &self,
        args: Vec<Value>,
        host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Result<Box<dyn Command>, ShellError> {
        Ok(Box::new(Ls {
            cwd: env.cwd().to_path_buf(),
        }))
    }
}

#[derive(new)]
pub struct Ls {
    cwd: PathBuf,
}

impl crate::Command for Ls {
    fn run(&mut self, stream: VecDeque<Value>) -> Result<VecDeque<ReturnValue>, ShellError> {
        let entries =
            std::fs::read_dir(&self.cwd).map_err((|e| ShellError::string(format!("{:?}", e))))?;

        let mut shell_entries = VecDeque::new();

        for entry in entries {
            let value = Value::Object(dir_entry_dict(&entry?)?);
            shell_entries.push_back(ReturnValue::Value(value))
        }

        Ok(shell_entries)
    }
}
