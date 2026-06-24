use super::utils;
use nu_engine::command_prelude::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Union;

impl Command for Union {
    fn name(&self) -> &str {
        "union"
    }

    fn signature(&self) -> Signature {
        Signature::build("union")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::table(), Type::table()),
            ])
            .required(
                "other",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "The other list to union with.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Returns a list of unique elements from both the input and the provided list."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["distinct", "merge", "deduplicate", "combine"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.take_metadata();

        let input_vals: Vec<Value> = input.into_iter().collect();
        let other_vals = utils::extract_other_list(engine_state, stack, call, head)?;

        let signals = engine_state.signals().clone();
        let total_capacity = input_vals.len() + other_vals.len();
        let mut seen: HashSet<String> = HashSet::with_capacity(total_capacity);
        let mut result = Vec::with_capacity(total_capacity);

        for val in input_vals {
            signals.check(&head)?;
            if seen.insert(utils::value_to_key(engine_state, &val, head)?) {
                result.push(val);
            }
        }

        for val in other_vals {
            signals.check(&head)?;
            if seen.insert(utils::value_to_key(engine_state, &val, head)?) {
                result.push(val);
            }
        }

        Ok(PipelineData::Value(Value::list(result, head), metadata))
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                example: "[1 2 3 4] | union [3 4 5 6]",
                description: "Return the union of two lists",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                    Value::test_int(5),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: "[1 1 2 3] | union [2 3 4]",
                description: "Union with duplicates in input",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                example: "[{a:1} {a:2}] | union [{a:2} {a:3}]",
                description: "Union of two tables (dedup rows)",
                result: Some(Value::test_list(vec![
                    Value::test_record(record!("a" => Value::test_int(1))),
                    Value::test_record(record!("a" => Value::test_int(2))),
                    Value::test_record(record!("a" => Value::test_int(3))),
                ])),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::Union;
    use nu_protocol::record;
    use nu_test_support::prelude::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Union)
    }

    #[test]
    fn union_basic() -> Result {
        test()
            .run("[1 2 3 4] | union [3 4 5 6]")
            .expect_value_eq([1, 2, 3, 4, 5, 6])
    }

    #[test]
    fn union_dedups_input() -> Result {
        test()
            .run("[1 1 2 3] | union [2 3 4]")
            .expect_value_eq([1, 2, 3, 4])
    }

    #[test]
    fn union_dedups_both() -> Result {
        test()
            .run("[1 2 2 3] | union [2 3 3 4]")
            .expect_value_eq([1, 2, 3, 4])
    }

    #[test]
    fn union_empty_input() -> Result {
        test().run("[] | union [1 2 3]").expect_value_eq([1, 2, 3])
    }

    #[test]
    fn union_empty_other() -> Result {
        test().run("[1 2 3] | union []").expect_value_eq([1, 2, 3])
    }

    #[test]
    fn union_both_empty() -> Result {
        test()
            .run("[] | union []")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn union_preserves_input_order() -> Result {
        test()
            .run("[c a b] | union [d e f] | str join '-'")
            .expect_value_eq("c-a-b-d-e-f")
    }

    #[test]
    fn union_with_other_overlap_first() -> Result {
        // Elements from other list that are already in input are skipped.
        test()
            .run("[1 2 3] | union [1 2 3 4 5]")
            .expect_value_eq([1, 2, 3, 4, 5])
    }

    #[test]
    fn union_tables() -> Result {
        let result: Value = test().run("[{a:1} {a:2}] | union [{a:2} {a:3}]")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_record(record!("a" => Value::test_int(1))),
                Value::test_record(record!("a" => Value::test_int(2))),
                Value::test_record(record!("a" => Value::test_int(3))),
            ])
        );
        Ok(())
    }

    #[test]
    fn union_mixed_types() -> Result {
        test()
            .run("[1 a 2.5] | union [2.5 b 3]")
            .expect_value_eq((1, "a", 2.5f64, "b", 3))
    }
}
