use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::object::Value;
use crate::prelude::*;

pub fn reject(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        if let Some(span) = args.name_span {
            return Err(ShellError::labeled_error(
                "Reject requires fields",
                "needs parameter",
                span,
            ));
        } else {
            return Err(ShellError::string("reject requires fields."));
        }
    }

    let fields: Result<Vec<String>, _> = args.positional.iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let stream = args
        .input
        .map(move |item| Value::Object(reject_fields(&item, &fields)))
        .map(|item| ReturnValue::Value(item));

    Ok(stream.boxed())
}
