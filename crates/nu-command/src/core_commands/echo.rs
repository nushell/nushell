use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct Echo;

impl Command for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn usage(&self) -> &str {
        "Returns its arguments, ignoring the piped-in value."
    }

    fn signature(&self) -> Signature {
        Signature::build("echo")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .rest("rest", SyntaxShape::Any, "the values to echo")
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"When given no arguments, it returns an empty string. When given one argument,
it returns it. Otherwise, it returns a list of the arguments. There is usually
little reason to use this over just writing the values as-is."#
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
                description: "Put a list of numbers in the pipeline. This is the same as [1 2 3].",
                example: "echo 1 2 3",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "Returns the piped-in value, by using the special $in variable to obtain it.",
                example: "echo $in",
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
