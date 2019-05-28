use crate::object::{Primitive, Value};
use crate::prelude::*;

pub fn to_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    Ok(out
        .map(|a| ReturnValue::Value(Value::Primitive(Primitive::String(serde_json::to_string(&a).unwrap()))))
        .boxed())
}
