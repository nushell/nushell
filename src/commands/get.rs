use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use crate::Text;

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
    if args.len() == 0 {
        if let Some(span) = args.name_span {
            return Err(ShellError::labeled_error(
                "Get requires a field or field path",
                "needs parameter",
                span,
            ));
        } else {
            return Err(ShellError::string("get requires fields."));
        }
    }

    let amount = args.expect_nth(0)?.as_i64();

    // If it's a number, get the row instead of the column
    if let Ok(amount) = amount {
        return Ok(args
            .input
            .skip(amount as u64)
            .take(1)
            .map(|v| ReturnValue::Value(v))
            .boxed());
    }

    let fields: Result<Vec<Text>, _> = args
        .args
        .positional
        .unwrap()
        .iter()
        .map(|a| a.as_string())
        .collect();

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
