use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct Uniq;

impl Command for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (
                    // -c
                    Type::List(Box::new(Type::Any)),
                    Type::Table(vec![]),
                ),
            ])
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

    fn usage(&self) -> &str {
        "Return the distinct values in the input."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["distinct", "deduplicate"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        uniq(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the distinct values of a list/table (remove duplicates so that each value occurs once only)",
                example: "[2 3 3 4] | uniq",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(3), Value::test_int(4)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the input values that occur more than once",
                example: "[1 2 2] | uniq -d",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the input values that occur once only",
                example: "[1 2 2] | uniq -u",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Ignore differences in case when comparing input values",
                example: "['hello' 'goodbye' 'Hello'] | uniq -i",
                result: Some(Value::List {
                    vals: vec![Value::test_string("hello"), Value::test_string("goodbye")],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return a table containing the distinct input values together with their counts",
                example: "[1 2 2] | uniq -c",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["value".to_string(), "count".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(1)],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["value".to_string(), "count".to_string()],
                            vals: vec![Value::test_int(2), Value::test_int(2)],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

struct ValueCounter {
    val: Value,
    val_to_compare: Value,
    count: i64,
}

impl PartialEq<Self> for ValueCounter {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl ValueCounter {
    fn new(val: Value, flag_ignore_case: bool) -> Self {
        ValueCounter {
            val: val.clone(),
            val_to_compare: if flag_ignore_case {
                clone_to_lowercase(&val)
            } else {
                val
            },
            count: 1,
        }
    }
}

fn clone_to_lowercase(value: &Value) -> Value {
    match value {
        Value::String { val: s, span } => Value::String {
            val: s.clone().to_lowercase(),
            span: *span,
        },
        Value::List { vals: vec, span } => Value::List {
            vals: vec
                .clone()
                .into_iter()
                .map(|v| clone_to_lowercase(&v))
                .collect(),
            span: *span,
        },
        Value::Record { cols, vals, span } => Value::Record {
            cols: cols.clone(),
            vals: vals
                .clone()
                .into_iter()
                .map(|v| clone_to_lowercase(&v))
                .collect(),
            span: *span,
        },
        other => other.clone(),
    }
}

fn generate_results_with_count(head: Span, uniq_values: Vec<ValueCounter>) -> Vec<Value> {
    uniq_values
        .into_iter()
        .map(|item| Value::Record {
            cols: vec!["value".to_string(), "count".to_string()],
            vals: vec![
                item.val,
                Value::Int {
                    val: item.count,
                    span: head,
                },
            ],
            span: head,
        })
        .collect()
}

fn uniq(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    let flag_show_count = call.has_flag("count");
    let flag_show_repeated = call.has_flag("repeated");
    let flag_ignore_case = call.has_flag("ignore-case");
    let flag_only_uniques = call.has_flag("unique");
    let metadata = input.metadata();

    let mut uniq_values = input
        .into_iter()
        .map(|item| ValueCounter::new(item, flag_ignore_case))
        .fold(Vec::<ValueCounter>::new(), |mut counter, item| {
            match counter
                .iter_mut()
                .find(|x| x.val_to_compare == item.val_to_compare)
            {
                Some(x) => x.count += 1,
                None => counter.push(item),
            };
            counter
        });

    if flag_show_repeated {
        uniq_values.retain(|value_count_pair| value_count_pair.count > 1);
    }

    if flag_only_uniques {
        uniq_values.retain(|value_count_pair| value_count_pair.count == 1);
    }

    let result = if flag_show_count {
        generate_results_with_count(head, uniq_values)
    } else {
        uniq_values.into_iter().map(|v| v.val).collect()
    };

    Ok(Value::List {
        vals: result,
        span: head,
    }
    .into_pipeline_data()
    .set_metadata(metadata))
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
