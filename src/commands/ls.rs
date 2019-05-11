use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{DirEntry, ShellObject, Value};
use crate::Args;
use crate::{Command, CommandSuccess};
use derive_new::new;
use std::path::PathBuf;
use sysinfo::SystemExt;

#[derive(new)]
pub struct LsBlueprint;

impl crate::CommandBlueprint for LsBlueprint {
    fn create(
        &self,
        args: Args,
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
    fn run(&mut self) -> Result<CommandSuccess, ShellError> {
        let entries =
            std::fs::read_dir(&self.cwd).map_err((|e| ShellError::new(format!("{:?}", e))))?;

        let mut shell_entries = vec![];

        for entry in entries {
            let value = Value::object(DirEntry::new(entry?)?);
            shell_entries.push(value)
        }

        Ok(CommandSuccess {
            value: Value::list(shell_entries),
            action: vec![],
        })
    }
}
