use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, TaggedDictBuilder, UntaggedValue, Value};
use std::collections::HashMap;

pub struct FromINI;

#[async_trait]
impl WholeStreamCommand for FromINI {
    fn name(&self) -> &str {
        "from ini"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ini")
    }

    fn usage(&self) -> &str {
        "Parse text as .ini and create table"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_ini(args).await
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

async fn from_ini(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.name_tag();
    let input = args.input;
    let concat_string = input.collect_string(tag.clone()).await?;

    match from_ini_string_to_value(concat_string.item, tag.clone()) {
        Ok(x) => match x {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => Ok(futures::stream::iter(list).to_output_stream()),
            x => Ok(OutputStream::one(x)),
        },
        Err(_) => Err(ShellError::labeled_error_with_secondary(
            "Could not parse as INI",
            "input cannot be parsed as INI",
            &tag,
            "value originates from here",
            concat_string.tag,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::FromINI;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromINI {})
    }
}
