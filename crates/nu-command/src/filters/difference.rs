use super::utils;
use nu_engine::command_prelude::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Difference;

impl Command for Difference {
    fn name(&self) -> &str {
        "difference"
    }

    fn signature(&self) -> Signature {
        Signature::build("difference")
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
                "The other list to subtract from the input.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Returns a list of unique elements in the input that are not present in the other list."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["subtract", "minus", "exclude", "remove"]
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

        let other_vals = utils::extract_other_list(engine_state, stack, call, head)?;

        let other_set: HashSet<String> = other_vals
            .iter()
            .map(|v| utils::value_to_key(engine_state, v, head))
            .collect::<Result<HashSet<_>, _>>()?;

        let signals = engine_state.signals().clone();
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::new();

        for val in input {
            signals.check(&head)?;
            let key = utils::value_to_key(engine_state, &val, head)?;
            if !other_set.contains(&key) && seen.insert(key) {
                result.push(val);
            }
        }

        Ok(PipelineData::Value(Value::list(result, head), metadata))
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                example: "[1 2 3 4] | difference [3 4 5 6]",
                description: "Return the difference of two lists",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                example: "[1 1 2 3] | difference [2 3]",
                description: "Difference with duplicates in input",
                result: Some(Value::test_list(vec![Value::test_int(1)])),
            },
            Example {
                example: "[{a:1} {a:2} {a:3}] | difference [{a:2} {a:4}]",
                description: "Difference of two tables",
                result: Some(Value::test_list(vec![
                    Value::test_record(record!("a" => Value::test_int(1))),
                    Value::test_record(record!("a" => Value::test_int(3))),
                ])),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::Difference;
    use nu_protocol::record;
    use nu_test_support::prelude::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Difference)
    }

    #[test]
    fn difference_basic() -> Result {
        test()
            .run("[1 2 3 4] | difference [3 4 5 6]")
            .expect_value_eq([1, 2])
    }

    #[test]
    fn difference_no_common() -> Result {
        test()
            .run("[1 2 3] | difference [4 5 6]")
            .expect_value_eq([1, 2, 3])
    }

    #[test]
    fn difference_all_common() -> Result {
        test()
            .run("[1 2 3] | difference [1 2 3]")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn difference_empty_input() -> Result {
        test()
            .run("[] | difference [1 2 3]")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn difference_empty_other() -> Result {
        test()
            .run("[1 2 3] | difference []")
            .expect_value_eq([1, 2, 3])
    }

    #[test]
    fn difference_dedups_output() -> Result {
        test()
            .run("[1 1 2 3] | difference [2]")
            .expect_value_eq([1, 3])
    }

    #[test]
    fn difference_preserves_input_order() -> Result {
        test()
            .run("[c a b d] | difference [a d] | str join '-'")
            .expect_value_eq("c-b")
    }

    #[test]
    fn difference_tables() -> Result {
        let result: Value = test().run("[{a:1} {a:2} {a:3}] | difference [{a:2} {a:4}]")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_record(record!("a" => Value::test_int(1))),
                Value::test_record(record!("a" => Value::test_int(3))),
            ])
        );
        Ok(())
    }

    #[test]
    fn difference_mixed_types() -> Result {
        test()
            .run("[1 a 2.5 true] | difference [2.5 b]")
            .expect_value_eq((1, "a", true))
    }

    #[test]
    fn difference_other_not_a_list() {
        let result: nu_test_support::Result = test().run("[1 2] | difference 42");
        assert!(result.is_err());
    }
}
