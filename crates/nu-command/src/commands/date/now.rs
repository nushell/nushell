use crate::prelude::*;
use chrono::{DateTime, Local};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

pub struct Date;

#[async_trait]
impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date now"
    }

    fn signature(&self) -> Signature {
        Signature::build("date now")
    }

    fn usage(&self) -> &str {
        "Get the current date."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        now(args).await
    }
}

pub async fn now(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.call_info.name_tag.clone();

    let now: DateTime<Local> = Local::now();

    let value = UntaggedValue::date(now.with_timezone(now.offset())).into_value(&tag);

    Ok(OutputStream::one(value))
}

#[cfg(test)]
mod tests {
    use super::Date;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Date {})
    }
}
