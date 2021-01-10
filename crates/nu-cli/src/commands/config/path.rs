use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config path"
    }

    fn signature(&self) -> Signature {
        Signature::build("config path")
    }

    fn usage(&self) -> &str {
        "return the path to the config file"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        path(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the path to the current config file",
            example: "config path",
            result: None,
        }]
    }
}

pub async fn path(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let path = config::default_path()?;

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::Primitive(Primitive::FilePath(path)).into_value(args.call_info.name_tag),
    )))
}
