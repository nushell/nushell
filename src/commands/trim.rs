use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;

// TODO: "Amount remaining" wrapper

pub fn trim(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;

    Ok(input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                ReturnValue::Value(Value::Primitive(Primitive::String(s.trim().to_string())))
            }
            x => ReturnValue::Value(x),
        })
        .boxed())
}
