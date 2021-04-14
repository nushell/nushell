use crate::prelude::*;
use chrono::{DateTime, Local};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue};

pub struct Date;

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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        now(args)
    }
}

pub fn now(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let args = args.evaluate_once()?;
    let tag = args.call_info.name_tag.clone();

    let now: DateTime<Local> = Local::now();

    let mut indexmap = IndexMap::new();
    indexmap.insert(
        "current date".to_string(),
        UntaggedValue::string(now.with_timezone(now.offset()).to_string()).into_value(&tag),
    );
    let value = UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag);

    Ok(ActionStream::one(value))
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
