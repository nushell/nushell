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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(tags(args))
    }
}

fn tags(args: CommandArgs) -> ActionStream {
    args.input
        .map(move |v| {
            TaggedDictBuilder::build(v.tag(), |tags| {
                if let Some(anchor) = anchor_as_value(&v) {
                    tags.insert_value("anchor", anchor);
                }

                tags.insert_value(
                    "span",
                    TaggedDictBuilder::build(v.tag(), |span_dict| {
                        let span = v.tag().span;
                        span_dict.insert_untagged("start", UntaggedValue::int(span.start() as i64));
                        span_dict.insert_untagged("end", UntaggedValue::int(span.end() as i64));
                    }),
                );
            })
        })
        .into_action_stream()
}

fn anchor_as_value(value: &Value) -> Option<Value> {
    let tag = value.tag();
    let anchor = tag.anchor;

    anchor.as_ref()?;

    Some(TaggedDictBuilder::build(value.tag(), |table| {
        let value = match anchor {
            Some(AnchorLocation::File(path)) => Some(("file", UntaggedValue::from(path))),
            Some(AnchorLocation::Url(destination)) => {
                Some(("url", UntaggedValue::from(destination)))
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
