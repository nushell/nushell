use crate::errors::ShellError;
use crate::object::base::select_fields;
use crate::object::Value;
use crate::prelude::*;

pub fn pick(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Pick requires fields",
            "needs parameter",
            args.name_span,
        ));
    }

    let fields: Result<Vec<String>, _> = args.positional.iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let objects = args
        .input
        .map(move |item| Value::Object(select_fields(&item, &fields)))
        .map(|item| ReturnValue::Value(item));

    let stream = Pin::new(Box::new(objects));
    Ok(stream)
}
