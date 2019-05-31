use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::debug;

// TODO: "Amount remaining" wrapper

pub fn split_row(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let args = args.args;

    let stream = input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = args[0].as_string().unwrap().replace("\\n", "\n");
                debug!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                debug!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnValue::Value(Value::Primitive(Primitive::String(s.to_string()))));
                }
                result
            }
            _ => {
                let result = VecDeque::new();
                result
            }
        })
        .flatten();

    Ok(stream.boxed())
}
