use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Collect;

impl Command for Collect {
    fn name(&self) -> &str {
        "collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("collect")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .optional(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run once the stream is collected.",
            )
            .switch(
                "keep-env",
                "Let the closure affect environment variables.",
                None,
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Collect a stream into a value."
    }

    fn extra_description(&self) -> &str {
        r#"If provided, run a closure with the collected value as input.

The entire stream will be collected into one value in memory, so if the stream
is particularly large, this can cause high memory usage."#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        eprintln!(
            "Tried to execute 'run' for the 'collect' command: this code path should never be reached in IR mode"
        );
        unreachable!();
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Use the second value in the stream.",
                example: "[1 2 3] | collect { |x| $x.1 }",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Read and write to the same file.",
                example: "open file.txt | collect | save -f file.txt",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Do;

    #[test]
    fn test_examples() {
        use crate::test_examples_with_commands;

        test_examples_with_commands(Collect {}, &[&Do {}])
    }
}
