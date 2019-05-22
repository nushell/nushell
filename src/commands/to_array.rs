use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

pub fn to_array(args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
    let out = args.input.into_iter().collect();
    Ok(ReturnValue::single(Value::List(out)))
}

crate fn stream_to_array(stream: VecDeque<Value>) -> VecDeque<Value> {
    let out = Value::List(stream.into_iter().collect());
    let mut stream = VecDeque::new();
    stream.push_back(out);
    stream
}
