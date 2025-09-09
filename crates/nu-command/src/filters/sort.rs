use nu_engine::command_prelude::*;
use nu_protocol::{ast::PathMember, casing::Casing};

use crate::Comparator;

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

    fn description(&self) -> &str {
        "Sort in increasing order."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[2 0 1] | sort",
                description: "Sort the list by increasing value",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                example: "[2 0 1] | sort --reverse",
                description: "Sort the list by decreasing value",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(1),
                    Value::test_int(0),
                ])),
            },
            Example {
                example: "[betty amy sarah] | sort",
                description: "Sort a list of strings",
                result: Some(Value::test_list(vec![
                    Value::test_string("amy"),
                    Value::test_string("betty"),
                    Value::test_string("sarah"),
                ])),
            },
            Example {
                example: "[betty amy sarah] | sort --reverse",
                description: "Sort a list of strings in reverse",
                result: Some(Value::test_list(vec![
                    Value::test_string("sarah"),
                    Value::test_string("betty"),
                    Value::test_string("amy"),
                ])),
            },
            Example {
                description: "Sort strings (case-insensitive)",
                example: "[airplane Truck Car] | sort -i",
                result: Some(Value::test_list(vec![
                    Value::test_string("airplane"),
                    Value::test_string("Car"),
                    Value::test_string("Truck"),
                ])),
            },
            Example {
                description: "Sort strings (reversed case-insensitive)",
                example: "[airplane Truck Car] | sort -i -r",
                result: Some(Value::test_list(vec![
                    Value::test_string("Truck"),
                    Value::test_string("Car"),
                    Value::test_string("airplane"),
                ])),
            },
            Example {
                description: "Sort alphanumeric strings in natural order",
                example: "[foo1 foo10 foo9] | sort -n",
                result: Some(Value::test_list(vec![
                    Value::test_string("foo1"),
                    Value::test_string("foo9"),
                    Value::test_string("foo10"),
                ])),
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
            value @ Value::List { .. } => {
                // If we have a table specifically, then we want to sort along each column.
                // Record's PartialOrd impl dictates that columns are compared in alphabetical order,
                // so we have to explicitly compare by each column.
                let r#type = value.get_type();
                let mut vec = value.into_list().expect("matched list above");
                if let Type::Table(cols) = r#type {
                    let columns: Vec<Comparator> = cols
                        .iter()
                        .map(|col| {
                            vec![PathMember::string(
                                col.0.clone(),
                                false,
                                Casing::Sensitive,
                                Span::unknown(),
                            )]
                        })
                        .map(|members| CellPath { members })
                        .map(Comparator::CellPath)
                        .collect();
                    crate::sort_by(&mut vec, columns, span, insensitive, natural)?;
                } else {
                    crate::sort(&mut vec, insensitive, natural)?;
                }

                if reverse {
                    vec.reverse()
                }

                Value::list(vec, span)
            }
            Value::Nothing { .. } => {
                return Err(ShellError::PipelineEmpty {
                    dst_span: value.span(),
                });
            }
            ref other => {
                return Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "record or list".to_string(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: call.head,
                    src_span: value.span(),
                });
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
