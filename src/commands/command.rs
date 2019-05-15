use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use std::path::PathBuf;

pub trait CommandBlueprint {
    fn create(
        &self,
        input: Vec<Value>,
        host: &dyn crate::Host,
        env: &mut crate::Environment,
    ) -> Result<Box<dyn Command>, ShellError>;
}

#[derive(Debug)]
pub enum CommandAction {
    ChangeCwd(PathBuf),
}

#[derive(Debug)]
pub enum ReturnValue {
    Value(Value),
    Action(CommandAction),
}

impl ReturnValue {
    crate fn single(value: Value) -> VecDeque<ReturnValue> {
        let mut v = VecDeque::new();
        v.push_back(ReturnValue::Value(value));
        v
    }

    crate fn change_cwd(path: PathBuf) -> ReturnValue {
        ReturnValue::Action(CommandAction::ChangeCwd(path))
    }
}

pub trait Command {
    fn run(&mut self, stream: VecDeque<Value>) -> Result<VecDeque<ReturnValue>, ShellError>;
}
