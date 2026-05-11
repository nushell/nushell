use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct CrossJoin;

impl Command for CrossJoin {
    fn name(&self) -> &str {
        "cross_join"
    }

    fn signature(&self) -> Signature {
        Signature::build("cross_join")
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
                (
                    Type::Any,
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
            ])
            .required(
                "right",
                SyntaxShape::Any,
                "The right-hand side list or table to cross join with.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Cross join (Cartesian product) the input with another list or table."
    }

    fn extra_description(&self) -> &str {
        "Produces all combinations (pairs) between the input and the given list or table.

When the input and right side are both tables (lists of records), the resulting
records have all columns combined. For plain lists, each result is a two-element list."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["cartesian", "product"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Cross join two integer lists to produce all pairs.",
                example: "[1 2] | cross_join [3 4]",
                result: Some(Value::test_list(vec![
                    Value::list(
                        vec![Value::test_int(1), Value::test_int(3)],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_int(1), Value::test_int(4)],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_int(2), Value::test_int(3)],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_int(2), Value::test_int(4)],
                        Span::test_data(),
                    ),
                ])),
            },
            Example {
                description: "Cross join two string lists.",
                example: "[dev qa prod] | cross_join [web db]",
                result: Some(Value::test_list(vec![
                    Value::list(
                        vec![Value::test_string("dev"), Value::test_string("web")],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_string("dev"), Value::test_string("db")],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_string("qa"), Value::test_string("web")],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_string("qa"), Value::test_string("db")],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_string("prod"), Value::test_string("web")],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_string("prod"), Value::test_string("db")],
                        Span::test_data(),
                    ),
                ])),
            },
            Example {
                description: "Cross join two tables, merging columns.",
                example: "[{a: 1} {a: 2}] | cross_join [{b: 3} {b: 4}]",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(3),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(4),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                        "b" => Value::test_int(3),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                        "b" => Value::test_int(4),
                    }),
                ])),
            },
            Example {
                description: "Cross join a scalar value with a list. The scalar is treated as a single-item list.",
                example: "1 | cross_join [a b c]",
                result: Some(Value::test_list(vec![
                    Value::list(
                        vec![Value::test_int(1), Value::test_string("a")],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_int(1), Value::test_string("b")],
                        Span::test_data(),
                    ),
                    Value::list(
                        vec![Value::test_int(1), Value::test_string("c")],
                        Span::test_data(),
                    ),
                ])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let right: Value = call.req(engine_state, stack, 0)?;
        let metadata = input.take_metadata();

        let collected_input = input.into_value(head)?;
        let left_vals = value_to_vec(collected_input, head);
        let right_vals = value_to_vec(right, head);

        let mut result = Vec::with_capacity(left_vals.len().saturating_mul(right_vals.len()));

        for left in &left_vals {
            for right in &right_vals {
                result.push(combine_values(left, right, head));
            }
        }

        Ok(PipelineData::value(Value::list(result, head), metadata))
    }
}

/// Convert a Value into a Vec<Value>, treating scalars as single-element lists.
fn value_to_vec(value: Value, _span: Span) -> Vec<Value> {
    match value {
        Value::List { vals, .. } => vals,
        Value::Nothing { .. } => Vec::new(),
        other => vec![other],
    }
}

/// Combine two values into a single value.
/// Records are merged (left columns take precedence). Everything else becomes a 2-element list.
fn combine_values(left: &Value, right: &Value, span: Span) -> Value {
    match (left, right) {
        (Value::Record { val: l, .. }, Value::Record { val: r, .. }) => {
            let mut record = (**l).clone();
            for (k, v) in r.iter() {
                if !record.contains(k) {
                    record.push(k.clone(), v.clone());
                }
            }
            Value::record(record, span)
        }
        _ => Value::list(vec![left.clone(), right.clone()], span),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(CrossJoin)
    }
}
