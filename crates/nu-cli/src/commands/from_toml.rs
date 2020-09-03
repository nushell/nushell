use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value,
};

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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_toml(args, registry).await
    }
}

// pub fn convert_toml_edit_doc_to_indexmap(
//     doc: &toml_edit::Document,
//     tag: impl Into<Tag>,
// ) -> Result<IndexMap<String, Value>, ShellError> {
//     // let value = convert_toml_value_to_nu_value(&parsed, tag);
//     // let tag = value.tag();
//     // match value.value {
//     //     UntaggedValue::Row(Dictionary { entries }) => Ok(entries),
//     //     other => Err(ShellError::type_error(
//     //         "Dictionary",
//     //         other.type_name().spanned(tag.span),
//     //     )),
//     // }
//     let tag = tag.into();
//     let mut map: IndexMap<String, Value> = IndexMap::new();
//     for (key, val) in doc.iter() {
//         let value = convert_toml_item_to_nu_value(&val, tag.clone());
//         println!("key: [{}] value: [{:?}]", key, &value);
//         map.insert(key.to_string(), value);
//     }

//     Ok(map)
// }

// pub fn convert_toml_item_to_nu_value(i: &toml_edit::Item, tag: impl Into<Tag>) -> Value {
//     let tag = tag.into();

//     match i {
//         toml_edit::Item::Value(v) => convert_toml_value_to_nu_value(&v, tag),
//         toml_edit::Item::Table(t) => convert_toml_table_to_nu_value(t, tag),
//         //TODO: Deal with this
//         // toml_edit::Item::ArrayOfTables(a) => convert_toml_table_to_nu_value(t, tag),
//         _ => UntaggedValue::Primitive(Primitive::String(String::from(""))).into_value(tag),
//     }
// }

// pub fn convert_toml_table_to_nu_value(
//     table: &dyn toml_edit::TableLike,
//     tag: impl Into<Tag>,
// ) -> Value {
//     let tag = tag.into();

//     UntaggedValue::Table(
//         table
//             .iter()
//             .map(|(_k, v)| convert_toml_item_to_nu_value(v, &tag))
//             .collect(),
//     )
//     .into_value(tag)
// }

pub fn convert_toml_value_to_nu_value(v: &toml_edit::Value, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    match v {
        toml_edit::Value::Boolean(b) => UntaggedValue::boolean(*b.value()).into_value(tag),
        toml_edit::Value::Integer(n) => UntaggedValue::int(*n.value()).into_value(tag),
        toml_edit::Value::Float(n) => UntaggedValue::decimal(*n.value()).into_value(tag),
        toml_edit::Value::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s.value()))).into_value(tag)
        }
        toml_edit::Value::Array(a) => UntaggedValue::Table(
            a.iter()
                .map(|x| convert_toml_value_to_nu_value(x, &tag))
                .collect(),
        )
        .into_value(tag),
        toml_edit::Value::DateTime(dt) => {
            UntaggedValue::Primitive(Primitive::String(dt.to_string())).into_value(tag)
        }
        toml_edit::Value::InlineTable(t) => {
            let mut collected = TaggedDictBuilder::new(&tag);

            for (k, v) in t.iter() {
                collected.insert_value(<&str>::clone(&k), convert_toml_value_to_nu_value(v, &tag));
            }

            collected.into_value()
        }
    }
}

pub fn from_toml_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Value, toml_edit::TomlError> {
    let v: toml_edit::Value = s.parse::<toml_edit::Value>()?;
    Ok(convert_toml_value_to_nu_value(&v, tag))
}

pub async fn from_toml(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;
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

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(FromTOML {})
    }
}
