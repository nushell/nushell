use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

fn get_member(path: &str, obj: &Value) -> Option<Value> {
    let mut current = obj;
    for p in path.split(".") {
        match current.get_data_by_key(p) {
            Some(v) => current = v,
            None => {
                return Some(Value::Error(Box::new(ShellError::string(format!(
                    "Object field name not found: {}",
                    p
                )))))
            }
        }
    }

    Some(current.copy())
}

pub fn get(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.is_empty() {
        return Err(ShellError::string("select requires a field"));
    }

    let fields: Result<Vec<String>, _> = args.positional.iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let stream = args
        .input
        .map(move |item| {
            let mut result = VecDeque::new();
            for field in &fields {
                match get_member(field, &item) {
                    Some(Value::List(l)) => {
                        for item in l {
                            result.push_back(ReturnValue::Value(item.copy()));
                        }
                    }
                    Some(x) => result.push_back(ReturnValue::Value(x.copy())),
                    None => {}
                }
            }

            result
        })
        .flatten();

    Ok(stream.boxed())
}
