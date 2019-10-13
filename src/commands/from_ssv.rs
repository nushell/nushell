use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

pub struct FromSSV;

#[derive(Deserialize)]
pub struct FromSSVArgs {
    headerless: bool,
}

const STRING_REPRESENTATION: &str = "from-ssv";

impl WholeStreamCommand for FromSSV {
    fn name(&self) -> &str {
        STRING_REPRESENTATION
    }

    fn signature(&self) -> Signature {
        Signature::build(STRING_REPRESENTATION).switch("headerless")
    }

    fn usage(&self) -> &str {
        "Parse text as whitespace-separated values and create a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_ssv)?.run()
    }
}

fn from_ssv_string_to_value(
    s: &str,
    headerless: bool,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, &str> {
    let mut lines = s.lines();
    let tag = tag.into();

    let headers = lines
        .next()
        .expect("No content.")
        .split_whitespace()
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();

    let header_row = if headerless {
        (0..headers.len())
            .map(|i| format!("Column{}", i + 1))
            .collect::<Vec<String>>()
    } else {
        headers
    };

    let rows = lines
        .map(|l| {
            let mut row = TaggedDictBuilder::new(tag);
            for (column, value) in header_row.iter().zip(l.split_whitespace()) {
                row.insert_tagged(
                    column.to_owned(),
                    Value::Primitive(Primitive::String(String::from(value))).tagged(tag),
                )
            }
            row.into_tagged_value()
        })
        .collect();

    Ok(Tagged::from_item(Value::Table(rows), tag))
}

fn from_ssv(
    FromSSVArgs { headerless }: FromSSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;
        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag);
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                    concat_string.push_str("\n");

                }
                _ => yield Err(ShellError::labeled_error_with_secondary (
                    "Expected a string from pipeline",
                    "requires string input",
                    name,
                    "value originates from here",
                    value_tag
                )),
            }
        }

        match from_ssv_string_to_value(&concat_string, headerless, name) {
            Ok(x) => match x {
                Tagged { item: Value::Table(list), ..} => {
                    for l in list { yield ReturnSuccess::value(l) }
                }
                x => yield ReturnSuccess::value(x)
            },
            Err(_) => if let Some(last_tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as SSV",
                    "input cannot be parsed ssv",
                    name,
                    "value originates from here",
                    last_tag,
                ))
            }
        }
    };

    Ok(stream.to_output_stream())
}
