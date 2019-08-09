use crate::errors::ShellError;
use crate::object::{TaggedDictBuilder, Value};
use crate::prelude::*;

pub fn tags(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let source_map = args.call_info.source_map.clone();
    Ok(args
        .input
        .values
        .map(move |v| {
            let mut tags = TaggedDictBuilder::new(v.tag());
            {
                let origin = v.origin();
                let span = v.span();
                let mut dict = TaggedDictBuilder::new(v.tag());
                dict.insert("start", Value::int(span.start as i64));
                dict.insert("end", Value::int(span.end as i64));
                match origin.map(|x| source_map.get(&x)).flatten() {
                    Some(SpanSource::File(source)) => {
                        dict.insert("origin", Value::string(source));
                    }
                    Some(SpanSource::Url(source)) => {
                        dict.insert("origin", Value::string(source));
                    }
                    _ => {}
                }
                tags.insert_tagged("span", dict.into_tagged_value());
            }

            tags.into_tagged_value()
        })
        .to_output_stream())
}
