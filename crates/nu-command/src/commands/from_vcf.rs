extern crate ical;
use crate::prelude::*;
use ical::parser::vcard::component::*;
use ical::property::Property;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromVcf;

#[async_trait]
impl WholeStreamCommand for FromVcf {
    fn name(&self) -> &str {
        "from vcf"
    }

    fn signature(&self) -> Signature {
        Signature::build("from vcf")
    }

    fn usage(&self) -> &str {
        "Parse text as .vcf and create table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_vcf(args).await
    }
}

async fn from_vcf(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.name_tag();
    let input = args.input;

    let input_string = input.collect_string(tag.clone()).await?.item;
    let input_bytes = input_string.into_bytes();
    let cursor = std::io::Cursor::new(input_bytes);
    let parser = ical::VcardParser::new(cursor);

    let iter = parser.map(move |contact| match contact {
        Ok(c) => ReturnSuccess::value(contact_to_value(c, tag.clone())),
        Err(_) => Err(ShellError::labeled_error(
            "Could not parse as .vcf",
            "input cannot be parsed as .vcf",
            tag.clone(),
        )),
    });

    Ok(futures::stream::iter(iter).to_output_stream())
}

fn contact_to_value(contact: VcardContact, tag: Tag) -> Value {
    let mut row = TaggedDictBuilder::new(tag.clone());
    row.insert_untagged("properties", properties_to_value(contact.properties, tag));
    row.into_value()
}

fn properties_to_value(properties: Vec<Property>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &properties
            .into_iter()
            .map(|prop| {
                let mut row = TaggedDictBuilder::new(tag.clone());

                let name = UntaggedValue::string(prop.name);
                let value = match prop.value {
                    Some(val) => UntaggedValue::string(val),
                    None => UntaggedValue::Primitive(Primitive::Nothing),
                };
                let params = match prop.params {
                    Some(param_list) => params_to_value(param_list, tag.clone()).into(),
                    None => UntaggedValue::Primitive(Primitive::Nothing),
                };

                row.insert_untagged("name", name);
                row.insert_untagged("value", value);
                row.insert_untagged("params", params);
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn params_to_value(params: Vec<(String, Vec<String>)>, tag: Tag) -> Value {
    let mut row = TaggedDictBuilder::new(tag);

    for (param_name, param_values) in params {
        let values: Vec<Value> = param_values.into_iter().map(|val| val.into()).collect();
        let values = UntaggedValue::table(&values);
        row.insert_untagged(param_name, values);
    }

    row.into_value()
}

#[cfg(test)]
mod tests {
    use super::FromVcf;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromVcf {})
    }
}
