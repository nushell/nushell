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
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, reject)?.run()
    }

    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject").rest()
    }
}

fn reject(
    RejectArgs { rest: fields }: RejectArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

    let stream = input
        .values
        .map(move |item| reject_fields(&item, &fields, item.tag()).into_tagged_value());

    Ok(stream.from_input_stream())
}
