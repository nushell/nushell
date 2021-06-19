use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

pub struct Uniq;

impl WholeStreamCommand for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq")
            .switch("count", "Count the unique rows", Some('c'))
            .switch(
                "repeated",
                "Count the rows that has more than one value",
                Some('d'),
            )
            .switch(
                "ignore-case",
                "Ignore differences in case when comparing",
                Some('i'),
            )
    }

    fn usage(&self) -> &str {
        "Return the unique rows."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        uniq(args)
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
                description: "Only print duplicate lines, one for each group",
                example: "echo [1 2 2] | uniq -d",
                result: Some(vec![UntaggedValue::int(2).into()]),
            },
            Example {
                description: "Ignore differences in case when comparing",
                example: "echo ['hello' 'goodbye' 'Hello'] | uniq -i",
                result: Some(vec![
                    UntaggedValue::string("hello").into(),
                    UntaggedValue::string("goodbye").into(),
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

fn to_lowercase(value: nu_protocol::Value) -> nu_protocol::Value {
    use nu_protocol::value::StringExt;

    if value.is_string() {
        value
            .value
            .expect_string()
            .to_lowercase()
            .to_string_value(value.tag)
    } else {
        value
    }
}

fn uniq(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let should_show_count = args.has_flag("count");
    let show_repeated = args.has_flag("repeated");
    let ignore_case = args.has_flag("ignore-case");
    let input = args.input;
    let uniq_values = {
        let mut counter = IndexMap::<nu_protocol::Value, usize>::new();
        for line in input.into_vec() {
            let item = if ignore_case {
                to_lowercase(line)
            } else {
                line
            };
            *counter.entry(item).or_insert(0) += 1;
        }
        counter
    };

    let mut values_vec_deque = VecDeque::new();

    let values = if show_repeated {
        uniq_values.into_iter().filter(|i| i.1 > 1).collect::<_>()
    } else {
        uniq_values
    };

    if should_show_count {
        for item in values {
            use nu_protocol::Value;
            let value = {
                match item.0.value {
                    UntaggedValue::Row(mut row) => {
                        row.entries.insert(
                            "count".to_string(),
                            UntaggedValue::int(item.1 as i64).into_untagged_value(),
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
                            UntaggedValue::int(item.1 as i64).into_untagged_value(),
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
                    #[cfg(feature = "dataframe")]
                    UntaggedValue::DataFrame(_) => {
                        return Err(ShellError::labeled_error(
                            "uniq -c cannot operate on data structs",
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
        for item in values {
            values_vec_deque.push_back(item.0);
        }
    }

    Ok(values_vec_deque.into_iter().into_action_stream())
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
