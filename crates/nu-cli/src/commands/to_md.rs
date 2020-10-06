use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use futures::StreamExt;
use nu_data::value::format_leaf;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct ToMarkdown;

#[derive(Deserialize)]
pub struct ToMarkdownArgs {
    pretty: bool,
}

#[async_trait]
impl WholeStreamCommand for ToMarkdown {
    fn name(&self) -> &str {
        "to md"
    }

    fn signature(&self) -> Signature {
        Signature::build("to md").switch(
            "pretty",
            "Formats the Markdown table to vertically align items",
            Some('p'),
        )
    }

    fn usage(&self) -> &str {
        "Convert table into simple Markdown"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_md(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an unformatted md string representing the contents of ls",
                example: "ls | to md",
                result: None,
            },
            Example {
                description: "Outputs a formatted md string representing the contents of ls",
                example: "ls | to md -p",
                result: None,
            },
        ]
    }
}

async fn to_md(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name_tag = args.call_info.name_tag.clone();
    let (ToMarkdownArgs { pretty }, input) = args.process(&registry).await?;
    let input: Vec<Value> = input.collect().await;
    let headers = nu_protocol::merge_descriptors(&input);
    let mut output_string = String::new();

    let mut column_length_vector: Vec<usize> = Vec::new();

    if pretty {
        if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
            for header in &headers {
                let htmlescape_header_string = &htmlescape::encode_minimal(&header);
                column_length_vector.push(htmlescape_header_string.len());
            }
        }

        for row in &input {
            if let UntaggedValue::Row(row) = row.value.clone() {
                for i in 0..headers.len() {
                    let data = row.get_data(&headers[i]);
                    let new_column_length =
                        format_leaf(data.borrow()).plain_string(100_000).len();

                    if column_length_vector[i] < new_column_length {
                        column_length_vector[i] = new_column_length;
                    }
                }
            }
        }
    }

    if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
        output_string.push_str("|");

        for i in 0..headers.len() {
            let htmlescape_string = htmlescape::encode_minimal(&headers[i]);
            let final_string = if pretty {
                get_padded_string(htmlescape_string, column_length_vector[i], ' ')
            } else {
                htmlescape_string
            };

            output_string.push_str(&final_string);
            output_string.push_str("|");
        }

        output_string.push_str("\n|");

        for column_length in column_length_vector.iter().take(headers.len()) {
            let final_string = if pretty {
                "-".repeat(*column_length)
            } else {
                String::from("-")
            };

            output_string.push_str(final_string.as_str());
            output_string.push_str("|");
        }

        output_string.push_str("\n");
    }

    for row in input {
        match row.value {
            UntaggedValue::Row(row) => {
                output_string.push_str("|");

                for i in 0..headers.len() {
                    let data = row.get_data(&headers[i]);
                    let leaf_string = format_leaf(data.borrow()).plain_string(100_000);
                    let final_string = if pretty {
                        get_padded_string(leaf_string, column_length_vector[i], ' ')
                    } else {
                        leaf_string
                    };

                    output_string.push_str(&final_string);
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

fn get_padded_string(text: String, desired_length: usize, character: char) -> String {
    let padding_length = desired_length - text.len();
    return format!("{}{}", text, character.to_string().repeat(padding_length));
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
