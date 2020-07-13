use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config clear"
    }

    fn signature(&self) -> Signature {
        Signature::build("config clear")
    }

    fn usage(&self) -> &str {
        "clear the config"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        clear(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the config (be careful!)",
            example: "config clear",
            result: None,
        }]
    }
}

pub async fn clear(
    args: CommandArgs,
    _registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.clone();

    // NOTE: None because we are not loading a new config file, we just want to read from the
    // existing config
    let mut result = crate::data::config::read(name_span, &None)?;

    result.clear();

    config::write(&result, &None)?;

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::Row(result.into()).into_value(args.call_info.name_tag),
    )))
}
