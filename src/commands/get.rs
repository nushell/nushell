use crate::errors::ShellError;
use crate::object::Value;
use crate::parser::Span;
use crate::prelude::*;

fn get_member(path: &str, span: Span, obj: &Spanned<Value>) -> Result<Spanned<Value>, ShellError> {
    let mut current = obj;
    for p in path.split(".") {
        match current.get_data_by_key(p) {
            Some(v) => current = v,
            None => {
                return Err(ShellError::labeled_error(
                    "Unknown field",
                    "object missing field",
                    span,
                ));
            }
        }
    }

    Ok(current.clone())
}

pub fn get(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Get requires a field or field path",
            "needs parameter",
            args.call_info.name_span,
        ));
    }

    let amount = args.expect_nth(0)?.as_i64();

    // If it's a number, get the row instead of the column
    if let Ok(amount) = amount {
        return Ok(args
            .input
            .values
            .skip(amount as u64)
            .take(1)
            .from_input_stream());
    }

    let fields: Result<Vec<(String, Span)>, _> = args
        .positional_iter()
        .map(|a| (a.as_string().map(|x| (x, a.span))))
        .collect();

    let fields = fields?;

    let stream = args
        .input
        .values
        .map(move |item| {
            let mut result = VecDeque::new();
            for field in &fields {
                match get_member(&field.0, field.1, &item) {
                    Ok(Spanned {
                        item: Value::List(l),
                        ..
                    }) => {
                        for item in l {
                            result.push_back(ReturnSuccess::value(item.clone()));
                        }
                    }
                    Ok(x) => result.push_back(ReturnSuccess::value(x.clone())),
                    Err(x) => result.push_back(Err(x)),
                }
            }

            result
        })
        .flatten();

    Ok(stream.to_output_stream())
}
