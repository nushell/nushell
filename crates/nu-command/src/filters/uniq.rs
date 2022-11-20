use std::collections::VecDeque;

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
                "Ignore differences in case when comparing input values",
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

fn to_lowercase(value: nu_protocol::Value) -> nu_protocol::Value {
    match value {
        Value::String { val: s, span } => Value::String {
            val: s.to_lowercase(),
            span,
        },
        other => other,
    }
}

fn generate_results_with_count(head: Span, uniq_values: Vec<(Value, i64)>) -> Vec<Value> {
    uniq_values
        .into_iter()
        .map(|item| Value::Record {
            cols: vec!["value".to_string(), "count".to_string()],
            vals: vec![
                item.0,
                Value::Int {
                    val: item.1,
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
    let should_show_count = call.has_flag("count");
    let show_repeated = call.has_flag("repeated");
    let ignore_case = call.has_flag("ignore-case");
    let only_uniques = call.has_flag("unique");
    let metadata = input.metadata();

    let uniq_values = {
        input
            .into_iter()
            .map(|item| {
                if ignore_case {
                    to_lowercase(item)
                } else {
                    item
                }
            })
            .fold(Vec::<(Value, i64)>::new(), |mut counter, item| {
                match counter.iter_mut().find(|x| x.0 == item) {
                    Some(x) => x.1 += 1,
                    None => counter.push((item, 1)),
                };
                counter
            })
            .into_iter()
            //Trusting the compiler to optimize loop invariants. If not, CPU branch predictor will skip around
            .filter(|value| show_repeated && (value.1 > 1))
            .filter(|value| only_uniques && (value.1 == 1))
            .collect::<Vec<(nu_protocol::Value, i64)>>()
    };

    let result = if should_show_count {
        generate_results_with_count(head, uniq_values)
    } else {
        uniq_values.into_iter().map(|v| v.0).collect()
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
