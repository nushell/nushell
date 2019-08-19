use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::prelude::*;

pub struct Reject;

impl WholeStreamCommand for Reject {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        reject(args, registry)
    }

    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject").required("fields", SyntaxType::Any)
    }
}

fn reject(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let (input, args) = args.parts();

    let fields: Result<Vec<String>, _> = args
        .positional
        .iter()
        .flatten()
        .map(|a| a.as_string())
        .collect();

    let fields = fields?;

    let stream = input
        .values
        .map(move |item| reject_fields(&item, &fields, item.tag()).into_tagged_value());

    Ok(stream.from_input_stream())
}
