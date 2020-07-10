use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "config"
    }

    fn signature(&self) -> Signature {
        Signature::build("config")
    }

    fn usage(&self) -> &str {
        "Configuration management."
    }

    async fn run(
        &self,
        args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let name_span = args.call_info.name_tag.clone();
        let name = args.call_info.name_tag;
        let result = crate::data::config::read(name_span, &None)?;

        Ok(futures::stream::iter(vec![ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(name),
        )])
        .to_output_stream())
    }
}
