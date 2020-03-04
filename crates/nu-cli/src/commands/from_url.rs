use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

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
    let name_span = tag.span;
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            latest_tag = Some(value.tag.clone());
            let value_span = value.tag.span;
            if let Ok(s) = value.as_string() {
                concat_string.push_str(&s);
            } else {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_span,
                ))
            }
        }

        let result = serde_urlencoded::from_str::<Vec<(String, String)>>(&concat_string);

        match result {
            Ok(result) => {
                let mut row = TaggedDictBuilder::new(tag);

                for (k,v) in result {
                    row.insert_untagged(k, UntaggedValue::string(v));
                }

                yield ReturnSuccess::value(row.into_value());
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
