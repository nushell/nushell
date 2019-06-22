use crate::errors::ShellError;
use crate::object::base::select_fields;
use crate::object::Value;
use crate::prelude::*;

pub fn pick(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        if let Some(span) = args.name_span {
            return Err(ShellError::labeled_error(
                "Pick requires fields",
                "needs parameter",
                span,
            ));
        } else {
            return Err(ShellError::string("pick requires fields."));
        }
    }

    let fields: Result<Vec<String>, _> = args.positional_iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let objects = args
        .input
        .map(move |item| Value::Object(select_fields(&item, &fields)))
        .map(|item| ReturnValue::Value(item));

    let stream = Pin::new(Box::new(objects));
    Ok(stream)
}
