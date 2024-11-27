use nu_engine::command_prelude::*;
use nu_protocol::ListStream;
use rand::prelude::{thread_rng, Rng};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random dice"
    }

    fn signature(&self) -> Signature {
        Signature::build("random dice")
            .input_output_types(vec![(Type::Nothing, Type::list(Type::Int))])
            .allow_variants_without_examples(true)
            .named(
                "dice",
                SyntaxShape::Int,
                "The amount of dice being rolled",
                Some('d'),
            )
            .named(
                "sides",
                SyntaxShape::Int,
                "The amount of sides a die has",
                Some('s'),
            )
            .category(Category::Random)
    }

    fn description(&self) -> &str {
        "Generate a random dice roll."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "die", "1-6"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        dice(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Roll 1 dice with 6 sides each",
                example: "random dice",
                result: None,
            },
            Example {
                description: "Roll 10 dice with 12 sides each",
                example: "random dice --dice 10 --sides 12",
                result: None,
            },
        ]
    }
}

fn dice(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let dice: usize = call.get_flag(engine_state, stack, "dice")?.unwrap_or(1);
    let sides: usize = call.get_flag(engine_state, stack, "sides")?.unwrap_or(6);

    let iter = (0..dice).map(move |_| {
        let mut thread_rng = thread_rng();
        Value::int(thread_rng.gen_range(1..sides + 1) as i64, span)
    });

    Ok(ListStream::new(iter, span, engine_state.signals().clone()).into())
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
