use crate::errors::ShellError;
use crate::object::Value;
use crate::parser::lexer::Span;
use crate::prelude::*;

fn get_member(path: &str, span: Span, obj: &Value) -> Option<Value> {
    let mut current = obj;
    for p in path.split(".") {
        match current.get_data_by_key(p) {
            Some(v) => current = v,
            None => {
                return Some(Value::Error(Box::new(ShellError::labeled_error(
                    "Unknown field",
                    "object missing field",
                    span,
                ))));
            }
        }
    }

    Some(current.copy())
}

pub fn get(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Get requires a field or field path",
            "needs parameter",
            args.name_span,
        ));
    }

    let amount = args.positional[0].as_i64();

    // If it's a number, get the row instead of the column
    if let Ok(amount) = amount {
        return Ok(args
            .input
            .skip(amount as u64)
            .take(1)
            .map(|v| ReturnValue::Value(v))
            .boxed());
    }

    let fields: Result<Vec<(String, Span)>, _> = args
        .positional
        .iter()
        .map(|a| (a.as_string().map(|x| (x, a.span))))
        .collect();
    let fields = fields?;

    let stream = args
        .input
        .map(move |item| {
            let mut result = VecDeque::new();
            for field in &fields {
                match get_member(&field.0, field.1, &item) {
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
