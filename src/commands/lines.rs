use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;

pub fn lines(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let span = args.name_span;

    let stream = input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let split_result: Vec<_> = s.lines().filter(|s| s.trim() != "").collect();

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnValue::Value(Value::Primitive(Primitive::String(
                        s.to_string(),
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
