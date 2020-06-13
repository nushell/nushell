use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromJSON;

#[derive(Deserialize)]
pub struct FromJSONArgs {
    objects: bool,
}

#[async_trait]
impl WholeStreamCommand for FromJSON {
    fn name(&self) -> &str {
        "from json"
    }

    fn signature(&self) -> Signature {
        Signature::build("from json").switch(
            "objects",
            "treat each line as a separate value",
            Some('o'),
        )
    }

    fn usage(&self) -> &str {
        "Parse text as .json and create table."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_json(args, registry).await
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

async fn from_json(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();
    let registry = registry.clone();

    let (FromJSONArgs { objects }, input) = args.process(&registry).await?;
    let concat_string = input.collect_string(name_tag.clone()).await?;

    let string_clone: Vec<_> = concat_string.item.lines().map(|x| x.to_string()).collect();

    if objects {
        Ok(
            futures::stream::iter(string_clone.into_iter().filter_map(move |json_str| {
                if json_str.is_empty() {
                    return None;
                }

                match from_json_string_to_value(json_str, &name_tag) {
                    Ok(x) => Some(ReturnSuccess::value(x)),
                    Err(e) => {
                        let mut message = "Could not parse as JSON (".to_string();
                        message.push_str(&e.to_string());
                        message.push_str(")");

                        Some(Err(ShellError::labeled_error_with_secondary(
                            message,
                            "input cannot be parsed as JSON",
                            name_tag.clone(),
                            "value originates from here",
                            concat_string.tag.clone(),
                        )))
                    }
                }
            }))
            .to_output_stream(),
        )
    } else {
        match from_json_string_to_value(concat_string.item, name_tag.clone()) {
            Ok(x) => match x {
                Value {
                    value: UntaggedValue::Table(list),
                    ..
                } => Ok(
                    futures::stream::iter(list.into_iter().map(ReturnSuccess::value))
                        .to_output_stream(),
                ),
                x => Ok(OutputStream::one(ReturnSuccess::value(x))),
            },
            Err(e) => {
                let mut message = "Could not parse as JSON (".to_string();
                message.push_str(&e.to_string());
                message.push_str(")");

                Ok(OutputStream::one(Err(
                    ShellError::labeled_error_with_secondary(
                        message,
                        "input cannot be parsed as JSON",
                        name_tag,
                        "value originates from here",
                        concat_string.tag,
                    ),
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FromJSON;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(FromJSON {})
    }
}
