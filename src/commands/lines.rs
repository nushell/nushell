use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn lines(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let span = args.name_span;

    let stream = input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = "\n";
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnValue::Value(Value::Primitive(Primitive::String(
                        s.into(),
                    ))));
                }
                result
            }
            _ => {
                let mut result = VecDeque::new();
                result.push_back(ReturnValue::Value(Value::Error(Box::new(
                    ShellError::maybe_labeled_error(
                        "Expected string values from pipeline",
                        "expects strings from pipeline",
                        span,
                    ),
                ))));
                result
            }
        })
        .flatten();

    Ok(stream.boxed())
}
