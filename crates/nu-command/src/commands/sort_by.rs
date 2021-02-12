use crate::prelude::*;
use nu_data::base::coerce_compare;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::ValueExt;

pub struct SortBy;

#[derive(Deserialize)]
pub struct SortByArgs {
    rest: Vec<Tagged<String>>,
    insensitive: bool,
    reverse: bool,
}

#[async_trait]
impl WholeStreamCommand for SortBy {
    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("sort-by")
            .switch(
                "insensitive",
                "Sort string-based columns case-insensitively",
                Some('i'),
            )
            .switch("reverse", "Sort in reverse order", Some('r'))
            .rest(SyntaxShape::String, "the column(s) to sort by")
    }

    fn usage(&self) -> &str {
        "Sort by the given columns, in increasing order."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        sort_by(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sort list by increasing value",
                example: "echo [4 2 3 1] | sort-by",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
            Example {
                description: "Sort list by decreasing value",
                example: "echo [2 3 4 1] | sort-by -r",
                result: Some(vec![
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(1).into(),
                ]),
            },
            Example {
                description: "Sort output by increasing file size",
                example: "ls | sort-by size",
                result: None,
            },
            Example {
                description: "Sort output by type, and then by file size for each type",
                example: "ls | sort-by type size",
                result: None,
            },
            Example {
                description: "Sort strings (case-sensitive)",
                example: "echo [airplane Truck Car] | sort-by",
                result: Some(vec![
                    UntaggedValue::string("Car").into(),
                    UntaggedValue::string("Truck").into(),
                    UntaggedValue::string("airplane").into(),
                ]),
            },
            Example {
                description: "Sort strings (reversed case-sensitive)",
                example: "echo [airplane Truck Car] | sort-by -r",
                result: Some(vec![
                    UntaggedValue::string("airplane").into(),
                    UntaggedValue::string("Truck").into(),
                    UntaggedValue::string("Car").into(),
                ]),
            },
            Example {
                description: "Sort strings (case-insensitive)",
                example: "echo [airplane Truck Car] | sort-by -i",
                result: Some(vec![
                    UntaggedValue::string("airplane").into(),
                    UntaggedValue::string("Car").into(),
                    UntaggedValue::string("Truck").into(),
                ]),
            },
            Example {
                description: "Sort strings (reversed case-insensitive)",
                example: "echo [airplane Truck Car] | sort-by -i -r",
                result: Some(vec![
                    UntaggedValue::string("Truck").into(),
                    UntaggedValue::string("Car").into(),
                    UntaggedValue::string("airplane").into(),
                ]),
            },
        ]
    }
}

async fn sort_by(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (
        SortByArgs {
            rest,
            insensitive,
            reverse,
        },
        mut input,
    ) = args.process().await?;
    let mut vec = input.drain_vec().await;

    sort(&mut vec, &rest, &tag, insensitive)?;

    if reverse {
        vec.reverse()
    }

    Ok(futures::stream::iter(vec.into_iter()).to_output_stream())
}

pub fn sort(
    vec: &mut [Value],
    keys: &[Tagged<String>],
    tag: impl Into<Tag>,
    insensitive: bool,
) -> Result<(), ShellError> {
    let tag = tag.into();

    if vec.is_empty() {
        return Err(ShellError::labeled_error(
            "no values to work with",
            "no values to work with",
            tag,
        ));
    }

    for sort_arg in keys.iter() {
        let match_test = &vec[0].get_data_by_key(sort_arg.borrow_spanned());
        if match_test.is_none() {
            return Err(ShellError::labeled_error(
                "Can not find column to sort by",
                "invalid column",
                sort_arg.borrow_spanned().span,
            ));
        }
    }

    match &vec[0] {
        Value {
            value: UntaggedValue::Primitive(_),
            ..
        } => {
            let should_sort_case_insensitively = insensitive && vec.iter().all(|x| x.is_string());

            if let Some(values) = vec
                .windows(2)
                .map(|elem| coerce_compare(&elem[0], &elem[1]))
                .find(|elem| elem.is_err())
            {
                let (type_1, type_2) = values
                    .err()
                    .expect("An error occurred in the checking of types");
                return Err(ShellError::labeled_error(
                    "Not all values can be compared",
                    format!(
                        "Unable to sort values, as \"{}\" cannot compare against \"{}\"",
                        type_1, type_2
                    ),
                    tag,
                ));
            }

            vec.sort_by(|a, b| {
                if should_sort_case_insensitively {
                    let lowercase_a_string = a.expect_string().to_ascii_lowercase();
                    let lowercase_b_string = b.expect_string().to_ascii_lowercase();

                    lowercase_a_string.cmp(&lowercase_b_string)
                } else {
                    coerce_compare(a, b).expect("Unimplemented BUG: What about primitives that don't have an order defined?").compare()
                }
            });
        }
        _ => {
            let calc_key = |item: &Value| {
                keys.iter()
                    .map(|f| {
                        let mut value_option = item.get_data_by_key(f.borrow_spanned());

                        if insensitive {
                            if let Some(value) = &value_option {
                                if let Ok(string_value) = value.as_string() {
                                    value_option = Some(
                                        UntaggedValue::string(string_value.to_ascii_lowercase())
                                            .into_value(value.tag.clone()),
                                    )
                                }
                            }
                        }

                        value_option
                    })
                    .collect::<Vec<Option<Value>>>()
            };
            vec.sort_by_cached_key(calc_key);
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SortBy;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SortBy {})
    }
}
