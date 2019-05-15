use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{dir_entry_dict, ShellObject, Value};
use crate::prelude::*;
use crate::Args;
use derive_new::new;
use std::path::{Path, PathBuf};
use sysinfo::SystemExt;

#[derive(new)]
pub struct ToArrayBlueprint;

impl crate::CommandBlueprint for ToArrayBlueprint {
    fn create(
        &self,
        args: Vec<Value>,
        host: &dyn Host,
        env: &mut Environment,
    ) -> Result<Box<dyn Command>, ShellError> {
        Ok(Box::new(ToArray))
    }
}

#[derive(new)]
pub struct ToArray;

impl crate::Command for ToArray {
    fn run(&mut self, stream: VecDeque<Value>) -> Result<VecDeque<ReturnValue>, ShellError> {
        let out = stream.into_iter().collect();
        Ok(ReturnValue::single(Value::List(out)))
    }
}

crate fn to_array(stream: VecDeque<Value>) -> VecDeque<Value> {
    let out = Value::List(stream.into_iter().collect());
    let mut stream = VecDeque::new();
    stream.push_back(out);
    stream
}
