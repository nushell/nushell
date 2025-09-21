use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Append;

impl Command for Append {
    fn name(&self) -> &str {
        "append"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("append")
            .input_output_types(vec![(Type::Any, Type::List(Box::new(Type::Any)))])
            .required(
                "row",
                SyntaxShape::Any,
                "The row, list, or table to append.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Append any number of rows to a table."
    }

    fn extra_description(&self) -> &str {
        r#"Be aware that this command 'unwraps' lists passed to it. So, if you pass a variable to it,
and you want the variable's contents to be appended without being unwrapped, it's wise to
pre-emptively wrap the variable in a list, like so: `append [$val]`. This way, `append` will
only unwrap the outer list, and leave the variable's contents untouched."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add", "concatenate"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[0 1 2 3] | append 4",
                description: "Append one int to a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                example: "0 | append [1 2 3]",
                description: "Append a list to an item",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            },
            Example {
                example: r#""a" | append ["b"] "#,
                description: "Append a list of string to a string",
                result: Some(Value::test_list(vec![
                    Value::test_string("a"),
                    Value::test_string("b"),
                ])),
            },
            Example {
                example: "[0 1] | append [2 3 4]",
                description: "Append three int items",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                example: "[0 1] | append [2 nu 4 shell]",
                description: "Append ints and strings",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_string("nu"),
                    Value::test_int(4),
                    Value::test_string("shell"),
                ])),
            },
            Example {
                example: "[0 1] | append 2..4",
                description: "Append a range of ints to a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
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
        let other: Value = call.req(engine_state, stack, 0)?;
        let metadata = input.metadata();

        Ok(input
            .into_iter()
            .chain(other.into_pipeline_data())
            .into_pipeline_data_with_metadata(call.head, engine_state.signals().clone(), metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Append {})
    }
}
