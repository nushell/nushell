use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromJSON;

#[derive(Deserialize)]
pub struct FromJSONArgs {
    objects: bool,
}

impl WholeStreamCommand for FromJSON {
    fn name(&self) -> &str {
        "from-json"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-json").switch(
            "objects",
            "treat each line as a separate value",
            Some('o'),
        )
    }

    fn usage(&self) -> &str {
        "Parse text as .json and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_json)?.run()
    }
}

fn convert_json_value_to_nu_value(v: &serde_hjson::Value, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    match v {
        serde_hjson::Value::Null => UntaggedValue::Primitive(Primitive::Nothing).into_value(&tag),
        serde_hjson::Value::Bool(b) => UntaggedValue::boolean(*b).into_value(&tag),
        serde_hjson::Value::F64(n) => UntaggedValue::decimal(*n).into_value(&tag),
        serde_hjson::Value::U64(n) => UntaggedValue::int(*n).into_value(&tag),
        serde_hjson::Value::I64(n) => UntaggedValue::int(*n).into_value(&tag),
        serde_hjson::Value::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(&tag)
        }
        serde_hjson::Value::Array(a) => UntaggedValue::Table(
            a.iter()
                .map(|x| convert_json_value_to_nu_value(x, &tag))
                .collect(),
        )
        .into_value(tag),
        serde_hjson::Value::Object(o) => {
            let mut collected = TaggedDictBuilder::new(&tag);
            for (k, v) in o.iter() {
                collected.insert_value(k.clone(), convert_json_value_to_nu_value(v, &tag));
            }

            collected.into_value()
        }
    }
}

pub fn from_json_string_to_value(s: String, tag: impl Into<Tag>) -> serde_hjson::Result<Value> {
    let v: serde_hjson::Value = serde_hjson::from_str(&s)?;
    Ok(convert_json_value_to_nu_value(&v, tag))
}

fn from_json(
    FromJSONArgs { objects }: FromJSONArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;

    let stream = async_stream! {
        let concat_string = input.collect_string(name_tag.clone()).await?;

        if objects {
            for json_str in concat_string.item.lines() {
                if json_str.is_empty() {
                    continue;
                }

                match from_json_string_to_value(json_str.to_string(), &name_tag) {
                    Ok(x) =>
                        yield ReturnSuccess::value(x),
                    Err(e) => {
                        let mut message = "Could not parse as JSON (".to_string();
                        message.push_str(&e.to_string());
                        message.push_str(")");

                        yield Err(ShellError::labeled_error_with_secondary(
                            message,
                            "input cannot be parsed as JSON",
                            &name_tag,
                            "value originates from here",
                            concat_string.tag.clone()))
                    }
                }
            }
        } else {
            match from_json_string_to_value(concat_string.item, name_tag.clone()) {
                Ok(x) =>
                    match x {
                        Value { value: UntaggedValue::Table(list), .. } => {
                            for l in list {
                                yield ReturnSuccess::value(l);
                            }
                        }
                        x => yield ReturnSuccess::value(x),
                    }
                Err(e) => {
                    let mut message = "Could not parse as JSON (".to_string();
                    message.push_str(&e.to_string());
                    message.push_str(")");

                    yield Err(ShellError::labeled_error_with_secondary(
                        message,
                        "input cannot be parsed as JSON",
                        name_tag,
                        "value originates from here",
                        concat_string.tag))
                }
            }
        }
    };

    Ok(stream.to_output_stream())
}
