use nu_engine::command_prelude::*;
use nu_protocol::IoStream;

#[derive(Clone)]
pub struct Echo;

impl Command for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn usage(&self) -> &str {
        "Returns or prints its arguments, ignoring the piped-in value."
    }

    fn signature(&self) -> Signature {
        Signature::build("echo")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .rest("rest", SyntaxShape::Any, "The values to echo.")
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"When given no arguments, it returns an empty string. When given one argument,
it returns it. Otherwise, it returns a list of the arguments. There is usually
little reason to use this over just writing the values as-is.

Note that if there is no next command in the pipeline, then the values will be printed instead of passed to the next command.
"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut args = call.rest(engine_state, stack, 0)?;
        let value = match args.len() {
            0 => Value::string("", call.head),
            1 => args.pop().expect("one element"),
            _ => Value::list(args, call.head),
        };
        let value = value.into_pipeline_data();
        match stack.stdout() {
            IoStream::Pipe | IoStream::Capture => Ok(value),
            IoStream::Null => Ok(PipelineData::Empty),
            IoStream::Inherit => {
                value.print(engine_state, stack, false, false)?;
                Ok(PipelineData::Empty)
            }
            IoStream::File(_) => todo!(),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put a list of numbers in the pipeline. This is the same as [1 2 3].",
                example: "echo 1 2 3",
                result: Some(Value::list(
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                    Span::test_data(),
                )),
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
