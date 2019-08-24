use crate::commands::WholeStreamCommand;
use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

pub struct FromTOML;

impl WholeStreamCommand for FromTOML {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_toml(args, registry)
    }

    fn name(&self) -> &str {
        "from-toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-toml")
    }
}

pub fn convert_toml_value_to_nu_value(v: &toml::Value, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    match v {
        toml::Value::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(tag),
        toml::Value::Integer(n) => Value::Primitive(Primitive::Int(*n)).tagged(tag),
        toml::Value::Float(n) => Value::Primitive(Primitive::Float(OF64::from(*n))).tagged(tag),
        toml::Value::String(s) => Value::Primitive(Primitive::String(String::from(s))).tagged(tag),
        toml::Value::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_toml_value_to_nu_value(x, tag))
                .collect(),
        )
        .tagged(tag),
        toml::Value::Datetime(dt) => {
            Value::Primitive(Primitive::String(dt.to_string())).tagged(tag)
        }
        toml::Value::Table(t) => {
            let mut collected = TaggedDictBuilder::new(tag);

            for (k, v) in t.iter() {
                collected.insert_tagged(k.clone(), convert_toml_value_to_nu_value(v, tag));
            }

            collected.into_tagged_value()
        }
    }
}

pub fn from_toml_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, toml::de::Error> {
    let v: toml::Value = s.parse::<toml::Value>()?;
    Ok(convert_toml_value_to_nu_value(&v, tag))
}

pub fn from_toml(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
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

        match from_toml_string_to_value(concat_string, span) {
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
                    "Could not parse as TOML",
                    "input cannot be parsed as TOML",
                    span,
                    "value originates from here",
                    last_tag.span,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
