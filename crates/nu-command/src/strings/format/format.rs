//use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Format;

impl Command for Format {
    fn name(&self) -> &str {
        "format"
    }

    fn signature(&self) -> Signature {
        Signature::build("format").required(
            "pattern",
            SyntaxShape::String,
            "the pattern to output. e.g.) \"{foo}: {bar}\"",
        )
    }

    fn usage(&self) -> &str {
        "Format columns into a string using a simple pattern."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        todo!()
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Print filenames with their sizes",
            example: "ls | format '{name}: {size}'",
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Format;
        use crate::test_examples;
        test_examples(Format {})
    }
}
