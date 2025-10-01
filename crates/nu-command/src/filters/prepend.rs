use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Prepend;

impl Command for Prepend {
    fn name(&self) -> &str {
        "prepend"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("prepend")
            .input_output_types(vec![(Type::Any, Type::List(Box::new(Type::Any)))])
            .required(
                "row",
                SyntaxShape::Any,
                "The row, list, or table to prepend.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Prepend any number of rows to a table."
    }

    fn extra_description(&self) -> &str {
        r#"Be aware that this command 'unwraps' lists passed to it. So, if you pass a variable to it,
and you want the variable's contents to be prepended without being unwrapped, it's wise to
pre-emptively wrap the variable in a list, like so: `prepend [$val]`. This way, `prepend` will
only unwrap the outer list, and leave the variable's contents untouched."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add", "concatenate"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "0 | prepend [1 2 3]",
                description: "prepend a list to an item",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(0),
                ])),
            },
            Example {
                example: r#""a" | prepend ["b"] "#,
                description: "Prepend a list of strings to a string",
                result: Some(Value::test_list(vec![
                    Value::test_string("b"),
                    Value::test_string("a"),
                ])),
            },
            Example {
                example: "[1 2 3 4] | prepend 0",
                description: "Prepend one int item",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                example: "[2 3 4] | prepend [0 1]",
                description: "Prepend two int items",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                ])),
            },
            Example {
                example: "[2 nu 4 shell] | prepend [0 1 rocks]",
                description: "Prepend ints and strings",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_string("rocks"),
                    Value::test_int(2),
                    Value::test_string("nu"),
                    Value::test_int(4),
                    Value::test_string("shell"),
                ])),
            },
            Example {
                example: "[3 4] | prepend 0..2",
                description: "Prepend a range",
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

        Ok(other
            .into_pipeline_data()
            .into_iter()
            .chain(input)
            .into_pipeline_data_with_metadata(call.head, engine_state.signals().clone(), metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Prepend {})
    }
}
