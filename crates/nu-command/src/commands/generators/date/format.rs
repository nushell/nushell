use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::fmt::{self, write};

pub struct Date;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        format(args)
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

pub fn format(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let format: Tagged<String> = args.req(0)?;
    let table: Option<bool> = args.get_flag("table")?;

    let input = if args.input.is_empty() {
        InputStream::one(super::now::date_now(&tag))
    } else {
        args.input
    };

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
