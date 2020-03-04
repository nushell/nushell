use crate::commands::WholeStreamCommand;
use crate::data::base::reject_fields;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
pub struct RejectArgs {
    rest: Vec<Tagged<String>>,
}

pub struct Reject;

impl WholeStreamCommand for Reject {
    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject").rest(SyntaxShape::Member, "the names of columns to remove")
    }

    fn usage(&self) -> &str {
        "Remove the given columns from the table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, reject)?.run()
    }
}

fn reject(
    RejectArgs { rest: fields }: RejectArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if fields.is_empty() {
        return Err(ShellError::labeled_error(
            "Reject requires fields",
            "needs parameter",
            name,
        ));
    }

    let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

    let stream = input
        .values
        .map(move |item| reject_fields(&item, &fields, &item.tag));

    Ok(stream.from_input_stream())
}
