use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::prelude::*;

pub fn reject(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let name_span = args.name_span();
    let args = args.evaluate_once(registry)?;
    let len = args.len();
    let span = args.name_span();
    let (input, args) = args.parts();

    if len == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Reject requires fields",
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

    let stream = input.values.map(move |item| {
        reject_fields(&item, &fields, item.span)
            .into_spanned_value()
            .spanned(name_span)
    });

    Ok(stream.from_input_stream())
}
