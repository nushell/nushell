use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};

pub struct Tags;

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(tags(args))
    }
}

fn tags(args: CommandArgs) -> OutputStream {
    args.input
        .map(move |v| {
            let mut tags = TaggedDictBuilder::new(v.tag());
            {
                let anchor = v.anchor();
                let span = v.tag.span;
                let mut dict = TaggedDictBuilder::new(v.tag());
                dict.insert_untagged("start", UntaggedValue::int(span.start() as i64));
                dict.insert_untagged("end", UntaggedValue::int(span.end() as i64));
                tags.insert_value("span", dict.into_value());

                match anchor {
                    Some(AnchorLocation::File(source)) => {
                        tags.insert_untagged("anchor", UntaggedValue::string(source));
                    }
                    Some(AnchorLocation::Url(source)) => {
                        tags.insert_untagged("anchor", UntaggedValue::string(source));
                    }
                    _ => {}
                }
            }

            tags.into_value()
        })
        .to_output_stream()
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
