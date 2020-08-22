use crate::prelude::*;
use chrono::{DateTime, Local};
use nu_errors::ShellError;

use crate::commands::date::utils::{date_to_value, date_to_value_raw};
use crate::commands::WholeStreamCommand;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Date;

#[derive(Deserialize)]
pub struct FormatArgs {
    format: Tagged<String>,
    raw: Option<bool>,
}

#[async_trait]
impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date format"
    }

    fn signature(&self) -> Signature {
        Signature::build("date format")
            .required("format", SyntaxShape::String, "strftime format")
            .switch("raw", "print date without tables", Some('r'))
    }

    fn usage(&self) -> &str {
        "format the current date using the given format string."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        format(args, registry).await
    }
}

pub async fn format(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let tag = args.call_info.name_tag.clone();
    let (FormatArgs { format, raw }, _) = args.process(&registry).await?;

    let dt_fmt = format.to_string();

    let value = {
        let local: DateTime<Local> = Local::now();
        if let Some(true) = raw {
            UntaggedValue::string(date_to_value_raw(local, dt_fmt)).into_untagged_value()
        } else {
            date_to_value(local, tag, dt_fmt)
        }
    };

    Ok(OutputStream::one(value))
}
