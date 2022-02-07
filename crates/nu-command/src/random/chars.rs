use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};
use rand::{
    distributions::{Alphanumeric, Distribution},
    thread_rng,
};

const DEFAULT_CHARS_LENGTH: usize = 25;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("random chars")
            .named("length", SyntaxShape::Int, "Number of chars", Some('l'))
            .category(Category::Random)
    }

    fn usage(&self) -> &str {
        "Generate random chars"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        chars(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate random chars",
                example: "random chars",
                result: None,
            },
            Example {
                description: "Generate random chars with specified length",
                example: "random chars -l 20",
                result: None,
            },
        ]
    }
}

fn chars(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let length: Option<usize> = call.get_flag(engine_state, stack, "length")?;

    let chars_length = length.unwrap_or(DEFAULT_CHARS_LENGTH);
    let mut rng = thread_rng();

    let random_string = Alphanumeric
        .sample_iter(&mut rng)
        .take(chars_length)
        .map(char::from)
        .collect::<String>();

    Ok(PipelineData::Value(
        Value::String {
            val: random_string,
            span,
        },
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
