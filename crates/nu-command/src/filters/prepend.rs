use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Prepend;

impl Command for Prepend {
    fn name(&self) -> &str {
        "prepend"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("prepend")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
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
                example: "[1,2,3,4] | prepend 0",
                description: "Prepend one Int item",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2,3,4] | prepend [0,1]",
                description: "Prepend two Int items",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2,nu,4,shell] | prepend [0,1,rocks]",
                description: "Prepend Ints and Strings",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_string("rocks"),
                        Value::test_int(2),
                        Value::test_string("nu"),
                        Value::test_int(4),
                        Value::test_string("shell"),
                    ],
                    span: Span::test_data(),
                }),
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
        let val: Value = call.req(engine_state, stack, 0)?;
        let vec: Vec<Value> = process_value(val);
        let metadata = input.metadata();

        Ok(vec
            .into_iter()
            .chain(input)
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone())
            .set_metadata(metadata))
    }
}

fn process_value(val: Value) -> Vec<Value> {
    match val {
        Value::List {
            vals: input_vals,
            span: _,
        } => {
            let mut output = vec![];
            for input_val in input_vals {
                output.push(input_val);
            }
            output
        }
        _ => {
            vec![val]
        }
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
