use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use rand::prelude::{thread_rng, Rng};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "random dice"
    }

    fn signature(&self) -> Signature {
        Signature::build("random dice")
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

    fn usage(&self) -> &str {
        "Generate a random dice roll"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
                example: "random dice -d 10 -s 12",
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
        Value::Int {
            val: thread_rng.gen_range(1..sides + 1) as i64,
            span,
        }
    });

    Ok(PipelineData::ListStream(
        ListStream::from_stream(iter, engine_state.ctrlc.clone()),
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
