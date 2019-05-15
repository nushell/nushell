use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
pub struct ToArrayBlueprint;

impl crate::CommandBlueprint for ToArrayBlueprint {
    fn create(
        &self,
        _args: Vec<Value>,
        _host: &dyn Host,
        _env: &mut Environment,
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
