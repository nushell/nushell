use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::prelude::*;

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
        Signature::build("reject").rest(SyntaxType::Member)
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
    if fields.len() == 0 {
        return Err(ShellError::labeled_error(
            "Reject requires fields",
            "needs parameter",
            name,
        ));
    }

    let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

    let stream = input
        .values
        .map(move |item| reject_fields(&item, &fields, item.tag()).into_tagged_value());

    Ok(stream.from_input_stream())
}
