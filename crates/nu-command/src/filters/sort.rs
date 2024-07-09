use alphanumeric_sort::compare_str;
use nu_engine::command_prelude::*;

use nu_protocol::{ast::PathMember, IntoValue};
use nu_utils::IgnoreCaseExt;
use std::cmp::Ordering;

use crate::{compare_by, compare_values, Comparator};

#[derive(Clone)]
pub struct Sort;

impl Command for Sort {
    fn name(&self) -> &str {
        "sort"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any))
                ),
                (Type::record(), Type::record())
            ])
    .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "ignore-case",
                "Sort string-based data case-insensitively",
                Some('i'),
            )
            .switch(
                "values",
                "If input is a single record, sort the record by values; ignored if input is not a single record",
                Some('v'),
            )
            .switch(
                "natural",
                "Sort alphanumeric string-based values naturally (1, 9, 10, 99, 100, ...)",
                Some('n'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sort in increasing order."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[2 0 1] | sort",
                description: "sort the list by increasing value",
                result: Some(Value::list(
                    vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[2 0 1] | sort --reverse",
                description: "sort the list by decreasing value",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(1), Value::test_int(0)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[betty amy sarah] | sort",
                description: "sort a list of strings",
                result: Some(Value::list(
                    vec![
                        Value::test_string("amy"),
                        Value::test_string("betty"),
                        Value::test_string("sarah"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[betty amy sarah] | sort --reverse",
                description: "sort a list of strings in reverse",
                result: Some(Value::list(
                    vec![
                        Value::test_string("sarah"),
                        Value::test_string("betty"),
                        Value::test_string("amy"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Sort strings (case-insensitive)",
                example: "[airplane Truck Car] | sort -i",
                result: Some(Value::list(
                    vec![
                        Value::test_string("airplane"),
                        Value::test_string("Car"),
                        Value::test_string("Truck"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Sort strings (reversed case-insensitive)",
                example: "[airplane Truck Car] | sort -i -r",
                result: Some(Value::list(
                    vec![
                        Value::test_string("Truck"),
                        Value::test_string("Car"),
                        Value::test_string("airplane"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Sort record by key (case-insensitive)",
                example: "{b: 3, a: 4} | sort",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(4),
                    "b" => Value::test_int(3),
                })),
            },
            Example {
                description: "Sort record by value",
                example: "{b: 4, a: 3, c:1} | sort -v",
                result: Some(Value::test_record(record! {
                    "c" => Value::test_int(1),
                    "a" => Value::test_int(3),
                    "b" => Value::test_int(4),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let reverse = call.has_flag(engine_state, stack, "reverse")?;
        let insensitive = call.has_flag(engine_state, stack, "ignore-case")?;
        let natural = call.has_flag(engine_state, stack, "natural")?;
        let sort_by_value = call.has_flag(engine_state, stack, "values")?;
        let metadata = input.metadata();

        let span = input.span().unwrap_or(call.head);
        let value = input.into_value(span)?;
        let sorted: Value = match value {
            Value::Record { val, .. } => {
                // Records have two sorting methods, toggled by presence or absence of -v
                let record = crate::sort_record(
                    val.into_owned(),
                    sort_by_value,
                    reverse,
                    insensitive,
                    natural,
                )?;
                Value::record(record, span)
            }
            Value::List { vals, .. } => {
                let mut vec = vals.to_owned();

                crate::sort(&mut vec, insensitive, natural)?;

                if reverse {
                    vec.reverse()
                }

                Value::list(vec, span)
            }
            Value::Nothing { .. } => {
                return Err(ShellError::PipelineEmpty {
                    dst_span: value.span(),
                })
            }
            _ => {
                return Err(ShellError::PipelineMismatch {
                    exp_input_type: "record or list".to_string(),
                    dst_span: call.head,
                    src_span: value.span(),
                })
            }
        };
        Ok(sorted.into_pipeline_data_with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {

    use nu_protocol::engine::CommandType;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Sort {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Sort.command_type(), CommandType::Builtin));
    }
}
