use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn split_column(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let positional: Vec<_> = args.positional_iter().cloned().collect();
    let span = args.name_span;

    if positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Split-column needs more information",
            "needs parameter (eg split-column \",\")",
            args.name_span,
        ));
    }

    let input = args.input;

    Ok(input
        .values
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = positional[0].as_string().unwrap().replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                // If they didn't provide column names, make up our own
                if (positional.len() - 1) == 0 {
                    let mut gen_columns = vec![];
                    for i in 0..split_result.len() {
                        gen_columns.push(format!("Column{}", i + 1));
                    }

                    let mut dict = crate::object::Dictionary::default();
                    for (&k, v) in split_result.iter().zip(gen_columns.iter()) {
                        dict.add(v.clone(), Value::Primitive(Primitive::String(k.into())));
                    }
                    ReturnSuccess::value(Value::Object(dict))
                } else if split_result.len() == (positional.len() - 1) {
                    let mut dict = crate::object::Dictionary::default();
                    for (&k, v) in split_result.iter().zip(positional.iter().skip(1)) {
                        dict.add(
                            v.as_string().unwrap(),
                            Value::Primitive(Primitive::String(k.into())),
                        );
                    }
                    ReturnSuccess::value(Value::Object(dict))
                } else {
                    let mut dict = crate::object::Dictionary::default();
                    for k in positional.iter().skip(1) {
                        dict.add(
                            k.as_string().unwrap().trim(),
                            Value::Primitive(Primitive::String("".into())),
                        );
                    }
                    ReturnSuccess::value(Value::Object(dict))
                }
            }
            _ => Err(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )),
        })
        .to_output_stream())
}
