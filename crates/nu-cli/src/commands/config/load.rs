use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct LoadArgs {
    load: Tagged<PathBuf>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config load"
    }

    fn signature(&self) -> Signature {
        Signature::build("config load").required(
            "load",
            SyntaxShape::Path,
            "Path to load the config from",
        )
    }

    fn usage(&self) -> &str {
        "Loads the config from the path given"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        set(args, registry).await
    }
}

pub async fn set(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let name_span = args.call_info.name_tag.clone();
    let (LoadArgs { load }, _) = args.process(&registry).await?;

    let configuration = load.item().clone();

    let result = crate::data::config::read(name_span, &Some(configuration))?;

    Ok(futures::stream::iter(vec![ReturnSuccess::value(
        UntaggedValue::Row(result.into()).into_value(name),
    )])
    .to_output_stream())
}
