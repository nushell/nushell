use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromTOML;

#[async_trait]
impl WholeStreamCommand for FromTOML {
    fn name(&self) -> &str {
        "from toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from toml")
    }

    fn usage(&self) -> &str {
        "Parse text as .toml and create table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_toml(args).await
    }
}

pub fn convert_toml_value_to_nu_value(v: &toml::Value, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();
    let span = tag.span;

    match v {
        toml::Value::Boolean(b) => UntaggedValue::boolean(*b).into_value(tag),
        toml::Value::Integer(n) => UntaggedValue::int(*n).into_value(tag),
        toml::Value::Float(n) => UntaggedValue::decimal_from_float(*n, span).into_value(tag),
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

pub async fn from_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.name_tag();
    let input = args.input;

    let concat_string = input.collect_string(tag.clone()).await?;
    Ok(
        match from_toml_string_to_value(concat_string.item, tag.clone()) {
            Ok(x) => match x {
                Value {
                    value: UntaggedValue::Table(list),
                    ..
                } => futures::stream::iter(list.into_iter().map(ReturnSuccess::value))
                    .to_output_stream(),
                x => OutputStream::one(ReturnSuccess::value(x)),
            },
            Err(_) => {
                return Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as TOML",
                    "input cannot be parsed as TOML",
                    &tag,
                    "value originates from here",
                    concat_string.tag,
                ))
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::FromTOML;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromTOML {})
    }
}
