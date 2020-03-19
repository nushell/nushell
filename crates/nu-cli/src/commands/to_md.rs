use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use futures::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct ToMarkdown;

impl WholeStreamCommand for ToMarkdown {
    fn name(&self) -> &str {
        "to-md"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-md")
    }

    fn usage(&self) -> &str {
        "Convert table into simple Markdown"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_html(args, registry)
    }
}

fn to_html(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_tag = args.name_tag();
    //let name_span = name_tag.span;
    let stream = async_stream! {
        let input: Vec<Value> = args.input.values.collect().await;
        let headers = nu_protocol::merge_descriptors(&input);
        let mut output_string = String::new();

        if !headers.is_empty() && (headers.len() > 1 || headers[0] != "<value>") {
            output_string.push_str("|");
            for header in &headers {
                output_string.push_str(&htmlescape::encode_minimal(&header));
                output_string.push_str("|");
            }
            output_string.push_str("\n|");
            for _ in &headers {
                output_string.push_str("-");
                output_string.push_str("|");
            }
            output_string.push_str("\n");
        }

        for row in input {
            match row.value {
                UntaggedValue::Row(row) => {
                    output_string.push_str("|");
                    for header in &headers {
                        let data = row.get_data(header);
                        output_string.push_str(&format_leaf(data.borrow()).plain_string(100_000));
                        output_string.push_str("|");
                    }
                    output_string.push_str("\n");
                }
                p => {
                    output_string.push_str(&(htmlescape::encode_minimal(&format_leaf(&p).plain_string(100_000))));
                    output_string.push_str("\n");
                }
            }
        }

        yield ReturnSuccess::value(UntaggedValue::string(output_string).into_value(name_tag));
    };

    Ok(stream.to_output_stream())
}
