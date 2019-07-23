use crate::context::CommandRegistry;
use crate::errors::ShellError;
use crate::object::base::select_fields;
use crate::prelude::*;

pub fn pick(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let len = args.len();
    let span = args.name_span();
    let (input, args) = args.parts();

    if len == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Pick requires fields",
            "needs parameter",
            span,
        ));
    }

    let fields: Result<Vec<String>, _> = args
        .positional
        .iter()
        .flatten()
        .map(|a| a.as_string())
        .collect();

    let fields = fields?;

    let objects = input
        .values
        .map(move |value| select_fields(&value.item, &fields, value.span));

    Ok(objects.from_input_stream())
}
