use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

pub struct Uniq;

#[async_trait]
impl WholeStreamCommand for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq").switch("count", "Count the unique rows", Some('c'))
    }

    fn usage(&self) -> &str {
        "Return the unique rows"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        uniq(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Remove duplicate rows of a list/table",
                example: "echo [2 3 3 4] | uniq",
                result: Some(vec![
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
            Example {
                description: "Remove duplicate rows and show counts of a list/table",
                example: "echo [1 2 2] | uniq -c",
                result: Some(vec![
                    UntaggedValue::row(indexmap! {
                    "value".to_string() => UntaggedValue::int(1).into(),
                    "count".to_string() => UntaggedValue::int(1).into(),
                    })
                    .into(),
                    UntaggedValue::row(indexmap! {
                    "value".to_string() => UntaggedValue::int(2).into(),
                    "count".to_string() => UntaggedValue::int(2).into(),
                    })
                    .into(),
                ]),
            },
        ]
    }
}

async fn uniq(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let should_show_count = args.has("count");
    let input = args.input;
    let uniq_values = {
        let mut counter = IndexMap::<nu_protocol::Value, usize>::new();
        for line in input.into_vec().await {
            *counter.entry(line).or_insert(0) += 1;
        }
        counter
    };

    let mut values_vec_deque = VecDeque::new();

    if should_show_count {
        for item in uniq_values {
            use nu_protocol::Value;
            let value = {
                match item.0.value {
                    UntaggedValue::Row(mut row) => {
                        row.entries.insert(
                            "count".to_string(),
                            UntaggedValue::int(item.1).into_untagged_value(),
                        );
                        Value {
                            value: UntaggedValue::Row(row),
                            tag: item.0.tag,
                        }
                    }
                    UntaggedValue::Primitive(p) => {
                        let mut map = IndexMap::<String, Value>::new();
                        map.insert(
                            "value".to_string(),
                            UntaggedValue::Primitive(p).into_untagged_value(),
                        );
                        map.insert(
                            "count".to_string(),
                            UntaggedValue::int(item.1).into_untagged_value(),
                        );
                        Value {
                            value: UntaggedValue::row(map),
                            tag: item.0.tag,
                        }
                    }
                    UntaggedValue::Table(_) => {
                        return Err(ShellError::labeled_error(
                            "uniq -c cannot operate on tables.",
                            "source",
                            item.0.tag.span,
                        ))
                    }
                    UntaggedValue::Error(_) | UntaggedValue::Block(_) => item.0,
                }
            };
            values_vec_deque.push_back(value);
        }
    } else {
        for item in uniq_values {
            values_vec_deque.push_back(item.0);
        }
    }

    Ok(futures::stream::iter(values_vec_deque).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Uniq;
    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Uniq {})
    }
}
