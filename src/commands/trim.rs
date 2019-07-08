use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;

// TODO: "Amount remaining" wrapper

pub fn trim(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let span = args.name_span;

    Ok(input
        .values
        .map(move |v| ReturnSuccess::value(String::check(&v)?.clone()))
        // Value::Primitive(Primitive::String(s)) => {
        //     ReturnSuccess::value(Value::Primitive(Primitive::String(s.trim().into())))
        // }
        // _ => Err(ShellError::maybe_labeled_error(
        //     "Expected string values from pipeline",
        //     "expects strings from pipeline",
        //     span,
        // )),
        .to_output_stream())
}
