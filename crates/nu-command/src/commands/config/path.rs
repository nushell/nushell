use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};

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
    Ok(OutputStream::one(ReturnSuccess::value(
        match args.scope.get_var("config-path") {
            Some(
                path
                @
                Value {
                    value: UntaggedValue::Primitive(Primitive::FilePath(_)),
                    ..
                },
            ) => path,
            _ => UntaggedValue::Primitive(Primitive::FilePath(nu_data::config::default_path()?))
                .into_value(args.call_info.name_tag),
        },
    )))
}
