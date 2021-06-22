use crate::prelude::*;
use chrono_tz::TZ_VARIANTS;
use indexmap::IndexMap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue};

pub struct Date;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        list_timezone(args)
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

fn list_timezone(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag;

    let list = TZ_VARIANTS.iter().map(move |tz| {
        let mut entries = IndexMap::new();

        entries.insert(
            "timezone".to_string(),
            UntaggedValue::string(tz.name()).into_value(&tag),
        );

        Ok(UntaggedValue::Row(Dictionary { entries }).into_value(&tag))
    });

    Ok(list.into_iter().into_input_stream())
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
