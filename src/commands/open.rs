use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use std::path::PathBuf;

pub fn open(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let cwd = args.env.lock().unwrap().cwd().to_path_buf();
    let mut full_path = PathBuf::from(cwd);
    match &args.positional[0] {
        Value::Primitive(Primitive::String(s)) => full_path.push(s),
        _ => {}
    }

    let contents = std::fs::read_to_string(&full_path).unwrap();

    let mut stream = VecDeque::new();

    let open_raw = match args.positional.get(1) {
        Some(Value::Primitive(Primitive::String(s))) if s == "--raw" => true,
        _ => false,
    };

    match full_path.extension() {
        Some(x) if x == "toml" && !open_raw => {
            stream.push_back(ReturnValue::Value(crate::commands::from_toml::from_toml_string_to_value(contents)));
        }
        Some(x) if x == "json" && !open_raw => {
            stream.push_back(ReturnValue::Value(crate::commands::from_json::from_json_string_to_value(contents)));
        }
        _ => {
            stream.push_back(ReturnValue::Value(Value::Primitive(Primitive::String(
                contents,
            ))));
        }
    }

    Ok(stream.boxed())
}
