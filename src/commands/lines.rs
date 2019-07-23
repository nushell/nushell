use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn lines(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let input = args.input;

    let input: InputStream = trace_stream!(target: "nu::trace_stream::lines", "input" = input);

    let stream = input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(s)) => {
                let split_result: Vec<_> = s.lines().filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnSuccess::value(
                        Value::Primitive(Primitive::String(s.into())).spanned_unknown(),
                    ));
                }
                result
            }
            _ => {
                let mut result = VecDeque::new();
                result.push_back(Err(ShellError::maybe_labeled_error(
                    "Expected string values from pipeline",
                    "expects strings from pipeline",
                    span,
                )));
                result
            }
        })
        .flatten();

    Ok(stream.to_output_stream())
}
