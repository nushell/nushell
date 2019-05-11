use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{DirEntry, ShellObject, Value};
use crate::Args;
use derive_new::new;
use std::path::{Path, PathBuf};
use sysinfo::SystemExt;

#[derive(new)]
pub struct CdBlueprint;

impl crate::CommandBlueprint for CdBlueprint {
    fn create(
        &self,
        args: Args,
        host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Result<Box<dyn crate::Command>, ShellError> {
        let target = match args.first() {
            // TODO: This needs better infra
            None => return Err(ShellError::new(format!("cd must take one arg"))),
            Some(v) => v.as_string()?.clone(),
        };

        Ok(Box::new(Cd {
            cwd: env.cwd().to_path_buf(),
            target,
        }))
    }
}

#[derive(new)]
pub struct Cd {
    cwd: PathBuf,
    target: String,
}

impl crate::Command for Cd {
    fn run(&mut self) -> Result<crate::CommandSuccess, ShellError> {
        Ok(crate::CommandSuccess {
            value: Value::nothing(),
            action: vec![crate::CommandAction::ChangeCwd(dunce::canonicalize(
                self.cwd.join(&self.target).as_path(),
            )?)],
        })
    }
}
