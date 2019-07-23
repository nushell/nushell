use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::Spanned;
use crate::prelude::*;
use log::trace;

pub fn split_row(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let len = args.len();
    let (input, args) = args.parts();

    let positional: Vec<Spanned<Value>> = args.positional.iter().flatten().cloned().collect();

    if len == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Split-row needs more information",
            "needs parameter (eg split-row \"\\n\")",
            span,
        ));
    }

    let stream = input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = positional[0].as_string().unwrap().replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                let mut result = VecDeque::new();
                for s in split_result {
                    result.push_back(ReturnSuccess::value(
                        Value::Primitive(Primitive::String(s.into())).spanned(v.span),
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
