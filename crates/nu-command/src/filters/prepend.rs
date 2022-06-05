use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Prepend;

impl Command for Prepend {
    fn name(&self) -> &str {
        "prepend"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("prepend")
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
                        Value::Int(0),
                        Value::Int(1),
                        Value::Int(2),
                        Value::Int(3),
                        Value::Int(4),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2,3,4] | prepend [0,1]",
                description: "Prepend two Int items",
                result: Some(Value::List {
                    vals: vec![
                        Value::Int(0),
                        Value::Int(1),
                        Value::Int(2),
                        Value::Int(3),
                        Value::Int(4),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2,nu,4,shell] | prepend [0,1,rocks]",
                description: "Prepend Ints and Strings",
                result: Some(Value::List {
                    vals: vec![
                        Value::Int(0),
                        Value::Int(1),
                        Value::String("rocks".into()),
                        Value::Int(2),
                        Value::String("nu".into()),
                        Value::Int(4),
                        Value::String("shell".into()),
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
