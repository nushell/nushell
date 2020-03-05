use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromTOML;

impl WholeStreamCommand for FromTOML {
    fn name(&self) -> &str {
        "from-toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-toml")
    }

    fn usage(&self) -> &str {
        "Parse text as .toml and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_toml(args, registry)
    }
}

pub fn convert_toml_value_to_nu_value(v: &toml::Value, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    match v {
        toml::Value::Boolean(b) => UntaggedValue::boolean(*b).into_value(tag),
        toml::Value::Integer(n) => UntaggedValue::int(*n).into_value(tag),
        toml::Value::Float(n) => UntaggedValue::decimal(*n).into_value(tag),
        toml::Value::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(tag)
        }
        toml::Value::Array(a) => UntaggedValue::Table(
            a.iter()
                .map(|x| convert_toml_value_to_nu_value(x, &tag))
                .collect(),
        )
        .into_value(tag),
        toml::Value::Datetime(dt) => {
            UntaggedValue::Primitive(Primitive::String(dt.to_string())).into_value(tag)
        }
        toml::Value::Table(t) => {
            let mut collected = TaggedDictBuilder::new(&tag);

            for (k, v) in t.iter() {
                collected.insert_value(k.clone(), convert_toml_value_to_nu_value(v, &tag));
            }

            collected.into_value()
        }
    }
}

pub fn from_toml_string_to_value(s: String, tag: impl Into<Tag>) -> Result<Value, toml::de::Error> {
    let v: toml::Value = s.parse::<toml::Value>()?;
    Ok(convert_toml_value_to_nu_value(&v, tag))
}

pub fn from_toml(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let name_span = tag.span;
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            latest_tag = Some(value.tag.clone());
            let value_span = value.tag.span;
            if let Ok(s) = value.as_string() {
                concat_string.push_str(&s);
            }
            else {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_span,
                ))
            }
        }

        match from_toml_string_to_value(concat_string, tag.clone()) {
            Ok(x) => match x {
                Value { value: UntaggedValue::Table(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => if let Some(last_tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as TOML",
                    "input cannot be parsed as TOML",
                    &tag,
                    "value originates from here",
                    last_tag,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
