use crate::prelude::*;
use futures::StreamExt;
use nu_data::value::format_leaf;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    pretty: bool,
    #[serde(rename = "per-element")]
    per_element: bool,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "to md"
    }

    fn signature(&self) -> Signature {
        Signature::build("to md")
            .switch(
                "pretty",
                "Formats the Markdown table to vertically align items",
                Some('p'),
            )
            .switch(
                "per-element",
                "treat each row as markdown syntax element",
                Some('e'),
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
                description: "Outputs an unformatted table markdown string (default)",
                example: "ls | to md",
                result: Some(vec![Value::from(one(r#"
                |name|type|chickens|modified|
                |-|-|-|-|
                |Andres.txt|File|10|1 year ago|
                |Jonathan|Dir|5|1 year ago|
                |Darren.txt|File|20|1 year ago|
                |Yehuda|Dir|4|1 year ago|
                "#))]),
            },
            Example {
                description: "Optionally, output a formatted markdown string",
                example: "ls | to md --pretty",
                result: Some(vec![Value::from(one(r#"
                    | name       | type | chickens | modified   |
                    | ---------- | ---- | -------- | ---------- |
                    | Andres.txt | File | 10       | 1 year ago |
                    | Jonathan   | Dir  | 5        | 1 year ago |
                    | Darren.txt | File | 20       | 1 year ago |
                    | Yehuda     | Dir  | 4        | 1 year ago |
                    "#))]),
            },
            Example {
                description: "Treat each row as a markdown element",
                example: "echo [[H1]; [\"Welcome to Nushell\"]] | append $(ls | first 2) | to md --per-element --pretty",
                result: Some(vec![Value::from(one(r#"
                    # Welcome to Nushell
                    | name       | type | chickens | modified   |
                    | ---------- | ---- | -------- | ---------- |
                    | Andres.txt | File | 10       | 1 year ago |
                    | Jonathan   | Dir  | 5        | 1 year ago |
                    "#))]),
            }
        ]
    }
}

async fn to_md(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();
    let (arguments, input) = args.process().await?;

    let input: Vec<Value> = input.collect().await;

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(process(&input, arguments)).into_value(if input.is_empty() {
            name_tag
        } else {
            input[0].tag()
        }),
    )))
}

fn process(
    input: &[Value],
    Arguments {
        pretty,
        per_element,
    }: Arguments,
) -> String {
    if per_element {
        input
            .iter()
            .map(|v| match &v.value {
                UntaggedValue::Table(values) => table(values, pretty),
                _ => fragment(v, pretty),
            })
            .collect::<String>()
    } else {
        table(&input, pretty)
    }
}

fn fragment(input: &Value, pretty: bool) -> String {
    let headers = input.data_descriptors();
    let mut out = String::new();

    if headers.len() == 1 {
        let markup = match (&headers[0]).to_ascii_lowercase().as_ref() {
            "h1" => "# ".to_string(),
            "h2" => "## ".to_string(),
            "h3" => "### ".to_string(),
            "blockquote" => "> ".to_string(),

            _ => return table(&[input.clone()], pretty),
        };

        out.push_str(&markup);
        out.push_str(&format_leaf(input.get_data(&headers[0]).borrow()).plain_string(100_000));
    } else if input.is_row() {
        let string = match input.row_entries().next() {
            Some(value) => value.1.as_string().unwrap_or_default(),
            None => String::from(""),
        };

        out = format_leaf(&UntaggedValue::from(string)).plain_string(100_000)
    } else {
        out = format_leaf(&input.value).plain_string(100_000)
    }

    out.push('\n');
    out
}

fn collect_headers(headers: &[String]) -> (Vec<String>, Vec<usize>) {
    let mut escaped_headers: Vec<String> = Vec::new();
    let mut column_widths: Vec<usize> = Vec::new();

    if !headers.is_empty() && (headers.len() > 1 || !headers[0].is_empty()) {
        for header in headers {
            let escaped_header_string = htmlescape::encode_minimal(&header);
            column_widths.push(escaped_header_string.len());
            escaped_headers.push(escaped_header_string);
        }
    } else {
        column_widths = vec![0; headers.len()]
    }

    (escaped_headers, column_widths)
}

fn table(input: &[Value], pretty: bool) -> String {
    let headers = nu_protocol::merge_descriptors(&input);

    let (escaped_headers, mut column_widths) = collect_headers(&headers);

    let mut escaped_rows: Vec<Vec<String>> = Vec::new();

    for row in input {
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

    output_string
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

fn one(string: &str) -> String {
    string
        .lines()
        .skip(1)
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join("\n")
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{fragment, one, table};
    use nu_protocol::{row, Value};

    #[test]
    fn render_h1() {
        let value = row! {"H1".into() => Value::from("Ecuador")};

        assert_eq!(fragment(&value, false), "# Ecuador\n");
    }

    #[test]
    fn render_h2() {
        let value = row! {"H2".into() => Value::from("Ecuador")};

        assert_eq!(fragment(&value, false), "## Ecuador\n");
    }

    #[test]
    fn render_h3() {
        let value = row! {"H3".into() => Value::from("Ecuador")};

        assert_eq!(fragment(&value, false), "### Ecuador\n");
    }

    #[test]
    fn render_blockquote() {
        let value = row! {"BLOCKQUOTE".into() => Value::from("Ecuador")};

        assert_eq!(fragment(&value, false), "> Ecuador\n");
    }

    #[test]
    fn render_table() {
        let value = vec![
            row! { "country".into() => Value::from("Ecuador")},
            row! { "country".into() => Value::from("New Zealand")},
            row! { "country".into() => Value::from("USA")},
        ];

        assert_eq!(
            table(&value, false),
            one(r#"
            |country|
            |-|
            |Ecuador|
            |New Zealand|
            |USA|
        "#)
        );

        assert_eq!(
            table(&value, true),
            one(r#"
            | country     |
            | ----------- |
            | Ecuador     |
            | New Zealand |
            | USA         |
        "#)
        );
    }
}
