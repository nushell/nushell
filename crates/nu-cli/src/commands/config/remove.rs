use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct RemoveArgs {
    remove: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config remove"
    }

    fn signature(&self) -> Signature {
        Signature::build("config remove").required(
            "remove",
            SyntaxShape::Any,
            "remove a value from the config",
        )
    }

    fn usage(&self) -> &str {
        "Removes a value from the config"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        remove(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove the startup commands",
            example: "config remove startup",
            result: None,
        }]
    }
}

pub async fn remove(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.clone();
    let (RemoveArgs { remove }, _) = args.process(&registry).await?;

    let mut result = crate::data::config::read(name_span, &None)?;

    let key = remove.to_string();

    if result.contains_key(&key) {
        result.swap_remove(&key);
        config::write(&result, &None)?;
        Ok(futures::stream::iter(vec![ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(remove.tag()),
        )])
        .to_output_stream())
    } else {
        Err(ShellError::labeled_error(
            "Key does not exist in config",
            "key",
            remove.tag(),
        ))
    }
}
