use crate::prelude::*;
use chrono_tz::TZ_VARIANTS;
use indexmap::IndexMap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, ReturnSuccess, Signature, UntaggedValue};

pub struct Date;

#[async_trait]
impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date list-timezone"
    }

    fn signature(&self) -> Signature {
        Signature::build("date list-timezone")
    }

    fn usage(&self) -> &str {
        "List supported time zones."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        list_timezone(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List all supported time zones",
                example: "date list-timezone",
                result: None,
            },
            Example {
                description: "List all supported European time zones",
                example: "date list-timezone | where timezone =~ Europe",
                result: None,
            },
        ]
    }
}

async fn list_timezone(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.call_info.name_tag.clone();

    let list = TZ_VARIANTS.iter().map(move |tz| {
        let mut entries = IndexMap::new();

        entries.insert(
            "timezone".to_string(),
            UntaggedValue::string(tz.name()).into_value(&tag),
        );

        Ok(ReturnSuccess::Value(
            UntaggedValue::Row(Dictionary { entries }).into_value(&tag),
        ))
    });

    Ok(futures::stream::iter(list).to_output_stream())
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
