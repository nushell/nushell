use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{DirEntry, ShellObject, Value};
use derive_new::new;
use std::path::{Path, PathBuf};
use sysinfo::SystemExt;

#[derive(new)]
pub struct Cd;

impl crate::Command for Cd {
    fn run(
        &mut self,
        args: Vec<String>,
        _host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Result<Value, ShellError> {
        env.cwd = dunce::canonicalize(env.cwd().join(&args[0]).as_path())?;
        Ok(Value::nothing())
    }
}
