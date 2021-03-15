use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};

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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clear(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the config (be careful!)",
            example: "config clear",
            result: None,
        }]
    }
}

pub async fn clear(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.clone();

    let path = match args.scope.get_var("config-path") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::FilePath(path)),
            ..
        }) => Some(path),
        _ => nu_data::config::default_path().ok(),
    };

    let mut result = nu_data::config::read(name_span, &path)?;

    result.clear();

    config::write(&result, &path)?;

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::Row(result.into()).into_value(args.call_info.name_tag),
    )))
}
