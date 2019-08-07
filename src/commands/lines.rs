use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn lines(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let span = args.call_info.name_span;

    let stream = input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(s)) => {
                let split_result: Vec<_> = s.lines().filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnSuccess::value(
                        Value::Primitive(Primitive::String(s.into())).tagged_unknown(),
                    ));
                }
                result
            }
            _ => {
                let mut result = VecDeque::new();
                result.push_back(Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    v.span(),
                )));
                result
            }
        })
        .flatten();

    Ok(stream.to_output_stream())
}
