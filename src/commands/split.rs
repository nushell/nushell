use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

// TODO: "Amount remaining" wrapper

pub fn split(args: CommandArgs) -> Result<OutputStream, ShellError> {
    //let splitter = args.args[0].as_string()?;
    let input = args.input;
    let args = args.args;

    Ok(input
        .map(move |v| match v {
            Value::Primitive(Primitive::String(s)) => {
                let splitter = args[0].as_string().unwrap();
                let split_result: Vec<_> = s.split(&splitter).filter(|s| s.trim() != "").collect();

                if split_result.len() == (args.len() - 1) {
                    let mut dict = crate::object::Dictionary::default();
                    for (k, v) in split_result.iter().zip(args.iter().skip(1)) {
                        dict.add(v.as_string().unwrap(), Value::Primitive(Primitive::String(k.to_string())));
                    }
                    ReturnValue::Value(Value::Object(dict))
                } else {
                    let mut dict = crate::object::Dictionary::default();
                    for k in args.iter().skip(1) {
                        dict.add(k.as_string().unwrap().trim(), Value::Primitive(Primitive::String("".to_string())));
                    }
                    ReturnValue::Value(Value::Object(dict))
                }
            }
            _ => ReturnValue::Value(Value::Object(crate::object::Dictionary::default())),
        })
        .boxed())
}
