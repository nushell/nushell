use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Primitive, Signature, UntaggedValue, Value};

pub struct Date;

impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date humanize"
    }

    fn signature(&self) -> Signature {
        Signature::build("date humanize").switch("table", "print date in a table", Some('t'))
    }

    fn usage(&self) -> &str {
        "Print a 'humanized' format for the date, relative to now."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        humanize(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Humanize the current date",
            example: "date now | date humanize",
            result: None,
        }]
    }
}

pub fn humanize(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let table: Option<bool> = args.get_flag("table")?;
    let input = args.input;

    Ok(input
        .map(move |value| match value {
            Value {
                value: UntaggedValue::Primitive(dt @ Primitive::Date(_)),
                ..
            } => {
                let output = nu_protocol::format_primitive(&dt, None);
                let value = if table.is_some() {
                    let mut indexmap = IndexMap::new();
                    indexmap.insert(
                        "formatted".to_string(),
                        UntaggedValue::string(&output).into_value(&tag),
                    );

                    UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag)
                } else {
                    UntaggedValue::string(&output).into_value(&tag)
                };

                Ok(value)
            }
            _ => Err(ShellError::labeled_error(
                "Expected a date from pipeline",
                "requires date input",
                &tag,
            )),
        })
        .into_input_stream())
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
