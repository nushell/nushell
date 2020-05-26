use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, ShellTypeName, Signature, SyntaxShape, UntaggedValue,
    Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

use chrono::DateTime;

#[derive(Deserialize)]
struct Arguments {
    format: Option<Tagged<String>>,
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert to datetime",
            example: "echo '16.11.1984 8:00 am +0000' | str to-datetime",
            result: None,
        }]
    }
}

#[derive(Clone)]
struct DatetimeFormat(String);

fn operate(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let (Arguments { format, rest }, mut input) = args.process(&registry).await?;

        let column_paths: Vec<_> = rest.iter().map(|x| x.clone()).collect();

        let options = if let Some(Tagged { item: fmt, tag }) = format {
            DatetimeFormat(fmt)
        } else {
            DatetimeFormat(String::from("%d.%m.%Y %H:%M %P %z"))
        };

        while let Some(v) = input.next().await {
            if column_paths.is_empty() {
                match action(&v, &options, v.tag()) {
                    Ok(out) => yield ReturnSuccess::value(out),
                    Err(err) => {
                        yield Err(err);
                        return;
                    }
                }
            } else {

                let mut ret = v.clone();

                for path in &column_paths {
                    let options = options.clone();
                    let swapping = ret.swap_data_by_column_path(path, Box::new(move |old| action(old, &options, old.tag())));

                    match swapping {
                        Ok(new_value) => {
                            ret = new_value;
                        }
                        Err(err) => {
                            yield Err(err);
                            return;
                        }
                    }
                }

                yield ReturnSuccess::value(ret);
            }
        }
    };

    Ok(stream.to_output_stream())
}

fn action(
    input: &Value,
    options: &DatetimeFormat,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let dt = &options.0;

            let out = match DateTime::parse_from_str(s, dt) {
                Ok(d) => UntaggedValue::date(d),
                Err(_) => UntaggedValue::string(s),
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
    use super::{action, DatetimeFormat, SubCommand};
    use nu_plugin::test_helpers::value::string;
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn takes_a_date_format() {
        let date_str = string("16.11.1984 8:00 am +0000");

        let fmt_options = DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string());

        let actual = action(&date_str, &fmt_options, Tag::unknown()).unwrap();

        match actual.value {
            UntaggedValue::Primitive(Primitive::Date(_)) => {}
            _ => panic!("Didn't convert to date"),
        }
    }
}
