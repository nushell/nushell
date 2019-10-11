use crate::commands::WholeStreamCommand;
use crate::data::{TaggedDictBuilder, Value};
use crate::errors::ShellError;
use crate::prelude::*;

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
    let source_map = args.call_info.source_map.clone();
    Ok(args
        .input
        .values
        .map(move |v| {
            let mut tags = TaggedDictBuilder::new(v.tag());
            {
                let anchor = v.anchor();
                let span = v.tag().span;
                let mut dict = TaggedDictBuilder::new(v.tag());
                dict.insert("start", Value::int(span.start() as i64));
                dict.insert("end", Value::int(span.end() as i64));
                tags.insert_tagged("span", dict.into_tagged_value());

                match source_map.get(&anchor) {
                    Some(AnchorLocation::File(source)) => {
                        tags.insert("anchor", Value::string(source));
                    }
                    Some(AnchorLocation::Url(source)) => {
                        tags.insert("anchor", Value::string(source));
                    }
                    _ => {}
                }
            }

            tags.into_tagged_value()
        })
        .to_output_stream())
}
