use crate::prelude::*;
use chrono::{DateTime, Utc};
use nu_errors::ShellError;

use crate::commands::date::utils::date_to_value;
use crate::commands::WholeStreamCommand;
use nu_protocol::Signature;

pub struct Date;

#[async_trait]
impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date utc"
    }

    fn signature(&self) -> Signature {
        Signature::build("date utc")
    }

    fn usage(&self) -> &str {
        "return the current date in utc."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        utc(args, registry).await
    }
}

pub async fn utc(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;
    let tag = args.call_info.name_tag.clone();

    let no_fmt = "".to_string();

    let value = {
        let local: DateTime<Utc> = Utc::now();
        date_to_value(local, tag, no_fmt)
    };

    Ok(OutputStream::one(value))
}
