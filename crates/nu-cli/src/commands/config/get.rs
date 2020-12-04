use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

#[derive(Deserialize)]
pub struct GetArgs {
    path: ColumnPath,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config get"
    }

    fn signature(&self) -> Signature {
        Signature::build("config get").required(
            "get",
            SyntaxShape::ColumnPath,
            "value to get from the config",
        )
    }

    fn usage(&self) -> &str {
        "Gets a value from the config"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        get(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the current startup commands",
            example: "config get startup",
            result: None,
        }]
    }
}

pub async fn get(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();
    let (GetArgs { path }, _) = args.process().await?;

    // NOTE: None because we are not loading a new config file, we just want to read from the
    // existing config
    let result = UntaggedValue::row(nu_data::config::read(&name_tag, &None)?).into_value(&name_tag);

    let value = crate::commands::get::get_column_path(&path, &result)?;

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
        x => OutputStream::one(ReturnSuccess::value(x)),
    })
}
