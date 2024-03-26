use nu_engine::command_prelude::*;

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
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .allow_variants_without_examples(true)
            .named("length", SyntaxShape::Int, "Number of chars", Some('l'))
            .category(Category::Random)
    }

    fn usage(&self) -> &str {
        "Generate random chars."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "character", "symbol", "alphanumeric"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
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
                example: "random chars --length 20",
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
        Value::string(random_string, span),
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
