use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct SetArgs {
    key: Tagged<String>,
    value: Value,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config set"
    }

    fn signature(&self) -> Signature {
        Signature::build("config set")
            .required("key", SyntaxShape::String, "variable name to set")
            .required("value", SyntaxShape::Any, "value to use")
    }

    fn usage(&self) -> &str {
        "Sets a value in the config"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        set(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Set completion_mode to circular",
            example: "config set [completion_mode circular]",
            result: None,
        }]
    }
}

pub async fn set(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.clone();
    let (SetArgs { key, value }, _) = args.process(&registry).await?;

    // NOTE: None because we are not loading a new config file, we just want to read from the
    // existing config
    let mut result = crate::data::config::read(name_span, &None)?;

    result.insert(key.to_string(), value.clone());

    config::write(&result, &None)?;

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::Row(result.into()).into_value(&value.tag),
    )))
}
