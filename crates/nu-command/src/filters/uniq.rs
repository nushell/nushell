use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::PipelineMetadata;
use nu_utils::IgnoreCaseExt;
use std::collections::{HashMap, hash_map::IntoIter};

#[derive(Clone)]
pub struct Uniq;

impl Command for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .switch(
                "count",
                "Return a table containing the distinct input values together with their counts",
                Some('c'),
            )
            .switch(
                "repeated",
                "Return the input values that occur more than once",
                Some('d'),
            )
            .switch(
                "ignore-case",
                "Compare input values case-insensitively",
                Some('i'),
            )
            .switch(
                "unique",
                "Return the input values that occur once only",
                Some('u'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Return the distinct values in the input."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["distinct", "deduplicate", "count"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mapper = Box::new(move |ms: ItemMapperState| -> ValueCounter {
            item_mapper(ms.item, ms.flag_ignore_case, ms.index)
        });

        let metadata = input.metadata();
        uniq(
            engine_state,
            stack,
            call,
            input.into_iter().collect(),
            mapper,
            metadata,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Return the distinct values of a list/table (remove duplicates so that each value occurs once only)",
                example: "[2 3 3 4] | uniq",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(3), Value::test_int(4)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Return the input values that occur more than once",
                example: "[1 2 2] | uniq -d",
                result: Some(Value::list(vec![Value::test_int(2)], Span::test_data())),
            },
            Example {
                description: "Return the input values that occur once only",
                example: "[1 2 2] | uniq --unique",
                result: Some(Value::list(vec![Value::test_int(1)], Span::test_data())),
            },
            Example {
                description: "Ignore differences in case when comparing input values",
                example: "['hello' 'goodbye' 'Hello'] | uniq --ignore-case",
                result: Some(Value::test_list(vec![
                    Value::test_string("hello"),
                    Value::test_string("goodbye"),
                ])),
            },
            Example {
                description: "Return a table containing the distinct input values together with their counts",
                example: "[1 2 2] | uniq --count",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "value" => Value::test_int(1),
                        "count" => Value::test_int(1),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_int(2),
                        "count" => Value::test_int(2),
                    }),
                ])),
            },
        ]
    }
}

pub struct ItemMapperState {
    pub item: Value,
    pub flag_ignore_case: bool,
    pub index: usize,
}

fn item_mapper(item: Value, flag_ignore_case: bool, index: usize) -> ValueCounter {
    ValueCounter::new(item, flag_ignore_case, index)
}

pub struct ValueCounter {
    val: Value,
    val_to_compare: Value,
    count: i64,
    index: usize,
}

impl PartialEq<Self> for ValueCounter {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl ValueCounter {
    fn new(val: Value, flag_ignore_case: bool, index: usize) -> Self {
        Self::new_vals_to_compare(val.clone(), flag_ignore_case, val, index)
    }
    pub fn new_vals_to_compare(
        val: Value,
        flag_ignore_case: bool,
        vals_to_compare: Value,
        index: usize,
    ) -> Self {
        ValueCounter {
            val,
            val_to_compare: if flag_ignore_case {
                clone_to_folded_case(&vals_to_compare.with_span(Span::unknown()))
            } else {
                vals_to_compare.with_span(Span::unknown())
            },
            count: 1,
            index,
        }
    }
}

fn clone_to_folded_case(value: &Value) -> Value {
    let span = value.span();
    match value {
        Value::String { val: s, .. } => Value::string(s.clone().to_folded_case(), span),
        Value::List { vals: vec, .. } => {
            Value::list(vec.iter().map(clone_to_folded_case).collect(), span)
        }
        Value::Record { val: record, .. } => Value::record(
            record
                .iter()
                .map(|(k, v)| (k.to_owned(), clone_to_folded_case(v)))
                .collect(),
            span,
        ),
        other => other.clone(),
    }
}

fn sort_attributes(val: Value) -> Value {
    let span = val.span();
    match val {
        Value::Record { val, .. } => {
            // TODO: sort inplace
            let sorted = val
                .into_owned()
                .into_iter()
                .sorted_by(|a, b| a.0.cmp(&b.0))
                .collect_vec();

            let record = sorted
                .into_iter()
                .map(|(k, v)| (k, sort_attributes(v)))
                .collect();

            Value::record(record, span)
        }
        Value::List { vals, .. } => {
            Value::list(vals.into_iter().map(sort_attributes).collect_vec(), span)
        }
        other => other,
    }
}

fn generate_key(engine_state: &EngineState, item: &ValueCounter) -> Result<String, ShellError> {
    let value = sort_attributes(item.val_to_compare.clone()); //otherwise, keys could be different for Records
    nuon::to_nuon(
        engine_state,
        &value,
        nuon::ToStyle::Default,
        Some(Span::unknown()),
        false,
    )
}

fn generate_results_with_count(head: Span, uniq_values: Vec<ValueCounter>) -> Vec<Value> {
    uniq_values
        .into_iter()
        .map(|item| {
            Value::record(
                record! {
                    "value" => item.val,
                    "count" => Value::int(item.count, head),
                },
                head,
            )
        })
        .collect()
}

pub fn uniq(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: Vec<Value>,
    item_mapper: Box<dyn Fn(ItemMapperState) -> ValueCounter>,
    metadata: Option<PipelineMetadata>,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let flag_show_count = call.has_flag(engine_state, stack, "count")?;
    let flag_show_repeated = call.has_flag(engine_state, stack, "repeated")?;
    let flag_ignore_case = call.has_flag(engine_state, stack, "ignore-case")?;
    let flag_only_uniques = call.has_flag(engine_state, stack, "unique")?;

    let signals = engine_state.signals().clone();
    let uniq_values = input
        .into_iter()
        .enumerate()
        .map_while(|(index, item)| {
            if signals.interrupted() {
                return None;
            }
            Some(item_mapper(ItemMapperState {
                item,
                flag_ignore_case,
                index,
            }))
        })
        .try_fold(
            HashMap::<String, ValueCounter>::new(),
            |mut counter, item| {
                let key = generate_key(engine_state, &item);

                match key {
                    Ok(key) => {
                        match counter.get_mut(&key) {
                            Some(x) => x.count += 1,
                            None => {
                                counter.insert(key, item);
                            }
                        };
                        Ok(counter)
                    }
                    Err(err) => Err(err),
                }
            },
        );

    let mut uniq_values: HashMap<String, ValueCounter> = uniq_values?;

    if flag_show_repeated {
        uniq_values.retain(|_v, value_count_pair| value_count_pair.count > 1);
    }

    if flag_only_uniques {
        uniq_values.retain(|_v, value_count_pair| value_count_pair.count == 1);
    }

    let uniq_values = sort(uniq_values.into_iter());

    let result = if flag_show_count {
        generate_results_with_count(head, uniq_values)
    } else {
        uniq_values.into_iter().map(|v| v.val).collect()
    };

    Ok(Value::list(result, head).into_pipeline_data_with_metadata(metadata))
}

fn sort(iter: IntoIter<String, ValueCounter>) -> Vec<ValueCounter> {
    iter.map(|item| item.1)
        .sorted_by(|a, b| a.index.cmp(&b.index))
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Uniq {})
    }
}
