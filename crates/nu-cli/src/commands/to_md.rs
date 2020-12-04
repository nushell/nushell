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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_md(args).await
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

async fn to_md(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();
    let (ToMarkdownArgs { pretty }, input) = args.process().await?;
    let input: Vec<Value> = input.collect().await;
    let headers = nu_protocol::merge_descriptors(&input);

    let mut escaped_headers: Vec<String> = Vec::new();
    let mut column_widths: Vec<usize> = Vec::new();

    if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
        for header in &headers {
            let escaped_header_string = htmlescape::encode_minimal(&header);
            column_widths.push(escaped_header_string.len());
            escaped_headers.push(escaped_header_string);
        }
    } else {
        column_widths = vec![0; headers.len()]
    }

    let mut escaped_rows: Vec<Vec<String>> = Vec::new();

    for row in &input {
        let mut escaped_row: Vec<String> = Vec::new();

        match row.value.clone() {
            UntaggedValue::Row(row) => {
                for i in 0..headers.len() {
                    let data = row.get_data(&headers[i]);
                    let value_string = format_leaf(data.borrow()).plain_string(100_000);
                    let new_column_width = value_string.len();

                    escaped_row.push(value_string);

                    if column_widths[i] < new_column_width {
                        column_widths[i] = new_column_width;
                    }
                }
            }
            p => {
                let value_string =
                    htmlescape::encode_minimal(&format_leaf(&p).plain_string(100_000));
                escaped_row.push(value_string);
            }
        }

        escaped_rows.push(escaped_row);
    }

    let output_string = if (column_widths.is_empty() || column_widths.iter().all(|x| *x == 0))
        && escaped_rows.is_empty()
    {
        String::from("")
    } else {
        get_output_string(&escaped_headers, &escaped_rows, &column_widths, pretty)
            .trim()
            .to_string()
    };

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(output_string).into_value(name_tag),
    )))
}

fn get_output_string(
    headers: &[String],
    rows: &[Vec<String>],
    column_widths: &[usize],
    pretty: bool,
) -> String {
    let mut output_string = String::new();

    if !headers.is_empty() {
        output_string.push('|');

        for i in 0..headers.len() {
            if pretty {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(
                    headers[i].clone(),
                    column_widths[i],
                    ' ',
                ));
                output_string.push(' ');
            } else {
                output_string.push_str(headers[i].as_str());
            }

            output_string.push('|');
        }

        output_string.push_str("\n|");

        #[allow(clippy::needless_range_loop)]
        for i in 0..headers.len() {
            if pretty {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(
                    String::from("-"),
                    column_widths[i],
                    '-',
                ));
                output_string.push(' ');
            } else {
                output_string.push('-');
            }

            output_string.push('|');
        }

        output_string.push('\n');
    }

    for row in rows {
        if !headers.is_empty() {
            output_string.push('|');
        }

        for i in 0..row.len() {
            if pretty {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(row[i].clone(), column_widths[i], ' '));
                output_string.push(' ');
            } else {
                output_string.push_str(row[i].as_str());
            }

            if !headers.is_empty() {
                output_string.push('|');
            }
        }

        output_string.push('\n');
    }

    output_string
}

fn get_padded_string(text: String, desired_length: usize, padding_character: char) -> String {
    let repeat_length = if text.len() > desired_length {
        0
    } else {
        desired_length - text.len()
    };

    format!(
        "{}{}",
        text,
        padding_character.to_string().repeat(repeat_length)
    )
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
