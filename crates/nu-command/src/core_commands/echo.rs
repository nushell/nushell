use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Echo;

impl Command for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn usage(&self) -> &str {
        "Echo the arguments back to the user."
    }

    fn signature(&self) -> Signature {
        Signature::build("echo")
            .rest("rest", SyntaxShape::Any, "the values to echo")
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        "Unlike `print`, this command returns an actual value that will be passed to the next command of the pipeline."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        call.rest(engine_state, stack, 0).map(|to_be_echoed| {
            let n = to_be_echoed.len();
            match n.cmp(&1usize) {
                //  More than one value is converted in a stream of values
                std::cmp::Ordering::Greater => PipelineData::ListStream(
                    ListStream::from_stream(to_be_echoed.into_iter(), engine_state.ctrlc.clone()),
                    None,
                ),

                //  But a single value can be forwarded as it is
                std::cmp::Ordering::Equal => PipelineData::Value(to_be_echoed[0].clone(), None),

                //  When there are no elements, we echo the empty string
                std::cmp::Ordering::Less => PipelineData::Value(
                    Value::String {
                        val: "".to_string(),
                        span: call.head,
                    },
                    None,
                ),
            }
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put a hello message in the pipeline",
                example: "echo 'hello'",
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Print the value of the special '$nu' variable",
                example: "echo $nu",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Echo;
        use crate::test_examples;
        test_examples(Echo {})
    }
}
