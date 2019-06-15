use crate::object::{Primitive, Value};
use crate::prelude::*;

pub fn to_json(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .map(move |a| match serde_json::to_string(&a) {
            Ok(x) => ReturnValue::Value(Value::Primitive(Primitive::String(x))),
            Err(_) => ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                "Can not convert to JSON string",
                "can not convert piped data to JSON string",
                span,
            )))),
        })
        .boxed())
}
