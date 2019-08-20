use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::errors::ShellError;
use crate::object::base::select_fields;
use crate::prelude::*;

#[derive(Deserialize)]
struct PickArgs {
    rest: Vec<Tagged<String>>,
}

pub struct Pick;

impl WholeStreamCommand for Pick {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, pick)?.run()
    }

    fn name(&self) -> &str {
        "pick"
    }

    fn signature(&self) -> Signature {
        Signature::build("pick").rest()
    }
}

fn pick(
    PickArgs { rest: fields }: PickArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let fields: Vec<_> = fields.iter().map(|f| f.item.clone()).collect();

    let objects = input
        .values
        .map(move |value| select_fields(&value.item, &fields, value.tag()));

    Ok(objects.from_input_stream())
}
