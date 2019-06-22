use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::trace;

// TODO: "Amount remaining" wrapper

pub fn split_column(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Split-column needs more information",
            "needs parameter (eg split-column \",\")",
            args.name_span,
        ));
    }
    let input = args.input;
    let span = args.name_span;
    let args = args.positional;

    Ok(input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = args[0].as_string().unwrap().replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                trace!("split result = {:?}", split_result);

                // If they didn't provide column names, make up our own
                if (args.len() - 1) == 0 {
                    let mut gen_columns = vec![];
                    for i in 0..split_result.len() {
                        gen_columns.push(format!("Column{}", i + 1));
                    }

                    let mut dict = crate::object::Dictionary::default();
                    for (k, v) in split_result.iter().zip(gen_columns.iter()) {
                        dict.add(
                            v.clone(),
                            Value::Primitive(Primitive::String(k.to_string())),
                        );
                    }
                    ReturnValue::Value(Value::Object(dict))
                } else if split_result.len() == (args.len() - 1) {
                    let mut dict = crate::object::Dictionary::default();
                    for (k, v) in split_result.iter().zip(args.iter().skip(1)) {
                        dict.add(
                            v.as_string().unwrap(),
                            Value::Primitive(Primitive::String(k.to_string())),
                        );
                    }
                    ReturnValue::Value(Value::Object(dict))
                } else {
                    let mut dict = crate::object::Dictionary::default();
                    for k in args.iter().skip(1) {
                        dict.add(
                            k.as_string().unwrap().trim(),
                            Value::Primitive(Primitive::String("".to_string())),
                        );
                    }
                    ReturnValue::Value(Value::Object(dict))
                }
            }
            _ => ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )))),
        })
        .boxed())
}
