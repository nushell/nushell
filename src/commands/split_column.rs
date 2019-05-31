use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use log::debug;

// TODO: "Amount remaining" wrapper

pub fn split_column(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let args = args.args;

    Ok(input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = args[0].as_string().unwrap().replace("\\n", "\n");
                debug!("splitting with {:?}", splitter);
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                debug!("split result = {:?}", split_result);

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
            _ => ReturnValue::Value(Value::Object(crate::object::Dictionary::default())),
        })
        .boxed())
}
