use crate::commands::WholeStreamCommand;
use crate::data::TaggedDictBuilder;
use crate::prelude::*;
use nu_protocol::{Signature};
use nu_errors::ShellError;

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        tags(args, registry)
    }
}

fn tags(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    Ok(args
        .input
        .values
        .map(move |v| {
            let mut tags = TaggedDictBuilder::new(v.tag());
            {
                let anchor = v.anchor();
                let span = v.tag.span;
                let mut dict = TaggedDictBuilder::new(v.tag());
                dict.insert_untagged("start", value::int(span.start() as i64));
                dict.insert_untagged("end", value::int(span.end() as i64));
                tags.insert_value("span", dict.into_value());

                match anchor {
                    Some(AnchorLocation::File(source)) => {
                        tags.insert_untagged("anchor", value::string(source));
                    }
                    Some(AnchorLocation::Url(source)) => {
                        tags.insert_untagged("anchor", value::string(source));
                    }
                    _ => {}
                }
            }

            tags.into_value()
        })
        .to_output_stream())
}
