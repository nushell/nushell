use crate::prelude::*;
use ::eml_parser::eml::*;
use ::eml_parser::EmlParser;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tagged;

pub struct FromEML;

const DEFAULT_BODY_PREVIEW: usize = 50;

#[derive(Deserialize, Clone)]
pub struct FromEMLArgs {
    #[serde(rename(deserialize = "preview-body"))]
    preview_body: Option<Tagged<usize>>,
}

#[async_trait]
impl WholeStreamCommand for FromEML {
    fn name(&self) -> &str {
        "from eml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from eml").named(
            "preview-body",
            SyntaxShape::Int,
            "How many bytes of the body to preview",
            Some('b'),
        )
    }

    fn usage(&self) -> &str {
        "Parse text as .eml and create table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_eml(args).await
    }
}

fn emailaddress_to_value(tag: &Tag, email_address: &EmailAddress) -> TaggedDictBuilder {
    let mut dict = TaggedDictBuilder::with_capacity(tag, 2);
    let (n, a) = match email_address {
        EmailAddress::AddressOnly { address } => {
            (UntaggedValue::nothing(), UntaggedValue::string(address))
        }
        EmailAddress::NameAndEmailAddress { name, address } => {
            (UntaggedValue::string(name), UntaggedValue::string(address))
        }
    };

    dict.insert_untagged("Name", n);
    dict.insert_untagged("Address", a);

    dict
}

fn headerfieldvalue_to_value(tag: &Tag, value: &HeaderFieldValue) -> UntaggedValue {
    use HeaderFieldValue::*;

    match value {
        SingleEmailAddress(address) => emailaddress_to_value(tag, address).into_untagged_value(),
        MultipleEmailAddresses(addresses) => UntaggedValue::Table(
            addresses
                .iter()
                .map(|a| emailaddress_to_value(tag, a).into_value())
                .collect(),
        ),
        Unstructured(s) => UntaggedValue::string(s),
        Empty => UntaggedValue::nothing(),
    }
}

async fn from_eml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let (eml_args, input): (FromEMLArgs, _) = args.process().await?;
    let value = input.collect_string(tag.clone()).await?;

    let body_preview = eml_args
        .preview_body
        .map(|b| b.item)
        .unwrap_or(DEFAULT_BODY_PREVIEW);

    let eml = EmlParser::from_string(value.item)
        .with_body_preview(body_preview)
        .parse()
        .map_err(|_| {
            ShellError::labeled_error(
                "Could not parse .eml file",
                "could not parse .eml file",
                &tag,
            )
        })?;

    let mut dict = TaggedDictBuilder::new(&tag);

    if let Some(subj) = eml.subject {
        dict.insert_untagged("Subject", UntaggedValue::string(subj));
    }

    if let Some(from) = eml.from {
        dict.insert_untagged("From", headerfieldvalue_to_value(&tag, &from));
    }

    if let Some(to) = eml.to {
        dict.insert_untagged("To", headerfieldvalue_to_value(&tag, &to));
    }

    for HeaderField { name, value } in eml.headers.iter() {
        dict.insert_untagged(name, headerfieldvalue_to_value(&tag, &value));
    }

    if let Some(body) = eml.body {
        dict.insert_untagged("Body", UntaggedValue::string(body));
    }

    Ok(OutputStream::one(ReturnSuccess::value(dict.into_value())))
}

#[cfg(test)]
mod tests {
    use super::FromEML;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromEML {})
    }
}
