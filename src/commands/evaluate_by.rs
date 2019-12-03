use crate::commands::WholeStreamCommand;
use crate::data::value;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{SpannedItem, Tagged};

pub struct EvaluateBy;

#[derive(Deserialize)]
pub struct EvaluateByArgs {
    evaluate_with: Option<Tagged<String>>,
}

impl WholeStreamCommand for EvaluateBy {
    fn name(&self) -> &str {
        "evaluate-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("evaluate-by").named(
            "evaluate_with",
            SyntaxShape::String,
            "the name of the column to evaluate by",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows evaluated by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, evaluate_by)?.run()
    }
}

pub fn evaluate_by(
    EvaluateByArgs { evaluate_with }: EvaluateByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;


        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {

            let evaluate_with = if let Some(evaluator) = evaluate_with {
                Some(evaluator.item().clone())
            } else {
                None
            };

            match evaluate(&values[0], evaluate_with, name) {
                Ok(evaluated) => yield ReturnSuccess::value(evaluated),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}

fn fetch(key: Option<String>) -> Box<dyn Fn(Value, Tag) -> Option<Value> + 'static> {
    Box::new(move |value: Value, tag| match &key {
        Some(key_given) => value.get_data_by_key(key_given[..].spanned(tag.span)),
        None => Some(value::int(1).into_value(tag)),
    })
}

pub fn evaluate(
    values: &Value,
    evaluator: Option<String>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let evaluate_with = match evaluator {
        Some(keyfn) => fetch(Some(keyfn)),
        None => fetch(None),
    };

    let results: Value = match values {
        Value {
            value: UntaggedValue::Table(datasets),
            ..
        } => {
            let datasets: Vec<_> = datasets
                .into_iter()
                .map(|subsets| match subsets {
                    Value {
                        value: UntaggedValue::Table(subsets),
                        ..
                    } => {
                        let subsets: Vec<_> = subsets
                            .clone()
                            .into_iter()
                            .map(|data| match data {
                                Value {
                                    value: UntaggedValue::Table(data),
                                    ..
                                } => {
                                    let data: Vec<_> = data
                                        .into_iter()
                                        .map(|x| evaluate_with(x.clone(), tag.clone()).unwrap())
                                        .collect();
                                    UntaggedValue::Table(data).into_value(&tag)
                                }
                                _ => UntaggedValue::Table(vec![]).into_value(&tag),
                            })
                            .collect();
                        UntaggedValue::Table(subsets).into_value(&tag)
                    }
                    _ => UntaggedValue::Table(vec![]).into_value(&tag),
                })
                .collect();

            UntaggedValue::Table(datasets.clone()).into_value(&tag)
        }
        _ => UntaggedValue::Table(vec![]).into_value(&tag),
    };

    Ok(results)
}

#[cfg(test)]
mod tests {

    use crate::commands::evaluate_by::{evaluate, fetch};
    use crate::commands::group_by::group;
    use crate::commands::t_sort_by::t_sort;
    use crate::data::value;
    use crate::prelude::*;
    use indexmap::IndexMap;
    use nu_protocol::{UntaggedValue, Value};
    use nu_source::TaggedItem;

    fn int(s: impl Into<BigInt>) -> Value {
        value::int(s).into_untagged_value()
    }

    fn string(input: impl Into<String>) -> Value {
        value::string(input.into()).into_untagged_value()
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        value::row(entries).into_untagged_value()
    }

    fn table(list: &Vec<Value>) -> Value {
        value::table(list).into_untagged_value()
    }

    fn nu_releases_sorted_by_date() -> Value {
        let key = String::from("date");

        t_sort(
            Some(key),
            None,
            &nu_releases_grouped_by_date(),
            Tag::unknown(),
        )
        .unwrap()
    }

    fn nu_releases_grouped_by_date() -> Value {
        let key = String::from("date").tagged_unknown();
        group(&key, nu_releases_commiters(), Tag::unknown()).unwrap()
    }

    fn nu_releases_commiters() -> Vec<Value> {
        vec![
            row(
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("August 23-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("August 23-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("September 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("September 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("AR"), "country".into() => string("EC"), "date".into() => string("October 10-2019")},
            ),
            row(
                indexmap! {"name".into() => string("JT"), "country".into() => string("NZ"), "date".into() => string("September 24-2019")},
            ),
            row(
                indexmap! {"name".into() => string("YK"), "country".into() => string("US"), "date".into() => string("August 23-2019")},
            ),
        ]
    }

    #[test]
    fn evaluator_fetches_by_column_if_supplied_a_column_name() {
        let subject = row(indexmap! { "name".into() => string("andres") });

        let evaluator = fetch(Some(String::from("name")));

        assert_eq!(evaluator(subject, Tag::unknown()), Some(string("andres")));
    }

    #[test]
    fn evaluator_returns_1_if_no_column_name_given() {
        let subject = row(indexmap! { "name".into() => string("andres") });
        let evaluator = fetch(None);

        assert_eq!(
            evaluator(subject, Tag::unknown()),
            Some(value::int(1).into_untagged_value())
        );
    }

    #[test]
    fn evaluates_the_tables() {
        assert_eq!(
            evaluate(&nu_releases_sorted_by_date(), None, Tag::unknown()).unwrap(),
            table(&vec![table(&vec![
                table(&vec![int(1), int(1), int(1)]),
                table(&vec![int(1), int(1), int(1)]),
                table(&vec![int(1), int(1), int(1)]),
            ]),])
        );
    }

    #[test]
    fn evaluates_the_tables_with_custom_evaluator() {
        let eval = String::from("name");

        assert_eq!(
            evaluate(&nu_releases_sorted_by_date(), Some(eval), Tag::unknown()).unwrap(),
            table(&vec![table(&vec![
                table(&vec![string("AR"), string("JT"), string("YK")]),
                table(&vec![string("AR"), string("YK"), string("JT")]),
                table(&vec![string("YK"), string("JT"), string("AR")]),
            ]),])
        );
    }
}
