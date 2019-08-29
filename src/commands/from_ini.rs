use crate::commands::WholeStreamCommand;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
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

fn convert_ini_second_to_nu_value(
    v: &HashMap<String, String>,
    tag: impl Into<Tag>,
) -> Tagged<Value> {
    let mut second = TaggedDictBuilder::new(tag);

    for (key, value) in v.into_iter() {
        second.insert(key.clone(), Primitive::String(value.clone()));
    }

    second.into_tagged_value()
}

fn convert_ini_top_to_nu_value(
    v: &HashMap<String, HashMap<String, String>>,
    tag: impl Into<Tag>,
) -> Tagged<Value> {
    let tag = tag.into();
    let mut top_level = TaggedDictBuilder::new(tag);

    for (key, value) in v.iter() {
        top_level.insert_tagged(key.clone(), convert_ini_second_to_nu_value(value, tag));
    }

    top_level.into_tagged_value()
}

pub fn from_ini_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, serde_ini::de::Error> {
    let v: HashMap<String, HashMap<String, String>> = serde_ini::from_str(&s)?;
    Ok(convert_ini_top_to_nu_value(&v, tag))
}

fn from_ini(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let input = args.input;

    let stream = async_stream_block! {
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
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    value_tag.span,
                )),

            }
        }

        match from_ini_string_to_value(concat_string, span) {
            Ok(x) => match x {
                Tagged { item: Value::List(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => if let Some(last_tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as INI",
                    "input cannot be parsed as INI",
                    span,
                    "value originates from here",
                    last_tag.span,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
