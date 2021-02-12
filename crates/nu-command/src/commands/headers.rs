use crate::prelude::*;
use futures::stream::StreamExt;
use indexmap::IndexMap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::Dictionary;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};

pub struct Headers;

#[async_trait]
impl WholeStreamCommand for Headers {
    fn name(&self) -> &str {
        "headers"
    }

    fn signature(&self) -> Signature {
        Signature::build("headers")
    }

    fn usage(&self) -> &str {
        "Use the first row of the table as column names"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        headers(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create headers for a raw string",
                example: r#"echo "a b c|1 2 3" | split row "|" | split column " " | headers"#,
                result: None,
            },
            Example {
                description: "Don't panic on rows with different headers",
                example: r#"echo "a b c|1 2 3|1 2 3 4" | split row "|" | split column " " | headers"#,
                result: None,
            },
        ]
    }
}

pub async fn headers(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let rows: Vec<Value> = input.collect().await;

    if rows.is_empty() {
        return Err(ShellError::untagged_runtime_error(
            "Couldn't find headers, was the input a properly formatted, non-empty table?",
        ));
    }

    //the headers are the first row in the table
    let headers: Vec<String> = match &rows[0].value {
        UntaggedValue::Row(d) => {
            Ok(d.entries
                .iter()
                .map(|(k, v)| {
                    match v.as_string() {
                        Ok(s) => s,
                        Err(_) => {
                            //If a cell that should contain a header name is empty, we name the column Column[index]
                            match d.entries.get_full(k) {
                                Some((index, _, _)) => format!("Column{}", index),
                                None => "unknownColumn".to_string(),
                            }
                        }
                    }
                })
                .collect())
        }
        _ => Err(ShellError::unexpected_eof(
            "Could not get headers, is the table empty?",
            rows[0].tag.span,
        )),
    }?;

    Ok(
        futures::stream::iter(rows.into_iter().skip(1).map(move |r| {
            //Each row is a dictionary with the headers as keys
            match &r.value {
                UntaggedValue::Row(d) => {
                    let mut entries = IndexMap::new();
                    for (i, header) in headers.iter().enumerate() {
                        let value = match d.entries.get_index(i) {
                            Some((_, value)) => value.clone(),
                            None => UntaggedValue::Primitive(Primitive::Nothing).into(),
                        };

                        entries.insert(header.clone(), value);
                    }
                    Ok(ReturnSuccess::Value(
                        UntaggedValue::Row(Dictionary { entries }).into_value(r.tag.clone()),
                    ))
                }
                _ => Err(ShellError::unexpected_eof(
                    "Couldn't iterate through rows, was the input a properly formatted table?",
                    r.tag.span,
                )),
            }
        }))
        .to_output_stream(),
    )
}

#[cfg(test)]
mod tests {
    use super::Headers;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Headers {})
    }
}
