use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    Dictionary, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use std::fmt::{self, write};

pub struct Date;

#[derive(Deserialize)]
pub struct FormatArgs {
    format: Tagged<String>,
    table: bool,
}

#[async_trait]
impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date format"
    }

    fn signature(&self) -> Signature {
        Signature::build("date format")
            .required("format", SyntaxShape::String, "strftime format")
            .switch("table", "print date in a table", Some('t'))
    }

    fn usage(&self) -> &str {
        "Format a given date using the given format string."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        format(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Format the current date",
                example: "date now | date format '%Y.%m.%d_%H %M %S,%z'",
                result: None,
            },
            Example {
                description: "Format the current date and print in a table",
                example: "date now | date format -t '%Y-%m-%d_%H:%M:%S %z'",
                result: None,
            },
        ]
    }
}

pub async fn format(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let (FormatArgs { format, table }, input) = args.process().await?;

    Ok(input
        .map(move |value| match value {
            Value {
                value: UntaggedValue::Primitive(Primitive::Date(dt)),
                ..
            } => {
                let mut output = String::new();
                if let Err(fmt::Error) =
                    write(&mut output, format_args!("{}", dt.format(&format.item)))
                {
                    Err(ShellError::labeled_error(
                        "The date format is invalid",
                        "invalid strftime format",
                        &format.tag,
                    ))
                } else {
                    let value = if table {
                        let mut indexmap = IndexMap::new();
                        indexmap.insert(
                            "formatted".to_string(),
                            UntaggedValue::string(&output).into_value(&tag),
                        );

                        UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag)
                    } else {
                        UntaggedValue::string(&output).into_value(&tag)
                    };

                    ReturnSuccess::value(value)
                }
            }
            _ => Err(ShellError::labeled_error(
                "Expected a date from pipeline",
                "requires date input",
                &tag,
            )),
        })
        .to_output_stream())
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
