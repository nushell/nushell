use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::Spanned;
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn split_row(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let positional: Vec<Spanned<Value>> = args.positional_iter().cloned().collect();

    if positional.len() == 0 {
        if let Some(span) = args.name_span {
            return Err(ShellError::labeled_error(
                "split-row requires arguments",
                "needs parameter",
                span,
            ));
        } else {
            return Err(ShellError::string("split-row requires arguments."));
        }
    }

    let input = args.input;

    let stream = input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = positional[0].as_string().unwrap().replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
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
                let result = VecDeque::new();
                result
            }
        })
        .flatten();

    Ok(stream.boxed())
}
