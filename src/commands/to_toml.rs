use crate::object::{Primitive, Value};
use crate::prelude::*;

pub fn to_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    Ok(out
        .map(|a| {
            ReturnValue::Value(Value::Primitive(Primitive::String(
                toml::to_string(&a).unwrap().into(),
            )))
        })
        .boxed())
}
