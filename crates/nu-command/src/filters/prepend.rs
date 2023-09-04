use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

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
                "the row, list, or table to prepend",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Prepend any number of rows to a table."
    }

    fn extra_usage(&self) -> &str {
        r#"Be aware that this command 'unwraps' lists passed to it. So, if you pass a variable to it,
and you want the variable's contents to be prepended without being unwrapped, it's wise to
pre-emptively wrap the variable in a list, like so: `prepend [$val]`. This way, `prepend` will
only unwrap the outer list, and leave the variable's contents untouched."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add", "concatenate"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "0 | prepend [1 2 3]",
                description: "prepend a list to an item",
                result: Some(Value::list(
                    vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(0),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: r#""a" | prepend ["b"] "#,
                description: "Prepend a list of strings to a string",
                result: Some(Value::list(
                    vec![Value::test_string("b"), Value::test_string("a")],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[1 2 3 4] | prepend 0",
                description: "Prepend one integer item",
                result: Some(Value::list(
                    vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[2 3 4] | prepend [0 1]",
                description: "Prepend two integer items",
                result: Some(Value::list(
                    vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[2 nu 4 shell] | prepend [0 1 rocks]",
                description: "Prepend integers and strings",
                result: Some(Value::list(
                    vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_string("rocks"),
                        Value::test_int(2),
                        Value::test_string("nu"),
                        Value::test_int(4),
                        Value::test_string("shell"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[3 4] | prepend 0..2",
                description: "Prepend a range",
                result: Some(Value::list(
                    vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    Span::test_data(),
                )),
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
            .into_pipeline_data(engine_state.ctrlc.clone())
            .set_metadata(metadata))
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
