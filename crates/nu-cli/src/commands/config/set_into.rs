use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct SetIntoArgs {
    set_into: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config set_into"
    }

    fn signature(&self) -> Signature {
        Signature::build("config set_into").required(
            "set_into",
            SyntaxShape::String,
            "sets a variable from values in the pipeline",
        )
    }

    fn usage(&self) -> &str {
        "Sets a value in the config"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        set_into(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Store the contents of the pipeline as a path",
            example: "echo ['/usr/bin' '/bin'] | config set_into path",
            result: None,
        }]
    }
}

pub async fn set_into(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.clone();
    let name = args.call_info.name_tag.clone();

    let (SetIntoArgs { set_into: v }, input) = args.process(&registry).await?;

    // NOTE: None because we are not loading a new config file, we just want to read from the
    // existing config
    let mut result = crate::data::config::read(name_span, &None)?;

    // In the original code, this is set to `Some` if the `load flag is set`
    let configuration = None;

    let rows: Vec<Value> = input.collect().await;
    let key = v.to_string();

    Ok(if rows.is_empty() {
        return Err(ShellError::labeled_error(
            "No values given for set_into",
            "needs value(s) from pipeline",
            v.tag(),
        ));
    } else if rows.len() == 1 {
        // A single value
        let value = &rows[0];

        result.insert(key, value.clone());

        config::write(&result, &configuration)?;

        OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(name),
        ))
    } else {
        // Take in the pipeline as a table
        let value = UntaggedValue::Table(rows).into_value(name.clone());

        result.insert(key, value);

        config::write(&result, &configuration)?;

        OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(name),
        ))
    })
}
