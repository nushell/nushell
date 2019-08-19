use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::errors::ShellError;
use crate::object::base::select_fields;
use crate::prelude::*;

pub struct Pick;

impl WholeStreamCommand for Pick {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        pick(args, registry)
    }

    fn name(&self) -> &str {
        "pick"
    }

    fn signature(&self) -> Signature {
        Signature::build("pick").required("fields", SyntaxType::Any)
    }
}

fn pick(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let (input, args) = args.parts();

    let fields: Result<Vec<String>, _> = args
        .positional
        .iter()
        .flatten()
        .map(|a| a.as_string())
        .collect();

    let fields = fields?;

    let objects = input
        .values
        .map(move |value| select_fields(&value.item, &fields, value.tag()));

    Ok(objects.from_input_stream())
}
