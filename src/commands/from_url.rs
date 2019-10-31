use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

pub struct FromURL;

impl WholeStreamCommand for FromURL {
    fn name(&self) -> &str {
        "from-url"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-url")
    }

    fn usage(&self) -> &str {
        "Parse url-encoded string as a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_url(args, registry)
    }
}

fn from_url(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag.clone());
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    &tag,
                    "value originates from here",
                    &value_tag,
                )),

            }
        }

        let result = serde_urlencoded::from_str::<Vec<(String, String)>>(&concat_string);

        match result {
            Ok(result) => {
                let mut row = TaggedDictBuilder::new(tag);

                for (k,v) in result {
                    row.insert(k, Value::string(v));
                }

                yield ReturnSuccess::value(row.into_tagged_value());
            }
            _ => {
                if let Some(last_tag) = latest_tag {
                    yield Err(ShellError::labeled_error_with_secondary(
                        "String not compatible with url-encoding",
                        "input not url-encoded",
                        tag,
                        "value originates from here",
                        last_tag,
                    ));
                }
            }
        }
    };

    Ok(stream.to_output_stream())
}
