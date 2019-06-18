use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn split_row(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Split-row needs more information",
            "needs parameter (eg split-row \"\\n\")",
            args.name_span,
        ));
    }

    let input = args.input;
    let span = args.name_span;
    let args = args.positional;

    let stream = input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = args[0].as_string().unwrap().replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

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
