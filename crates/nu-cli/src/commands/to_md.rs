use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use futures::StreamExt;
use nu_data::value::format_leaf;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct ToMarkdown;

#[async_trait]
impl WholeStreamCommand for ToMarkdown {
    fn name(&self) -> &str {
        "to md"
    }

    fn signature(&self) -> Signature {
        Signature::build("to md")
    }

    fn usage(&self) -> &str {
        "Convert table into simple Markdown"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_html(args, registry).await
    }
}

async fn to_html(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let args = args.evaluate_once(&registry).await?;
    let name_tag = args.name_tag();
    let input: Vec<Value> = args.input.collect().await;
    let headers = nu_protocol::merge_descriptors(&input);
    let mut output_string = String::new();

    if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
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
                output_string.push_str(
                    &(htmlescape::encode_minimal(&format_leaf(&p).plain_string(100_000))),
                );
                output_string.push_str("\n");
            }
        }
    }

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(output_string).into_value(name_tag),
    )))
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::ToMarkdown;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(ToMarkdown {})?)
    }
}
