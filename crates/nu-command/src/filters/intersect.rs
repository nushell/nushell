use super::utils;
use nu_engine::command_prelude::*;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Intersect;

impl Command for Intersect {
    fn name(&self) -> &str {
        "intersect"
    }

    fn signature(&self) -> Signature {
        Signature::build("intersect")
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
                "The other list to intersect with.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Returns a list of unique elements present in both the input and the provided list."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["common", "shared", "overlap", "filter"]
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
            if other_set.contains(&key) && seen.insert(key) {
                result.push(val);
            }
        }

        Ok(PipelineData::Value(Value::list(result, head), metadata))
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                example: "[1 2 3 4] | intersect [3 4 5 6]",
                description: "Return the intersection of two lists",
                result: Some(Value::test_list(vec![
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                example: "[1 2 3] | intersect [4 5 6]",
                description: "Intersection with no common elements",
                result: Some(Value::test_list(vec![])),
            },
            Example {
                example: "[{a:1} {a:2} {a:3}] | intersect [{a:2} {a:3} {a:4}]",
                description: "Intersection of two tables",
                result: Some(Value::test_list(vec![
                    Value::test_record(record!("a" => Value::test_int(2))),
                    Value::test_record(record!("a" => Value::test_int(3))),
                ])),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::Intersect;
    use nu_protocol::record;
    use nu_test_support::prelude::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Intersect)
    }

    #[test]
    fn intersect_basic() -> Result {
        test()
            .run("[1 2 3 4] | intersect [3 4 5 6]")
            .expect_value_eq([3, 4])
    }

    #[test]
    fn intersect_no_common() -> Result {
        test()
            .run("[1 2 3] | intersect [4 5 6]")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn intersect_all_common() -> Result {
        test()
            .run("[1 2 3] | intersect [1 2 3]")
            .expect_value_eq([1, 2, 3])
    }

    #[test]
    fn intersect_empty_input() -> Result {
        test()
            .run("[] | intersect [1 2 3]")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn intersect_empty_other() -> Result {
        test()
            .run("[1 2 3] | intersect []")
            .expect_value_eq(Value::test_list(vec![]))
    }

    #[test]
    fn intersect_dedups_output() -> Result {
        test()
            .run("[1 1 2 3] | intersect [1 2 2 3]")
            .expect_value_eq([1, 2, 3])
    }

    #[test]
    fn intersect_preserves_input_order() -> Result {
        test()
            .run("[c a b d] | intersect [a d] | str join '-'")
            .expect_value_eq("a-d")
    }

    #[test]
    fn intersect_tables() -> Result {
        let result: Value = test().run("[{a:1} {a:2} {a:3}] | intersect [{a:2} {a:3} {a:4}]")?;
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_record(record!("a" => Value::test_int(2))),
                Value::test_record(record!("a" => Value::test_int(3))),
            ])
        );
        Ok(())
    }

    #[test]
    fn intersect_mixed_types() -> Result {
        test()
            .run("[1 a 2.5 true] | intersect [2.5 b true]")
            .expect_value_eq((2.5f64, true))
    }

    #[test]
    fn intersect_other_not_a_list() {
        let result: nu_test_support::Result = test().run("[1 2] | intersect 42");
        assert!(result.is_err());
    }
}
