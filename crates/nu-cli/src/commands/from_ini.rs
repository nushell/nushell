use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};
use std::collections::HashMap;

pub struct FromINI;

impl WholeStreamCommand for FromINI {
    fn name(&self) -> &str {
        "from-ini"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-ini")
    }

    fn usage(&self) -> &str {
        "Parse text as .ini and create table"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_ini(args, registry)
    }
}

fn convert_ini_second_to_nu_value(v: &HashMap<String, String>, tag: impl Into<Tag>) -> Value {
    let mut second = TaggedDictBuilder::new(tag);

    for (key, value) in v.iter() {
        second.insert_untagged(key.clone(), Primitive::String(value.clone()));
    }

    second.into_value()
}

fn convert_ini_top_to_nu_value(
    v: &HashMap<String, HashMap<String, String>>,
    tag: impl Into<Tag>,
) -> Value {
    let tag = tag.into();
    let mut top_level = TaggedDictBuilder::new(tag.clone());

    for (key, value) in v.iter() {
        top_level.insert_value(
            key.clone(),
            convert_ini_second_to_nu_value(value, tag.clone()),
        );
    }

    top_level.into_value()
}

pub fn from_ini_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Value, serde_ini::de::Error> {
    let v: HashMap<String, HashMap<String, String>> = serde_ini::from_str(&s)?;
    Ok(convert_ini_top_to_nu_value(&v, tag))
}

fn from_ini(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let concat_string = input.collect_string(tag.clone()).await?;

        match from_ini_string_to_value(concat_string.item, tag.clone()) {
            Ok(x) => match x {
                Value { value: UntaggedValue::Table(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as INI",
                    "input cannot be parsed as INI",
                    &tag,
                    "value originates from here",
                    concat_string.tag,
                ))
            }
        }
    };

    Ok(stream.to_output_stream())
}
