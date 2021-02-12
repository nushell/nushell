use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, ShellTypeName, Signature, SyntaxShape, UntaggedValue,
    Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

use chrono::{DateTime, FixedOffset, LocalResult, Offset, TimeZone};

#[derive(Deserialize)]
struct Arguments {
    format: Option<Tagged<String>>,
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str to-datetime"
    }

    fn signature(&self) -> Signature {
        Signature::build("str to-datetime")
            .named(
                "format",
                SyntaxShape::String,
                "Specify date and time formatting",
                Some('f'),
            )
            .rest(
                SyntaxShape::ColumnPath,
                "optionally convert text into datetime by column paths",
            )
    }

    fn usage(&self) -> &str {
        "converts text into datetime"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert to datetime",
                example: "echo '16.11.1984 8:00 am +0000' | str to-datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime",
                example: "echo '2020-08-04T16:39:18+00:00' | str to-datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime using a custom format",
                example: "echo '20200904_163918+0000' | str to-datetime -f '%Y%m%d_%H%M%S%z'",
                result: None,
            },
        ]
    }
}

#[derive(Clone)]
struct DatetimeFormat(String);

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { format, rest }, input) = args.process().await?;

    let column_paths: Vec<_> = rest;

    let options = if let Some(Tagged { item: fmt, .. }) = format {
        Some(DatetimeFormat(fmt))
    } else {
        None
    };

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &options, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let options = options.clone();

                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &options, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(
    input: &Value,
    options: &Option<DatetimeFormat>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let out = match options {
                Some(dt) => match DateTime::parse_from_str(s, &dt.0) {
                    Ok(d) => UntaggedValue::date(d),
                    Err(reason) => {
                        return Err(ShellError::labeled_error(
                            format!("could not parse as datetime using format '{}'", dt.0),
                            reason.to_string(),
                            tag.into().span,
                        ))
                    }
                },
                None => match dtparse::parse(s) {
                    Ok((native_dt, fixed_offset)) => {
                        let offset = match fixed_offset {
                            Some(fo) => fo,
                            None => FixedOffset::east(0).fix(),
                        };
                        match offset.from_local_datetime(&native_dt) {
                            LocalResult::Single(d) => UntaggedValue::date(d),
                            LocalResult::Ambiguous(d, _) => UntaggedValue::date(d),
                            LocalResult::None => {
                                return Err(ShellError::labeled_error(
                                    "could not convert to a timezone-aware datetime",
                                    "local time representation is invalid",
                                    tag.into().span,
                                ))
                            }
                        }
                    }
                    Err(reason) => {
                        return Err(ShellError::labeled_error(
                            "could not parse as datetime",
                            reason.to_string(),
                            tag.into().span,
                        ))
                    }
                },
            };

            Ok(out.into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.into().span,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{action, DatetimeFormat, SubCommand};
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn takes_a_date_format() {
        let date_str = string("16.11.1984 8:00 am +0000");

        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));

        let actual = action(&date_str, &fmt_options, Tag::unknown()).unwrap();

        match actual.value {
            UntaggedValue::Primitive(Primitive::Date(_)) => {}
            _ => panic!("Didn't convert to date"),
        }
    }

    #[test]
    fn takes_iso8601_date_format() {
        let date_str = string("2020-08-04T16:39:18+00:00");
        let actual = action(&date_str, &None, Tag::unknown()).unwrap();
        match actual.value {
            UntaggedValue::Primitive(Primitive::Date(_)) => {}
            _ => panic!("Didn't convert to date"),
        }
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_datetimelike_string() {
        let date_str = string("16.11.1984 8:00 am Oops0000");

        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));

        let actual = action(&date_str, &fmt_options, Tag::unknown());

        assert!(actual.is_err());
    }
}
