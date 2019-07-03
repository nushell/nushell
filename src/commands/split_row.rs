use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::Spanned;
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn split_row(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let positional: Vec<Spanned<Value>> = args.positional_iter().cloned().collect();
    let span = args.name_span;

    if positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Split-row needs more information",
            "needs parameter (eg split-row \"\\n\")",
            args.name_span,
        ));
    }

    let input = args.input;

    let stream = input
        .values
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = positional[0].as_string().unwrap().replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnSuccess::value(Value::Primitive(Primitive::String(
                        s.into(),
                    ))));
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
