use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;

// TODO: "Amount remaining" wrapper

pub fn trim(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let span = args.name_span;

    Ok(input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                ReturnValue::Value(Value::Primitive(Primitive::String(s.trim().to_string())))
            }
            _ => ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )))),
        })
        .boxed())
}
