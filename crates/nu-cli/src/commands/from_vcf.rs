extern crate ical;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use ical::parser::vcard::component::*;
use ical::property::Property;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};
use std::io::BufReader;

pub struct FromVcf;

impl WholeStreamCommand for FromVcf {
    fn name(&self) -> &str {
        "from-vcf"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-vcf")
    }

    fn usage(&self) -> &str {
        "Parse text as .vcf and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_vcf(args, registry)
    }
}

fn from_vcf(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let input_string = input.collect_string(tag.clone()).await?.item;
        let input_bytes = input_string.as_bytes();
        let buf_reader = BufReader::new(input_bytes);
        let parser = ical::VcardParser::new(buf_reader);

        for contact in parser {
            match contact {
                Ok(c) => yield ReturnSuccess::value(contact_to_value(c, tag.clone())),
                Err(_) => yield Err(ShellError::labeled_error(
                    "Could not parse as .vcf",
                    "input cannot be parsed as .vcf",
                    tag.clone()
                )),
            }
        }
    };

    Ok(stream.to_output_stream())
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
