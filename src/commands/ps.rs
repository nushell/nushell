use crate::errors::ShellError;
use crate::object::process::Process;
use crate::object::{ShellObject, Value};
use crate::prelude::*;
use crate::Command;
use derive_new::new;
use std::cell::RefCell;
use std::rc::Rc;
use sysinfo::SystemExt;

#[derive(new)]
pub struct PsBlueprint;

impl crate::CommandBlueprint for PsBlueprint {
    fn create(
        &self,
        args: Vec<Value>,
        host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Result<Box<dyn Command>, ShellError> {
        Ok(Box::new(Ps::new()))
    }
}

#[derive(new)]
pub struct Ps;

impl crate::Command for Ps {
    fn run(&mut self, stream: VecDeque<Value>) -> Result<VecDeque<ReturnValue>, ShellError> {
        let mut system = sysinfo::System::new();
        system.refresh_all();

        let list = system.get_process_list();

        let list = list
            .into_iter()
            .map(|(_, process)| {
                ReturnValue::Value(Value::Object(Box::new(Process::new(process.clone()))))
            })
            .collect::<VecDeque<_>>();

        Ok(list)
    }
}
