use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value, ValueStream,
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
        Signature::build("echo").rest("rest", SyntaxShape::Any, "the values to echo")
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        call.rest(engine_state, stack, 0).map(|to_be_echoed| {
            PipelineData::Stream(ValueStream::from_stream(
                to_be_echoed.into_iter(),
                engine_state.ctrlc.clone(),
            ))
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put a hello message in the pipeline",
                example: "echo 'hello'",
                result: Some(Value::List {
                    vals: vec![Value::test_string("hello")],
                    span: Span::new(0, 0),
                }),
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
