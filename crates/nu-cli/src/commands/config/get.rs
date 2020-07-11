use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct GetArgs {
    get: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config get"
    }

    fn signature(&self) -> Signature {
        Signature::build("config get").required(
            "get",
            SyntaxShape::Any,
            "value to get from the config",
        )
    }

    fn usage(&self) -> &str {
        "Gets a value from the config"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        get(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the current startup commands",
            example: "config get startup",
            result: None,
        }]
    }
}

pub async fn get(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.clone();
    let (GetArgs { get }, _) = args.process(&registry).await?;

    // NOTE: None because we are not loading a new config file, we just want to read from the
    // existing config
    let result = crate::data::config::read(name_span, &None)?;

    let key = get.to_string();
    let value = result
        .get(&key)
        .ok_or_else(|| ShellError::labeled_error("Missing key in config", "key", get.tag()))?;

    Ok(match value {
        Value {
            value: UntaggedValue::Table(list),
            ..
        } => {
            let list: Vec<_> = list
                .iter()
                .map(|x| ReturnSuccess::value(x.clone()))
                .collect();

            futures::stream::iter(list).to_output_stream()
        }
        x => {
            let x = x.clone();
            OutputStream::one(ReturnSuccess::value(x))
        }
    })
}
