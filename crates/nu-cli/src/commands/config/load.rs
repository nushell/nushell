use crate::prelude::*;
use nu_engine::WholeStreamCommand;
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
            SyntaxShape::FilePath,
            "Path to load the config from",
        )
    }

    fn usage(&self) -> &str {
        "Loads the config from the path given"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        set(args).await
    }
}

pub async fn set(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let name_span = args.call_info.name_tag.clone();
    let (LoadArgs { load }, _) = args.process().await?;

    let configuration = load.item().clone();

    let result = nu_data::config::read(name_span, &Some(configuration))?;

    Ok(futures::stream::iter(vec![ReturnSuccess::value(
        UntaggedValue::Row(result.into()).into_value(name),
    )])
    .to_output_stream())
}
