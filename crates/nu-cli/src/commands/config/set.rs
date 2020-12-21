use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

#[derive(Deserialize)]
pub struct SetArgs {
    path: ColumnPath,
    value: Value,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "config set"
    }

    fn signature(&self) -> Signature {
        Signature::build("config set")
            .required("key", SyntaxShape::ColumnPath, "variable name to set")
            .required("value", SyntaxShape::Any, "value to use")
    }

    fn usage(&self) -> &str {
        "Sets a value in the config"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        set(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set auto pivoting",
                example: "config set pivot_mode always",
                result: None,
            },
            Example {
                description: "Set line editor options",
                example: "config set line_editor [[edit_mode, completion_type]; [emacs circular]]",
                result: None,
            },
            Example {
                description: "Set coloring options",
                example: "config set color_config [[header_align header_bold]; [left $true]]",
                result: None,
            },
            Example {
                description: "Set nested options",
                example: "config set color_config.header_color white",
                result: None,
            },
        ]
    }
}

pub async fn set(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();
    let (SetArgs { path, mut value }, _) = args.process().await?;

    // NOTE: None because we are not loading a new config file, we just want to read from the
    // existing config
    let raw_entries = nu_data::config::read(&name_tag, &None)?;
    let configuration = UntaggedValue::row(raw_entries).into_value(&name_tag);

    if let UntaggedValue::Table(rows) = &value.value {
        if rows.len() == 1 && rows[0].is_row() {
            value = rows[0].clone();
        }
    }

    match configuration.forgiving_insert_data_at_column_path(&path, value) {
        Ok(Value {
            value: UntaggedValue::Row(changes),
            ..
        }) => {
            config::write(&changes.entries, &None)?;

            Ok(OutputStream::one(ReturnSuccess::value(
                UntaggedValue::Row(changes).into_value(name_tag),
            )))
        }
        Ok(_) => Ok(OutputStream::empty()),
        Err(reason) => Err(reason),
    }
}
