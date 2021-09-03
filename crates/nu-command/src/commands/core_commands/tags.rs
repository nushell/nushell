use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct Tags;

impl WholeStreamCommand for Tags {
    fn name(&self) -> &str {
        "tags"
    }

    fn signature(&self) -> Signature {
        Signature::build("tags")
    }

    fn usage(&self) -> &str {
        "Read the tags (metadata) for values."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(tags(args))
    }
}

fn build_tag_table(tag: impl Into<Tag>) -> Value {
    let tag = tag.into();
    let span = tag.span;

    TaggedDictBuilder::build(tag.clone(), |tags| {
        if let Some(anchor) = anchor_as_value(&tag) {
            tags.insert_value("anchor", anchor);
        }

        tags.insert_value(
            "span",
            TaggedDictBuilder::build(tag.clone(), |span_dict| {
                span_dict.insert_untagged("start", UntaggedValue::int(span.start() as i64));
                span_dict.insert_untagged("end", UntaggedValue::int(span.end() as i64));
            }),
        );
    })
}

fn tags(args: CommandArgs) -> OutputStream {
    if args.input.is_empty() {
        OutputStream::one(build_tag_table(&args.name_tag()))
    } else {
        args.input
            .map(move |v| build_tag_table(v.tag()))
            .into_output_stream()
    }
}

fn anchor_as_value(tag: &Tag) -> Option<Value> {
    let anchor = tag.anchor.as_ref();

    anchor.as_ref()?;

    Some(TaggedDictBuilder::build(tag, |table| {
        let value = match anchor {
            Some(AnchorLocation::File(path)) => {
                Some(("file", UntaggedValue::from(path.to_string())))
            }
            Some(AnchorLocation::Url(destination)) => {
                Some(("url", UntaggedValue::from(destination.to_string())))
            }
            Some(AnchorLocation::Source(text)) => Some((
                "source",
                UntaggedValue::Primitive(Primitive::String(text.to_string())),
            )),
            None => None,
        };

        if let Some((key, value)) = value {
            table.insert_untagged(key, value);
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Tags;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Tags {})
    }
}
